//! The continuous loops: one looping audio entity per distinct authored
//! thruster hum, plus the RCS fine-adjust loop, each tracking how hard
//! the ships it belongs to are burning and muting behind a frozen sim.

use std::collections::HashMap;

use bevy::{audio::Volume, prelude::*};

use super::mixing::{
    distance_attenuation, engine_volume, listener_position, rcs_volume, SfxListenerMarker,
};
use crate::{
    prelude::*,
    sections::{
        controller_section::ControllerSectionSounds, thruster_section::ThrusterSectionLoopSound,
    },
};

/// Marker for one looping engine-hum audio entity, keyed by the resolved
/// [`Handle<AudioSource>`] it loops (one entity per DISTINCT authored hum;
/// hum). Entities persist for the session like the old single loop did - a hum
/// that goes quiet holds volume 0.
#[derive(Component)]
pub(super) struct ThrusterLoopSfx(Handle<AudioSource>);

/// Spawn a looping engine-hum entity for every hum handle the compute pass
/// discovered that has no loop entity yet. Each starts silent;
/// [`apply_thruster_loop_volume`] raises it with its handle's smoothed level.
/// `PlaybackSettings::LOOP` keeps it playing for the whole session.
pub(super) fn ensure_thruster_loops(
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
pub(super) struct ThrusterHumVolume {
    hums: HashMap<Handle<AudioSource>, HumLevels>,
}

/// The entity a thruster's hum contribution is attributed to: its
/// [`SpaceshipRootMarker`] ancestor (one hum source per ship), or the thruster
/// itself when the walk leaves the tree without finding one (torpedo
/// thrusters hang off the projectile, not a ship root; bare rigs have no
/// parent at all), so a rootless thruster attenuates at its own pose.
pub(super) fn hum_source_root(
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
/// Per-ship attribution: the throttle is averaged over each ship's own active
/// thrusters (summing would pin to max the moment more than one fires), scaled
/// by [`distance_attenuation`] from the listener to that ship's root, and the
/// loudest ship wins - so a distant AI ship's burn no longer raises a
/// full-volume hum in the player's ear. The global average this replaces
/// predated multiple audible ships.
///
/// The PLAYER's ship is exempt from attenuation: the camera rig sits 11-32 u
/// out depending on mode (Normal/FreeLook are already past `SFX_NEAR_DISTANCE`)
/// and the orbit survey dolly stretches it to `SURVEY_MAX_DISTANCE` = 250 u,
/// deep into the rolloff band - your own engines must not fade out because the
/// camera pulled back for the shot. A missing listener falls back to no
/// attenuation, mirroring [`play_positional`].
pub(super) fn compute_thruster_hum_volume(
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
    // (sum, count) per pair. AUTHORED-OR-SILENT: a thruster with no loop_sound
    // contributes to no hum. Resolving here is idempotent (the asset server
    // dedups by path), so thrusters authoring the same ref share one handle and
    // one loop entity.
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
pub(super) fn apply_thruster_loop_volume(
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
pub(super) struct RcsLoopSfx(Handle<AudioSource>);

/// The live RCS-loop volumes PER RESOLVED HANDLE, written by
/// [`compute_rcs_loop_volume`] and read by [`apply_rcs_loop_volume`]. Split
/// from the `AudioSink` write so the volume logic stays headless-testable,
/// exactly like [`ThrusterHumVolume`]. Reuses [`HumLevels`] (target +
/// smoothed).
#[derive(Resource, Default, Debug)]
pub(super) struct RcsLoopVolume {
    loops: HashMap<Handle<AudioSource>, HumLevels>,
}

/// Spawn a looping RCS-hiss entity for every handle the compute pass discovered
/// without a loop yet. Each starts silent; [`apply_rcs_loop_volume`] raises it.
pub(super) fn ensure_rcs_loops(
    vol: Res<RcsLoopVolume>,
    existing: Query<&RcsLoopSfx>,
    mut commands: Commands,
) {
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
/// `RcsIntent` magnitude on the ship root, resolved through each live
/// controller section's authored `rcs_loop` handle. CONTROLLER-based and
/// DRIVER-agnostic: the intent is written by the player's SHIFT modal OR the
/// autopilot (ORBIT trim, STOP/GOTO settle), so both make the same sound. Gated
/// on the controller granting [`FlightVerb::Rcs`], mirroring `rcs_burn_system`
/// - a hull that cannot RCS makes no RCS hiss. Per-ship attribution,
/// loudest-wins-per-handle, distance attenuation (player exempt) and
/// exponential smoothing all match [`compute_thruster_hum_volume`].
pub(super) fn compute_rcs_loop_volume(
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
pub(super) fn apply_rcs_loop_volume(
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

/// Silence the engine loop while the sim is frozen (the pause overlay OR the
/// Tab NOVA OS); one-shot SFX are naturally quiet then (no events fire in a
/// frozen sim). Pause every looping SFX sink (thruster hum + RCS hiss) - audio
/// sinks do not follow `Time<Virtual>`, so without this a loop keeps roaring at
/// its last volume while the game is frozen.
pub(super) fn pause_loops(
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

pub(super) fn resume_loops(
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
    use super::{
        super::mixing::{ENGINE_MAX_VOLUME, RCS_MAX_VOLUME, SFX_FAR_DISTANCE},
        *,
    };

    /// App rig for the hum-volume computation: the real
    /// [`compute_thruster_hum_volume`] system over production markers, no audio
    /// device needed (the sink-apply half is split off for exactly this).
    /// Mirrors the production shape: thruster sections are `ChildOf` children
    /// of a `SpaceshipRootMarker` root (input/player/mod.rs), torpedo thrusters
    /// are children of the projectile root with their own `GlobalTransform`
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
        // The playtest bug: an AI ship burning at full throttle
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

        // Delivery guard for the null assertion: the SAME ship moved
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
        // Two ships burning DIFFERENT authored hums: each handle gets its own
        // level - per-handle grouping, not a single global loop. The
        // half-throttle ship's quieter hum must not be swallowed by the other
        // handle's louder one (the old single-loop max would have).
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
