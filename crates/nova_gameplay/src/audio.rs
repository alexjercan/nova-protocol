//! Nova's audio wiring: map gameplay events to placeholder sound effects.
//!
//! The generic playback machinery lives in `bevy_common_systems`: [`SfxPlugin`]
//! spawns a self-despawning audio entity for every [`PlaySfx`], and
//! [`SoundBank`] is a keyed registry of loaded handles. This module owns only
//! the *game-specific* part - the mapping from Nova gameplay events to sounds -
//! so the reusable half stays promotable and this half stays Nova's.
//!
//! Four cues are one-shots fired from existing seams via observers, so no
//! gameplay system has to know about audio:
//! - a section/asteroid destroyed or a torpedo detonating -> `Explosion`
//!   (`On<Add, IntegrityDestroyMarker>`);
//! - damage applied to a target -> `Impact` (`On<HealthApplyDamage>`);
//! - a turret round spawned -> `TurretFire` (`On<Add, TurretBulletProjectileMarker>`);
//! - a torpedo spawned -> `TorpedoLaunch` (`On<Add, TorpedoProjectileMarker>`).
//!
//! The fifth cue, the thruster engine hum, is continuous: one looping audio
//! entity whose volume tracks how hard the ship is thrusting.
//!
//! The [`SoundBank<NovaSfx>`] resource is inserted by `nova_assets` once assets
//! load; every system here degrades gracefully (does nothing) until it exists.

use bevy::{audio::Volume, prelude::*};

use crate::prelude::*;

/// Keys for Nova's sound effects, naming each cue the game plays. Used as the
/// [`SoundBank`] key so call sites read `bank.get(NovaSfx::Explosion)`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NovaSfx {
    /// Continuous engine hum, looped; volume tracks thruster input.
    ThrusterLoop,
    /// A PDC/turret round is fired.
    TurretFire,
    /// A torpedo leaves its bay.
    TorpedoLaunch,
    /// A section/asteroid is destroyed or a torpedo detonates.
    Explosion,
    /// Damage is applied to a target.
    Impact,
}

/// The `(key, base-filename)` pairs Nova loads into its [`SoundBank`]. The bank
/// applies the `sounds/<name>.wav` convention, so these map to
/// `assets/sounds/<name>.wav`. Shared with `nova_assets`, which does the load.
pub const NOVA_SFX_FILES: [(NovaSfx, &str); 5] = [
    (NovaSfx::ThrusterLoop, "thruster_loop"),
    (NovaSfx::TurretFire, "turret_fire"),
    (NovaSfx::TorpedoLaunch, "torpedo_launch"),
    (NovaSfx::Explosion, "explosion"),
    (NovaSfx::Impact, "impact"),
];

/// Per-cue playback volumes. The PDC fires ~100 rounds/s and impacts arrive in
/// blast-sized bursts, so those two are quiet; destruction and launch are the
/// punchy moments.
const TURRET_FIRE_VOLUME: f32 = 0.15;
const IMPACT_VOLUME: f32 = 0.35;
const EXPLOSION_VOLUME: f32 = 0.6;
const TORPEDO_LAUNCH_VOLUME: f32 = 0.7;

/// Minimum seconds between successive turret-fire and impact one-shots. Without
/// this the ~100/s PDC and the many-collider blast hits would each spawn a storm
/// of overlapping audio entities that reads as a wall of noise; collapsing them
/// to a bounded rate keeps the cue legible and the entity churn sane.
const TURRET_FIRE_MIN_INTERVAL: f32 = 0.05;
const IMPACT_MIN_INTERVAL: f32 = 0.04;
/// A dying multi-section ship marks every section destroyed in the same frame;
/// this collapses that burst into a single explosion instead of N overlapping
/// ones (which would clip). Short enough that genuinely separate kills >60ms
/// apart still each sound.
const EXPLOSION_MIN_INTERVAL: f32 = 0.06;

/// Loudest the engine hum ever gets (at full thrust), on the linear scale.
const ENGINE_MAX_VOLUME: f32 = 0.4;

/// Last-played timestamps for the throttled cues, in seconds since startup.
/// Both start at negative infinity so the first event of each kind always fires,
/// even at `t == 0`, rather than being swallowed by the initial interval.
#[derive(Resource)]
struct SfxThrottle {
    last_turret_fire: f32,
    last_impact: f32,
    last_explosion: f32,
}

impl Default for SfxThrottle {
    fn default() -> Self {
        Self {
            last_turret_fire: f32::NEG_INFINITY,
            last_impact: f32::NEG_INFINITY,
            last_explosion: f32::NEG_INFINITY,
        }
    }
}

/// Whether `now` is at least `min_interval` seconds after `last`; if so, advance
/// `last` to `now` and return true. Pure so it can be unit-tested without audio.
fn throttle(last: &mut f32, now: f32, min_interval: f32) -> bool {
    if now - *last >= min_interval {
        *last = now;
        true
    } else {
        false
    }
}

/// Map an average per-thruster throttle (0..1) to a linear engine-hum volume:
/// silent at rest, scaling linearly to [`ENGINE_MAX_VOLUME`] at full throttle.
/// The caller averages over the active thrusters rather than summing, so the hum
/// tracks how hard the ship is burning instead of pinning to max the moment more
/// than one thruster fires. The clamp guards out-of-range input. Pure for tests.
fn engine_volume(avg_throttle: f32) -> f32 {
    avg_throttle.clamp(0.0, 1.0) * ENGINE_MAX_VOLUME
}

/// Plugin wiring Nova's gameplay events to sound effects. Adds the reusable
/// [`SfxPlugin`] and Nova's own observers + the thruster-loop systems.
#[derive(Default)]
pub struct NovaAudioPlugin;

impl Plugin for NovaAudioPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaAudioPlugin: build");

        // Generic fire-and-forget SFX playback (PlaySfx / SoundBank live here).
        if !app.is_plugin_added::<SfxPlugin>() {
            app.add_plugins(SfxPlugin);
        }

        app.init_resource::<SfxThrottle>();

        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(on_damage_play_impact);
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(on_torpedo_launch_play_sfx);

        app.add_systems(
            Update,
            (ensure_thruster_loop, update_thruster_loop_volume).chain(),
        );
    }
}

/// Explosion cue on any destruction (section, asteroid, or torpedo detonation,
/// which all funnel through `IntegrityDestroyMarker`).
fn on_destroyed_play_explosion(
    _add: On<Add, IntegrityDestroyMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    time: Res<Time>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    if throttle(
        &mut throttle_state.last_explosion,
        time.elapsed_secs(),
        EXPLOSION_MIN_INTERVAL,
    ) {
        commands.play_sfx_volume(bank.get(NovaSfx::Explosion), EXPLOSION_VOLUME);
    }
}

/// Impact cue whenever damage is applied. Throttled because a single blast deals
/// damage to many colliders in one frame.
fn on_damage_play_impact(
    _damage: On<HealthApplyDamage>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    time: Res<Time>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    if throttle(
        &mut throttle_state.last_impact,
        time.elapsed_secs(),
        IMPACT_MIN_INTERVAL,
    ) {
        commands.play_sfx_volume(bank.get(NovaSfx::Impact), IMPACT_VOLUME);
    }
}

/// Turret-fire cue when a round spawns. Throttled hard because the PDC fires at
/// a high rate.
fn on_turret_fire_play_sfx(
    _add: On<Add, TurretBulletProjectileMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    time: Res<Time>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    if throttle(
        &mut throttle_state.last_turret_fire,
        time.elapsed_secs(),
        TURRET_FIRE_MIN_INTERVAL,
    ) {
        commands.play_sfx_volume(bank.get(NovaSfx::TurretFire), TURRET_FIRE_VOLUME);
    }
}

/// Launch cue when a torpedo projectile spawns.
fn on_torpedo_launch_play_sfx(
    _add: On<Add, TorpedoProjectileMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    commands.play_sfx_volume(bank.get(NovaSfx::TorpedoLaunch), TORPEDO_LAUNCH_VOLUME);
}

/// Marker for the single looping engine-hum audio entity.
#[derive(Component)]
struct ThrusterLoopSfx;

/// Spawn the looping engine-hum entity once, after the sound bank exists. It
/// starts silent; [`update_thruster_loop_volume`] raises its volume with thrust.
/// `PlaybackSettings::LOOP` keeps it playing for the whole session.
fn ensure_thruster_loop(
    bank: Option<Res<SoundBank<NovaSfx>>>,
    existing: Query<(), With<ThrusterLoopSfx>>,
    mut commands: Commands,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(bank) = bank else { return };

    commands.spawn((
        Name::new("Thruster Loop Sfx"),
        ThrusterLoopSfx,
        AudioPlayer(bank.get(NovaSfx::ThrusterLoop)),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
    ));
}

/// Drive the engine-hum volume from how hard the ship is thrusting, smoothing
/// toward the target so throttle changes fade rather than click.
///
/// The throttle is averaged over every active thruster in the world (a single
/// hum for "the ship is burning"); per-ship attribution would need to relate
/// each thruster to the player root and is left for when there is more than one
/// audible ship. The `AudioSink` appears a frame or two after the entity spawns,
/// so this no-ops until it is present.
fn update_thruster_loop_volume(
    time: Res<Time>,
    q_thrusters: Query<
        &ThrusterSectionInput,
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_sink: Query<&mut AudioSink, With<ThrusterLoopSfx>>,
    mut smoothed: Local<f32>,
) {
    let Ok(mut sink) = q_sink.single_mut() else {
        return;
    };

    // Average the throttle over the active thrusters so the hum tracks how hard
    // the ship is burning; summing would pin it to max the moment more than one
    // thruster fires (each input is a 0..1 throttle).
    let (sum, count) = q_thrusters
        .iter()
        .fold((0.0f32, 0u32), |(sum, count), input| {
            (sum + input.0.abs(), count + 1)
        });
    let avg_throttle = if count > 0 { sum / count as f32 } else { 0.0 };
    let target = engine_volume(avg_throttle);

    // Exponential smoothing, framerate-independent: ~8 units/s of catch-up.
    let alpha = (time.delta_secs() * 8.0).clamp(0.0, 1.0);
    *smoothed += (target - *smoothed) * alpha;

    sink.set_volume(Volume::Linear(*smoothed));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_blocks_until_the_interval_elapses() {
        // Real usage seeds `last` at NEG_INFINITY (see SfxThrottle::default) so the
        // first event always fires, even at t=0.
        let mut last = f32::NEG_INFINITY;
        assert!(throttle(&mut last, 0.0, 0.05));
        assert_eq!(last, 0.0);
        // Too soon: blocked, last unchanged.
        assert!(!throttle(&mut last, 0.03, 0.05));
        assert_eq!(last, 0.0);
        // Exactly at the interval: fires and advances.
        assert!(throttle(&mut last, 0.05, 0.05));
        assert_eq!(last, 0.05);
        // Immediately after: blocked again.
        assert!(!throttle(&mut last, 0.06, 0.05));
    }

    #[test]
    fn sfx_throttle_default_lets_the_first_event_fire() {
        let mut state = SfxThrottle::default();
        assert!(throttle(
            &mut state.last_turret_fire,
            0.0,
            TURRET_FIRE_MIN_INTERVAL
        ));
        assert!(throttle(&mut state.last_impact, 0.0, IMPACT_MIN_INTERVAL));
        assert!(throttle(
            &mut state.last_explosion,
            0.0,
            EXPLOSION_MIN_INTERVAL
        ));
    }

    #[test]
    fn engine_volume_is_silent_at_rest_and_saturates_at_full_thrust() {
        assert_eq!(engine_volume(0.0), 0.0);
        assert_eq!(engine_volume(1.0), ENGINE_MAX_VOLUME);
        // Multiple thrusters cannot push past the ceiling.
        assert_eq!(engine_volume(3.5), ENGINE_MAX_VOLUME);
        // Partial thrust scales linearly.
        assert!((engine_volume(0.5) - ENGINE_MAX_VOLUME * 0.5).abs() < f32::EPSILON);
        // Negative input (reverse) is treated by magnitude at the call site, but
        // guard the clamp here too.
        assert_eq!(engine_volume(-1.0), 0.0);
    }

    #[test]
    fn every_nova_sfx_key_has_a_file() {
        // Guards against adding a NovaSfx variant without a placeholder asset.
        use NovaSfx::*;
        for key in [ThrusterLoop, TurretFire, TorpedoLaunch, Explosion, Impact] {
            assert!(
                NOVA_SFX_FILES.iter().any(|(k, _)| *k == key),
                "NovaSfx::{key:?} is missing from NOVA_SFX_FILES"
            );
        }
    }
}
