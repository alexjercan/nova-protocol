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
//! - a turret round spawned -> the firing turret's authored `fire_sound`
//!   (`On<Add, TurretBulletProjectileMarker>`, authored-or-silent);
//! - a torpedo spawned -> the bay's authored `launch_sound`
//!   (`On<Add, TorpedoProjectileMarker>`, authored-or-silent).
//!
//! The fifth cue, the thruster engine hum, is continuous: one looping audio
//! entity per DISTINCT authored `loop_sound` (task 20260717-101650 - thrusters
//! sharing a sound share a loop), each tracking how hard the ships burning
//! that sound are thrusting.
//!
//! The four one-shots are **distance-attenuated**: their volume is scaled by how
//! far the event is from the listener (the camera carrying
//! [`SfxListenerMarker`], i.e. the gameplay camera), so a distant
//! explosion is quieter than one next to you. This is a volume-only rolloff for
//! the cinematic feel, not true spatialization - stereo panning would need bevy
//! spatial audio (`SpatialListener` + `spatial: true`) and is a future step. The
//! thruster hum attenuates per SHIP: each ship's throttle-driven contribution is
//! scaled by its root's distance to the listener and the loudest wins PER HUM
//! SOUND, except
//! the player's own ship, which is never attenuated (the camera rig sits 11-32 u
//! out by mode and the orbit survey dolly stretches it to 250 u, deep in the
//! rolloff band; see `compute_thruster_hum_volume`).
//!
//! The [`SoundBank<UiSfx>`] resource is inserted by `nova_assets` once assets
//! load; every system here degrades gracefully (does nothing) until the
//! resources it needs exist. World sounds carry no bank at all - each cue
//! resolves its target's authored `AssetRef` (authored-or-silent).

use std::collections::HashMap;

use bevy::{audio::Volume, prelude::*};

use crate::{
    prelude::*,
    sections::{
        controller_section::ControllerSectionSounds,
        thruster_section::ThrusterSectionLoopSound,
        torpedo_section::{TorpedoSectionLaunchSound, TorpedoSectionSpawnerEntity},
        turret_section::{TurretSectionDryFireSound, TurretSectionFireSound, TurretSectionPartOf},
    },
};

/// Keys for the game's UI/interface sound effects - engine chrome, like
/// `assets/icons/`: loaded from the root `assets/sounds/`, NOT part of any mod
/// and never referenceable by content (spike 20260717-101524). Everything a
/// player would call "the interface" lives here; every world/gameplay sound is
/// mod content, authored on its owning section/object config as an
/// `AssetRef<AudioSource>` field (spike 20260717-101524's end state - the
/// transitional WorldSfx bank is gone).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UiSfx {
    /// A new objective was posted to the panel (non-positional).
    ObjectiveNew,
    /// An objective was completed (non-positional).
    ObjectiveComplete,
    /// A menu button was pressed (New Game / Sandbox / Settings / Exit and the
    /// pause/mods buttons) - a crisp UI click (task 20260714-090006). Fired from
    /// `nova_menu`'s global `On<Activate>` observer.
    MenuSelect,
    /// A comms line just SHOWED on the panel (task 20260717-163033) - a
    /// soft radio blip so a story beat registers mid-fight. PLACEHOLDER
    /// AUDIO: reuses ui_toggle.wav until real comms art lands (distinct
    /// key so the swap is one file-map line).
    CommsLine,
    /// A pause overlay open/close toggle via ESC - a soft two-state UI blip
    /// (task 20260714-090006).
    UiToggle,
}

/// The `(key, base-filename)` pairs for the UI bank. Loaded by
/// `nova_assets::register_sounds` via `SoundBank::load`, whose
/// `sounds/<name>.wav` convention maps these to the root `assets/sounds/` -
/// engine chrome, outside every mod.
pub const UI_SFX_FILES: [(UiSfx, &str); 5] = [
    (UiSfx::ObjectiveNew, "objective_new"),
    (UiSfx::ObjectiveComplete, "objective_complete"),
    (UiSfx::MenuSelect, "menu_select"),
    (UiSfx::UiToggle, "ui_toggle"),
    // Placeholder file (see the key's doc): swap for real comms art.
    (UiSfx::CommsLine, "ui_toggle"),
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
/// salvage plugin (which owns `SalvageCrateMarker`), keeping every cue volume
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

/// Loudest the RCS fine-adjust loop ever gets (at full-deflection intent).
/// Below [`ENGINE_MAX_VOLUME`]: RCS is a gentle nudge, not the main drive.
const RCS_MAX_VOLUME: f32 = 0.22;

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

/// RCS fine-adjust loop volume from the ship's `RcsIntent` magnitude (the burn
/// effort, ~0..1 per axis; a diagonal command can exceed 1, hence the clamp).
/// A touch quieter than the main-drive hum ([`RCS_MAX_VOLUME`] < ENGINE_MAX):
/// RCS is a gentle station-keeping push, not a burn. Pure for tests.
fn rcs_volume(effort: f32) -> f32 {
    effort.clamp(0.0, 1.0) * RCS_MAX_VOLUME
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
        app.add_systems(OnEnter(crate::PauseStates::Paused), pause_loops);
        app.add_systems(OnExit(crate::PauseStates::Paused), resume_loops);

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
                ensure_thruster_loops,
                compute_thruster_hum_volume,
                apply_thruster_loop_volume,
            )
                .chain()
                .in_set(SpaceshipSectionSystems),
        );

        // The RCS fine-adjust loop (task 20260718-201532) polls `RcsIntent`,
        // written by the player modal and the autopilot both, so it joins the
        // same scenario-gated set as the thruster hum for the same reasons
        // (silent in the editor, muted on pause).
        app.init_resource::<RcsLoopVolume>();
        app.add_systems(
            Update,
            (
                ensure_rcs_loops,
                compute_rcs_loop_volume,
                apply_rcs_loop_volume,
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

/// Find the nearest [`ImpactDestroySounds`] on `entity` or an ancestor. The
/// damage/destroy observers' target is the entity carrying Health - for
/// sections that IS the section entity, but an asteroid keeps its Health on a
/// child node while the sounds snapshot sits on the rock's parent bundle, so
/// the lookup walks up (bounded by the hierarchy, like `hum_source_root`).
fn impact_destroy_sounds<'a>(
    entity: Entity,
    q_sounds: &'a Query<&ImpactDestroySounds>,
    q_child_of: &Query<&ChildOf>,
) -> Option<&'a ImpactDestroySounds> {
    let mut current = entity;
    loop {
        if let Ok(sounds) = q_sounds.get(current) {
            return Some(sounds);
        }
        match q_child_of.get(current) {
            Ok(&ChildOf(parent)) => current = parent,
            Err(_) => return None,
        }
    }
}

/// Explosion cue on any destruction (section, asteroid, or torpedo detonation,
/// which all funnel through `IntegrityDestroyMarker`).
fn on_destroyed_play_explosion(
    add: On<Add, IntegrityDestroyMarker>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_sounds: Query<&ImpactDestroySounds>,
    q_child_of: Query<&ChildOf>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    // The destroyed entity has existed for frames, so its GlobalTransform is
    // valid world-space.
    let Ok(source) = q_transform.get(add.entity) else {
        return;
    };
    // AUTHORED-OR-SILENT (spike 20260717-101524): the destruction voice is the
    // TARGET's authored destroy_sound (per-target = per-material), found on the
    // entity or an ancestor (asteroid node shape) and resolved here.
    let Some(handle) = impact_destroy_sounds(add.entity, &q_sounds, &q_child_of)
        .and_then(|s| s.destroy.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Explosion(area_cell(pos)),
        time.elapsed_secs(),
        EXPLOSION_MIN_INTERVAL,
    ) {
        play_positional_handle(
            &mut commands,
            handle,
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
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_sounds: Query<&ImpactDestroySounds>,
    q_child_of: Query<&ChildOf>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Ok(source) = q_transform.get(damage.entity) else {
        return;
    };
    // AUTHORED-OR-SILENT: the hit voice is the TARGET's authored impact_sound
    // (per-target = per-material), found on the entity or an ancestor.
    let Some(handle) = impact_destroy_sounds(damage.entity, &q_sounds, &q_child_of)
        .and_then(|s| s.impact.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Impact(area_cell(pos)),
        time.elapsed_secs(),
        IMPACT_MIN_INTERVAL,
    ) {
        play_positional_handle(
            &mut commands,
            handle,
            IMPACT_VOLUME,
            pos,
            listener_position(&q_camera),
        );
    }
}

/// Turret-fire cue when a round spawns. Throttled hard because the PDC fires at
/// a high rate.
///
/// AUTHORED-OR-SILENT (spike 20260717-101524): the sound is the firing turret's
/// [`TurretSectionConfig::fire_sound`], snapshotted at spawn as
/// [`TurretSectionFireSound`] and resolved here - content owns the sound, and a
/// turret that authors none fires silently (every base turret authors it via
/// gen_content, so the shipped game is unchanged; the old global bank fallback
/// is gone with its `WorldSfx::TurretFire` key). Everything else (per-turret
/// throttle key, distance attenuation, positioning) is unchanged.
fn on_turret_fire_play_sfx(
    add: On<Add, TurretBulletProjectileMarker>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_projectile: Query<(&Transform, &TurretSectionPartOf)>,
    q_fire_sound: Query<&TurretSectionFireSound>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    // The projectile is a freshly-spawned ROOT entity, so its GlobalTransform is
    // still identity this frame; its local Transform is already world-space.
    // `TurretSectionPartOf` names the firing turret, so each gun throttles on its
    // own key - the fix for "only one of several guns is audible".
    let Ok((transform, part_of)) = q_projectile.get(add.entity) else {
        return;
    };
    // No authored sound -> silent (still stamp the throttle key? No: an
    // unauthored turret plays nothing, so there is nothing to rate-limit).
    let Some(handle) = q_fire_sound
        .get(part_of.0)
        .ok()
        .and_then(|s| s.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    if throttle_state.allow(
        ThrottleKey::TurretFire(part_of.0),
        time.elapsed_secs(),
        TURRET_FIRE_MIN_INTERVAL,
    ) {
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
///
/// AUTHORED-OR-SILENT (spike 20260717-101524): the sound is the firing bay's
/// [`TorpedoSectionConfig::launch_sound`], snapshotted onto the bay's spawner
/// as [`TorpedoSectionLaunchSound`] and reached from the projectile via its
/// [`TorpedoSectionSpawnerEntity`] back-ref (the same path the launch flash
/// effect takes). A bay that authors none launches silently; base bays author
/// it via gen_content, so the shipped game is unchanged.
fn on_torpedo_launch_play_sfx(
    add: On<Add, TorpedoProjectileMarker>,
    asset_server: Res<AssetServer>,
    q_projectile: Query<(&Transform, &TorpedoSectionSpawnerEntity)>,
    q_launch_sound: Query<&TorpedoSectionLaunchSound>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut commands: Commands,
) {
    // Freshly-spawned root entity: use local Transform (== world) this frame.
    let Ok((source, spawner)) = q_projectile.get(add.entity) else {
        return;
    };
    let Some(handle) = q_launch_sound
        .get(spawner.0)
        .ok()
        .and_then(|s| s.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    play_positional_handle(
        &mut commands,
        handle,
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
/// The PLAYER ship's controller sounds: the first controller section whose
/// `ChildOf` parent carries [`PlayerSpaceshipMarker`]. The radar/lock/safety
/// messages are player-scoped (no entity payload), so this lookup names the
/// computer whose authored voice plays them. `None` when no player controller
/// exists (menu, editor, tests) - the cues stay silent, and readers must still
/// drain.
fn player_controller_sounds<'a>(
    q_controller: &'a Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player: &Query<(), With<PlayerSpaceshipMarker>>,
) -> Option<&'a ControllerSectionSounds> {
    q_controller
        .iter()
        .find(|(_, ChildOf(ship))| q_player.contains(*ship))
        .map(|(sounds, _)| sounds)
}

fn play_lock_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_controller: Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut acquired: MessageReader<RadarLockAcquired>,
    mut retargeted: MessageReader<RadarRetargeted>,
    mut cleared: MessageReader<LockClearedToast>,
    mut denied: MessageReader<RadarDenied>,
) {
    // DRAIN each reader unconditionally (count, not next): a leftover unread
    // message would replay the cue on the NEXT frame - and with no player
    // controller (menu, editor, headless tests) the cues are silent but the
    // cursors must still advance (the old no-bank drain, same reason).
    let acquired_now = acquired.read().count() > 0;
    let retargeted_now = retargeted.read().count() > 0;
    let cleared_now = cleared.read().count() > 0;
    let denied_now = denied.read().count() > 0;
    let Some(sounds) = player_controller_sounds(&q_controller, &q_player) else {
        return;
    };
    // AUTHORED-OR-SILENT (spike 20260717-101524): each cue plays the player
    // controller's own authored ref, resolved here; an unauthored cue is
    // silent. Base controllers author all of them via gen_content.
    let mut play = |ref_opt: &Option<AssetRef<AudioSource>>, volume: f32| {
        if let Some(handle) = ref_opt.as_ref().map(|r| r.resolve(&asset_server)) {
            commands.play_sfx_volume(handle, volume);
        }
    };
    if acquired_now {
        play(&sounds.lock_on, LOCK_ON_VOLUME);
    }
    // The acquire and a retarget can both land in the frames of one gesture, but
    // never the same frame for the same slot (acquire is the first resolve,
    // retarget every change after). Suppress the tick on the acquire frame
    // anyway so a gesture that resolves and immediately settles plays only the
    // richer LockOn, never LockOn + tick.
    if retargeted_now && !acquired_now {
        play(&sounds.radar_retarget, RADAR_RETARGET_VOLUME);
    }
    if cleared_now {
        play(&sounds.lock_off, LOCK_OFF_VOLUME);
    }
    if denied_now {
        play(&sounds.radar_deny, RADAR_DENY_VOLUME);
    }
}

/// The safety re-engage click on the PLAYER's hot -> cold edge (a held
/// burst must not just silently stop - deferred from 20260713-082337, now
/// that the sfx batch exists). Changed-gated; the Local remembers the last
/// seen state so an unrelated change (spawn) cannot click.
fn play_safety_engaged_cue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_controller: Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player_sounds: Query<(), With<PlayerSpaceshipMarker>>,
    q_player: Query<&WeaponsHot, (With<PlayerSpaceshipMarker>, Changed<WeaponsHot>)>,
    mut was_hot: Local<bool>,
) {
    for hot in &q_player {
        let is_hot = hot.0;
        if *was_hot && !is_hot {
            // AUTHORED-OR-SILENT: the click is the player controller's own
            // authored safety_on ref (the weapons computer's voice).
            if let Some(handle) = player_controller_sounds(&q_controller, &q_player_sounds)
                .and_then(|sounds| sounds.safety_on.as_ref())
                .map(|r| r.resolve(&asset_server))
            {
                commands.play_sfx_volume(handle, SAFETY_ON_VOLUME);
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
/// AUTHORED-OR-SILENT (spike 20260717-101524): the click is the turret's own
/// [`TurretSectionConfig::dry_fire_sound`] (snapshotted as
/// [`TurretSectionDryFireSound`], resolved here); a turret that authors none
/// runs dry silently. The edge latch still advances for every turret so an
/// authored sound added later (live edit) does not replay a stale edge.
fn play_dry_fire_cue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_turret: Query<
        (
            Entity,
            &TurretSectionInput,
            Option<&SectionAmmo>,
            Option<&TurretSectionDryFireSound>,
            &ChildOf,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_ship: Query<&WeaponsHot, With<PlayerSpaceshipMarker>>,
    mut latched: Local<HashMap<Entity, bool>>,
) {
    for (turret, input, ammo, dry_sound, ChildOf(ship)) in &q_turret {
        // Dry-firing = trigger held, weapons hot, magazine present and empty, on
        // the player's ship. `q_ship` matches only the player, so a non-player
        // parent reads `hot == false` and never dry-fires.
        let hot = q_ship.get(*ship).is_ok_and(|weapons| weapons.0);
        let empty = ammo.is_some_and(SectionAmmo::is_empty);
        let dry = **input && hot && empty;
        let was = latched.entry(turret).or_insert(false);
        if dry && !*was {
            if let Some(handle) = dry_sound
                .and_then(|s| s.0.as_ref())
                .map(|r| r.resolve(&asset_server))
            {
                commands.play_sfx_volume(handle, DRY_FIRE_VOLUME);
            }
        }
        *was = dry;
    }
}

/// Marker for one looping engine-hum audio entity, keyed by the resolved
/// [`Handle<AudioSource>`] it loops (one entity per DISTINCT authored hum;
/// task 20260717-101650). Entities persist for the session like the old
/// single loop did - a hum that goes quiet holds volume 0.
#[derive(Component)]
struct ThrusterLoopSfx(Handle<AudioSource>);

/// Spawn a looping engine-hum entity for every hum handle the compute pass
/// discovered that has no loop entity yet. Each starts silent;
/// [`apply_thruster_loop_volume`] raises it with its handle's smoothed level.
/// `PlaybackSettings::LOOP` keeps it playing for the whole session.
fn ensure_thruster_loops(
    hum: Res<ThrusterHumVolume>,
    existing: Query<&ThrusterLoopSfx>,
    mut commands: Commands,
) {
    for handle in hum.hums.keys() {
        if existing.iter().any(|sfx| sfx.0 == *handle) {
            continue;
        }
        commands.spawn((
            Name::new("Thruster Loop Sfx"),
            ThrusterLoopSfx(handle.clone()),
            AudioPlayer(handle.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
    }
}

/// One hum's live volume pair: where it wants to be this frame and the
/// smoothed level chasing it.
#[derive(Default, Debug)]
struct HumLevels {
    /// The loudest per-ship contribution for this handle, each
    /// `engine_volume(avg throttle) * distance attenuation`.
    target: f32,
    /// The smoothed volume actually applied to the sink, chasing `target`.
    smoothed: f32,
}

/// The live engine-hum volumes PER RESOLVED HANDLE, written by
/// [`compute_thruster_hum_volume`] and read by [`apply_thruster_loop_volume`].
/// Split from the `AudioSink` write so the volume logic is App-testable
/// headless - an `AudioSink` cannot be constructed without an audio output
/// device. Entries persist once seen (bounded by the session's distinct
/// authored hums); a handle nobody burns smooths down to 0.
#[derive(Resource, Default, Debug)]
struct ThrusterHumVolume {
    hums: HashMap<Handle<AudioSource>, HumLevels>,
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
    asset_server: Res<AssetServer>,
    q_thrusters: Query<
        (Entity, &ThrusterSectionInput, &ThrusterSectionLoopSound),
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

    // Group the active AUTHORED thrusters' throttle by (hum handle, source):
    // (sum, count) per pair. AUTHORED-OR-SILENT (spike 20260717-101524): a
    // thruster with no loop_sound contributes to no hum. Resolving here is
    // idempotent (the asset server dedups by path), so thrusters authoring the
    // same ref share one handle and one loop entity.
    #[allow(clippy::type_complexity)]
    let mut per_pair: HashMap<(Handle<AudioSource>, Entity), (f32, u32)> = HashMap::new();
    for (thruster, input, loop_sound) in &q_thrusters {
        let Some(handle) = loop_sound.0.as_ref().map(|r| r.resolve(&asset_server)) else {
            continue;
        };
        let source = hum_source_root(thruster, &q_child_of, &q_is_root);
        let slot = per_pair.entry((handle, source)).or_insert((0.0, 0));
        slot.0 += input.0.abs();
        slot.1 += 1;
    }

    // Per handle: loudest ship wins. Max, not sum: distinct ships burning the
    // SAME hum do not stack its loop past the per-ship ceiling; DIFFERENT hums
    // are independent loops and may sound together.
    let mut targets: HashMap<Handle<AudioSource>, f32> = HashMap::new();
    for ((handle, source), (sum, count)) in &per_pair {
        let avg_throttle = sum / *count as f32;
        let attenuation = if q_is_player.contains(*source) {
            1.0
        } else {
            match (listener, q_pose.get(*source)) {
                (Some(l), Ok(pose)) => distance_attenuation(l.distance(pose.translation())),
                // No listener or no pose: full volume, like the one-shots.
                _ => 1.0,
            }
        };
        let level = engine_volume(avg_throttle) * attenuation;
        let slot = targets.entry(handle.clone()).or_insert(0.0);
        *slot = slot.max(level);
    }

    // Fold into the persistent map: unseen handles keep an entry targeting 0
    // (their loop smooths down and idles), new handles join. Exponential
    // smoothing per handle, framerate-independent: ~8 units/s of catch-up.
    let alpha = (time.delta_secs() * 8.0).clamp(0.0, 1.0);
    for levels in hum.hums.values_mut() {
        levels.target = 0.0;
    }
    for (handle, target) in targets {
        hum.hums.entry(handle).or_default().target = target;
    }
    for levels in hum.hums.values_mut() {
        levels.smoothed += (levels.target - levels.smoothed) * alpha;
    }
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
    mute: Option<Res<crate::settings::HarnessMute>>,
    mut q_sink: Query<(&mut AudioSink, &ThrusterLoopSfx)>,
) {
    let mute = mute.map(|m| *m).unwrap_or_default();
    let master = master.map(|m| m.output_gain(mute)).unwrap_or(1.0);
    for (mut sink, sfx) in &mut q_sink {
        let smoothed = hum.hums.get(&sfx.0).map(|l| l.smoothed).unwrap_or(0.0);
        sink.set_volume(Volume::Linear(smoothed * master));
    }
}

/// Marker for one looping RCS-hiss audio entity, keyed by the resolved
/// [`Handle<AudioSource>`] it loops (one entity per DISTINCT authored
/// controller `rcs_loop`, mirroring [`ThrusterLoopSfx`]). Persists for the
/// session; an idle loop holds volume 0.
#[derive(Component)]
struct RcsLoopSfx(Handle<AudioSource>);

/// The live RCS-loop volumes PER RESOLVED HANDLE, written by
/// [`compute_rcs_loop_volume`] and read by [`apply_rcs_loop_volume`]. Split from
/// the `AudioSink` write so the volume logic stays headless-testable, exactly
/// like [`ThrusterHumVolume`]. Reuses [`HumLevels`] (target + smoothed).
#[derive(Resource, Default, Debug)]
struct RcsLoopVolume {
    loops: HashMap<Handle<AudioSource>, HumLevels>,
}

/// Spawn a looping RCS-hiss entity for every handle the compute pass discovered
/// without a loop yet. Each starts silent; [`apply_rcs_loop_volume`] raises it.
fn ensure_rcs_loops(vol: Res<RcsLoopVolume>, existing: Query<&RcsLoopSfx>, mut commands: Commands) {
    for handle in vol.loops.keys() {
        if existing.iter().any(|sfx| sfx.0 == *handle) {
            continue;
        }
        commands.spawn((
            Name::new("RCS Loop Sfx"),
            RcsLoopSfx(handle.clone()),
            AudioPlayer(handle.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
    }
}

/// Drive the RCS-loop volume from how hard each ship is fine-adjusting - the
/// `RcsIntent` magnitude on the ship root, resolved through each live controller
/// section's authored `rcs_loop` handle. CONTROLLER-based and DRIVER-agnostic:
/// the intent is written by the player's SHIFT modal OR the autopilot (ORBIT
/// trim, STOP/GOTO settle), so both make the same sound. Gated on the controller
/// granting [`FlightVerb::Rcs`], mirroring `rcs_burn_system` - a hull that cannot
/// RCS makes no RCS hiss. Per-ship attribution, loudest-wins-per-handle,
/// distance attenuation (player exempt) and exponential smoothing all match
/// [`compute_thruster_hum_volume`].
fn compute_rcs_loop_volume(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    q_controllers: Query<
        (&ChildOf, &ControllerSectionSounds, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_intent: Query<&RcsIntent>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_pose: Query<&GlobalTransform>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut vol: ResMut<RcsLoopVolume>,
) {
    let listener = listener_position(&q_camera);

    // Per handle: the loudest ship burning that authored rcs_loop wins.
    let mut targets: HashMap<Handle<AudioSource>, f32> = HashMap::new();
    for (&ChildOf(root), sounds, withheld) in &q_controllers {
        // Same capability gate as rcs_burn_system: no Rcs verb, no hiss.
        if !withheld.is_none_or(|w| w.granted(FlightVerb::Rcs)) {
            continue;
        }
        // AUTHORED-OR-SILENT: a controller with no rcs_loop makes no sound.
        let Some(handle) = sounds.rcs_loop.as_ref().map(|r| r.resolve(&asset_server)) else {
            continue;
        };
        // The burn effort is the ship-root intent both drivers write.
        let Ok(intent) = q_intent.get(root) else {
            continue;
        };
        let effort = intent.0.length();
        if effort <= 1e-4 {
            continue;
        }
        let attenuation = if q_is_player.contains(root) {
            1.0
        } else {
            match (listener, q_pose.get(root)) {
                (Some(l), Ok(pose)) => distance_attenuation(l.distance(pose.translation())),
                _ => 1.0,
            }
        };
        let level = rcs_volume(effort) * attenuation;
        let slot = targets.entry(handle).or_insert(0.0);
        *slot = slot.max(level);
    }

    let alpha = (time.delta_secs() * 8.0).clamp(0.0, 1.0);
    for levels in vol.loops.values_mut() {
        levels.target = 0.0;
    }
    for (handle, target) in targets {
        vol.loops.entry(handle).or_default().target = target;
    }
    for levels in vol.loops.values_mut() {
        levels.smoothed += (levels.target - levels.smoothed) * alpha;
    }
}

/// Copy the computed RCS-loop volume onto the loop's sink. Mirrors
/// [`apply_thruster_loop_volume`] (no-ops until the sink appears; scales by
/// [`MasterVolume`] because it sets its own sink volume every frame).
fn apply_rcs_loop_volume(
    vol: Res<RcsLoopVolume>,
    master: Option<Res<crate::settings::MasterVolume>>,
    mute: Option<Res<crate::settings::HarnessMute>>,
    mut q_sink: Query<(&mut AudioSink, &RcsLoopSfx)>,
) {
    let mute = mute.map(|m| *m).unwrap_or_default();
    let master = master.map(|m| m.output_gain(mute)).unwrap_or(1.0);
    for (mut sink, sfx) in &mut q_sink {
        let smoothed = vol.loops.get(&sfx.0).map(|l| l.smoothed).unwrap_or(0.0);
        sink.set_volume(Volume::Linear(smoothed * master));
    }
}

/// Silence the engine loop while the pause overlay is up; one-shot SFX are
/// naturally quiet then (no events fire in a frozen sim).
/// Pause every looping SFX sink (thruster hum + RCS hiss) behind the pause
/// overlay - audio sinks do not follow `Time<Virtual>`, so without this a loop
/// keeps roaring at its last volume while the game is frozen.
fn pause_loops(
    q_thruster: Query<&AudioSink, With<ThrusterLoopSfx>>,
    q_rcs: Query<&AudioSink, With<RcsLoopSfx>>,
) {
    for sink in &q_thruster {
        sink.pause();
    }
    for sink in &q_rcs {
        sink.pause();
    }
}

fn resume_loops(
    q_thruster: Query<&AudioSink, With<ThrusterLoopSfx>>,
    q_rcs: Query<&AudioSink, With<RcsLoopSfx>>,
) {
    for sink in &q_thruster {
        sink.play();
    }
    for sink in &q_rcs {
        sink.play();
    }
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
            .spawn((
                GlobalTransform::default(),
                ChildOf(parent),
                // Authored-or-silent: the rig's target must author its impact
                // voice for the cue to fire at all (task 20260717-101641).
                ImpactDestroySounds {
                    impact: Some(AssetRef::from("base/sounds/impact.wav")),
                    destroy: None,
                },
            ))
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
    /// observer, capturing the played handle. No audio device needed (nothing
    /// constructs an `AudioSink`), and no bank: the cue is authored-or-silent,
    /// resolving the turret's own snapshot against the `AssetServer`.
    fn turret_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
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
    fn a_turret_with_a_declared_fire_sound_plays_that_handle() {
        // The section-authored audio path (tasks 20260717-002228/-101624): a
        // turret carrying a `TurretSectionFireSound(Some(AssetRef))` must have
        // the cue RESOLVE that ref and play its handle - a mod turret sounds
        // like its own gun. Delivery guard for the silent test below.
        let mut app = turret_fire_app();
        let mod_sound: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/sounds/railgun.wav");

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
    fn a_turret_without_a_declared_fire_sound_fires_silently() {
        // Authored-or-silent (spike 20260717-101524): no snapshot (or a `None`
        // one) means NO sound - the old global bank fallback is gone with the
        // `WorldSfx::TurretFire` key. The authored test above is the delivery
        // guard proving this rig's cue path plays when a sound exists, so this
        // silence is the gate at work, not a dead rig.
        let mut app = turret_fire_app();

        let bare = app.world_mut().spawn_empty().id();
        fire_round(&mut app, bare);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "no snapshot -> silent"
        );

        let unauthored = app.world_mut().spawn(TurretSectionFireSound(None)).id();
        fire_round(&mut app, unauthored);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "a None snapshot (config left fire_sound unset) -> silent"
        );
    }

    #[test]
    fn a_torpedo_bay_with_a_declared_launch_sound_plays_it_and_silent_without() {
        // Same authored-or-silent seam for the torpedo bay (task
        // 20260717-101624): the projectile reaches the bay's spawner via its
        // `TorpedoSectionSpawnerEntity` back-ref; an authored
        // `TorpedoSectionLaunchSound` plays, an unauthored bay is silent. The
        // authored half doubles as the delivery guard.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_torpedo_launch_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let expected: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/torpedo_launch.wav");

        let authored = app
            .world_mut()
            .spawn(TorpedoSectionLaunchSound(Some(AssetRef::from(
                "base/sounds/torpedo_launch.wav",
            ))))
            .id();
        app.world_mut().spawn((
            TorpedoProjectileMarker,
            Transform::default(),
            TorpedoSectionSpawnerEntity(authored),
        ));
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "an authored launch_sound must resolve + play"
        );

        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let silent = app.world_mut().spawn(TorpedoSectionLaunchSound(None)).id();
        app.world_mut().spawn((
            TorpedoProjectileMarker,
            Transform::default(),
            TorpedoSectionSpawnerEntity(silent),
        ));
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "an unauthored bay launches silently"
        );
    }

    #[test]
    fn impact_and_destroy_play_the_targets_authored_sounds_or_stay_silent() {
        // Per-target voices (task 20260717-101641): the hit/destroyed entity's
        // own authored refs play; an unauthored target is silent. The authored
        // half is the delivery guard for the silent half.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_damage_play_impact);
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let thud: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/thud.wav");
        let boom: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/boom.wav");

        // Authored target: impact plays ITS thud.
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                ImpactDestroySounds {
                    impact: Some(AssetRef::from("mods/x/thud.wav")),
                    destroy: Some(AssetRef::from("mods/x/boom.wav")),
                },
            ))
            .id();
        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 1.0,
        });
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(thud));

        // Destruction plays ITS boom (different cell so the throttle is clean).
        app.world_mut()
            .entity_mut(target)
            .insert(GlobalTransform::from(Transform::from_translation(
                Vec3::splat(SFX_AREA_CELL * 10.0),
            )));
        app.world_mut()
            .entity_mut(target)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(boom));

        // Unauthored target: both cues silent.
        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let silent = app
            .world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(
                Vec3::splat(SFX_AREA_CELL * 20.0),
            )))
            .id();
        app.world_mut().trigger(HealthApplyDamage {
            entity: silent,
            source: None,
            amount: 1.0,
        });
        app.world_mut()
            .entity_mut(silent)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "an unauthored target is silent for both cues"
        );
    }

    #[test]
    fn the_sound_lookup_walks_up_to_the_asteroid_parent() {
        // The asteroid shape: Health (and the destroy marker) live on a CHILD
        // node while ImpactDestroySounds sits on the rock's parent bundle - the
        // observers must find it by walking up (task 20260717-101641).
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let crack: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/explosion.wav");

        let rock = app
            .world_mut()
            .spawn(ImpactDestroySounds {
                impact: None,
                destroy: Some(AssetRef::from("base/sounds/explosion.wav")),
            })
            .id();
        let node = app
            .world_mut()
            .spawn((GlobalTransform::default(), ChildOf(rock)))
            .id();
        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(crack),
            "the destroy cue must find the parent's authored sound via the walk"
        );
    }

    /// App rig for the lock/safety controller cues: the real systems with a
    /// `PlaySfx` capture. No bank - the cues resolve the player controller's
    /// authored refs (authored-or-silent).
    fn controller_cue_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarRetargeted>();
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarDenied>();
        app.add_systems(Update, (play_lock_cues, play_safety_engaged_cue));
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        app
    }

    /// A player ship carrying a controller with the given sounds; returns the
    /// ship entity.
    fn spawn_player_controller(app: &mut App, sounds: ControllerSectionSounds) -> Entity {
        let ship = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        app.world_mut().spawn((sounds, ChildOf(ship)));
        ship
    }

    #[test]
    fn lock_cue_plays_the_player_controllers_authored_sound() {
        // The controller-owned cue path (task 20260717-101633): a lock acquire
        // plays the PLAYER controller's authored lock_on ref. Delivery guard
        // for the silent cases below.
        let mut app = controller_cue_app();
        spawn_player_controller(
            &mut app,
            ControllerSectionSounds {
                lock_on: Some(AssetRef::from("mods/x/sounds/chirp.wav")),
                ..default()
            },
        );
        let expected: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/sounds/chirp.wav");
        app.world_mut()
            .write_message(RadarLockAcquired { combat: true });
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "the player controller's authored lock_on must play"
        );
    }

    #[test]
    fn lock_cues_are_silent_without_a_player_controller_and_still_drain() {
        // No player controller (menu/editor/headless): silent, but the reader
        // cursors MUST advance - a message sent while controller-less must not
        // replay once a controller appears.
        let mut app = controller_cue_app();
        app.world_mut()
            .write_message(RadarLockAcquired { combat: true });
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "no player controller -> silent"
        );

        // Controller arrives AFTER the message was drained: no stale replay.
        spawn_player_controller(
            &mut app,
            ControllerSectionSounds {
                lock_on: Some(AssetRef::from("mods/x/sounds/chirp.wav")),
                ..default()
            },
        );
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "a drained message must not replay when the controller appears"
        );

        // And an unauthored cue on an existing controller stays silent while a
        // different authored cue plays (per-cue authorship, not all-or-nothing).
        app.world_mut().write_message(RadarDenied);
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "unauthored radar_deny -> silent"
        );
        app.world_mut()
            .write_message(RadarLockAcquired { combat: true });
        app.update();
        assert!(
            app.world().resource::<LastPlayed>().0.is_some(),
            "the authored lock_on still plays (delivery guard)"
        );
    }

    #[test]
    fn safety_cue_plays_the_controllers_authored_click_on_hot_to_cold() {
        let mut app = controller_cue_app();
        let ship = spawn_player_controller(
            &mut app,
            ControllerSectionSounds {
                safety_on: Some(AssetRef::from("base/sounds/safety_on.wav")),
                ..default()
            },
        );
        let expected: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/safety_on.wav");
        app.world_mut().entity_mut(ship).insert(WeaponsHot(true));
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "arming is silent"
        );
        app.world_mut().entity_mut(ship).insert(WeaponsHot(false));
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "the hot -> cold edge plays the controller's authored click"
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
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<ThrusterHumVolume>();
        app.add_systems(Update, compute_thruster_hum_volume);
        app
    }

    /// The standard authored hum for rig thrusters (the base default's path).
    const RIG_HUM: &str = "base/sounds/thruster_loop.wav";

    fn rig_hum_handle(app: &App) -> Handle<AudioSource> {
        app.world().resource::<AssetServer>().load(RIG_HUM)
    }

    /// The (target, smoothed) pair for the rig's standard hum, or (0, 0) when
    /// no thruster has raised it yet.
    fn rig_hum_levels(app: &App) -> (f32, f32) {
        let handle = rig_hum_handle(app);
        app.world()
            .resource::<ThrusterHumVolume>()
            .hums
            .get(&handle)
            .map(|l| (l.target, l.smoothed))
            .unwrap_or((0.0, 0.0))
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
            ThrusterSectionLoopSound(Some(AssetRef::from(RIG_HUM))),
            ChildOf(root),
        ));
        root
    }

    fn hum_target(app: &mut App) -> f32 {
        app.update();
        rig_hum_levels(app).0
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
            ThrusterSectionLoopSound(Some(AssetRef::from(RIG_HUM))),
            ChildOf(torpedo),
            GlobalTransform::from(Transform::from_translation(Vec3::new(400.0, 0.0, 0.0))),
        ));
        assert_eq!(hum_target(&mut app), 0.0, "far torpedo thruster: silent");

        // And a near one is heard.
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(1.0),
            ThrusterSectionLoopSound(Some(AssetRef::from(RIG_HUM))),
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
        let (_, mut last) = rig_hum_levels(&app);
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(4));
            app.update();
            let (target, smoothed) = rig_hum_levels(&app);
            assert!(
                smoothed >= last && smoothed <= target,
                "smoothed must rise monotonically toward the target, got {smoothed} after {last}"
            );
            last = smoothed;
        }
        assert!(last > 0.0, "smoothed must have started chasing the target");
    }

    #[test]
    fn distinct_hum_sounds_get_independent_loops() {
        // Two ships burning DIFFERENT authored hums (task 20260717-101650):
        // each handle gets its own level - per-handle grouping, not a single
        // global loop. The half-throttle ship's quieter hum must not be
        // swallowed by the other handle's louder one (the old single-loop max
        // would have).
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        spawn_burning_ship(&mut app, Vec3::ZERO, 0.5); // RIG_HUM at half
        let other = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::from(Transform::from_translation(Vec3::new(5.0, 0.0, 0.0))),
            ))
            .id();
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(1.0),
            ThrusterSectionLoopSound(Some(AssetRef::from("mods/x/sounds/ion_whine.wav"))),
            ChildOf(other),
        ));
        app.update();

        let (rig_target, _) = rig_hum_levels(&app);
        assert!(
            (rig_target - engine_volume(0.5)).abs() < 1e-6,
            "the rig hum tracks ITS ship, got {rig_target}"
        );
        let whine: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/sounds/ion_whine.wav");
        let whine_target = app
            .world()
            .resource::<ThrusterHumVolume>()
            .hums
            .get(&whine)
            .map(|l| l.target)
            .unwrap_or(0.0);
        assert!(
            (whine_target - engine_volume(1.0)).abs() < 1e-6,
            "the mod hum tracks ITS ship independently, got {whine_target}"
        );
    }

    #[test]
    fn an_unauthored_thruster_contributes_no_hum() {
        // Authored-or-silent: a thruster with no loop_sound raises nothing -
        // the map stays empty. The burning-ship rigs above are the delivery
        // guard (same spawn shape WITH the ref hums).
        let mut app = hum_app();
        spawn_listener_at(&mut app, Vec3::ZERO);
        let root = app
            .world_mut()
            .spawn((SpaceshipRootMarker, GlobalTransform::default()))
            .id();
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(1.0),
            ThrusterSectionLoopSound(None),
            ChildOf(root),
        ));
        app.update();
        assert!(
            app.world().resource::<ThrusterHumVolume>().hums.is_empty(),
            "an unauthored thruster must raise no hum entry"
        );
    }

    #[test]
    fn every_ui_sfx_key_has_a_file() {
        // Guards against adding a UiSfx variant without a placeholder asset.
        use UiSfx::*;
        for key in [ObjectiveNew, ObjectiveComplete, MenuSelect, UiToggle] {
            assert!(
                UI_SFX_FILES.iter().any(|(k, _)| *k == key),
                "UiSfx::{key:?} is missing from UI_SFX_FILES"
            );
        }
    }

    /// An App rig for the dry-fire cue: the real `play_dry_fire_cue` system with
    /// a `PlaySfx` counter, no audio device needed. No bank: the cue is
    /// authored-or-silent, so each test turret authors its own click via
    /// [`dry_click`].
    fn dry_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<PlayedSfx>();
        app.add_systems(Update, play_dry_fire_cue);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app
    }

    /// An authored dry-fire click for test turrets (the base default's path).
    fn dry_click() -> TurretSectionDryFireSound {
        TurretSectionDryFireSound(Some(AssetRef::from("base/sounds/dry_fire.wav")))
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
                dry_click(),
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
                dry_click(),
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
            dry_click(),
            ChildOf(player_hot),
        ));
        // Player + hot + empty + held, but NO authored dry_fire_sound: gated on
        // authorship (authored-or-silent, spike 20260717-101524).
        app.world_mut().spawn((
            TurretSectionMarker,
            TurretSectionInput(true),
            SectionAmmo::new(0),
            TurretSectionDryFireSound(None),
            ChildOf(player_hot),
        ));

        app.update();
        assert_eq!(
            dings(&app),
            1,
            "only the player's hot, empty, held, AUTHORED turret dry-fires"
        );
    }

    #[test]
    fn rcs_volume_is_silent_at_rest_and_saturates_at_full_deflection() {
        assert_eq!(rcs_volume(0.0), 0.0);
        assert_eq!(rcs_volume(1.0), RCS_MAX_VOLUME);
        // A diagonal command can exceed 1; the clamp holds it at the ceiling.
        assert_eq!(rcs_volume(1.7), RCS_MAX_VOLUME);
        assert!((rcs_volume(0.5) - RCS_MAX_VOLUME * 0.5).abs() < f32::EPSILON);
    }

    /// The base RCS loop path.
    const RIG_RCS: &str = "base/sounds/rcs_loop.wav";

    fn rcs_loop_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<RcsLoopVolume>();
        app.add_systems(Update, compute_rcs_loop_volume);
        app
    }

    fn rig_rcs_target(app: &App) -> f32 {
        let handle = app.world().resource::<AssetServer>().load(RIG_RCS);
        app.world()
            .resource::<RcsLoopVolume>()
            .loops
            .get(&handle)
            .map(|l| l.target)
            .unwrap_or(0.0)
    }

    /// A ship with an RCS-authoring controller child, carrying `intent` on the
    /// root. `deny_rcs` withholds the verb; marked as the player so attenuation
    /// is a deterministic 1.0 (no listener needed).
    fn spawn_rcs_ship(app: &mut App, intent: Vec3, deny_rcs: bool) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_translation(Vec3::ZERO)),
                RcsIntent(intent),
            ))
            .id();
        let sounds = ControllerSectionSounds {
            rcs_loop: Some(AssetRef::from(RIG_RCS)),
            ..Default::default()
        };
        let mut ctrl = app
            .world_mut()
            .spawn((ControllerSectionMarker, sounds, ChildOf(root)));
        if deny_rcs {
            ctrl.insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
        }
        root
    }

    #[test]
    fn rcs_loop_plays_while_the_controller_burns_and_mutes_at_rest() {
        let mut app = rcs_loop_app();
        let ship = spawn_rcs_ship(&mut app, Vec3::new(1.0, 0.0, 0.0), false);
        app.update();
        assert!(
            (rig_rcs_target(&app) - RCS_MAX_VOLUME).abs() < 1e-4,
            "a full-deflection RCS burn drives the loop to its ceiling (got {})",
            rig_rcs_target(&app)
        );

        // Intent falls to zero (the mouse stopped / the autopilot settled): the
        // loop target must drop back to silence.
        app.world_mut()
            .entity_mut(ship)
            .insert(RcsIntent(Vec3::ZERO));
        app.update();
        assert_eq!(
            rig_rcs_target(&app),
            0.0,
            "the loop mutes when the RCS stops burning"
        );
    }

    #[test]
    fn rcs_loop_is_silent_without_the_rcs_verb() {
        // Same non-zero intent, but the controller withholds Rcs - no hiss, the
        // same capability gate rcs_burn_system applies.
        let mut app = rcs_loop_app();
        spawn_rcs_ship(&mut app, Vec3::new(1.0, 0.0, 0.0), true);
        app.update();
        assert_eq!(
            rig_rcs_target(&app),
            0.0,
            "a controller that does not grant Rcs makes no RCS sound"
        );
    }
}
