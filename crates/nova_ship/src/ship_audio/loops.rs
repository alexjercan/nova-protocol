//! The continuous loops: one engine-hum voice per (ship, authored hum sound)
//! pair, the RCS fine-adjust loop on the same shape, and the railgun's
//! capacitor bank - which is deliberately NOT on that shape, because a weapon
//! winding up is the one loop in the game that must not ease.
//!
//! PER SOURCE, not per sound: two ships burning past the camera are two voices
//! following two ships, so they attenuate and pan independently. Everything
//! after "how hard is this ship burning" belongs to the engine - the rolloff,
//! the pan, the master, the freeze behind a paused sim, the teardown when a
//! scenario unloads, and the cap on how many exterior loops may sound at once.

use std::collections::HashMap;

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    levels::{engine_volume, rcs_volume},
    routing::{owning_root, route_from},
    RAILGUN_CHARGE_FLOOR, RAILGUN_CHARGE_MAX_VOLUME, RAILGUN_CHARGE_TOP_SPEED,
};
use crate::{
    prelude::*,
    sections::{
        controller_section::ControllerSectionSounds, railgun_section::RailgunSectionChargeSound,
        thruster_section::ThrusterSectionLoopSound,
    },
};

/// How fast a loop's level chases its target, in units per second.
/// Framerate-independent, and slow enough that a throttle change fades rather
/// than clicks.
const LOOP_SMOOTHING_RATE: f32 = 8.0;

/// The line between "this source has a voice" and "it does not". A loop with
/// nothing left to track is retired below it, and a source under it is never
/// given one: its owner is gone or idle, and holding an open sink for silence
/// buys nothing. One number for both arms of `reconcile_loops`, so a silent
/// source cannot be spawned by one and retired by the other on alternate
/// frames.
const LOOP_RETIRE_LEVEL: f32 = 1e-4;

/// One ship's engine hum: whose burn it tracks, and which authored sound it
/// plays. One voice per pair, so a ship authoring two different hums gets two
/// and two ships sharing one hum still pan apart.
#[derive(Component)]
pub(super) struct ThrusterLoopSfx {
    source: Entity,
    handle: Handle<AudioSource>,
}

/// One ship's RCS hiss, on the same per-pair shape as [`ThrusterLoopSfx`].
#[derive(Component)]
pub(super) struct RcsLoopSfx {
    source: Entity,
    handle: Handle<AudioSource>,
}

/// Drive the engine hum from how hard each ship is burning.
///
/// The throttle is averaged over each ship's own active thrusters (summing
/// would pin to max the moment more than one fires) and smoothed toward the
/// target so throttle changes fade. There is no attenuation here and no player
/// special case: the voice's ROUTE says whether it is the pilot's own hull or a
/// ship out in the world, and the engine takes it from there.
pub(super) fn drive_thruster_loops(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    q_thrusters: Query<
        (Entity, &ThrusterSectionInput, &ThrusterSectionLoopSound),
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_loops: Query<(Entity, &ThrusterLoopSfx, &mut SfxVoice)>,
) {
    // AUTHORED-OR-SILENT: a thruster with no loop_sound contributes to no hum.
    // Resolving here is idempotent (the asset server dedups by path), so
    // thrusters authoring the same ref share one handle.
    let mut burn: HashMap<(Entity, Handle<AudioSource>), (f32, u32)> = HashMap::new();
    for (thruster, input, loop_sound) in &q_thrusters {
        let Some(handle) = loop_sound.0.as_ref().map(|r| r.resolve(&asset_server)) else {
            continue;
        };
        let source = owning_root(thruster, &q_child_of, &q_is_root);
        let slot = burn.entry((source, handle)).or_insert((0.0, 0));
        slot.0 += input.0.abs();
        slot.1 += 1;
    }
    let targets: HashMap<(Entity, Handle<AudioSource>), f32> = burn
        .into_iter()
        .map(|(pair, (sum, count))| (pair, engine_volume(sum / count as f32)))
        .collect();

    reconcile_loops(
        &mut commands,
        &time,
        &q_is_player,
        targets,
        q_loops,
        |sfx: &ThrusterLoopSfx| (sfx.source, sfx.handle.clone()),
        |source, handle| ThrusterLoopSfx { source, handle },
        "Thruster Loop Sfx",
    );
}

/// Drive the RCS hiss from how hard each ship is fine-adjusting.
///
/// CONTROLLER-based and DRIVER-agnostic: the `RcsIntent` on the ship root is
/// written by the player's modal and by the autopilot both, so both make the
/// same sound. Gated on the controller granting [`FlightVerb::Rcs`], mirroring
/// `rcs_burn_system` - a hull that cannot RCS makes no RCS hiss.
pub(super) fn drive_rcs_loops(
    mut commands: Commands,
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
    q_loops: Query<(Entity, &RcsLoopSfx, &mut SfxVoice)>,
) {
    let mut targets: HashMap<(Entity, Handle<AudioSource>), f32> = HashMap::new();
    for (&ChildOf(root), sounds, withheld) in &q_controllers {
        if !withheld.is_none_or(|w| w.granted(FlightVerb::Rcs)) {
            continue;
        }
        // AUTHORED-OR-SILENT: a controller with no rcs_loop makes no sound.
        let Some(handle) = sounds.rcs_loop.as_ref().map(|r| r.resolve(&asset_server)) else {
            continue;
        };
        let Ok(intent) = q_intent.get(root) else {
            continue;
        };
        let effort = intent.0.length();
        if effort <= 1e-4 {
            continue;
        }
        let level = rcs_volume(effort);
        let slot = targets.entry((root, handle)).or_insert(0.0);
        *slot = slot.max(level);
    }

    reconcile_loops(
        &mut commands,
        &time,
        &q_is_player,
        targets,
        q_loops,
        |sfx: &RcsLoopSfx| (sfx.source, sfx.handle.clone()),
        |source, handle| RcsLoopSfx { source, handle },
        "RCS Loop Sfx",
    );
}

/// One charging lance's capacitor bank. Per SECTION, not per ship: the charge
/// clock is the gun's, so two lances on one hull wind up independently.
#[derive(Component)]
pub(super) struct RailgunChargeLoopSfx {
    section: Entity,
}

/// Drive the lance's charge loop from where each gun is in its cycle.
///
/// NOT on [`reconcile_loops`], and the difference is the point. The hum loops
/// EASE, because a throttle change that clicked would be a fault; this one must
/// not. A commit has to be audible on the frame it happens - it is the tell the
/// whole weapon is balanced around - and the shot has to cut the bank dead,
/// because the report is 20 dB louder and a fading capacitor smeared under it
/// is just mud. So the voice opens at its floor level and is despawned outright
/// the frame the gun leaves [`RailgunCharge::Charging`].
///
/// Two things rise together with progress: the LEVEL, from
/// [`RAILGUN_CHARGE_FLOOR`] of the ceiling to all of it, and the playback RATE,
/// to [`RAILGUN_CHARGE_TOP_SPEED`]. The rate is what makes it read as
/// approaching something rather than holding a note, and the loop is authored
/// on even partials so its seam stays silent at any rate in between.
///
/// AUTHORED-OR-SILENT: a lance with no `charge_sound` charges quietly.
pub(super) fn drive_railgun_charge_loops(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_railgun: Query<
        (
            Entity,
            &RailgunSectionConfigHelper,
            &RailgunCharge,
            &RailgunSectionChargeSound,
        ),
        (With<RailgunSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut q_loops: Query<(Entity, &RailgunChargeLoopSfx, &mut SfxVoice)>,
) {
    let mut charging: HashMap<Entity, (Handle<AudioSource>, f32)> = HashMap::new();
    for (section, config, charge, sound) in &q_railgun {
        if !matches!(charge, RailgunCharge::Charging { .. }) {
            continue;
        }
        let Some(handle) = sound.0.as_ref().map(|r| r.resolve(&asset_server)) else {
            continue;
        };
        charging.insert(section, (handle, charge.progress(config.charge_seconds)));
    }

    for (voice_entity, loop_sfx, mut voice) in &mut q_loops {
        let Some((_, progress)) = charging.remove(&loop_sfx.section) else {
            commands.entity(voice_entity).despawn();
            continue;
        };
        voice.volume = charge_volume(progress);
        voice.speed = charge_speed(progress);
        // Refreshed every frame, like the hum loops': a gun can change hands
        // between one scenario and the next.
        let root = owning_root(loop_sfx.section, &q_child_of, &q_is_root);
        voice.route = route_from(root, &q_is_player);
    }

    for (section, (handle, progress)) in charging {
        let root = owning_root(section, &q_child_of, &q_is_root);
        commands.spawn((
            Name::new("Railgun Charge Loop Sfx"),
            RailgunChargeLoopSfx { section },
            SfxVoice::looping(handle, route_from(root, &q_is_player))
                .following(section)
                .with_volume(charge_volume(progress))
                .with_speed(charge_speed(progress)),
        ));
    }
}

/// The bank's level at `progress`, from the floor to the ceiling.
fn charge_volume(progress: f32) -> f32 {
    RAILGUN_CHARGE_MAX_VOLUME * (RAILGUN_CHARGE_FLOOR + (1.0 - RAILGUN_CHARGE_FLOOR) * progress)
}

/// The bank's playback rate at `progress`, from normal to the top speed.
fn charge_speed(progress: f32) -> f32 {
    1.0 + (RAILGUN_CHARGE_TOP_SPEED - 1.0) * progress
}

/// Bring the live loop voices in line with what is burning this frame: ease the
/// ones that still have a source, retire the ones that have faded out, and open
/// a voice for a pair that has none.
///
/// Shared by the two loop passes because the ONLY thing that differs between an
/// engine hum and an RCS hiss is the level curve that produced `targets`.
///
/// [`LOOP_RETIRE_LEVEL`] is the hinge, and both arms read it: a source below it
/// gets no voice and a voice below it is retired, so a silent source settles at
/// "no voice" instead of trading between the two arms every frame.
#[expect(
    clippy::too_many_arguments,
    reason = "the generic reconciler takes both marker adapters plus the queries it drives"
)]
fn reconcile_loops<M: Component>(
    commands: &mut Commands,
    time: &Time,
    q_is_player: &Query<(), With<PlayerSpaceshipMarker>>,
    mut targets: HashMap<(Entity, Handle<AudioSource>), f32>,
    mut q_loops: Query<(Entity, &M, &mut SfxVoice)>,
    pair_of: impl Fn(&M) -> (Entity, Handle<AudioSource>),
    marker_of: impl Fn(Entity, Handle<AudioSource>) -> M,
    name: &'static str,
) {
    let alpha = (time.delta_secs() * LOOP_SMOOTHING_RATE).clamp(0.0, 1.0);
    for (entity, marker, mut voice) in &mut q_loops {
        let pair = pair_of(marker);
        let target = targets.remove(&pair).unwrap_or(0.0);
        voice.volume += (target - voice.volume) * alpha;
        if target < LOOP_RETIRE_LEVEL && voice.volume < LOOP_RETIRE_LEVEL {
            commands.entity(entity).despawn();
            continue;
        }
        // Refreshed rather than fixed at spawn: a ship can become (or stop
        // being) the player's between one scenario and the next.
        voice.route = route_from(pair.0, q_is_player);
    }
    for ((source, handle), target) in targets {
        // The SAME threshold the retire arm just used, and the two must stay
        // the same one. A source under it - an idle thruster, a torpedo
        // coasting at zero headroom - would otherwise be spawned on one frame
        // and retired on the next, opening and closing a rodio sink forever.
        if target < LOOP_RETIRE_LEVEL {
            continue;
        }
        commands.spawn((
            Name::new(name),
            marker_of(source, handle.clone()),
            SfxVoice::looping(handle, route_from(source, q_is_player))
                .following(source)
                .with_volume(target * alpha),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::levels::{ENGINE_MAX_VOLUME, RCS_MAX_VOLUME},
        *,
    };

    /// One test frame, long enough that the smoothing moves visibly and short
    /// enough that it still takes several frames to arrive.
    const TEST_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

    /// App rig for the loop passes: the real systems over production markers,
    /// no audio device needed (the engine owns every sink write, and this
    /// layer only reports how hard a ship is burning). Time is stepped by hand
    /// so the smoothing is deterministic; the first update still runs at dt 0,
    /// so every assertion below takes at least two.
    fn loop_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(TEST_FRAME));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, (drive_thruster_loops, drive_rcs_loops));
        app
    }

    /// The standard authored hum for rig thrusters (the base default's path).
    const RIG_HUM: &str = "base/sounds/thruster_loop.wav";
    /// The base RCS loop path.
    const RIG_RCS: &str = "base/sounds/rcs_loop.wav";

    /// Every live loop voice as `(route, volume, followed entity)`.
    fn voices(app: &mut App) -> Vec<(AudioRoute, f32, Option<Entity>)> {
        let mut query = app.world_mut().query::<&SfxVoice>();
        query
            .iter(app.world())
            .map(|voice| {
                let followed = match voice.source {
                    SfxSource::Follow(entity) => Some(entity),
                    _ => None,
                };
                (voice.route, voice.volume, followed)
            })
            .collect()
    }

    /// The one voice following `source`, or `None`.
    fn voice_for(app: &mut App, source: Entity) -> Option<(AudioRoute, f32)> {
        voices(app)
            .into_iter()
            .find(|(_, _, followed)| *followed == Some(source))
            .map(|(route, volume, _)| (route, volume))
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

    /// Run until the smoothing has caught up, so a test can assert on the level
    /// rather than on the ramp.
    fn settle(app: &mut App) {
        for _ in 0..96 {
            app.update();
        }
    }

    #[test]
    fn every_burning_ship_gets_its_own_voice_following_its_own_hull() {
        // The fix for the one global position-less loop: two ships burning are
        // two voices at two places, so the engine can pan them apart.
        let mut app = loop_app();
        let near = spawn_burning_ship(&mut app, Vec3::new(30.0, 0.0, 0.0), 1.0);
        let far = spawn_burning_ship(&mut app, Vec3::new(-400.0, 0.0, 0.0), 1.0);
        settle(&mut app);

        let followed: Vec<Option<Entity>> = voices(&mut app).into_iter().map(|v| v.2).collect();
        assert_eq!(followed.len(), 2, "one voice per burning ship");
        assert!(followed.contains(&Some(near)) && followed.contains(&Some(far)));
    }

    #[test]
    fn the_players_own_burn_is_routed_to_the_hull_and_everyone_elses_outside() {
        // The routing fact that replaced `if player { skip attenuation }`: your
        // engines are the room you are sitting in, whatever the camera does.
        let mut app = loop_app();
        let player = spawn_burning_ship(&mut app, Vec3::ZERO, 1.0);
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerSpaceshipMarker);
        let raider = spawn_burning_ship(&mut app, Vec3::new(60.0, 0.0, 0.0), 1.0);
        settle(&mut app);

        assert_eq!(
            voice_for(&mut app, player).map(|v| v.0),
            Some(AudioRoute::Hull)
        );
        assert_eq!(
            voice_for(&mut app, raider).map(|v| v.0),
            Some(AudioRoute::Exterior)
        );
    }

    #[test]
    fn the_hum_level_is_the_ships_own_throttle_undimmed_by_distance() {
        // Distance is the engine's business now: this layer reports the burn
        // and nothing else, so a ship 400 u out still reports its full curve.
        let mut app = loop_app();
        let ship = spawn_burning_ship(&mut app, Vec3::new(400.0, 0.0, 0.0), 0.5);
        settle(&mut app);

        let (_, level) = voice_for(&mut app, ship).expect("the burning ship has a voice");
        assert!(
            (level - engine_volume(0.5)).abs() < 1e-3,
            "expected the raw throttle curve {}, got {level}",
            engine_volume(0.5)
        );
        assert!(engine_volume(1.0) == ENGINE_MAX_VOLUME);
    }

    #[test]
    fn the_hum_eases_toward_its_target_instead_of_snapping() {
        let mut app = loop_app();
        let ship = spawn_burning_ship(&mut app, Vec3::ZERO, 1.0);
        app.update(); // first frame: dt = 0

        let mut last = 0.0;
        for _ in 0..5 {
            app.update();
            let (_, level) = voice_for(&mut app, ship).expect("the voice exists");
            assert!(
                level >= last && level <= ENGINE_MAX_VOLUME,
                "the level must rise monotonically toward the target, got {level} after {last}"
            );
            last = level;
        }
        assert!(last > 0.0, "the level must have started chasing the target");
    }

    #[test]
    fn a_ship_that_stops_burning_fades_out_and_retires_its_voice() {
        let mut app = loop_app();
        let ship = spawn_burning_ship(&mut app, Vec3::ZERO, 1.0);
        settle(&mut app);
        assert!(voice_for(&mut app, ship).is_some());

        // Cut the throttle: the voice must fade rather than cut, then go.
        let mut thrusters = app
            .world_mut()
            .query_filtered::<&mut ThrusterSectionInput, With<ThrusterSectionMarker>>();
        for mut input in thrusters.iter_mut(app.world_mut()) {
            input.0 = 0.0;
        }
        app.update();
        let mid = voice_for(&mut app, ship).map(|v| v.1);
        assert!(
            mid.is_some_and(|level| level > 0.0 && level < ENGINE_MAX_VOLUME),
            "the level must ease down, not cut, got {mid:?}"
        );

        settle(&mut app);
        assert!(
            voice_for(&mut app, ship).is_none(),
            "a faded-out loop retires instead of holding an open sink at silence"
        );

        // And STAYS gone. This assertion used to hold on frame parity alone:
        // the retired source was respawned on the next frame and retired on
        // the one after, forever.
        for frame in 0..9 {
            app.update();
            assert!(
                voice_for(&mut app, ship).is_none(),
                "the idle ship took its voice back on frame {frame}"
            );
        }
    }

    #[test]
    fn an_idle_thruster_never_opens_a_voice_at_all() {
        // A torpedo coasting at zero headroom is exactly this case, and there
        // are hundreds of them in a salvo. Spawning one silent voice per
        // source per frame - and retiring it the next - churned a rodio sink
        // open and closed for every one.
        let mut app = loop_app();
        let ship = spawn_burning_ship(&mut app, Vec3::ZERO, 0.0);

        for frame in 0..24 {
            app.update();
            assert!(
                voice_for(&mut app, ship).is_none(),
                "an idle thruster opened a sink on frame {frame}"
            );
        }
    }

    #[test]
    fn a_ship_with_two_authored_hums_gets_a_voice_for_each() {
        let mut app = loop_app();
        let ship = spawn_burning_ship(&mut app, Vec3::ZERO, 1.0);
        app.world_mut().spawn((
            ThrusterSectionMarker,
            ThrusterSectionInput(1.0),
            ThrusterSectionLoopSound(Some(AssetRef::from("mods/x/sounds/ion_whine.wav"))),
            ChildOf(ship),
        ));
        settle(&mut app);

        let following_ship = voices(&mut app)
            .into_iter()
            .filter(|(_, _, followed)| *followed == Some(ship))
            .count();
        assert_eq!(following_ship, 2, "one voice per distinct authored hum");
    }

    #[test]
    fn an_unauthored_thruster_makes_no_voice() {
        // Authored-or-silent. The burning-ship rigs above are the delivery
        // guard (same spawn shape WITH the ref hums).
        let mut app = loop_app();
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
        settle(&mut app);
        assert!(voices(&mut app).is_empty());
    }

    #[test]
    fn a_rootless_thruster_is_heard_at_its_own_pose() {
        // Torpedo shape: the thruster hangs off a projectile root that is NOT a
        // SpaceshipRootMarker, so it follows itself.
        let mut app = loop_app();
        let torpedo = app.world_mut().spawn(GlobalTransform::default()).id();
        let thruster = app
            .world_mut()
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(1.0),
                ThrusterSectionLoopSound(Some(AssetRef::from(RIG_HUM))),
                ChildOf(torpedo),
                GlobalTransform::from(Transform::from_translation(Vec3::new(40.0, 0.0, 0.0))),
            ))
            .id();
        settle(&mut app);

        assert_eq!(
            voice_for(&mut app, thruster).map(|v| v.0),
            Some(AudioRoute::Exterior),
            "a torpedo's own thruster follows the torpedo, not a ship"
        );
    }

    /// A ship with an RCS-authoring controller child, carrying `intent` on the
    /// root. `deny_rcs` withholds the verb.
    fn spawn_rcs_ship(app: &mut App, intent: Vec3, deny_rcs: bool) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::from(Transform::from_translation(Vec3::ZERO)),
                RcsIntent(intent),
            ))
            .id();
        let sounds = ControllerSectionSounds {
            rcs_loop: Some(AssetRef::from(RIG_RCS)),
            ..Default::default()
        };
        let mut controller =
            app.world_mut()
                .spawn((ControllerSectionMarker, sounds, ChildOf(root)));
        if deny_rcs {
            controller.insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
        }
        root
    }

    #[test]
    fn the_rcs_loop_plays_while_the_controller_burns_and_retires_at_rest() {
        let mut app = loop_app();
        let ship = spawn_rcs_ship(&mut app, Vec3::new(1.0, 0.0, 0.0), false);
        settle(&mut app);
        let (route, level) = voice_for(&mut app, ship).expect("a burning ship hisses");
        assert_eq!(route, AudioRoute::Exterior);
        assert!(
            (level - RCS_MAX_VOLUME).abs() < 1e-3,
            "a full-deflection burn drives the loop to its ceiling, got {level}"
        );

        // Intent falls to zero (the mouse stopped / the autopilot settled).
        app.world_mut()
            .entity_mut(ship)
            .insert(RcsIntent(Vec3::ZERO));
        settle(&mut app);
        assert!(
            voice_for(&mut app, ship).is_none(),
            "the loop mutes and goes"
        );
    }

    #[test]
    fn the_rcs_loop_is_silent_without_the_rcs_verb() {
        // Same non-zero intent, but the controller withholds Rcs - the same
        // capability gate `rcs_burn_system` applies.
        let mut app = loop_app();
        let ship = spawn_rcs_ship(&mut app, Vec3::new(1.0, 0.0, 0.0), true);
        settle(&mut app);
        assert!(voice_for(&mut app, ship).is_none());
    }

    /// The base lance's charge sound path.
    const RIG_CHARGE: &str = "base/sounds/railgun_charge.wav";

    /// A lance on its own ship, charged to `progress` of a two-second charge.
    /// `sound` is the authored file, or `None` for a gun that voices nothing.
    fn spawn_charging_lance(app: &mut App, sound: Option<&str>, progress: f32) -> Entity {
        let root = app
            .world_mut()
            .spawn((SpaceshipRootMarker, GlobalTransform::default()))
            .id();
        app.world_mut()
            .spawn((
                // The production bundle, so the test reads the same authored
                // snapshot the spawn path writes.
                preview_railgun_section(RailgunSectionConfig {
                    charge_seconds: 2.0,
                    charge_sound: sound.map(AssetRef::from),
                    ..default()
                }),
                RailgunCharge::Charging {
                    elapsed: 2.0 * progress,
                },
                ChildOf(root),
            ))
            .id()
    }

    /// The one charge voice following `gun`, as `(volume, speed)`.
    fn charge_voice(app: &mut App, gun: Entity) -> Option<(f32, f32)> {
        let mut query = app
            .world_mut()
            .query::<(&RailgunChargeLoopSfx, &SfxVoice)>();
        query
            .iter(app.world())
            .find(|(loop_sfx, _)| loop_sfx.section == gun)
            .map(|(_, voice)| (voice.volume, voice.speed))
    }

    #[test]
    fn the_capacitor_opens_at_its_floor_and_arrives_loud_and_fast() {
        // The one loop that must NOT ease: a bank winding up has to be at its
        // authored floor on the FIRST frame of the charge, and the rise the
        // player hears is the charge itself, not a smoothing envelope.
        let mut app = charge_app();
        let gun = spawn_charging_lance(&mut app, Some(RIG_CHARGE), 0.0);
        app.update();
        let (opened, opening_speed) = charge_voice(&mut app, gun).expect("the bank opens a voice");
        assert!(
            (opened - RAILGUN_CHARGE_MAX_VOLUME * RAILGUN_CHARGE_FLOOR).abs() < 1e-5,
            "an opening bank starts at its floor, got {opened}"
        );
        assert!(
            (opening_speed - 1.0).abs() < 1e-5,
            "an opening bank plays at its authored rate, got {opening_speed}"
        );

        app.world_mut()
            .entity_mut(gun)
            .insert(RailgunCharge::Charging { elapsed: 2.0 });
        app.update();
        let (full, full_speed) = charge_voice(&mut app, gun).expect("the voice stays open");
        assert!(
            (full - RAILGUN_CHARGE_MAX_VOLUME).abs() < 1e-5,
            "a full bank is at the ceiling, got {full}"
        );
        assert!(
            (full_speed - RAILGUN_CHARGE_TOP_SPEED).abs() < 1e-5,
            "a full bank is at top speed, got {full_speed}"
        );
    }

    #[test]
    fn the_bank_goes_silent_the_frame_the_charge_ends() {
        // The shot cuts to the muzzle report; a capacitor tail fading under it
        // would read as the gun still winding up after it fired.
        let mut app = charge_app();
        let gun = spawn_charging_lance(&mut app, Some(RIG_CHARGE), 0.5);
        app.update();
        assert!(charge_voice(&mut app, gun).is_some());

        app.world_mut().entity_mut(gun).insert(RailgunCharge::Ready);
        app.update();
        assert!(
            charge_voice(&mut app, gun).is_none(),
            "leaving the charge retires the voice outright, with no tail"
        );
    }

    #[test]
    fn a_lance_that_authors_no_charge_sound_winds_up_quietly() {
        let mut app = charge_app();
        let gun = spawn_charging_lance(&mut app, None, 0.5);
        app.update();
        assert!(charge_voice(&mut app, gun).is_none());
    }

    /// App rig for the capacitor pass. Separate from [`loop_app`] because the
    /// charge loop is deliberately not on the smoothed reconciler.
    fn charge_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, drive_railgun_charge_loops);
        app
    }
}
