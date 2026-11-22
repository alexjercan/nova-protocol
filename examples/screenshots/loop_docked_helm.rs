//! loop_docked_helm: a mining tug docked to a light freighter takes the HELM
//! and flies the joined pair as one body.
//!
//! The berth is the lesson_cargo stage (port collars square, 5 m apart), not
//! a derelict: the tender is intact, Neutral and uncrewed. The walk docks with the real `dock` key, takes the HELM with the
//! real `dock_helm` key, and engages a real GOTO on a beacon set abeam the
//! player - off to the side, so the pair has to yaw before it can burn
//! toward it.
//!
//! Run modes, all under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the `news-0150-docked-helm`
//!   webm, then a HUD still and a Map still of the pair's predicted GOTO
//!   path (`docked-goto-hud.png`, `docked-goto-map.png`), and a Map still
//!   pulled back to the wheel's reach (`docked-goto-map-reach.png`). The
//!   beacon is no Map contact, so the reach is the route's, not the
//!   contacts'.
//!
//! After the loop the walk takes the HELM again, so the waiting GOTO flies
//! the pair, and checks that the production prediction
//! (`nova_ship::flight::prediction`) and the holo ribbon draw the pair's
//! path to its arrival. The Map route strokes are crate-private to
//! `nova_interface`, so the Map still is checked by eye only.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_docked_helm --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_docked_helm")]
#[command(version = "1.0.0")]
#[command(about = "Capture a docked pair taking the HELM and flying as one body", long_about = None)]
struct Cli;

/// Scenario id of the player's hull.
const PLAYER_ID: &str = "player";
/// Scenario id of the ship it docks with.
const TENDER_ID: &str = "tender";
/// Scenario id of the beacon the HELM flies to.
const BEACON_ID: &str = "helm_beacon";

/// The tug at the berth, where its port collar face stands 5 m from the
/// tender's (the lesson_cargo stage).
const PLAYER_POSITION: Meters3 = Meters3::new(0.0, 0.0, -140.0);
/// The tender, off the tug's port collar.
const TENDER_POSITION: Meters3 = Meters3::new(-55.0, 0.0, -170.0);
/// Turned 180 degrees about Y, so its port collar faces the tug's.
const TENDER_ROTATION: Quat = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);
/// Abeam the berth, to +X: the HELM must yaw the pair about 90 degrees
/// before it can burn toward this.
const BEACON_POSITION: Meters3 = Meters3::new(3_000.0, 0.0, -140.0);
/// Radar signature; lock range is 30 times it, so 12 km here - the lock this
/// walk takes directly, not through a radar gesture, so this is content
/// realism rather than a thing the walk depends on.
const BEACON_LOCK_SIGNATURE: Meters = Meters(400.0);

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0150-docked-helm";
#[cfg(feature = "debug")]
const HUD_SHOT: &str = "docked-goto-hud.png";
#[cfg(feature = "debug")]
const MAP_SHOT: &str = "docked-goto-map.png";
#[cfg(feature = "debug")]
const MAP_REACH_SHOT: &str = "docked-goto-map-reach.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LoopCapturePlugin::default());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(
            Update,
            assert_pair_one_body.run_if(resource_exists::<AssertPairOneBodyGate>),
        );
        app.add_plugins(helm_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let player = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Mining Tug".to_string(),
            position: PLAYER_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                // The walk never fires the beam; the scenario lint refuses a player
                // ship whose mining section has no binding.
                input_mapping: BTreeMap::from([(
                    "mining_beam".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyV)],
                )]),
            }),
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, "mining_tug")),
            ..default()
        }),
    });
    let tender = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TENDER_ID.to_string(),
            name: "Light Freighter".to_string(),
            position: TENDER_POSITION,
            rotation: TENDER_ROTATION,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, "light_freighter")),
            ..default()
        }),
    });
    let beacon = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Helm Beacon".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "HELM".to_string(),
            radius: Meters(60.0),
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: None,
            lock_signature: Some(BEACON_LOCK_SIGNATURE),
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The mining tug docked to a light freighter, flown by the HELM.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![player, tender, beacon],
                ThreePointRig::around("helm", PLAYER_POSITION, 25.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "loop_docked_helm".to_string(),
            "Docked Helm".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The pair's pose the instant the HELM is about to be taken: the heading
/// the turn is measured against, and the tender's pose in the player's
/// frame, which the joint must hold fixed for the rest of the walk.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct PairStart {
    heading: Vec3,
    partner_offset: Vec3,
    partner_turn: Quat,
}

/// Gates `assert_pair_one_body`: present only while the HELM is turning and
/// burning the pair, so the joint-drift check runs exactly where a drift
/// would show and nowhere else.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct AssertPairOneBodyGate;

/// Real-seconds backstop for every wait, the turn included: the capture clock
/// pins game time per frame, so a slow host spends more real seconds on the
/// same beat, and the loop's own 600-frame cap bounds game time.
#[cfg(feature = "debug")]
const STEP_DEADLINE: f32 = 120.0;
/// Game seconds the lock settles before the pose is recorded.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 0.5;
/// Game seconds the loop holds on NEUTRAL before the HELM is taken.
#[cfg(feature = "debug")]
const NEUTRAL_HOLD_SECS: f32 = 1.0;
/// Degrees off `PairStart::heading` the player must have turned.
#[cfg(feature = "debug")]
const TURN_DEGREES: f32 = 60.0;
/// Meters per second the pair must be making.
#[cfg(feature = "debug")]
const BURN_SPEED_MPS: f32 = 20.0;
/// Game seconds the loop holds after the HELM is handed back.
#[cfg(feature = "debug")]
const ENDING_HOLD_SECS: f32 = 1.5;
/// Real-seconds backstop for the loop write.
#[cfg(feature = "debug")]
const WRITE_DEADLINE: f32 = 60.0;
/// Real-seconds backstop for the pair's predicted path to appear: the
/// predictor flies the leg to its arrival in passes of 120 ticks, and each
/// frame takes seconds on a software-rendered GPU.
#[cfg(feature = "debug")]
const PREDICTION_DEADLINE: f32 = 300.0;
/// Frames the posed camera and the ribbon settle before the HUD still.
#[cfg(feature = "debug")]
const HUD_SETTLE_FRAMES: u32 = 10;
/// Frames the Map pane renders before its still: its offscreen scene builds
/// over frames.
#[cfg(feature = "debug")]
const MAP_SETTLE_FRAMES: u32 = 20;

#[cfg(feature = "debug")]
fn root(world: &mut World, id: &str) -> Entity {
    kit::ship_root(world, id).unwrap_or_else(|| panic!("loop_docked_helm: no ship {id}"))
}

#[cfg(feature = "debug")]
fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

/// Advance once the HELM chip is on screen in `state`: `Available` offers the
/// helm, `Hot` says the player holds it. The label reads `HELM` in both, so
/// the paint is what the player reads.
#[cfg(feature = "debug")]
fn helm_chip_is(
    state: DockChipState,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let index = DOCK_VERBS
        .iter()
        .position(|verb| *verb == "HELM")
        .expect("loop_docked_helm: HELM is a dock verb");
    when(move |world| {
        world
            .try_query::<(&DockChip, &DockChipState, &Visibility)>()
            .is_some_and(|mut chips| {
                chips.iter(world).any(|(chip, chip_state, visibility)| {
                    chip.0 == index && *chip_state == state && *visibility != Visibility::Hidden
                })
            })
    })
}

/// The player's `DockedShip::drives` and the pair's `DockingConnection::helm`
/// agree with `held`.
#[cfg(feature = "debug")]
fn assert_helm(world: &mut World, held: bool) {
    let player = root(world, PLAYER_ID);
    let docked = *world
        .get::<DockedShip>(player)
        .unwrap_or_else(|| panic!("loop_docked_helm: the player is not docked"));
    assert_eq!(docked.drives, held, "loop_docked_helm: DockedShip::drives");
    let connection = *world
        .get::<DockingConnection>(docked.connection)
        .unwrap_or_else(|| panic!("loop_docked_helm: the docking connection is gone"));
    let expected = if held {
        DockedHelmType::Held(player)
    } else {
        DockedHelmType::Neutral
    };
    assert_eq!(
        connection.helm, expected,
        "loop_docked_helm: DockingConnection::helm"
    );
}

/// Record the pair's pose the instant the HELM is about to be taken: the
/// player's heading, and the tender's pose in the player's frame.
#[cfg(feature = "debug")]
fn record_pair_start(world: &mut World) {
    use avian3d::prelude::{Position, Rotation};

    let player = root(world, PLAYER_ID);
    let tender = root(world, TENDER_ID);
    let player_position = world.get::<Position>(player).unwrap().0;
    let player_rotation = world.get::<Rotation>(player).unwrap().0;
    let tender_position = world.get::<Position>(tender).unwrap().0;
    let tender_rotation = world.get::<Rotation>(tender).unwrap().0;
    world.insert_resource(PairStart {
        heading: player_rotation * Vec3::NEG_Z,
        partner_offset: player_rotation.inverse() * (tender_position - player_position),
        partner_turn: player_rotation.inverse() * tender_rotation,
    });
}

/// Advance once the player has turned at least [`TURN_DEGREES`] off
/// [`PairStart::heading`] and the pair is making at least [`BURN_SPEED_MPS`].
#[cfg(feature = "debug")]
fn pair_turned_and_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use avian3d::prelude::{LinearVelocity, Rotation};

    when(|world| {
        let Some(start) = world.get_resource::<PairStart>() else {
            return false;
        };
        let Some(mut q_player) =
            world.try_query_filtered::<(&Rotation, &LinearVelocity), With<PlayerSpaceshipMarker>>()
        else {
            return false;
        };
        let Some((rotation, velocity)) = q_player.iter(world).next() else {
            return false;
        };
        let forward = rotation.0 * Vec3::NEG_Z;
        let turned = start.heading.angle_between(forward).to_degrees() >= TURN_DEGREES;
        let burning = MetersPerSecond::from_engine(velocity.0.length()).get() >= BURN_SPEED_MPS;
        turned && burning
    })
}

/// While [`AssertPairOneBodyGate`] is present: the tender's pose in the
/// player's frame stays within 1 m and 1 deg of [`PairStart`], and both
/// hulls still carry [`DockedShip`] - the joint, not the player's steering,
/// is what this watches.
#[cfg(feature = "debug")]
fn assert_pair_one_body(
    start: Option<Res<PairStart>>,
    q_player: Query<
        (
            &avian3d::prelude::Position,
            &avian3d::prelude::Rotation,
            Has<DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_ships: Query<
        (
            &EntityId,
            &avian3d::prelude::Position,
            &avian3d::prelude::Rotation,
            Has<DockedShip>,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    let Some(start) = start else {
        return;
    };
    let Ok((player_position, player_rotation, player_docked)) = q_player.single() else {
        return;
    };
    assert!(
        player_docked,
        "loop_docked_helm: the player left the dock mid-turn"
    );
    let Some((_, tender_position, tender_rotation, tender_docked)) =
        q_ships.iter().find(|(id, ..)| id.0 == TENDER_ID)
    else {
        panic!("loop_docked_helm: the tender is gone mid-turn");
    };
    assert!(
        tender_docked,
        "loop_docked_helm: the tender left the dock mid-turn"
    );
    let offset = player_rotation.0.inverse() * (tender_position.0 - player_position.0);
    let turn = player_rotation.0.inverse() * tender_rotation.0;
    let offset_drift = Meters::from_engine((offset - start.partner_offset).length()).get();
    let turn_drift = turn.angle_between(start.partner_turn).to_degrees();
    assert!(
        offset_drift < 1.0,
        "loop_docked_helm: the joint drifted {offset_drift:.2} m"
    );
    assert!(
        turn_drift < 1.0,
        "loop_docked_helm: the joint drifted {turn_drift:.2} deg"
    );
}

/// The player's `FlightPrediction` has a path, and the holo ribbon draws it.
#[cfg(feature = "debug")]
fn pair_path_drawn() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    when(|world| {
        let Some(mut q_player) =
            world.try_query_filtered::<(Entity, &FlightPrediction), With<PlayerSpaceshipMarker>>()
        else {
            return false;
        };
        let Some((player, prediction)) = q_player.iter(world).next() else {
            return false;
        };
        prediction.points.len() >= 3
            && world
                .try_query::<&TrajectoryRibbonSegment>()
                .is_some_and(|mut segments| {
                    segments.iter(world).any(|segment| segment.ship == player)
                })
    })
}

/// The interface is up on `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(
    pane: nova_protocol::nova_interface::pane::InterfacePaneType,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    when(move |world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<nova_protocol::nova_interface::pane::InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// The pair's path: the player still drives the docked pair, the
/// prediction follows the pair's centre of mass (not the tug's), and the
/// ribbon draws every segment and ends on the prediction's arrival point.
#[cfg(feature = "debug")]
fn assert_pair_path(world: &mut World) {
    use avian3d::prelude::ComputedCenterOfMass;

    assert_helm(world, true);
    let player = root(world, PLAYER_ID);
    let prediction = world
        .get::<FlightPrediction>(player)
        .unwrap_or_else(|| panic!("loop_docked_helm: the docked pair has no FlightPrediction"));
    let flown_point = prediction.flown_point;
    let arrival = *prediction
        .points
        .last()
        .expect("loop_docked_helm: a prediction has points");
    let tug_center = world
        .get::<ComputedCenterOfMass>(player)
        .expect("loop_docked_helm: the tug has a centre of mass")
        .0;
    let beacon = world
        .query::<(&EntityId, &GlobalTransform)>()
        .iter(world)
        .find(|(id, _)| id.0 == BEACON_ID)
        .map(|(_, transform)| transform.translation())
        .unwrap_or_else(|| panic!("loop_docked_helm: no beacon {BEACON_ID}"));
    let (mut drawn, mut hidden, mut end) = (0u32, 0u32, None);
    let mut segments = world.query::<(&TrajectoryRibbonSegment, &Transform, &Visibility)>();
    for (segment, transform, visibility) in segments.iter(world) {
        if segment.ship != player {
            continue;
        }
        drawn += 1;
        if *visibility == Visibility::Hidden {
            hidden += 1;
        }
        let far =
            transform.translation + transform.rotation.mul_vec3(Vec3::Y) * transform.scale.y * 0.5;
        if end.is_none_or(|(index, _)| segment.index > index) {
            end = Some((segment.index, far));
        }
    }
    let end = end.map(|(_, far)| far);
    info!(
        "loop_docked_helm: the pair's path flies from {:.1} m off the tug's centre of mass, has \
         {drawn} ribbon segment(s), {hidden} hidden, ends {:?} off the predicted arrival, and \
         arrives {:.0} m from the beacon",
        Meters::from_engine(flown_point.distance(tug_center)).get(),
        end.map(|far| far.distance(arrival)),
        Meters::from_engine(arrival.distance(beacon)).get(),
    );
    assert!(
        flown_point.distance(tug_center) > 0.1,
        "loop_docked_helm: the prediction follows the tug's centre of mass, not the pair's"
    );
    assert!(drawn >= 1, "loop_docked_helm: the ribbon drew no segment");
    assert_eq!(hidden, 0, "loop_docked_helm: a ribbon segment is hidden");
    assert!(
        end.is_some_and(|far| far.distance(arrival) < 0.01),
        "loop_docked_helm: the ribbon must end on the predicted arrival"
    );
}

/// Pin the camera behind and above the pair, looking down its track, so the
/// pair and its path to the beacon share the frame.
#[cfg(feature = "debug")]
fn frame_pair_path(world: &mut World) {
    use avian3d::prelude::Position;

    let player = root(world, PLAYER_ID);
    let at = world.get::<Position>(player).unwrap().0;
    let beacon = world
        .query::<(&EntityId, &GlobalTransform)>()
        .iter(world)
        .find(|(id, _)| id.0 == BEACON_ID)
        .map(|(_, transform)| transform.translation())
        .unwrap_or_else(|| panic!("loop_docked_helm: no beacon {BEACON_ID}"));
    let track = (beacon - at).normalize_or(Vec3::X);
    let eye = at - track * Meters(160.0).to_engine() + Vec3::Y * Meters(70.0).to_engine();
    let look_at = at + track * Meters(400.0).to_engine();
    pose_camera(
        world,
        Meters3::from_engine(eye),
        Meters3::from_engine(look_at),
    );
}

/// Press and release a key action, one frame each.
#[cfg(feature = "debug")]
fn tap(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    label: &str,
    action: &'static str,
    landed: std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("press {label}"))
        .on_enter(press_action(action))
        .until(frames(1))
        .add()
        .step(format!("let {label} up"))
        .on_enter(release_action(action))
        .until(landed)
        .deadline(STEP_DEADLINE)
        .add()
}

/// Dock, take the HELM, fly the pair as one body, hand the HELM back, then
/// take it again and shoot the pair's predicted GOTO path on the HUD and the Map.
#[cfg(feature = "debug")]
fn helm_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the tug and the tender")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(STEP_DEADLINE)
        .add()
        .step("settle the scene and drop the status bar")
        .on_enter(hide_status_bar)
        .until(and(scenario_is_built(), frames(10)))
        .deadline(STEP_DEADLINE)
        .add()
        .step("lock the tender")
        .on_enter(|world: &mut World| {
            let player = root(world, PLAYER_ID);
            let tender = root(world, TENDER_ID);
            world.entity_mut(player).insert(TravelLock(Some(tender)));
        })
        .until(frames(1))
        .add();
    let script = tap(
        script,
        "the dock key",
        "dock",
        any_entity::<(With<PlayerSpaceshipMarker>, With<DockedShip>)>(),
    )
    .step("check the dock starts neutral")
    .on_enter(|world: &mut World| assert_helm(world, false))
    .until(frames(1))
    .add()
    .step("wait for the HELM chip to offer the helm")
    .until(helm_chip_is(DockChipState::Available))
    .deadline(STEP_DEADLINE)
    .add()
    .step("lock the beacon")
    .on_enter(|world: &mut World| {
        let player = root(world, PLAYER_ID);
        let beacon = world
            .query::<(Entity, &EntityId)>()
            .iter(world)
            .find(|(_, id)| id.0 == BEACON_ID)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("loop_docked_helm: no beacon {BEACON_ID}"));
        world.entity_mut(player).insert(TravelLock(Some(beacon)));
    })
    .until(frames(1))
    .add()
    .step("settle the lock")
    .until(elapsed(SETTLE_SECS))
    .add()
    .step("record the pair's start pose")
    .on_enter(record_pair_start)
    .until(frames(1))
    .add()
    .step("open the docked-helm loop on NEUTRAL")
    .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
    .until(elapsed(NEUTRAL_HOLD_SECS))
    .add();
    let script = tap(
        script,
        "the helm key",
        "dock_helm",
        helm_chip_is(DockChipState::Hot),
    )
    .step("check the HELM is taken")
    .on_enter(|world: &mut World| assert_helm(world, true))
    .until(frames(1))
    .add();
    let script = tap(
        script,
        "the goto key",
        "autopilot_goto",
        any_entity::<(With<PlayerSpaceshipMarker>, With<Autopilot>)>(),
    );
    let script = script
        .step("watch the pair turn and burn as one body")
        .on_enter(|world: &mut World| world.insert_resource(AssertPairOneBodyGate))
        .until(pair_turned_and_burning())
        .deadline(STEP_DEADLINE)
        .add()
        .step("stop the joint-drift watch")
        .on_enter(|world: &mut World| {
            world.remove_resource::<AssertPairOneBodyGate>();
        })
        .until(frames(1))
        .add();
    let script = tap(
        script,
        "the helm key",
        "dock_helm",
        helm_chip_is(DockChipState::Available),
    )
    .step("check the HELM is handed back")
    .on_enter(|world: &mut World| {
        assert_helm(world, false);
        let player = root(world, PLAYER_ID);
        let tender = root(world, TENDER_ID);
        assert!(
            world.get::<DockedShip>(player).is_some(),
            "loop_docked_helm: the player undocked"
        );
        assert!(
            world.get::<DockedShip>(tender).is_some(),
            "loop_docked_helm: the tender undocked"
        );
    })
    .until(frames(1))
    .add();
    let script = script
        .step("hold the handed-back helm")
        .until(elapsed(ENDING_HOLD_SECS))
        .add()
        .step("close the docked-helm loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(WRITE_DEADLINE)
        .add();
    // The GOTO waits while the HELM is neutral; taking it back flies the
    // pair again, and the pair's path is what the stills show.
    let script = tap(
        script,
        "the helm key again",
        "dock_helm",
        helm_chip_is(DockChipState::Hot),
    )
    .step("wait for the pair's predicted path")
    .until(pair_path_drawn())
    .deadline(PREDICTION_DEADLINE)
    .add()
    .step("check the pair's path")
    .on_enter(assert_pair_path)
    .until(frames(1))
    .add()
    .step("frame the pair's path for the HUD still")
    .on_enter(|world: &mut World| {
        if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
            *hud = HudVisibility::On;
        }
        frame_pair_path(world);
    })
    .until(frames(HUD_SETTLE_FRAMES))
    .add()
    .step("shoot the HUD")
    .on_enter(|world: &mut World| shoot(world, HUD_SHOT))
    .until(shot_written(HUD_SHOT))
    .deadline(WRITE_DEADLINE)
    .add();
    tap(
        script,
        "the interface key",
        "interface_toggle",
        the_interface_shows(nova_protocol::nova_interface::pane::InterfacePaneType::Map),
    )
    .step("settle the map")
    .until(frames(MAP_SETTLE_FRAMES))
    .add()
    .step("shoot the map")
    .on_enter(|world: &mut World| shoot(world, MAP_SHOT))
    .until(shot_written(MAP_SHOT))
    .deadline(WRITE_DEADLINE)
    .add()
    .step("pull the map back to the wheel's reach")
    .on_enter(pull_map_back)
    .until(frames(MAP_SETTLE_FRAMES))
    .add()
    .step("report the map's reach")
    .on_enter(report_map_reach)
    .until(frames(1))
    .add()
    .step("shoot the map at its reach")
    .on_enter(|world: &mut World| shoot(world, MAP_REACH_SHOT))
    .until(shot_written(MAP_REACH_SHOT))
    .deadline(WRITE_DEADLINE)
    .add()
}

/// Turn the wheel back far past any reach in one frame: the Map clamps the
/// zoom to what the scene needs.
#[cfg(feature = "debug")]
fn pull_map_back(world: &mut World) {
    use bevy::input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    };

    world.write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: -400.0,
        window: Entity::PLACEHOLDER,
        phase: TouchPhase::Moved,
    });
}

/// Log the Map camera's distance from the pair against the reach the
/// contacts alone give and the reach the route's end needs.
#[cfg(feature = "debug")]
fn report_map_reach(world: &mut World) {
    use bevy::ecs::system::RunSystemOnce;
    use nova_protocol::nova_interface::map::prelude::{map_radius_max, map_spread, MapContacts};

    let player = root(world, PLAYER_ID);
    let at = world.get::<GlobalTransform>(player).unwrap().translation();
    let route_end = world
        .get::<FlightPrediction>(player)
        .and_then(|prediction| prediction.points.last().copied())
        .expect("loop_docked_helm: the pair has a predicted route");
    let contacts_reach = world
        .run_system_once(move |contacts: MapContacts| map_radius_max(map_spread(&contacts, at)))
        .expect("loop_docked_helm: the map contacts read");
    let camera = world
        .query::<(&Name, &GlobalTransform)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == "NovaOsMapCamera")
        .map(|(_, transform)| transform.translation())
        .expect("loop_docked_helm: the map camera");
    info!(
        "loop_docked_helm: map reach: camera {:.0} m from the pair, contacts alone allow \
         {:.0} m, the route end is {:.0} m out and allows {:.0} m",
        Meters::from_engine(camera.distance(at)).get(),
        Meters::from_engine(contacts_reach).get(),
        Meters::from_engine(route_end.distance(at)).get(),
        Meters::from_engine(map_radius_max(route_end.distance(at))).get(),
    );
}
