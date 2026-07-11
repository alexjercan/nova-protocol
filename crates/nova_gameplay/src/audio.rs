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
//! The four one-shots are **distance-attenuated**: their volume is scaled by how
//! far the event is from the listener (the camera carrying
//! [`SfxListenerMarker`], i.e. the gameplay camera), so a distant
//! explosion is quieter than one next to you. This is a volume-only rolloff for
//! the cinematic feel, not true spatialization - stereo panning would need bevy
//! spatial audio (`SpatialListener` + `spatial: true`) and is a future step. The
//! thruster hum is the player's own ship, so it is never attenuated.
//!
//! The [`SoundBank<NovaSfx>`] resource is inserted by `nova_assets` once assets
//! load; every system here degrades gracefully (does nothing) until it exists.

use std::collections::HashMap;

use bevy::{audio::Volume, prelude::*};

use crate::{prelude::*, sections::turret_section::TurretSectionPartOf};

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

/// Per-cue *base* playback volumes (at point-blank; distance attenuation scales
/// them down from here). The PDC fires ~100 rounds/s and impacts arrive in
/// blast-sized bursts, so those two are quiet; destruction and launch are the
/// punchy moments. Kept modest so nothing is harsh up close.
const TURRET_FIRE_VOLUME: f32 = 0.10;
const IMPACT_VOLUME: f32 = 0.22;
const EXPLOSION_VOLUME: f32 = 0.40;
const TORPEDO_LAUNCH_VOLUME: f32 = 0.45;

/// Distance-attenuation rolloff for positional cues, in world units. A cue plays
/// at full base volume within `SFX_NEAR_DISTANCE`, is inaudible beyond
/// `SFX_FAR_DISTANCE`, and rolls off between (see [`distance_attenuation`]). Tune
/// by ear for the scene scale (Nova ships are a few units across; combat happens
/// over dozens).
const SFX_NEAR_DISTANCE: f32 = 20.0;
const SFX_FAR_DISTANCE: f32 = 320.0;

/// Shape of the distance rolloff between NEAR and FAR. Loudness perception is
/// logarithmic, so a linear *amplitude* ramp sounds flat for most of the range
/// and then cliffs to silence near the end. Decaying the amplitude geometrically
/// toward this floor instead gives a roughly constant dB-per-distance falloff, so
/// the *perceived* volume fades evenly. Smaller floor = steeper decay / more
/// perceived range; 0.05 is about -26 dB at the far end (before the final remap
/// to true zero).
const SFX_ROLLOFF_FLOOR: f32 = 0.05;

/// Below this final (attenuated) linear volume a one-shot is not worth spawning -
/// it would be inaudible. Skipping it avoids audio-entity churn for far events.
const SFX_AUDIBLE_THRESHOLD: f32 = 0.01;

/// Minimum seconds between successive turret-fire and impact one-shots. Without
/// this the ~100/s PDC and the many-collider blast hits would each spawn a storm
/// of overlapping audio entities that reads as a wall of noise; collapsing them
/// to a bounded rate keeps the cue legible and the entity churn sane.
const TURRET_FIRE_MIN_INTERVAL: f32 = 0.05;
const IMPACT_MIN_INTERVAL: f32 = 0.04;

/// World-cell size (units) for grouping co-located area cues (impact, explosion).
/// A blast hitting many colliders of one ship, or a ship's sections all destroyed
/// at once, fall in the same cell and collapse to a single sound; events far
/// enough apart get their own. Small enough to keep distinct ships/impacts
/// separate. Turret fire is keyed by entity instead, so it does not use this.
const SFX_AREA_CELL: f32 = 6.0;

/// Drop throttle keys not touched within this many seconds, so the per-source map
/// stays bounded as ships move through new cells and turrets come and go.
const SFX_THROTTLE_PRUNE_WINDOW: f32 = 2.0;
/// A dying multi-section ship marks every section destroyed in the same frame;
/// this collapses that burst into a single explosion instead of N overlapping
/// ones (which would clip). Short enough that genuinely separate kills >60ms
/// apart still each sound.
const EXPLOSION_MIN_INTERVAL: f32 = 0.06;

/// Loudest the engine hum ever gets (at full thrust), on the linear scale.
const ENGINE_MAX_VOLUME: f32 = 0.3;

/// Per-source throttle key. Turret fire is keyed by the firing turret entity so
/// each gun sounds independently (even two guns on one ship); the area cues are
/// keyed by a quantized world cell so a co-located burst collapses to one sound
/// while distinct locations each sound. Keying globally (one timestamp per cue)
/// was the bug where a second gun firing in the same window was silenced.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ThrottleKey {
    TurretFire(Entity),
    Impact(IVec3),
    Explosion(IVec3),
}

/// Quantize a world position to a [`SFX_AREA_CELL`]-sized integer cell, so nearby
/// events share a key and far ones do not.
fn area_cell(pos: Vec3) -> IVec3 {
    (pos / SFX_AREA_CELL).floor().as_ivec3()
}

/// Last-played timestamp per throttle key, in seconds since startup. A key that
/// is absent has never played, so its first event always fires.
#[derive(Resource, Default)]
struct SfxThrottle {
    last: HashMap<ThrottleKey, f32>,
}

impl SfxThrottle {
    /// If `key` has not sounded within `min_interval` seconds, stamp it `now` and
    /// return true; otherwise false. Each key throttles independently.
    fn allow(&mut self, key: ThrottleKey, now: f32, min_interval: f32) -> bool {
        let last = self.last.entry(key).or_insert(f32::NEG_INFINITY);
        if now - *last >= min_interval {
            *last = now;
            true
        } else {
            false
        }
    }

    /// Drop keys idle for longer than `window` seconds so the map stays bounded.
    fn prune(&mut self, now: f32, window: f32) {
        self.last.retain(|_, &mut last| now - last < window);
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

/// Distance rolloff in [0, 1]: full within [`SFX_NEAR_DISTANCE`], zero beyond
/// [`SFX_FAR_DISTANCE`]. Between them the amplitude decays *geometrically* toward
/// [`SFX_ROLLOFF_FLOOR`] (constant dB per distance), not linearly, so the
/// perceived loudness fades evenly instead of staying flat and then cliffing -
/// the fix for "same volume then instantly zero". The geometric curve is remapped
/// from `[floor, 1]` back to `[0, 1]` so it still reaches exactly zero at FAR.
/// Pure for unit testing.
fn distance_attenuation(distance: f32) -> f32 {
    if distance <= SFX_NEAR_DISTANCE {
        1.0
    } else if distance >= SFX_FAR_DISTANCE {
        0.0
    } else {
        let t = (distance - SFX_NEAR_DISTANCE) / (SFX_FAR_DISTANCE - SFX_NEAR_DISTANCE);
        let decayed = SFX_ROLLOFF_FLOOR.powf(t);
        (decayed - SFX_ROLLOFF_FLOOR) / (1.0 - SFX_ROLLOFF_FLOOR)
    }
}

/// Play a positional one-shot: scale `base_volume` by the distance attenuation
/// from `listener` to `source`, and skip entirely when the result is inaudible.
/// A missing listener (no camera yet) falls back to full base volume rather than
/// silence.
fn play_positional(
    commands: &mut Commands,
    bank: &SoundBank<NovaSfx>,
    key: NovaSfx,
    base_volume: f32,
    source: Vec3,
    listener: Option<Vec3>,
) {
    let attenuation = listener.map_or(1.0, |l| distance_attenuation(l.distance(source)));
    let volume = base_volume * attenuation;
    if volume < SFX_AUDIBLE_THRESHOLD {
        return;
    }
    commands.play_sfx_volume(bank.get(key), volume);
}

/// Marks the camera that acts as the SFX/juice listener: distance attenuation
/// for the one-shot cues, camera-shake trauma, and the flash-ring facing all key
/// off this entity. Exactly one camera should carry it at a time - the gameplay
/// (scenario) camera, tagged where it is spawned. "First `Camera3d`" was the old
/// signal, but ECS query order is unspecified, so a second camera (minimap,
/// render-to-texture, a leftover editor camera) could flip the listener frame to
/// frame; the explicit marker makes it stable. The editor camera deliberately
/// does not carry it: no gameplay cues fire in the editor, and the shake
/// component should never attach there.
///
/// (Checked at introduction time: the editor -> scenario transition never has
/// two `Camera3d` alive at once - the editor camera is `DespawnOnExit(Editor)`,
/// applied before `OnEnter(Scenario)` spawns the scenario camera - so the old
/// assumption was latent, not a live bug.)
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SfxListenerMarker;

/// The listener position for distance attenuation: the marked gameplay camera's
/// world translation, or `None` if no listener exists yet (early startup, or the
/// editor).
fn listener_position(q_camera: &Query<&GlobalTransform, With<SfxListenerMarker>>) -> Option<Vec3> {
    q_camera.iter().next().map(|t| t.translation())
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
        app.register_type::<SfxListenerMarker>();

        // Audio sinks do not follow Time<Virtual>: without this the thruster
        // hum keeps roaring at its last volume behind the pause overlay
        // (review R1.5).
        app.add_systems(OnEnter(crate::PauseStates::Paused), pause_thruster_loops);
        app.add_systems(OnExit(crate::PauseStates::Paused), resume_thruster_loops);

        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(on_damage_play_impact);
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(on_torpedo_launch_play_sfx);

        // The thruster hum polls `ThrusterSectionInput`, so it must be gated to
        // the running simulation exactly like the thruster physics/shader. Joining
        // `SpaceshipSectionSystems` inherits whatever run condition consumers of
        // that input use - crucially nova_scenario's `run_if(scenario_is_live)`
        // - so the hum stays silent while building in the editor (no scenario is
        // loaded there) and plays wherever one is live, the main menu's ambience
        // backdrop included. (The one-shot cues need no gating: they fire on
        // spawn/damage/destroy events that only occur inside this same gated set.)
        app.add_systems(
            Update,
            (ensure_thruster_loop, update_thruster_loop_volume)
                .chain()
                .in_set(SpaceshipSectionSystems),
        );

        // Pure map cleanup; harmless to run always and keeps memory bounded.
        app.add_systems(Update, prune_sfx_throttle);
    }
}

/// Keep the per-source throttle map bounded by dropping idle keys.
fn prune_sfx_throttle(time: Res<Time>, mut throttle_state: ResMut<SfxThrottle>) {
    throttle_state.prune(time.elapsed_secs(), SFX_THROTTLE_PRUNE_WINDOW);
}

/// Explosion cue on any destruction (section, asteroid, or torpedo detonation,
/// which all funnel through `IntegrityDestroyMarker`).
fn on_destroyed_play_explosion(
    add: On<Add, IntegrityDestroyMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    // The destroyed entity has existed for frames, so its GlobalTransform is
    // valid world-space.
    let Ok(source) = q_transform.get(add.entity) else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Explosion(area_cell(pos)),
        time.elapsed_secs(),
        EXPLOSION_MIN_INTERVAL,
    ) {
        play_positional(
            &mut commands,
            &bank,
            NovaSfx::Explosion,
            EXPLOSION_VOLUME,
            pos,
            listener_position(&q_camera),
        );
    }
}

/// Impact cue whenever damage is applied. Throttled because a single blast deals
/// damage to many colliders in one frame.
///
/// Propagation caveat: `HealthApplyDamage` auto-propagates up `ChildOf`
/// (section -> ship root), and ship death depends on that bubbling, so it must
/// not be stopped here - but a global observer fires once per hop, which would
/// double the cue whenever the section and root land in different area cells.
/// Reacting only to the original target keeps one hit = one cue, and the
/// original target is also the better cue position: the actual hit location,
/// not the ship root's origin. Any future damage-cue observer needs this same
/// guard.
fn on_damage_play_impact(
    damage: On<HealthApplyDamage>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Some(bank) = bank else { return };
    let Ok(source) = q_transform.get(damage.entity) else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Impact(area_cell(pos)),
        time.elapsed_secs(),
        IMPACT_MIN_INTERVAL,
    ) {
        play_positional(
            &mut commands,
            &bank,
            NovaSfx::Impact,
            IMPACT_VOLUME,
            pos,
            listener_position(&q_camera),
        );
    }
}

/// Turret-fire cue when a round spawns. Throttled hard because the PDC fires at
/// a high rate.
fn on_turret_fire_play_sfx(
    add: On<Add, TurretBulletProjectileMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    time: Res<Time>,
    q_projectile: Query<(&Transform, &TurretSectionPartOf)>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    // The projectile is a freshly-spawned ROOT entity, so its GlobalTransform is
    // still identity this frame; its local Transform is already world-space.
    // `TurretSectionPartOf` names the firing turret, so each gun throttles on its
    // own key - the fix for "only one of several guns is audible".
    let Ok((transform, part_of)) = q_projectile.get(add.entity) else {
        return;
    };
    if throttle_state.allow(
        ThrottleKey::TurretFire(part_of.0),
        time.elapsed_secs(),
        TURRET_FIRE_MIN_INTERVAL,
    ) {
        play_positional(
            &mut commands,
            &bank,
            NovaSfx::TurretFire,
            TURRET_FIRE_VOLUME,
            transform.translation,
            listener_position(&q_camera),
        );
    }
}

/// Launch cue when a torpedo projectile spawns.
fn on_torpedo_launch_play_sfx(
    add: On<Add, TorpedoProjectileMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    q_transform: Query<&Transform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut commands: Commands,
) {
    let Some(bank) = bank else { return };
    // Freshly-spawned root entity: use local Transform (== world) this frame.
    let Ok(source) = q_transform.get(add.entity) else {
        return;
    };
    play_positional(
        &mut commands,
        &bank,
        NovaSfx::TorpedoLaunch,
        TORPEDO_LAUNCH_VOLUME,
        source.translation,
        listener_position(&q_camera),
    );
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
    fn throttle_blocks_one_key_until_the_interval_elapses() {
        let key = ThrottleKey::Explosion(IVec3::ZERO);
        let mut state = SfxThrottle::default();
        // First event of a key always fires (absent -> NEG_INFINITY).
        assert!(state.allow(key, 0.0, 0.05));
        // Too soon: blocked.
        assert!(!state.allow(key, 0.03, 0.05));
        // Exactly at the interval: fires again.
        assert!(state.allow(key, 0.05, 0.05));
        // Immediately after: blocked.
        assert!(!state.allow(key, 0.06, 0.05));
    }

    #[test]
    fn throttle_is_independent_per_key() {
        // The bug fix: two distinct sources firing in the same instant both play.
        let mut world = World::new();
        let gun_a = ThrottleKey::TurretFire(world.spawn_empty().id());
        let gun_b = ThrottleKey::TurretFire(world.spawn_empty().id());
        let mut state = SfxThrottle::default();
        assert!(state.allow(gun_a, 0.0, 0.05));
        assert!(
            state.allow(gun_b, 0.0, 0.05),
            "a second gun must not be silenced"
        );
        // Same gun again in the same window is still throttled.
        assert!(!state.allow(gun_a, 0.0, 0.05));
        // Different cue kinds at the same cell are independent too.
        assert!(state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04));
        assert!(state.allow(ThrottleKey::Explosion(IVec3::ZERO), 0.0, 0.06));
    }

    #[test]
    fn prune_drops_only_idle_keys() {
        let mut state = SfxThrottle::default();
        state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04); // last = 0.0
        state.allow(ThrottleKey::Impact(IVec3::ONE), 9.5, 0.04); // last = 9.5
        state.prune(10.0, 2.0); // window 2s at now=10 -> keep >8.0
        assert_eq!(state.last.len(), 1);
        assert!(state.last.contains_key(&ThrottleKey::Impact(IVec3::ONE)));
    }

    #[test]
    fn area_cell_groups_nearby_and_separates_distant() {
        // Points within one cell share a key; points cells apart do not.
        assert_eq!(
            area_cell(Vec3::ZERO),
            area_cell(Vec3::splat(SFX_AREA_CELL * 0.5))
        );
        assert_ne!(
            area_cell(Vec3::ZERO),
            area_cell(Vec3::splat(SFX_AREA_CELL * 1.5))
        );
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
    fn distance_attenuation_rolls_off_between_near_and_far() {
        // Full within the near radius (including at exactly near).
        assert_eq!(distance_attenuation(0.0), 1.0);
        assert_eq!(distance_attenuation(SFX_NEAR_DISTANCE), 1.0);
        // Silent at/beyond the far radius (endpoints are clean 1 and 0).
        assert_eq!(distance_attenuation(SFX_FAR_DISTANCE), 0.0);
        assert_eq!(distance_attenuation(SFX_FAR_DISTANCE + 100.0), 0.0);

        // Monotonic decreasing in the rolloff band.
        let mid = (SFX_NEAR_DISTANCE + SFX_FAR_DISTANCE) / 2.0;
        let a = distance_attenuation(SFX_NEAR_DISTANCE + 10.0);
        let m = distance_attenuation(mid);
        let b = distance_attenuation(SFX_FAR_DISTANCE - 10.0);
        assert!(a > m && m > b, "attenuation should decrease with distance");

        // Convex/perceptual: the geometric curve sits *below* the old linear line
        // (which would be 0.5 at the midpoint), so loudness is already clearly
        // reduced by the middle distances instead of staying flat then cliffing.
        assert!(
            m < 0.5,
            "midpoint should be well below the linear 0.5, got {m}"
        );
        // Values stay in range.
        for d in [30.0, 100.0, 200.0, 300.0] {
            let v = distance_attenuation(d);
            assert!(
                (0.0..=1.0).contains(&v),
                "attenuation out of range at {d}: {v}"
            );
        }
    }

    #[test]
    fn listener_position_uses_the_marked_camera_not_any_camera3d() {
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        // An unmarked Camera3d must not be the listener...
        world.spawn((
            Camera3d::default(),
            GlobalTransform::from(Transform::from_translation(Vec3::new(5.0, 0.0, 0.0))),
        ));
        let mut state: SystemState<Query<&GlobalTransform, With<SfxListenerMarker>>> =
            SystemState::new(&mut world);
        assert_eq!(
            listener_position(&state.get(&world).unwrap()),
            None,
            "no marked listener -> None (graceful full-volume fallback)"
        );

        // ...only the camera carrying the marker is.
        let pos = Vec3::new(0.0, 3.0, -7.0);
        world.spawn((
            Camera3d::default(),
            SfxListenerMarker,
            GlobalTransform::from(Transform::from_translation(pos)),
        ));
        assert_eq!(listener_position(&state.get(&world).unwrap()), Some(pos));
    }

    /// Count of `PlaySfx` triggers observed, standing in for "sounds played".
    #[derive(Resource, Default)]
    struct PlayedSfx(usize);

    #[test]
    fn a_propagated_hit_on_a_straddling_hierarchy_plays_one_impact() {
        // The audio side of the PR #54 regression (and PR #53 F3):
        // `HealthApplyDamage` auto-propagates child -> parent, and with the
        // parent one area cell away the per-cell throttle cannot collapse the
        // hops, so one hit played two impact sounds. The original-target guard
        // must keep it at exactly one.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<PlayedSfx>();
        let bank = SoundBank::load(app.world().resource::<AssetServer>(), NOVA_SFX_FILES);
        app.insert_resource(bank);
        app.add_observer(on_damage_play_impact);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);

        let parent = app
            .world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(
                Vec3::new(SFX_AREA_CELL * 4.0, 0.0, 0.0),
            )))
            .id();
        let child = app
            .world_mut()
            .spawn((GlobalTransform::default(), ChildOf(parent)))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: child,
            source: None,
            amount: 10.0,
        });
        // The observer plays via `Commands`, so the queued `PlaySfx` triggers
        // only fire on the next flush.
        app.world_mut().flush();

        assert_eq!(
            app.world().resource::<PlayedSfx>().0,
            1,
            "one hit must play exactly one impact sound"
        );
        // The cue is keyed (and positioned) at the hit location's cell, not the
        // parent's.
        let throttle = app.world().resource::<SfxThrottle>();
        assert!(throttle
            .last
            .contains_key(&ThrottleKey::Impact(area_cell(Vec3::ZERO))));
        assert_eq!(throttle.last.len(), 1);
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

/// Silence the engine loop while the pause overlay is up; one-shot SFX are
/// naturally quiet then (no events fire in a frozen sim).
fn pause_thruster_loops(q_sink: Query<&AudioSink, With<ThrusterLoopSfx>>) {
    for sink in &q_sink {
        sink.pause();
    }
}

fn resume_thruster_loops(q_sink: Query<&AudioSink, With<ThrusterLoopSfx>>) {
    for sink in &q_sink {
        sink.play();
    }
}
