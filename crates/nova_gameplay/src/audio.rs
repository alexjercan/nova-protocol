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
//! thruster hum attenuates per SHIP: each ship's throttle-driven contribution is
//! scaled by its root's distance to the listener and the loudest wins, except
//! the player's own ship, which is never attenuated (the camera rig sits 11-32 u
//! out by mode and the orbit survey dolly stretches it to 250 u, deep in the
//! rolloff band; see [`compute_thruster_hum_volume`]).
//!
//! The [`SoundBank<NovaSfx>`] resource is inserted by `nova_assets` once assets
//! load; every system here degrades gracefully (does nothing) until it exists.

use std::collections::HashMap;

use bevy::{audio::Volume, prelude::*};

use crate::{
    prelude::*,
    sections::turret_section::{TurretSectionFireSound, TurretSectionPartOf},
};

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
    /// A new objective was posted to the panel (UI cue, non-positional).
    ObjectiveNew,
    /// An objective was completed (UI cue, non-positional).
    ObjectiveComplete,
    /// A radar gesture acquired its first target (UI cue, once per gesture -
    /// Q3a of spike 20260713-110039).
    LockOn,
    /// A tap-clear released a lock (UI cue; pairs with the unlatch ghost).
    LockOff,
    /// The weapons safety re-engaged - the player's hot -> cold edge (UI
    /// cue; a held burst must not just silently stop).
    SafetyOn,
    /// A radar hold was denied - the computer grants no Lock capability
    /// (UI cue, F7/Q8a).
    RadarDeny,
    /// A salvage crate was picked up - a light per-crate "ding", quieter than
    /// and separate from the objective chime (task 20260714-090002). Fired
    /// from `nova_scenario`'s salvage plugin, which owns the crate marker.
    SalvagePickup,
    /// A menu button was pressed (New Game / Sandbox / Settings / Exit and the
    /// pause/mods buttons) - a crisp UI click (task 20260714-090006). Fired from
    /// `nova_menu`'s global `On<Activate>` observer.
    MenuSelect,
    /// A pause overlay open/close toggle via ESC - a soft two-state UI blip
    /// (task 20260714-090006).
    UiToggle,
    /// A turret pulled its trigger on an empty magazine - a dull dry-fire click,
    /// so a held burst that runs dry is not silently blocked (task
    /// 20260714-090006).
    DryFire,
    /// A held radar gesture re-designated to a new target - a subtle, lower
    /// tick, distinct from the once-per-gesture [`LockOn`] acquire cue (task
    /// 20260714-090006).
    RadarRetarget,
}

/// The `(key, base-filename)` pairs Nova loads into its [`SoundBank`]. Shared
/// with `nova_assets::register_sounds`, which does the load: since the base
/// sounds moved UNDER `assets/base/` (task 20260717-002228) it maps each name to
/// `base/sounds/<name>.wav` via `SoundBank::load_paths`, so these files are the
/// base game's own bundled cues (and mods can reference them via
/// `dep://base/sounds/<name>.wav`).
pub const NOVA_SFX_FILES: [(NovaSfx, &str); 16] = [
    (NovaSfx::ThrusterLoop, "thruster_loop"),
    (NovaSfx::TurretFire, "turret_fire"),
    (NovaSfx::TorpedoLaunch, "torpedo_launch"),
    (NovaSfx::Explosion, "explosion"),
    (NovaSfx::Impact, "impact"),
    (NovaSfx::ObjectiveNew, "objective_new"),
    (NovaSfx::ObjectiveComplete, "objective_complete"),
    (NovaSfx::LockOn, "lock_on"),
    (NovaSfx::LockOff, "lock_off"),
    (NovaSfx::SafetyOn, "safety_on"),
    (NovaSfx::RadarDeny, "radar_deny"),
    (NovaSfx::SalvagePickup, "salvage_pickup"),
    (NovaSfx::MenuSelect, "menu_select"),
    (NovaSfx::UiToggle, "ui_toggle"),
    (NovaSfx::DryFire, "dry_fire"),
    (NovaSfx::RadarRetarget, "radar_retarget"),
];

/// Per-cue *base* playback volumes (at point-blank; distance attenuation scales
/// them down from here). The PDC fires ~100 rounds/s and impacts arrive in
/// blast-sized bursts, so those two are quiet; destruction and launch are the
/// punchy moments. Kept modest so nothing is harsh up close.
const TURRET_FIRE_VOLUME: f32 = 0.10;
const IMPACT_VOLUME: f32 = 0.22;
const EXPLOSION_VOLUME: f32 = 0.40;
const TORPEDO_LAUNCH_VOLUME: f32 = 0.45;

/// UI (non-positional) volumes for the lock/safety cues - informational
/// ticks, kept under the combat sounds.
const LOCK_ON_VOLUME: f32 = 0.30;
const LOCK_OFF_VOLUME: f32 = 0.28;
const SAFETY_ON_VOLUME: f32 = 0.30;
const RADAR_DENY_VOLUME: f32 = 0.26;

/// The salvage-pickup "ding". Deliberately quieter than the objective chime
/// (`OBJECTIVE_COMPLETE_VOLUME` 0.38 / `OBJECTIVE_NEW_VOLUME` 0.30) so a crate
/// pickup reads as a light per-item confirmation, not a beat completion (task
/// 20260714-090002). `pub` because the cue is fired from `nova_scenario`'s
/// salvage plugin (which owns [`SalvageCrateMarker`]), keeping every cue volume
/// defined here in the audio module.
pub const SALVAGE_PICKUP_VOLUME: f32 = 0.22;

/// UI/feedback volumes for the menu and turret/radar cues (task
/// 20260714-090006). All non-positional, kept in the informational-tick band
/// with the lock/safety cues. The retarget tick is the quietest - it can repeat
/// several times within one held gesture, so it must stay well under the
/// once-per-gesture `LOCK_ON_VOLUME` acquire cue. The dry-fire and retarget cues
/// are played here in this module (private), but `MENU_SELECT`/`UI_TOGGLE` are
/// fired from `nova_menu`, so those two are `pub` - keeping every cue volume
/// defined here in the audio module.
pub const MENU_SELECT_VOLUME: f32 = 0.28;
pub const UI_TOGGLE_VOLUME: f32 = 0.24;
const DRY_FIRE_VOLUME: f32 = 0.22;
const RADAR_RETARGET_VOLUME: f32 = 0.18;

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
/// The caller averages over each ship's active thrusters rather than summing, so
/// the hum tracks how hard that ship is burning instead of pinning to max the
/// moment more than one thruster fires. The clamp guards out-of-range input.
/// Pure for tests.
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
    play_positional_handle(commands, bank.get(key), base_volume, source, listener);
}

/// The handle-taking core of [`play_positional`]: same distance attenuation and
/// audible-threshold gate, but for an already-resolved [`Handle<AudioSource>`]
/// rather than a bank key. Lets a caller play a section's own authored sound
/// (a resolved [`AssetRef<AudioSource>`]) through the exact same positional path
/// the bank cues use.
fn play_positional_handle(
    commands: &mut Commands,
    handle: Handle<AudioSource>,
    base_volume: f32,
    source: Vec3,
    listener: Option<Vec3>,
) {
    let attenuation = listener.map_or(1.0, |l| distance_attenuation(l.distance(source)));
    let volume = base_volume * attenuation;
    if volume < SFX_AUDIBLE_THRESHOLD {
        return;
    }
    commands.play_sfx_volume(handle, volume);
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

        // Lock/safety UI cues (spike 20260713-110039): message-driven
        // one-shots, so no gating needed - the writers (radar search, tap
        // observer) are themselves pause-gated. The dry-fire click (task
        // 20260714-090006) polls turret input/ammo and edge-latches per turret;
        // a pause freezes the input so no fresh edge fires while paused.
        app.add_systems(
            Update,
            (play_lock_cues, play_safety_engaged_cue, play_dry_fire_cue),
        );

        // The thruster hum polls `ThrusterSectionInput`, so it must be gated to
        // the running simulation exactly like the thruster physics/shader. Joining
        // `SpaceshipSectionSystems` inherits whatever run condition consumers of
        // that input use - crucially nova_scenario's `run_if(scenario_is_live)`
        // - so the hum stays silent while building in the editor (no scenario is
        // loaded there) and plays wherever one is live, the main menu's ambience
        // backdrop included. (The one-shot cues need no gating: they fire on
        // spawn/damage/destroy events that only occur inside this same gated set.)
        app.init_resource::<ThrusterHumVolume>();
        app.add_systems(
            Update,
            (
                ensure_thruster_loop,
                compute_thruster_hum_volume,
                apply_thruster_loop_volume,
            )
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
///
/// The firing turret (named by `TurretSectionPartOf`) may declare its own
/// authored fire sound via [`TurretSectionConfig::fire_sound`], resolved at
/// spawn into a [`TurretSectionFireSound`] handle: when present that handle
/// plays instead of the global [`NovaSfx::TurretFire`] cue, so a modded turret
/// can ship its own weapon sound. Everything else (per-turret throttle key,
/// distance attenuation, positioning) is unchanged.
fn on_turret_fire_play_sfx(
    add: On<Add, TurretBulletProjectileMarker>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_projectile: Query<(&Transform, &TurretSectionPartOf)>,
    q_fire_sound: Query<&TurretSectionFireSound>,
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
        // Prefer the firing turret's own authored sound, resolving its AssetRef
        // here (idempotent - the asset server dedups by path, so a base turret's
        // `base/sounds/turret_fire.wav` yields the SAME handle the bank loaded;
        // only a mod's own sound diverges). Fall back to the bank cue when the
        // turret declared none.
        let handle = q_fire_sound
            .get(part_of.0)
            .ok()
            .and_then(|s| s.0.as_ref())
            .map(|r| r.resolve(&asset_server))
            .unwrap_or_else(|| bank.get(NovaSfx::TurretFire));
        play_positional_handle(
            &mut commands,
            handle,
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

/// The lock-gesture UI cues (non-positional one-shots, like the objective
/// cues): LockOn once per radar gesture ([`RadarLockAcquired`] already
/// fires acquire-only, Q3a), LockOff per cleared lock, the capability
/// deny buzz ([`RadarDenied`], F7/Q8a), and the subtle retarget tick
/// ([`RadarRetargeted`], task 20260714-090006). One cue per kind per frame - a
/// staged double-clear in one frame plays one LockOff, not a chord.
fn play_lock_cues(
    mut commands: Commands,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    mut acquired: MessageReader<RadarLockAcquired>,
    mut retargeted: MessageReader<RadarRetargeted>,
    mut cleared: MessageReader<LockClearedToast>,
    mut denied: MessageReader<RadarDenied>,
) {
    let Some(bank) = bank else {
        // No bank (headless tests, assets not loaded): drain quietly so the
        // cursors do not replay stale messages once it appears.
        acquired.read().for_each(|_| {});
        retargeted.read().for_each(|_| {});
        cleared.read().for_each(|_| {});
        denied.read().for_each(|_| {});
        return;
    };
    // DRAIN each reader (count, not next): a leftover unread message would
    // replay the cue on the NEXT frame.
    let acquired_now = acquired.read().count() > 0;
    let retargeted_now = retargeted.read().count() > 0;
    if acquired_now {
        commands.play_sfx_volume(bank.get(NovaSfx::LockOn), LOCK_ON_VOLUME);
    }
    // The acquire and a retarget can both land in the frames of one gesture, but
    // never the same frame for the same slot (acquire is the first resolve,
    // retarget every change after). Suppress the tick on the acquire frame
    // anyway so a gesture that resolves and immediately settles plays only the
    // richer LockOn, never LockOn + tick.
    if retargeted_now && !acquired_now {
        commands.play_sfx_volume(bank.get(NovaSfx::RadarRetarget), RADAR_RETARGET_VOLUME);
    }
    if cleared.read().count() > 0 {
        commands.play_sfx_volume(bank.get(NovaSfx::LockOff), LOCK_OFF_VOLUME);
    }
    if denied.read().count() > 0 {
        commands.play_sfx_volume(bank.get(NovaSfx::RadarDeny), RADAR_DENY_VOLUME);
    }
}

/// The safety re-engage click on the PLAYER's hot -> cold edge (a held
/// burst must not just silently stop - deferred from 20260713-082337, now
/// that the sfx batch exists). Changed-gated; the Local remembers the last
/// seen state so an unrelated change (spawn) cannot click.
fn play_safety_engaged_cue(
    mut commands: Commands,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    q_player: Query<&WeaponsHot, (With<PlayerSpaceshipMarker>, Changed<WeaponsHot>)>,
    mut was_hot: Local<bool>,
) {
    for hot in &q_player {
        let is_hot = hot.0;
        if *was_hot && !is_hot {
            if let Some(bank) = &bank {
                commands.play_sfx_volume(bank.get(NovaSfx::SafetyOn), SAFETY_ON_VOLUME);
            }
        }
        *was_hot = is_hot;
    }
}

/// The dry-fire click on the PLAYER's turrets (task 20260714-090006): when a
/// turret's trigger is held with weapons hot but its magazine is empty, the
/// shoot system silently blocks the shot (`shoot_spawn_projectile`, the empty
/// magazine `continue`). This gives that dead trigger a voice - a dull click on
/// the RISING EDGE of the empty-and-pulling state, so a held burst that runs dry
/// is not just silence.
///
/// Edge-latched per turret so holding an empty trigger clicks once, not every
/// frame; a release-and-re-pull clicks again. Player-only: `q_ship` is filtered
/// to `PlayerSpaceshipMarker`, so an AI turret running dry never reaches the cue
/// (it would otherwise click in the player's ear). A turret with no `SectionAmmo`
/// (unlimited ammo, e.g. the shakedown player) never dry-fires.
fn play_dry_fire_cue(
    mut commands: Commands,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    q_turret: Query<
        (Entity, &TurretSectionInput, Option<&SectionAmmo>, &ChildOf),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_ship: Query<&WeaponsHot, With<PlayerSpaceshipMarker>>,
    mut latched: Local<HashMap<Entity, bool>>,
) {
    for (turret, input, ammo, ChildOf(ship)) in &q_turret {
        // Dry-firing = trigger held, weapons hot, magazine present and empty, on
        // the player's ship. `q_ship` matches only the player, so a non-player
        // parent reads `hot == false` and never dry-fires.
        let hot = q_ship.get(*ship).is_ok_and(|weapons| weapons.0);
        let empty = ammo.is_some_and(SectionAmmo::is_empty);
        let dry = **input && hot && empty;
        let was = latched.entry(turret).or_insert(false);
        if dry && !*was {
            if let Some(bank) = &bank {
                commands.play_sfx_volume(bank.get(NovaSfx::DryFire), DRY_FIRE_VOLUME);
            }
        }
        *was = dry;
    }
}

/// Marker for the single looping engine-hum audio entity.
#[derive(Component)]
struct ThrusterLoopSfx;

/// Spawn the looping engine-hum entity once, after the sound bank exists. It
/// starts silent; [`apply_thruster_loop_volume`] raises its volume with the
/// thrust-driven target computed by [`compute_thruster_hum_volume`].
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

/// The live engine-hum volume, written by [`compute_thruster_hum_volume`] and
/// read by [`apply_thruster_loop_volume`]. Split from the `AudioSink` write so
/// the volume logic is App-testable headless - an `AudioSink` cannot be
/// constructed without an audio output device.
#[derive(Resource, Default, Debug)]
struct ThrusterHumVolume {
    /// Where the hum wants to be this frame: the loudest per-ship
    /// contribution, each `engine_volume(avg throttle) * distance
    /// attenuation`.
    target: f32,
    /// The smoothed volume actually applied to the sink, chasing `target`.
    smoothed: f32,
}

/// The entity a thruster's hum contribution is attributed to: its
/// [`SpaceshipRootMarker`] ancestor (one hum source per ship), or the thruster
/// itself when the walk leaves the tree without finding one (torpedo
/// thrusters hang off the projectile, not a ship root; bare rigs have no
/// parent at all), so a rootless thruster attenuates at its own pose.
fn hum_source_root(
    thruster: Entity,
    q_child_of: &Query<&ChildOf>,
    q_is_root: &Query<(), With<SpaceshipRootMarker>>,
) -> Entity {
    let mut entity = thruster;
    loop {
        if q_is_root.contains(entity) {
            return entity;
        }
        match q_child_of.get(entity) {
            Ok(&ChildOf(parent)) => entity = parent,
            Err(_) => return thruster,
        }
    }
}

/// Drive the engine-hum volume from how hard each ship is thrusting, scaled by
/// how far that ship is from the listener, smoothing toward the target so
/// throttle changes fade rather than click.
///
/// Per-ship attribution (task 20260711-183417): the throttle is averaged over
/// each ship's own active thrusters (summing would pin to max the moment more
/// than one fires), scaled by [`distance_attenuation`] from the listener to
/// that ship's root, and the loudest ship wins - so a distant AI ship's burn
/// no longer raises a full-volume hum in the player's ear. The global average
/// this replaces predated multiple audible ships.
///
/// The PLAYER's ship is exempt from attenuation: the camera rig sits 11-32 u
/// out depending on mode (Normal/FreeLook are already past
/// `SFX_NEAR_DISTANCE`) and the orbit survey dolly stretches it to
/// `SURVEY_MAX_DISTANCE` = 250 u, deep into the rolloff band - your own
/// engines must not fade out because the camera pulled back for the shot. A
/// missing listener falls back to no attenuation, mirroring
/// [`play_positional`].
fn compute_thruster_hum_volume(
    time: Res<Time>,
    q_thrusters: Query<
        (Entity, &ThrusterSectionInput),
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_pose: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut hum: ResMut<ThrusterHumVolume>,
) {
    let listener = listener_position(&q_camera);

    // Group the active thrusters' throttle by hum source (ship root or
    // rootless thruster): (sum, count) per source.
    let mut per_source: HashMap<Entity, (f32, u32)> = HashMap::new();
    for (thruster, input) in &q_thrusters {
        let source = hum_source_root(thruster, &q_child_of, &q_is_root);
        let slot = per_source.entry(source).or_insert((0.0, 0));
        slot.0 += input.0.abs();
        slot.1 += 1;
    }

    // Loudest ship wins. Max, not sum: distinct ships' hums do not stack the
    // single loop past its per-ship ceiling.
    let mut target = 0.0f32;
    for (source, (sum, count)) in &per_source {
        let avg_throttle = sum / *count as f32;
        let attenuation = if q_is_player.contains(*source) {
            1.0
        } else {
            match (listener, q_pose.get(*source)) {
                (Some(l), Ok(pose)) => distance_attenuation(l.distance(pose.translation())),
                // No listener or no pose: full volume, like play_positional.
                _ => 1.0,
            }
        };
        target = target.max(engine_volume(avg_throttle) * attenuation);
    }
    hum.target = target;

    // Exponential smoothing, framerate-independent: ~8 units/s of catch-up.
    let alpha = (time.delta_secs() * 8.0).clamp(0.0, 1.0);
    hum.smoothed += (target - hum.smoothed) * alpha;
}

/// Copy the computed hum volume onto the loop's sink. The `AudioSink` appears
/// a frame or two after the loop entity spawns, so this no-ops until then.
/// One delta from the pre-split code: `smoothed` keeps advancing while the
/// sink is absent, so a scene that loads with hot engines starts the loop at
/// the caught-up volume instead of fading up from silence - those first
/// frames have nothing to fade from, and a correct level beats a late ramp.
/// `master` is `Option` so audio-only test rigs that never add the settings
/// plugin keep full volume instead of panicking on a missing resource; the
/// loop is scaled by [`MasterVolume`] here because it sets its own sink volume
/// every frame and so bypasses the `GlobalVolume` path bevy applies to
/// freshly-spawned one-shot sinks.
fn apply_thruster_loop_volume(
    hum: Res<ThrusterHumVolume>,
    master: Option<Res<crate::settings::MasterVolume>>,
    mut q_sink: Query<&mut AudioSink, With<ThrusterLoopSfx>>,
) {
    let Ok(mut sink) = q_sink.single_mut() else {
        return;
    };
    let master = master.map(|m| m.factor()).unwrap_or(1.0);
    sink.set_volume(Volume::Linear(hum.smoothed * master));
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

    /// The handle of the last `PlaySfx` observed, so a test can assert WHICH
    /// sound played (not just that one did) - the discriminator between a
    /// section's own authored fire sound and the global bank cue.
    #[derive(Resource, Default)]
    struct LastPlayed(Option<Handle<AudioSource>>);

    /// App rig for the turret-fire cue: the real `on_turret_fire_play_sfx`
    /// observer with a loaded bank, capturing the played handle. No audio device
    /// needed (nothing constructs an `AudioSink`). The bank is built with
    /// `load_paths` + `base/sounds/` to mirror production `register_sounds`
    /// (nova_assets), so the bank's `TurretFire` handle is keyed by the exact
    /// path a base turret's ref resolves to.
    fn turret_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        let paths: Vec<(NovaSfx, String)> = NOVA_SFX_FILES
            .iter()
            .map(|(key, name)| (*key, format!("base/sounds/{name}.wav")))
            .collect();
        let bank = SoundBank::load_paths(
            app.world().resource::<AssetServer>(),
            paths.iter().map(|(key, path)| (*key, path.as_str())),
        );
        app.insert_resource(bank);
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        app
    }

    /// Spawn a turret round parented (by `TurretSectionPartOf`) to `turret`,
    /// firing the `On<Add, TurretBulletProjectileMarker>` cue observer.
    fn fire_round(app: &mut App, turret: Entity) {
        app.world_mut().spawn((
            TurretBulletProjectileMarker,
            Transform::default(),
            TurretSectionPartOf(turret),
        ));
        app.world_mut().flush();
    }

    #[test]
    fn a_turret_with_a_declared_fire_sound_plays_that_handle_not_the_bank() {
        // The section-authored audio path (task 20260717-002228): a turret
        // carrying a `TurretSectionFireSound(Some(AssetRef))` must have the cue
        // RESOLVE that ref and play its handle, not the global `NovaSfx::TurretFire`
        // bank cue - so a mod turret sounds like its own gun.
        let mut app = turret_fire_app();
        let bank_fire = app
            .world()
            .resource::<SoundBank<NovaSfx>>()
            .get(NovaSfx::TurretFire);
        // A distinct path standing in for a mod's own shipped sound; resolving it
        // (asset_server.load, same as the observer) yields a different handle than
        // the bank's, so the assertion is a real substitution.
        let mod_sound: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/sounds/railgun.wav");
        assert_ne!(mod_sound, bank_fire, "the rig's two handles must differ");

        let turret = app
            .world_mut()
            .spawn(TurretSectionFireSound(Some(AssetRef::from(
                "mods/x/sounds/railgun.wav",
            ))))
            .id();
        fire_round(&mut app, turret);

        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(mod_sound),
            "a turret with a declared fire_sound must resolve + play its own handle"
        );
    }

    #[test]
    fn a_turret_without_a_declared_fire_sound_falls_back_to_the_bank_cue() {
        // The unchanged-default guard: a turret that left `fire_sound` unset (no
        // `TurretSectionFireSound` component) must still fire the global bank cue,
        // so existing turrets are audibly identical. Also the delivery guard for
        // the test above - it proves the cue fires at all, so the override
        // assertion is a real substitution, not a silent no-op.
        let mut app = turret_fire_app();
        let bank_fire = app
            .world()
            .resource::<SoundBank<NovaSfx>>()
            .get(NovaSfx::TurretFire);

        let turret = app.world_mut().spawn_empty().id();
        fire_round(&mut app, turret);

        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(bank_fire),
            "a turret without a fire_sound must fall back to NovaSfx::TurretFire"
        );
    }

    /// App rig for the hum-volume computation: the real
    /// [`compute_thruster_hum_volume`] system over production markers, no
    /// audio device needed (the sink-apply half is split off for exactly
    /// this). Mirrors the production shape verified in the task's plan pass:
    /// thruster sections are `ChildOf` children of a `SpaceshipRootMarker`
    /// root (input/player.rs:186), torpedo thrusters are children of the
    /// projectile root with their own `GlobalTransform`
    /// (torpedo_section/projectile.rs).
    fn hum_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ThrusterHumVolume>();
        app.add_systems(Update, compute_thruster_hum_volume);
        app
    }

    fn spawn_listener_at(app: &mut App, pos: Vec3) {
        app.world_mut().spawn((
            SfxListenerMarker,
            GlobalTransform::from(Transform::from_translation(pos)),
        ));
    }

    /// A one-thruster ship at `pos`. The root carries the marker + pose; the
    /// thruster is a plain child, like the shipped assembly.
    fn spawn_burning_ship(app: &mut App, pos: Vec3, throttle: f32) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::from(Transform::from_translation(pos)),
            ))
            .id();
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(throttle),
            ChildOf(root),
        ));
        root
    }

    fn hum_target(app: &mut App) -> f32 {
        app.update();
        app.world().resource::<ThrusterHumVolume>().target
    }

    #[test]
    fn a_distant_ships_burn_does_not_raise_the_hum() {
        // The 2026-07-11 playtest bug: an AI ship burning at full throttle
        // beyond SFX_FAR_DISTANCE must contribute nothing, exactly like a
        // one-shot from the same distance.
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        let ship = spawn_burning_ship(&mut app, Vec3::new(500.0, 0.0, 0.0), 1.0);

        assert_eq!(
            hum_target(&mut app),
            0.0,
            "a ship 500 u away (FAR = {SFX_FAR_DISTANCE}) must be inaudible"
        );

        // Delivery guard for the null assertion (R1.2): the SAME ship moved
        // inside the rolloff band must be heard - proving the entity is
        // visible to the system and the zero above is attenuation at work,
        // not a rig the query never matched.
        app.world_mut()
            .entity_mut(ship)
            .insert(GlobalTransform::from(Transform::from_translation(
                Vec3::new(100.0, 0.0, 0.0),
            )));
        assert!(
            hum_target(&mut app) > 0.0,
            "the same ship inside the band must be audible"
        );
    }

    #[test]
    fn a_midrange_ships_hum_is_scaled_by_distance_attenuation() {
        // Expected value composed from the production helpers, not
        // re-derived: engine_volume x distance_attenuation at the ship's
        // distance.
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        spawn_burning_ship(&mut app, Vec3::new(170.0, 0.0, 0.0), 0.8);

        let expected = engine_volume(0.8) * distance_attenuation(170.0);
        let target = hum_target(&mut app);
        assert!(
            (target - expected).abs() < 1e-6,
            "midrange ship: got {target}, expected {expected}"
        );
        // The rolloff must actually bite for the assertion to mean anything.
        assert!(expected > 0.0 && expected < engine_volume(0.8));
    }

    #[test]
    fn the_players_own_burn_is_never_attenuated() {
        // The camera rig sits past SFX_NEAR_DISTANCE by design and the orbit
        // survey dolly stretches it to 250 u - the player's own engines must
        // not fade because the shot pulled back.
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::new(0.0, 0.0, 250.0));
        let ship = spawn_burning_ship(&mut app, Vec3::ZERO, 1.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(PlayerSpaceshipMarker);

        assert_eq!(
            hum_target(&mut app),
            ENGINE_MAX_VOLUME,
            "player ship at survey-dolly distance must stay at full hum"
        );
    }

    #[test]
    fn ships_combine_by_loudest_not_by_global_average() {
        // Two ships inside NEAR: a half-throttle player and a full-throttle
        // AI. The old global average would read 0.75; per-ship max must read
        // the full-throttle ship. Also pins that ships do not SUM past the
        // per-ship ceiling.
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        let player = spawn_burning_ship(&mut app, Vec3::new(5.0, 0.0, 0.0), 0.5);
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerSpaceshipMarker);
        spawn_burning_ship(&mut app, Vec3::new(0.0, 5.0, 0.0), 1.0);

        let target = hum_target(&mut app);
        assert_eq!(
            target,
            engine_volume(1.0),
            "loudest ship wins; global averaging would give {}",
            engine_volume(0.75)
        );
    }

    #[test]
    fn a_rootless_thruster_attenuates_at_its_own_pose() {
        // Torpedo shape: the thruster hangs off a projectile root that is NOT
        // a SpaceshipRootMarker, so it attributes to itself and attenuates at
        // its own GlobalTransform. Far torpedo: silent.
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        let torpedo = app.world_mut().spawn(GlobalTransform::default()).id();
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(1.0),
            ChildOf(torpedo),
            GlobalTransform::from(Transform::from_translation(Vec3::new(400.0, 0.0, 0.0))),
        ));
        assert_eq!(hum_target(&mut app), 0.0, "far torpedo thruster: silent");

        // And a near one is heard.
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(1.0),
            GlobalTransform::from(Transform::from_translation(Vec3::new(10.0, 0.0, 0.0))),
        ));
        assert_eq!(
            hum_target(&mut app),
            engine_volume(1.0),
            "near rootless thruster: full contribution"
        );
    }

    #[test]
    fn the_hum_smooths_toward_its_target_instead_of_jumping() {
        // The smoothing moved from a Local into the resource with the
        // compute/apply split; pin that it still eases instead of snapping.
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        spawn_burning_ship(&mut app, Vec3::ZERO, 1.0);

        app.update(); // first frame: dt = 0, smoothed stays put
        let mut last = app.world().resource::<ThrusterHumVolume>().smoothed;
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(4));
            app.update();
            let hum = app.world().resource::<ThrusterHumVolume>();
            assert!(
                hum.smoothed >= last && hum.smoothed <= hum.target,
                "smoothed must rise monotonically toward the target, got {} after {last}",
                hum.smoothed
            );
            last = hum.smoothed;
        }
        assert!(last > 0.0, "smoothed must have started chasing the target");
    }

    #[test]
    fn every_nova_sfx_key_has_a_file() {
        // Guards against adding a NovaSfx variant without a placeholder asset.
        use NovaSfx::*;
        for key in [
            ThrusterLoop,
            TurretFire,
            TorpedoLaunch,
            Explosion,
            Impact,
            ObjectiveNew,
            ObjectiveComplete,
            LockOn,
            LockOff,
            SafetyOn,
            RadarDeny,
            SalvagePickup,
            MenuSelect,
            UiToggle,
            DryFire,
            RadarRetarget,
        ] {
            assert!(
                NOVA_SFX_FILES.iter().any(|(k, _)| *k == key),
                "NovaSfx::{key:?} is missing from NOVA_SFX_FILES"
            );
        }
    }

    /// An App rig for the dry-fire cue: the real `play_dry_fire_cue` system with
    /// a loaded bank and a `PlaySfx` counter, no audio device needed.
    fn dry_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(SoundBank::load(
            app.world().resource::<AssetServer>(),
            NOVA_SFX_FILES,
        ));
        app.init_resource::<PlayedSfx>();
        app.add_systems(Update, play_dry_fire_cue);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app
    }

    fn dings(app: &App) -> usize {
        app.world().resource::<PlayedSfx>().0
    }

    #[test]
    fn dry_fire_clicks_on_the_empty_pull_edge_then_stays_quiet_while_held() {
        let mut app = dry_fire_app();
        let player = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(true)))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                TurretSectionInput(true),
                SectionAmmo::new(0),
                ChildOf(player),
            ))
            .id();

        // Trigger held on an empty magazine: one click on the rising edge.
        app.update();
        assert_eq!(dings(&app), 1, "the empty pull edge clicks once");

        // Still held: no repeat (the latch suppresses a per-frame buzz).
        app.update();
        assert_eq!(
            dings(&app),
            1,
            "holding an empty trigger does not machine-gun"
        );

        // Release then re-pull: a fresh edge clicks again.
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionInput(false));
        app.update();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionInput(true));
        app.update();
        assert_eq!(
            dings(&app),
            2,
            "a re-pull on an empty magazine clicks again"
        );
    }

    #[test]
    fn dry_fire_is_gated_to_the_player_hot_and_empty() {
        // Four turrets in one frame; only the player + hot + empty + held one
        // may click. The `== 1` is self-guarding: it is also the delivery guard
        // that the rig fires at all, so the three silent cases are real gates,
        // not a dead system.
        let mut app = dry_fire_app();
        let player_hot = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(true)))
            .id();
        let player_cold = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(false)))
            .id();
        // An AI ship: hot weapons, but no player marker.
        let ai = app.world_mut().spawn(WeaponsHot(true)).id();

        let held_empty = |app: &mut App, ship: Entity| {
            app.world_mut().spawn((
                TurretSectionMarker,
                TurretSectionInput(true),
                SectionAmmo::new(0),
                ChildOf(ship),
            ));
        };
        held_empty(&mut app, player_hot); // valid: clicks (delivery guard)
        held_empty(&mut app, player_cold); // gated: weapons cold
        held_empty(&mut app, ai); // gated: not the player
                                  // Player + hot but a LOADED magazine: gated on ammo.
        app.world_mut().spawn((
            TurretSectionMarker,
            TurretSectionInput(true),
            SectionAmmo::new(3),
            ChildOf(player_hot),
        ));

        app.update();
        assert_eq!(
            dings(&app),
            1,
            "only the player's hot, empty, held turret dry-fires"
        );
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
