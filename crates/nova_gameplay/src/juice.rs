//! Nova's combat juice: moment-to-moment feedback when a shot lands or a target
//! dies. Two effects (camera shake and flash rings), both driven off the same
//! existing seams the audio layer uses so no gameplay system has to know about
//! them:
//!
//! - damage applied to a target -> a small camera-shake kick + an impact flash
//!   (`On<HealthApplyDamage>`);
//! - a section/asteroid destroyed or a torpedo detonating -> a big camera-shake
//!   kick + a destruction flash (`On<Add, IntegrityDestroyMarker>`).
//!
//! **Camera shake** reuses the generic trauma model from `bevy_common_systems`
//! ([`CameraShakePlugin`]): it is drift-free (offset is un-applied and re-applied
//! around the base-writing driver) and already orders itself around the chase
//! camera. This module only *feeds* it trauma; [`ensure_camera_shake`] attaches a
//! [`CameraShake`] (configured from [`JuiceSettings`]) to the gameplay camera.
//!
//! **Impact / hit-flash FX** are drawn with gizmos, not spawned meshes or
//! particles: a camera-facing ring at the event position that expands and fades
//! over a fraction of a second. Gizmos are wasm-safe (the particle system is still
//! wasm-blocked, 162908) and, being immediate-mode, incur zero asset/entity churn
//! even when a blast hits many colliders in one frame. Section render meshes live
//! in shared, gltf-instanced children, so an overlay ring is chosen over recoloring
//! their materials (a true per-section emissive flash is a possible follow-up).
//!
//! Both effects are **distance-attenuated** from the gameplay camera (the trauma
//! impulse and the ring alpha both scale with the falloff) and **per-area-cell
//! throttled**, mirroring `audio.rs`: a blast that damages a dozen colliders of
//! one ship in a single frame collapses to one kick and one flash, and a distant
//! event kicks/flashes weaker than one in your face. Every tunable
//! lives on the [`JuiceSettings`] resource (with per-effect enable toggles and a
//! master switch) so a settings menu can bind to it later. All the math a headless
//! run cannot exercise (the rendering) is pushed into pure helpers that are
//! unit-tested.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{FlashSettings, JuiceSettings, NovaJuicePlugin, ShakeSettings};
}

/// World-cell size (units) for grouping co-located juice events, matching the
/// audio layer's `SFX_AREA_CELL`. A blast hitting many colliders of one ship, or a
/// ship's sections all destroyed at once, fall in the same cell and collapse to a
/// single kick/flash; events far enough apart get their own.
const JUICE_AREA_CELL: f32 = 6.0;

/// Minimum seconds between successive impact / destruction juice events per cell.
/// Without this a single blast's many-collider damage burst would stack a dozen
/// identical kicks (saturating trauma instantly) and a dozen overlapping rings.
/// A dying multi-section ship marks every section in one frame, so destruction is
/// throttled too, just a touch looser so genuinely separate kills still each read.
const IMPACT_MIN_INTERVAL: f32 = 0.04;
const DESTROY_MIN_INTERVAL: f32 = 0.06;

/// Drop throttle keys not touched within this many seconds, so the per-cell map
/// stays bounded as combat moves through new cells (mirrors the audio throttle).
const JUICE_THROTTLE_PRUNE_WINDOW: f32 = 2.0;

/// Hard cap on simultaneously-tracked flashes. Throttling already bounds the spawn
/// rate and the draw system prunes expired ones every frame, so this is only a
/// runaway backstop; when full, the newest flash is dropped rather than unbounded
/// growth.
const MAX_ACTIVE_FLASHES: usize = 64;

/// Which kind of juice event a flash represents, selecting its color/size/duration
/// from [`FlashSettings`].
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
            hit_trauma: 0.08,
            destroy_trauma: 0.24,
            // Snappier decay so a kick settles quickly rather than lingering.
            decay: 2.4,
            // A small translational shudder with only a whisper of rotational kick,
            // so the camera trembles in place instead of visibly swinging.
            max_offset: Vec3::new(0.18, 0.18, 0.1),
            max_kick: Vec3::new(0.008, 0.008, 0.012),
            exponent: 2.0,
        }
    }
}

/// Tunables for the gizmo impact/destruction flash rings.
#[derive(Clone, Debug, Reflect)]
pub struct FlashSettings {
    /// Master toggle for the flash FX.
    pub enabled: bool,
    /// Ring color for impact flashes (alpha is driven by the fade, so any alpha
    /// here is ignored at draw time).
    pub impact_color: Color,
    /// Ring color for destruction flashes.
    pub destroy_color: Color,
    /// Peak radius an impact ring expands to, world units.
    pub impact_radius: f32,
    /// Peak radius a destruction ring expands to, world units.
    pub destroy_radius: f32,
    /// Lifetime of an impact flash, seconds.
    pub impact_duration: f32,
    /// Lifetime of a destruction flash, seconds.
    pub destroy_duration: f32,
    /// Concentric rings drawn per flash; >1 gives a little shockwave depth. Each
    /// extra ring trails the leading edge slightly.
    pub ring_count: u32,
}

impl Default for FlashSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            // Warm spark for hits, hot white-orange for kills.
            impact_color: Color::srgb(1.0, 0.85, 0.4),
            destroy_color: Color::srgb(1.0, 0.6, 0.25),
            impact_radius: 1.2,
            destroy_radius: 4.0,
            impact_duration: 0.18,
            destroy_duration: 0.45,
            ring_count: 2,
        }
    }
}

impl FlashSettings {
    /// Peak radius for a flash of `kind`.
    fn radius(&self, kind: JuiceEventKind) -> f32 {
        match kind {
            JuiceEventKind::Impact => self.impact_radius,
            JuiceEventKind::Destroy => self.destroy_radius,
        }
    }

    /// Lifetime for a flash of `kind`.
    fn duration(&self, kind: JuiceEventKind) -> f32 {
        match kind {
            JuiceEventKind::Impact => self.impact_duration,
            JuiceEventKind::Destroy => self.destroy_duration,
        }
    }

    /// Base (opaque) color for a flash of `kind`.
    fn color(&self, kind: JuiceEventKind) -> Color {
        match kind {
            JuiceEventKind::Impact => self.impact_color,
            JuiceEventKind::Destroy => self.destroy_color,
        }
    }
}

/// All combat-juice tunables in one resource, so a future settings menu can edit a
/// single reflected struct. Systems read it every frame; changes to the shake
/// fields are pushed onto the live [`CameraShake`] by [`sync_camera_shake_config`].
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct JuiceSettings {
    /// Kill switch for all juice at once (a settings-menu "reduce motion" toggle).
    pub master_enabled: bool,
    /// Camera-shake tunables.
    pub shake: ShakeSettings,
    /// Flash-FX tunables.
    pub flash: FlashSettings,
    /// A juice event at or nearer than this to the camera fires at full strength.
    pub near_distance: f32,
    /// A juice event at or beyond this is fully attenuated (no kick, no flash).
    pub far_distance: f32,
}

impl Default for JuiceSettings {
    fn default() -> Self {
        Self {
            master_enabled: true,
            shake: ShakeSettings::default(),
            flash: FlashSettings::default(),
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

    /// Whether the flash effect should run (master + per-effect toggle).
    fn flash_on(&self) -> bool {
        self.master_enabled && self.flash.enabled
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

/// One in-flight flash ring. Position and distance strength are fixed at spawn;
/// the draw system derives the current radius/alpha from `age = now - start_secs`
/// and scales the alpha by `strength`.
#[derive(Clone, Copy, Debug)]
struct Flash {
    pos: Vec3,
    start_secs: f32,
    kind: JuiceEventKind,
    /// Distance falloff (`0..1`) captured at emit time, so a far event draws a
    /// fainter ring than one in your face (the visual analog of the attenuated
    /// trauma). Radius is left at world scale - perspective already shrinks a
    /// distant ring, so scaling radius too would double-attenuate.
    strength: f32,
}

/// The set of flashes currently being drawn. Pushed by the event observers, drawn
/// and pruned by [`draw_juice_flashes`].
#[derive(Resource, Default)]
struct ActiveJuiceFx {
    flashes: Vec<Flash>,
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
/// camera exists yet (early startup). Takes the first `Camera3d`, matching the
/// audio layer's listener.
fn listener_position(q_camera: &Query<&GlobalTransform, With<Camera3d>>) -> Option<Vec3> {
    q_camera.iter().next().map(|t| t.translation())
}

/// Normalized progress `0..1` of a flash of `age` seconds and total `duration`, or
/// `>= 1.0` once it has fully elapsed. A non-positive duration reads as instantly
/// done. Pure for unit testing.
fn flash_progress(age: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        1.0
    } else {
        age / duration
    }
}

/// Ring radius at progress `t` (0..1), easing out from ~0 toward `peak` so the ring
/// leaps out then decelerates (`1 - (1 - t)^2`). Clamped so out-of-range `t` is
/// safe. Pure for unit testing.
fn flash_radius(t: f32, peak: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    peak * eased
}

/// Ring alpha at progress `t` (0..1): opaque at the start, fading quadratically to
/// zero at the end so the tail lingers faintly rather than cutting out. Clamped for
/// safety. Pure for unit testing.
fn flash_alpha(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let remaining = 1.0 - t;
    remaining * remaining
}

/// Plugin wiring Nova's combat feedback: the reusable [`CameraShakePlugin`] plus
/// Nova's own trauma-feeding observers, gizmo flash FX, and the [`JuiceSettings`]
/// resource that tunes them.
#[derive(Default)]
pub struct NovaJuicePlugin;

impl Plugin for NovaJuicePlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaJuicePlugin: build");

        // Generic drift-free trauma shake (CameraShake / CameraShakeInput live here).
        if !app.is_plugin_added::<CameraShakePlugin>() {
            app.add_plugins(CameraShakePlugin);
        }

        app.init_resource::<JuiceSettings>()
            // Register the whole reflected tree, not just the root, so the debug
            // WorldInspector and a future settings menu can traverse into the nested
            // shake/flash configs rather than seeing them as unregistered.
            .register_type::<JuiceSettings>()
            .register_type::<ShakeSettings>()
            .register_type::<FlashSettings>()
            .register_type::<JuiceEventKind>()
            .init_resource::<JuiceThrottle>()
            .init_resource::<ActiveJuiceFx>();

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

        // Drawing reads the camera's GlobalTransform, so it must run after transform
        // propagation, exactly like the debug gizmo systems.
        app.add_systems(
            PostUpdate,
            draw_juice_flashes.after(TransformSystems::Propagate),
        );
    }
}

/// Keep the per-cell throttle map bounded by dropping idle keys.
fn prune_juice_throttle(time: Res<Time>, mut throttle: ResMut<JuiceThrottle>) {
    throttle.prune(time.elapsed_secs(), JUICE_THROTTLE_PRUNE_WINDOW);
}

/// Attach a [`CameraShake`] (configured from settings) to any gameplay `Camera3d`
/// that lacks one. Runs every frame but no-ops once the camera has the component;
/// this handles the camera being (re)spawned or swapped (Nova toggles the camera's
/// controller between WASD and chase, but the entity persists).
fn ensure_camera_shake(
    settings: Res<JuiceSettings>,
    q_camera: Query<Entity, (With<Camera3d>, Without<CameraShake>)>,
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

/// Shared reaction to a juice event at `pos`: add attenuated trauma to the camera(s)
/// and queue a flash, each gated by its own enable toggle and throttle. Called by
/// both observers so impact and destruction share one code path.
#[allow(clippy::too_many_arguments)]
fn emit_juice(
    pos: Vec3,
    kind: JuiceEventKind,
    now: f32,
    settings: &JuiceSettings,
    listener: Option<Vec3>,
    throttle: &mut JuiceThrottle,
    fx: &mut ActiveJuiceFx,
    q_shake_input: &mut Query<&mut CameraShakeInput>,
) {
    let falloff = listener.map_or(1.0, |l| {
        distance_falloff(
            l.distance(pos),
            settings.near_distance,
            settings.far_distance,
        )
    });
    // Fully attenuated events do nothing at all - no kick, no ring, no throttle
    // stamp - so a far-off skirmish stays quiet even before throttling.
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

    if settings.flash_on() {
        if fx.flashes.len() < MAX_ACTIVE_FLASHES {
            fx.flashes.push(Flash {
                pos,
                start_secs: now,
                kind,
                strength: falloff,
            });
        } else {
            trace!("emit_juice: flash cap reached, dropping flash");
        }
    }
}

/// Impact juice whenever damage is applied to a living target.
fn on_damage_juice(
    damage: On<HealthApplyDamage>,
    settings: Res<JuiceSettings>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<Camera3d>>,
    mut throttle: ResMut<JuiceThrottle>,
    mut fx: ResMut<ActiveJuiceFx>,
    mut q_shake_input: Query<&mut CameraShakeInput>,
) {
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
        &mut fx,
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
    q_camera: Query<&GlobalTransform, With<Camera3d>>,
    mut throttle: ResMut<JuiceThrottle>,
    mut fx: ResMut<ActiveJuiceFx>,
    mut q_shake_input: Query<&mut CameraShakeInput>,
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
        &mut fx,
        &mut q_shake_input,
    );
}

/// Draw every active flash as concentric camera-facing rings whose radius expands
/// and alpha fades over the flash lifetime, then drop the ones that have elapsed.
fn draw_juice_flashes(
    time: Res<Time>,
    settings: Res<JuiceSettings>,
    q_camera: Query<&GlobalTransform, With<Camera3d>>,
    mut fx: ResMut<ActiveJuiceFx>,
    mut gizmos: Gizmos,
) {
    let now = time.elapsed_secs();
    let cam_pos = listener_position(&q_camera);

    // Prune first so a disabled/duration-changed setting cannot strand old flashes.
    fx.flashes.retain(|flash| {
        flash_progress(now - flash.start_secs, settings.flash.duration(flash.kind)) < 1.0
    });

    if !settings.flash_on() {
        return;
    }

    for flash in &fx.flashes {
        let age = now - flash.start_secs;
        let duration = settings.flash.duration(flash.kind);
        let t = flash_progress(age, duration);
        let peak = settings.flash.radius(flash.kind);
        let base = settings.flash.color(flash.kind).to_srgba();

        // Face the ring toward the camera so it always reads as a disc, not an
        // edge-on line. Fall back to a fixed orientation if the camera is missing
        // or sits exactly on the flash.
        let rotation = cam_pos
            .and_then(|c| Dir3::new(c - flash.pos).ok())
            .map(|dir| Quat::from_rotation_arc(Vec3::Z, dir.as_vec3()))
            .unwrap_or(Quat::IDENTITY);

        // Trailing concentric rings for a bit of shockwave depth: each inner ring
        // lags the leading edge by a fraction of the lifetime.
        let ring_count = settings.flash.ring_count.max(1);
        for ring in 0..ring_count {
            let lag = ring as f32 * 0.15;
            let rt = (t - lag).clamp(0.0, 1.0);
            let radius = flash_radius(rt, peak);
            // Lifetime fade scaled by the distance strength captured at emit,
            // so a far event's ring is faint from its first frame.
            let alpha = flash_alpha(rt) * flash.strength;
            if alpha <= 0.0 || radius <= 0.0 {
                continue;
            }
            let color = Color::srgba(base.red, base.green, base.blue, alpha);
            gizmos.circle(Isometry3d::new(flash.pos, rotation), radius, color);
        }
    }
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
    fn flash_progress_maps_age_to_unit_and_guards_zero_duration() {
        assert_eq!(flash_progress(0.0, 0.2), 0.0);
        assert!((flash_progress(0.1, 0.2) - 0.5).abs() < 1e-6);
        assert!(flash_progress(0.2, 0.2) >= 1.0);
        // A zero/negative duration reads as instantly elapsed (never drawn).
        assert!(flash_progress(0.0, 0.0) >= 1.0);
    }

    #[test]
    fn flash_radius_expands_from_zero_to_peak_and_eases_out() {
        assert_eq!(flash_radius(0.0, 4.0), 0.0);
        assert!((flash_radius(1.0, 4.0) - 4.0).abs() < 1e-6);
        // Ease-out: past the halfway radius by the time-midpoint.
        assert!(flash_radius(0.5, 4.0) > 2.0);
        // Clamps out-of-range progress.
        assert_eq!(flash_radius(-1.0, 4.0), 0.0);
        assert!((flash_radius(2.0, 4.0) - 4.0).abs() < 1e-6);
    }

    #[test]
    fn flash_alpha_fades_from_opaque_to_transparent() {
        assert_eq!(flash_alpha(0.0), 1.0);
        assert_eq!(flash_alpha(1.0), 0.0);
        // Monotonic decreasing.
        assert!(flash_alpha(0.25) > flash_alpha(0.75));
        // Clamps out-of-range progress.
        assert_eq!(flash_alpha(-1.0), 1.0);
        assert_eq!(flash_alpha(2.0), 0.0);
    }

    #[test]
    fn default_settings_are_sane() {
        let s = JuiceSettings::default();
        assert!(s.master_enabled);
        assert!(s.shake_on() && s.flash_on());
        // Destruction should out-shake and out-flash an impact.
        assert!(s.shake.destroy_trauma > s.shake.hit_trauma);
        assert!(s.flash.destroy_radius > s.flash.impact_radius);
        assert!(s.flash.destroy_duration > s.flash.impact_duration);
        // Ranges are well-formed so the falloff never divides by zero.
        assert!(s.far_distance > s.near_distance);
        // Toggling the master switch disables both effects.
        let off = JuiceSettings {
            master_enabled: false,
            ..JuiceSettings::default()
        };
        assert!(!off.shake_on() && !off.flash_on());
    }

    // --- Observer-level integration tests -------------------------------------
    //
    // These exercise the wiring the pure helpers cannot: that the event observers
    // actually feed trauma into `CameraShakeInput` and queue a `Flash`, that the
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
        app.init_resource::<ActiveJuiceFx>();
        app.add_observer(on_damage_juice);
        app.add_observer(on_destroy_juice);
        app
    }

    /// Spawn an entity carrying only a `CameraShakeInput`, standing in for the
    /// gameplay camera's shake sink.
    fn spawn_shake_sink(app: &mut App) -> Entity {
        app.world_mut().spawn(CameraShakeInput::default()).id()
    }

    /// Spawn a positioned target the observers can read a world position from.
    fn spawn_at(app: &mut App, pos: Vec3) -> Entity {
        app.world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(pos)))
            .id()
    }

    /// Spawn a gameplay camera (the attenuation listener) at `pos`.
    fn spawn_camera_at(app: &mut App, pos: Vec3) {
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from(Transform::from_translation(pos)),
        ));
    }

    fn trauma_of(app: &App, sink: Entity) -> f32 {
        app.world()
            .get::<CameraShakeInput>(sink)
            .unwrap()
            .add_trauma
    }

    fn flash_count(app: &App) -> usize {
        app.world().resource::<ActiveJuiceFx>().flashes.len()
    }

    #[test]
    fn damage_event_feeds_impact_trauma_and_queues_a_flash() {
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
        assert_eq!(flash_count(&app), 1);
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
        assert_eq!(flash_count(&app), 1);
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

        // Only the first of the co-located pair passes: one flash, one trauma impulse.
        let expected = JuiceSettings::default().shake.hit_trauma;
        assert!((trauma_of(&app, sink) - expected).abs() < 1e-6);
        assert_eq!(flash_count(&app), 1);
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

        assert_eq!(flash_count(&app), 2);
    }

    #[test]
    fn a_mid_range_event_scales_trauma_and_flash_strength() {
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
        let flashes = &app.world().resource::<ActiveJuiceFx>().flashes;
        assert_eq!(flashes.len(), 1);
        assert!((flashes[0].strength - 0.5).abs() < 1e-6);
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
        assert_eq!(flash_count(&app), 0);
        // A far event must not consume throttle state either, so a near event in
        // the same cell right after still fires.
        assert!(app.world().resource::<JuiceThrottle>().last.is_empty());
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
        assert_eq!(flash_count(&app), 0);
    }
}
