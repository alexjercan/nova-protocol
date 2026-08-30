//! Nova's combat juice: moment-to-moment feedback when a shot lands or a target
//! dies. Two effects (camera shake and a spark burst), both driven off the same
//! existing seams the audio layer uses so no gameplay system has to know about
//! them:
//!
//! - damage applied to a target -> a small camera-shake kick + a spark burst
//!   (`On<HealthApplyDamage>`);
//! - a section/asteroid destroyed or a torpedo detonating -> a big camera-shake
//!   kick + a bigger burst (`On<Add, IntegrityDestroyMarker>`).
//!
//! **Camera shake** reuses the generic trauma model from [`shake`](crate::shake)
//! ([`CameraShakePlugin`]): it is drift-free (offset is un-applied and re-applied
//! around the base-writing driver) and already orders itself around the chase
//! camera. This module only *feeds* it trauma; `ensure_camera_shake` attaches a
//! [`CameraShake`] (configured from [`JuiceSettings`]) to the gameplay camera.
//!
//! **The impact burst** is an [`ImpactSparks`] request, which
//! [`impact_spark`](crate::impact_spark) answers with a handful of short-lived
//! incandescent streaks. What used to stand here was an expanding camera-facing
//! gizmo ring, drawn immediate-mode for zero entity churn - cheap, and the most
//! seen and least believable effect in a fight. A ring is a diagram of a hit.
//! Sparks are the hit.
//!
//! This module decides WHETHER a burst happens and how big; the spark module
//! decides what one looks like. Everything below the split - the throttle, the
//! distance falloff, the settings toggle - is inherited by whatever replaces
//! the look.
//!
//! Both effects are **distance-attenuated** from the gameplay camera (the one
//! carrying `SfxListenerMarker`, shared with the audio layer; the trauma
//! impulse and the spark count both scale with the falloff) and
//! **per-area-cell throttled**, mirroring `audio/`: a blast that damages a
//! dozen colliders of one ship in a single frame collapses to one kick and one
//! burst, and a distant event kicks weaker and throws fewer sparks than one in
//! your face. Every tunable
//! lives on the [`JuiceSettings`] resource (with per-effect enable toggles and a
//! master switch) so a settings menu can bind to it later. All the math a headless
//! run cannot exercise (the rendering) is pushed into pure helpers that are
//! unit-tested.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::prelude::*;

/// The shake and spark settings, `JuiceSettings` and `NovaJuicePlugin`.
pub mod prelude {
    pub use super::{JuiceSettings, NovaJuicePlugin, ShakeSettings, SparkSettings};
}

/// World-cell size (units) for grouping co-located juice events, matching the
/// audio layer's `SFX_AREA_CELL`. A blast hitting many colliders of one ship, or a
/// ship's sections all destroyed at once, fall in the same cell and collapse to a
/// single kick/burst; events far enough apart get their own.
const JUICE_AREA_CELL: f32 = 6.0;

/// Minimum seconds between successive impact / destruction juice events per cell.
/// Without this a single blast's many-collider damage burst would stack a dozen
/// identical kicks (saturating trauma instantly) and a dozen overlapping bursts.
/// A dying multi-section ship marks every section in one frame, so destruction is
/// throttled too, just a touch looser so genuinely separate kills still each read.
const IMPACT_MIN_INTERVAL: f32 = 0.04;
const DESTROY_MIN_INTERVAL: f32 = 0.06;

/// Drop throttle keys not touched within this many seconds, so the per-cell map
/// stays bounded as combat moves through new cells (mirrors the audio throttle).
const JUICE_THROTTLE_PRUNE_WINDOW: f32 = 2.0;

/// Which kind of juice event this is, selecting its trauma from
/// [`ShakeSettings`] and its spark count from [`SparkSettings`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum JuiceEventKind {
    /// Damage landed on a still-living target.
    Impact,
    /// A target was destroyed / a torpedo detonated.
    Destroy,
}

/// Tunables for the trauma-driven camera shake. Fed into a [`CameraShake`] on the
/// gameplay camera; `hit_trauma`/`destroy_trauma` are the per-event impulses.
#[derive(Clone, Debug, Reflect)]
pub struct ShakeSettings {
    /// Master toggle for camera shake.
    pub enabled: bool,
    /// Trauma added by one (attenuated) damage event.
    pub hit_trauma: f32,
    /// Trauma added by one (attenuated) destruction event.
    pub destroy_trauma: f32,
    /// Trauma decay per second (passed to [`CameraShake::decay`]).
    pub decay: f32,
    /// Peak positional offset at full trauma, world units ([`CameraShake::max_offset`]).
    pub max_offset: Vec3,
    /// Peak rotational kick at full trauma, radians ([`CameraShake::max_kick`]).
    pub max_kick: Vec3,
    /// Trauma->amount exponent ([`CameraShake::exponent`]); 2.0 is the classic value.
    pub exponent: f32,
}

impl Default for ShakeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            // Kept deliberately subtle: a single PDC round is barely a flicker, and
            // even a close detonation is a short bump, not a screen-thrower. These
            // are the *point-blank* impulses; distance attenuation (see
            // `JuiceSettings::near_distance`/`far_distance`) scales them down fast,
            // so anything more than a few units away is gentler still.
            //
            // Softened from the first tune, which read as thrashing under
            // sustained fire (owner: "feels really bad"). The offset/kick are
            // re-sampled EVERY frame, so aggressiveness is frame-rate noise,
            // not swing - the kick (rotation) reads harshest and is cut most.
            // DEFAULTS only; the settings menu edits the live resource.
            // Old -> new:
            //   hit_trauma     0.08 -> 0.05   (sustained fire saturates slower)
            //   destroy_trauma 0.24 -> 0.16   (a kill still out-kicks a hit 3:1)
            //   decay          2.4  -> 3.2    (a burst settles in ~0.3 s, not 0.4)
            //   max_offset     (0.18, 0.18, 0.10)   -> (0.10, 0.10, 0.06)
            //   max_kick       (0.008, 0.008, 0.012) -> (0.003, 0.003, 0.0045)
            //   exponent       2.0 unchanged (the classic curve already crushes
            //                  residual trauma; raising it would erase single hits)
            hit_trauma: 0.05,
            destroy_trauma: 0.16,
            decay: 3.2,
            max_offset: Vec3::new(0.10, 0.10, 0.06),
            max_kick: Vec3::new(0.003, 0.003, 0.0045),
            exponent: 2.0,
        }
    }
}

/// Tunables for the impact spark burst.
///
/// Counts and nothing else: how HOT a spark is, how fast it leaves and how long
/// it lives are properties of incandescent metal, not preferences, and they
/// live with the look in [`impact_spark`](crate::impact_spark). What a player
/// or a graphics tier wants to turn down is density.
#[derive(Clone, Debug, Reflect)]
pub struct SparkSettings {
    /// Master toggle for the spark burst.
    pub enabled: bool,
    /// Sparks thrown by a point-blank hit on a living target.
    pub impact_count: u32,
    /// Sparks thrown by a point-blank destruction.
    pub destroy_count: u32,
}

impl Default for SparkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            // A kill throws roughly four times a hit. The ratio matters more
            // than either number: under sustained PDC fire the impact burst is
            // on screen constantly, so it has to stay small enough that a kill
            // still reads as a different event.
            impact_count: 5,
            destroy_count: 20,
        }
    }
}

impl SparkSettings {
    /// Point-blank spark count for an event of `kind`.
    fn count(&self, kind: JuiceEventKind) -> u32 {
        match kind {
            JuiceEventKind::Impact => self.impact_count,
            JuiceEventKind::Destroy => self.destroy_count,
        }
    }
}

/// All combat-juice tunables in one resource, so a future settings menu can edit a
/// single reflected struct. Systems read it every frame; changes to the shake
/// fields are pushed onto the live [`CameraShake`] by `sync_camera_shake_config`.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct JuiceSettings {
    /// Kill switch for all juice at once (a settings-menu "reduce motion" toggle).
    pub master_enabled: bool,
    /// Camera-shake tunables.
    pub shake: ShakeSettings,
    /// Spark-burst tunables.
    pub sparks: SparkSettings,
    /// A juice event at or nearer than this to the camera fires at full strength.
    pub near_distance: f32,
    /// A juice event at or beyond this is fully attenuated (no kick, no sparks).
    pub far_distance: f32,
}

impl Default for JuiceSettings {
    fn default() -> Self {
        Self {
            master_enabled: true,
            shake: ShakeSettings::default(),
            sparks: SparkSettings::default(),
            // Only a near, in-your-face event shakes at full strength; the camera
            // chases the player from ~20 units back, so `near_distance` is kept
            // tight (roughly the ship's own length) and `far_distance` well inside
            // the audio range, making the shake fall off with distance noticeably
            // faster than the sound does - a detonation across the arena is a faint
            // tremor, one on your hull is a real bump.
            near_distance: 8.0,
            far_distance: 200.0,
        }
    }
}

impl JuiceSettings {
    /// Whether the shake effect should run (master + per-effect toggle).
    fn shake_on(&self) -> bool {
        self.master_enabled && self.shake.enabled
    }

    /// Whether the spark burst should run (master + per-effect toggle).
    fn sparks_on(&self) -> bool {
        self.master_enabled && self.sparks.enabled
    }
}

/// Per-throttle-key last-fired timestamp, keyed by event kind and world cell, so a
/// co-located burst collapses while distinct locations each fire. Mirrors the audio
/// layer's `SfxThrottle`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ThrottleKey {
    Impact(IVec3),
    Destroy(IVec3),
}

/// Last-fired timestamp per throttle key, seconds since startup. An absent key has
/// never fired, so its first event always passes.
#[derive(Resource, Default)]
struct JuiceThrottle {
    last: HashMap<ThrottleKey, f32>,
}

impl JuiceThrottle {
    /// If `key` has not fired within `min_interval` seconds, stamp it `now` and
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

/// Quantize a world position to a [`JUICE_AREA_CELL`]-sized integer cell, so nearby
/// events share a throttle key and far ones do not.
fn area_cell(pos: Vec3) -> IVec3 {
    (pos / JUICE_AREA_CELL).floor().as_ivec3()
}

/// Distance attenuation in `[0, 1]`: full within `near`, zero at/beyond `far`, with
/// a smoothstep ramp between so the falloff eases in and out rather than kinking at
/// the endpoints. A degenerate `far <= near` collapses to a hard near/far step.
/// Pure for unit testing.
fn distance_falloff(distance: f32, near: f32, far: f32) -> f32 {
    if distance <= near {
        1.0
    } else if distance >= far || far <= near {
        0.0
    } else {
        let t = (distance - near) / (far - near);
        // Smoothstep on the *remaining* loudness (1 - t) so full at near, zero at far.
        let s = 1.0 - t;
        s * s * (3.0 - 2.0 * s)
    }
}

/// The gameplay camera's world position (the attenuation listener), or `None` if no
/// listener exists yet (early startup, or the editor). Keys off
/// [`SfxListenerMarker`], the same explicit listener the audio layer uses, so shake
/// and spark attenuation can never diverge from the sound attenuation.
fn listener_position(q_camera: &Query<&GlobalTransform, With<SfxListenerMarker>>) -> Option<Vec3> {
    q_camera.iter().next().map(|t| t.translation())
}

/// How many sparks a burst of `kind` throws at `falloff` distance strength.
///
/// At least one whenever the event fired at all: an event that passed the
/// falloff gate and the throttle happened, and a burst that rounds to nothing
/// is a hit with no cue. Pure for unit testing.
fn spark_count(base: u32, falloff: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "falloff is clamped to 0..=1 by `distance_falloff`, so the product fits"
    )]
    let scaled = (base as f32 * falloff.clamp(0.0, 1.0)).round() as u32;
    scaled.max(1)
}

/// Plugin wiring Nova's combat feedback: the reusable [`CameraShakePlugin`] plus
/// Nova's own trauma-feeding observers, the impact burst, and the [`JuiceSettings`]
/// resource that tunes them.
#[derive(Default)]
pub struct NovaJuicePlugin;

impl Plugin for NovaJuicePlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaJuicePlugin: build");

        // Generic drift-free trauma shake (CameraShake / CameraShakeInput live here).
        if !app.is_plugin_added::<CameraShakePlugin>() {
            app.add_plugins(CameraShakePlugin);
        }

        app.init_resource::<JuiceSettings>()
            // Register the whole reflected tree, not just the root, so the debug
            // WorldInspector and a future settings menu can traverse into the nested
            // shake/spark configs rather than seeing them as unregistered.
            .register_type::<JuiceSettings>()
            .register_type::<ShakeSettings>()
            .register_type::<SparkSettings>()
            .register_type::<JuiceEventKind>()
            .init_resource::<JuiceThrottle>();

        // The look the burst takes. Added HERE rather than left to the app, so
        // a request this module makes always has an answer.
        if !app.is_plugin_added::<ImpactSparkPlugin>() {
            app.add_plugins(ImpactSparkPlugin);
        }

        app.add_observer(on_damage_juice);
        app.add_observer(on_destroy_juice);

        app.add_systems(
            Update,
            (
                ensure_camera_shake,
                sync_camera_shake_config,
                prune_juice_throttle,
            ),
        );
    }
}

/// Keep the per-cell throttle map bounded by dropping idle keys.
fn prune_juice_throttle(time: Res<Time>, mut throttle: ResMut<JuiceThrottle>) {
    throttle.prune(time.elapsed_secs(), JUICE_THROTTLE_PRUNE_WINDOW);
}

/// Attach a [`CameraShake`] (configured from settings) to the marked gameplay
/// camera when it lacks one. Runs every frame but no-ops once the camera has the
/// component; this handles the camera being (re)spawned or swapped (Nova toggles
/// the camera's controller between WASD and chase, but the entity persists).
/// Attaching is mode-agnostic ON PURPOSE: the free-fly WASD phase of that same
/// entity is muted by `CameraShakeSuppressed`, maintained by `nova_ship`'s
/// camera authority - the layer that knows which rig drives - not by removing
/// the shake here. Scoped to [`SfxListenerMarker`] rather than any `Camera3d`,
/// so the editor camera (or a future minimap camera) never grows a shake.
fn ensure_camera_shake(
    settings: Res<JuiceSettings>,
    q_camera: Query<Entity, (With<SfxListenerMarker>, Without<CameraShake>)>,
    mut commands: Commands,
) {
    for camera in &q_camera {
        commands.entity(camera).insert(CameraShake {
            decay: settings.shake.decay,
            max_offset: settings.shake.max_offset,
            max_kick: settings.shake.max_kick,
            exponent: settings.shake.exponent,
        });
    }
}

/// Push live [`JuiceSettings`] shake tunables onto existing [`CameraShake`]
/// components when the settings change, so a settings menu edit takes effect without
/// respawning the camera. No-ops on the common unchanged frame.
fn sync_camera_shake_config(settings: Res<JuiceSettings>, mut q_shake: Query<&mut CameraShake>) {
    if !settings.is_changed() {
        return;
    }
    for mut shake in &mut q_shake {
        shake.decay = settings.shake.decay;
        shake.max_offset = settings.shake.max_offset;
        shake.max_kick = settings.shake.max_kick;
        shake.exponent = settings.shake.exponent;
    }
}

/// Shared reaction to a juice event at `pos`: add attenuated trauma to the marked
/// gameplay camera and ask for a spark burst, each gated by its own toggle and
/// throttle. Called by both observers so impact and destruction share one code
/// path. The shake-input query is scoped to [`SfxListenerMarker`] so trauma lands
/// only on the listener camera, never on some other `CameraShakeInput` holder.
#[expect(
    clippy::too_many_arguments,
    reason = "one system fed by every cue the juice reads"
)]
fn emit_juice(
    pos: Vec3,
    kind: JuiceEventKind,
    now: f32,
    settings: &JuiceSettings,
    listener: Option<Vec3>,
    throttle: &mut JuiceThrottle,
    commands: &mut Commands,
    q_shake_input: &mut Query<&mut CameraShakeInput, With<SfxListenerMarker>>,
) {
    let falloff = listener.map_or(1.0, |l| {
        distance_falloff(
            l.distance(pos),
            settings.near_distance,
            settings.far_distance,
        )
    });
    // Fully attenuated events do nothing at all - no kick, no sparks, no
    // throttle stamp - so a far-off skirmish stays quiet even before throttling.
    if falloff <= 0.0 {
        return;
    }

    let (min_interval, throttle_key) = match kind {
        JuiceEventKind::Impact => (IMPACT_MIN_INTERVAL, ThrottleKey::Impact(area_cell(pos))),
        JuiceEventKind::Destroy => (DESTROY_MIN_INTERVAL, ThrottleKey::Destroy(area_cell(pos))),
    };
    if !throttle.allow(throttle_key, now, min_interval) {
        return;
    }

    if settings.shake_on() {
        let base = match kind {
            JuiceEventKind::Impact => settings.shake.hit_trauma,
            JuiceEventKind::Destroy => settings.shake.destroy_trauma,
        };
        let trauma = base * falloff;
        for mut input in q_shake_input.iter_mut() {
            input.add_trauma += trauma;
        }
    }

    if settings.sparks_on() {
        // The falloff is spent on COUNT and not on speed or size: perspective
        // already shrinks a distant burst, so shrinking it again would
        // double-attenuate, while thinning it is the one axis the camera does
        // not apply for free.
        commands.trigger(ImpactSparks {
            at: pos,
            count: spark_count(settings.sparks.count(kind), falloff),
            force: 1.0,
        });
    }
}

/// Impact juice whenever damage is applied to a living target.
///
/// Propagation caveat: `HealthApplyDamage` auto-propagates up `ChildOf`
/// (section -> ship root), and ship death depends on that bubbling, so it must
/// not be stopped here - but a global observer fires once per hop, which would
/// double the kick/burst whenever the section and root land in different area
/// cells (2x trauma plus a phantom burst at the ship root's origin). Reacting
/// only to the original target keeps one hit = one cue, and the original
/// target is also the better cue position: the actual hit location. Any future
/// damage-cue observer needs this same guard (mirrors `audio/`).
fn on_damage_juice(
    damage: On<HealthApplyDamage>,
    settings: Res<JuiceSettings>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle: ResMut<JuiceThrottle>,
    mut commands: Commands,
    mut q_shake_input: Query<&mut CameraShakeInput, With<SfxListenerMarker>>,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    if !settings.master_enabled {
        return;
    }
    let Ok(source) = q_transform.get(damage.entity) else {
        return;
    };
    emit_juice(
        source.translation(),
        JuiceEventKind::Impact,
        time.elapsed_secs(),
        &settings,
        listener_position(&q_camera),
        &mut throttle,
        &mut commands,
        &mut q_shake_input,
    );
}

/// Destruction juice on any destroy (section, asteroid, or torpedo detonation, all
/// of which funnel through `IntegrityDestroyMarker`).
fn on_destroy_juice(
    add: On<Add, IntegrityDestroyMarker>,
    settings: Res<JuiceSettings>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle: ResMut<JuiceThrottle>,
    mut commands: Commands,
    mut q_shake_input: Query<&mut CameraShakeInput, With<SfxListenerMarker>>,
) {
    if !settings.master_enabled {
        return;
    }
    // The destroyed entity has existed for frames, so its GlobalTransform is valid.
    let Ok(source) = q_transform.get(add.entity) else {
        return;
    };
    emit_juice(
        source.translation(),
        JuiceEventKind::Destroy,
        time.elapsed_secs(),
        &settings,
        listener_position(&q_camera),
        &mut throttle,
        &mut commands,
        &mut q_shake_input,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_blocks_one_key_until_the_interval_elapses() {
        let key = ThrottleKey::Impact(IVec3::ZERO);
        let mut state = JuiceThrottle::default();
        // First event of a key always fires (absent -> NEG_INFINITY).
        assert!(state.allow(key, 0.0, 0.04));
        // Too soon: blocked.
        assert!(!state.allow(key, 0.02, 0.04));
        // At the interval: fires again.
        assert!(state.allow(key, 0.04, 0.04));
    }

    #[test]
    fn throttle_is_independent_per_key() {
        let mut state = JuiceThrottle::default();
        // Distinct cells of the same kind are independent...
        assert!(state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04));
        assert!(state.allow(ThrottleKey::Impact(IVec3::ONE), 0.0, 0.04));
        // ...and impact vs destroy at the same cell are independent too.
        assert!(state.allow(ThrottleKey::Destroy(IVec3::ZERO), 0.0, 0.06));
        // Same key again in the window is still blocked.
        assert!(!state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04));
    }

    #[test]
    fn prune_drops_only_idle_keys() {
        let mut state = JuiceThrottle::default();
        state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04); // last = 0.0
        state.allow(ThrottleKey::Impact(IVec3::ONE), 9.5, 0.04); // last = 9.5
        state.prune(10.0, 2.0); // keep last > 8.0
        assert_eq!(state.last.len(), 1);
        assert!(state.last.contains_key(&ThrottleKey::Impact(IVec3::ONE)));
    }

    #[test]
    fn area_cell_groups_nearby_and_separates_distant() {
        assert_eq!(
            area_cell(Vec3::ZERO),
            area_cell(Vec3::splat(JUICE_AREA_CELL * 0.5))
        );
        assert_ne!(
            area_cell(Vec3::ZERO),
            area_cell(Vec3::splat(JUICE_AREA_CELL * 1.5))
        );
    }

    #[test]
    fn distance_falloff_is_full_near_zero_far_and_monotonic_between() {
        let (near, far) = (20.0, 320.0);
        assert_eq!(distance_falloff(0.0, near, far), 1.0);
        assert_eq!(distance_falloff(near, near, far), 1.0);
        assert_eq!(distance_falloff(far, near, far), 0.0);
        assert_eq!(distance_falloff(far + 50.0, near, far), 0.0);

        // Monotonic decreasing across the ramp.
        let a = distance_falloff(near + 10.0, near, far);
        let m = distance_falloff((near + far) / 2.0, near, far);
        let b = distance_falloff(far - 10.0, near, far);
        assert!(a > m && m > b, "falloff should decrease with distance");
        for d in [30.0, 100.0, 200.0, 300.0] {
            let v = distance_falloff(d, near, far);
            assert!((0.0..=1.0).contains(&v), "falloff out of range at {d}: {v}");
        }
    }

    #[test]
    fn distance_falloff_handles_degenerate_range() {
        // far <= near collapses to a hard near/far step rather than dividing by zero.
        assert_eq!(distance_falloff(10.0, 20.0, 20.0), 1.0);
        assert_eq!(distance_falloff(30.0, 20.0, 20.0), 0.0);
    }

    #[test]
    fn the_spark_count_thins_with_distance_but_never_to_nothing() {
        assert_eq!(spark_count(20, 1.0), 20);
        assert_eq!(spark_count(20, 0.5), 10);
        // An event that fired at all throws at least one spark: a hit that
        // passed the falloff gate and the throttle happened, and rounding it
        // away leaves a hit with no cue.
        assert_eq!(spark_count(5, 0.01), 1);
        assert_eq!(spark_count(0, 1.0), 1);
        // Out-of-range strength cannot inflate the burst.
        assert_eq!(spark_count(4, 2.0), 4);
        assert_eq!(spark_count(4, -1.0), 1);
    }

    #[test]
    fn default_settings_are_sane() {
        let s = JuiceSettings::default();
        assert!(s.master_enabled);
        assert!(s.shake_on() && s.sparks_on());
        // Destruction should out-shake and out-spark an impact.
        assert!(s.shake.destroy_trauma > s.shake.hit_trauma);
        assert!(s.sparks.destroy_count > s.sparks.impact_count);
        // Ranges are well-formed so the falloff never divides by zero.
        assert!(s.far_distance > s.near_distance);
        // Toggling the master switch disables both effects.
        let off = JuiceSettings {
            master_enabled: false,
            ..JuiceSettings::default()
        };
        assert!(!off.shake_on() && !off.sparks_on());
    }

    //
    // These exercise the wiring the pure helpers cannot: that the event observers
    // actually feed trauma into `CameraShakeInput` and ask for sparks, that the
    // per-cell throttle collapses a co-located burst, that distance attenuation
    // scales/suppresses through the observers, and that the master switch
    // suppresses everything. Most run without a camera, so the attenuation
    // listener is `None` (falloff 1.0) and trauma lands at exactly the configured
    // impulse; the attenuation tests spawn a positioned `Camera3d`.

    /// A minimal app with the juice resources + event observers and no camera, so
    /// distance attenuation is a no-op and trauma equals the raw per-event impulse.
    fn juice_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<JuiceSettings>();
        app.init_resource::<JuiceThrottle>();
        app.init_resource::<AskedBursts>();
        app.add_observer(on_damage_juice);
        app.add_observer(on_destroy_juice);
        // Record the ASK rather than adding `ImpactSparkPlugin`: what this
        // module owes is a correctly gated, correctly placed, correctly sized
        // request, and the spark module's own tests cover what a request
        // becomes. It also keeps the rig free of asset stores.
        app.add_observer(|burst: On<ImpactSparks>, mut asked: ResMut<AskedBursts>| {
            asked.0.push(*burst);
        });
        app
    }

    /// Every [`ImpactSparks`] the observers asked for this run, in order.
    #[derive(Resource, Default)]
    struct AskedBursts(Vec<ImpactSparks>);

    /// Spawn an entity carrying a `CameraShakeInput` plus the listener marker,
    /// standing in for the gameplay camera's shake sink (trauma is scoped to the
    /// marked camera, so an unmarked input would receive nothing).
    fn spawn_shake_sink(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((SfxListenerMarker, CameraShakeInput::default()))
            .id()
    }

    /// Spawn a positioned target the observers can read a world position from.
    fn spawn_at(app: &mut App, pos: Vec3) -> Entity {
        app.world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(pos)))
            .id()
    }

    /// Spawn a gameplay camera (the marked attenuation listener) at `pos`.
    fn spawn_camera_at(app: &mut App, pos: Vec3) {
        app.world_mut().spawn((
            Camera3d::default(),
            SfxListenerMarker,
            GlobalTransform::from(Transform::from_translation(pos)),
        ));
    }

    fn trauma_of(app: &App, sink: Entity) -> f32 {
        app.world()
            .get::<CameraShakeInput>(sink)
            .unwrap()
            .add_trauma
    }

    /// Every ask recorded so far, flushing first.
    ///
    /// `Commands::trigger` from inside an observer is DEFERRED: the ask has not
    /// happened until the world flushes, which is what the running app does at
    /// the end of every schedule. Reading without the flush sees nothing and
    /// reads as a missing cue.
    fn bursts(app: &mut App) -> &[ImpactSparks] {
        app.world_mut().flush();
        &app.world().resource::<AskedBursts>().0
    }

    fn burst_count(app: &mut App) -> usize {
        bursts(app).len()
    }

    #[test]
    fn damage_event_feeds_impact_trauma_and_asks_for_sparks() {
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        let target = spawn_at(&mut app, Vec3::ZERO);

        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 10.0,
        });

        // No camera -> falloff 1.0 -> trauma is exactly the impact impulse.
        let expected = JuiceSettings::default().shake.hit_trauma;
        assert!((trauma_of(&app, sink) - expected).abs() < 1e-6);
        assert_eq!(burst_count(&mut app), 1);
    }

    #[test]
    fn destroy_event_feeds_the_larger_destruction_trauma() {
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        let target = spawn_at(&mut app, Vec3::ZERO);

        // Inserting the destroy marker fires the `On<Add, IntegrityDestroyMarker>`
        // observer.
        app.world_mut()
            .entity_mut(target)
            .insert(IntegrityDestroyMarker);

        let expected = JuiceSettings::default().shake.destroy_trauma;
        assert!((trauma_of(&app, sink) - expected).abs() < 1e-6);
        assert_eq!(burst_count(&mut app), 1);
        assert_eq!(
            bursts(&mut app)[0].count,
            JuiceSettings::default().sparks.destroy_count,
            "a point-blank kill throws the full destruction burst"
        );
    }

    #[test]
    fn a_co_located_burst_collapses_to_one_via_the_throttle() {
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        // Two targets in the same area cell, damaged in the same frame (elapsed 0).
        let a = spawn_at(&mut app, Vec3::ZERO);
        let b = spawn_at(&mut app, Vec3::splat(JUICE_AREA_CELL * 0.25));

        for target in [a, b] {
            app.world_mut().trigger(HealthApplyDamage {
                entity: target,
                source: None,
                amount: 5.0,
            });
        }

        // Only the first of the co-located pair passes: one burst, one trauma
        // impulse.
        let expected = JuiceSettings::default().shake.hit_trauma;
        assert!((trauma_of(&app, sink) - expected).abs() < 1e-6);
        assert_eq!(burst_count(&mut app), 1);
    }

    #[test]
    fn a_propagated_hit_on_a_straddling_hierarchy_fires_one_cue() {
        // `HealthApplyDamage` auto-propagates child -> parent, and with the
        // parent one area cell away the per-cell throttle cannot collapse the
        // hops - one hit read as two cues (bursts = 2, trauma = 2x the tuned
        // impulse, plus a phantom burst at the parent's origin). The
        // original-target guard must keep it at exactly one.
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        let parent = spawn_at(&mut app, Vec3::new(JUICE_AREA_CELL * 4.0, 0.0, 0.0));
        let child = spawn_at(&mut app, Vec3::ZERO);
        app.world_mut().entity_mut(child).insert(ChildOf(parent));

        app.world_mut().trigger(HealthApplyDamage {
            entity: child,
            source: None,
            amount: 10.0,
        });

        let expected = JuiceSettings::default().shake.hit_trauma;
        assert!(
            (trauma_of(&app, sink) - expected).abs() < 1e-6,
            "one hit must add exactly one trauma impulse, got {}",
            trauma_of(&app, sink)
        );
        assert_eq!(
            burst_count(&mut app),
            1,
            "one hit must ask for exactly one burst"
        );
        // And the cue sits at the hit location, not the parent's origin.
        assert_eq!(bursts(&mut app)[0].at, Vec3::ZERO);
    }

    #[test]
    fn distinct_cells_both_fire() {
        let mut app = juice_test_app();
        let _sink = spawn_shake_sink(&mut app);
        let a = spawn_at(&mut app, Vec3::ZERO);
        let b = spawn_at(&mut app, Vec3::new(JUICE_AREA_CELL * 4.0, 0.0, 0.0));

        for target in [a, b] {
            app.world_mut().trigger(HealthApplyDamage {
                entity: target,
                source: None,
                amount: 5.0,
            });
        }

        assert_eq!(burst_count(&mut app), 2);
    }

    #[test]
    fn a_mid_range_event_scales_trauma_and_thins_the_burst() {
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        let target = spawn_at(&mut app, Vec3::ZERO);
        // The smoothstep falloff is exactly 0.5 at the midpoint of the ramp.
        let s = JuiceSettings::default();
        let mid = (s.near_distance + s.far_distance) / 2.0;
        spawn_camera_at(&mut app, Vec3::new(mid, 0.0, 0.0));

        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 10.0,
        });

        let expected = s.shake.hit_trauma * 0.5;
        assert!((trauma_of(&app, sink) - expected).abs() < 1e-6);
        assert_eq!(burst_count(&mut app), 1);
        assert_eq!(
            bursts(&mut app)[0].count,
            spark_count(s.sparks.impact_count, 0.5),
            "half strength throws half the sparks"
        );
    }

    #[test]
    fn a_fully_attenuated_event_does_nothing_and_stamps_no_throttle() {
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        let target = spawn_at(&mut app, Vec3::ZERO);
        let far = JuiceSettings::default().far_distance;
        spawn_camera_at(&mut app, Vec3::new(far + 50.0, 0.0, 0.0));

        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 10.0,
        });

        assert_eq!(trauma_of(&app, sink), 0.0);
        assert_eq!(burst_count(&mut app), 0);
        // A far event must not consume throttle state either, so a near event in
        // the same cell right after still fires.
        assert!(app.world().resource::<JuiceThrottle>().last.is_empty());
    }

    #[test]
    fn trauma_lands_only_on_the_marked_listener() {
        let mut app = juice_test_app();
        let marked = spawn_shake_sink(&mut app);
        // An unmarked shake input (e.g. some other camera with a CameraShake)
        // must receive no trauma from gameplay juice.
        let unmarked = app.world_mut().spawn(CameraShakeInput::default()).id();
        let target = spawn_at(&mut app, Vec3::ZERO);

        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 10.0,
        });

        let expected = JuiceSettings::default().shake.hit_trauma;
        assert!((trauma_of(&app, marked) - expected).abs() < 1e-6);
        assert_eq!(trauma_of(&app, unmarked), 0.0);
    }

    #[test]
    fn attenuation_listens_from_the_marked_camera_not_any_camera3d() {
        let mut app = juice_test_app();
        let sink = spawn_shake_sink(&mut app);
        let target = spawn_at(&mut app, Vec3::ZERO);
        let far = JuiceSettings::default().far_distance;
        // The marked listener is out of range; an unmarked Camera3d sits right on
        // the event. Under the old "first Camera3d" rule the unmarked one could
        // win and the event would fire at full strength.
        spawn_camera_at(&mut app, Vec3::new(far + 50.0, 0.0, 0.0));
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from(Transform::from_translation(Vec3::ZERO)),
        ));

        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 10.0,
        });

        assert_eq!(trauma_of(&app, sink), 0.0);
        assert_eq!(burst_count(&mut app), 0);
    }

    #[test]
    fn camera_shake_attaches_only_to_the_marked_camera() {
        let mut app = App::new();
        app.init_resource::<JuiceSettings>();
        app.add_systems(Update, ensure_camera_shake);
        let marked = app
            .world_mut()
            .spawn((Camera3d::default(), SfxListenerMarker))
            .id();
        let unmarked = app.world_mut().spawn(Camera3d::default()).id();

        app.update();

        assert!(app.world().get::<CameraShake>(marked).is_some());
        assert!(
            app.world().get::<CameraShake>(unmarked).is_none(),
            "an unmarked Camera3d (editor/minimap) must not grow a CameraShake"
        );
    }

    #[test]
    fn master_switch_off_suppresses_both_effects() {
        let mut app = juice_test_app();
        app.world_mut()
            .resource_mut::<JuiceSettings>()
            .master_enabled = false;
        let sink = spawn_shake_sink(&mut app);
        let target = spawn_at(&mut app, Vec3::ZERO);

        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 10.0,
        });
        app.world_mut()
            .entity_mut(target)
            .insert(IntegrityDestroyMarker);

        assert_eq!(trauma_of(&app, sink), 0.0);
        assert_eq!(burst_count(&mut app), 0);
    }
}
