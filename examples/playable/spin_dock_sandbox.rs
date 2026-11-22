//! spin_dock_sandbox: dock with a tumbling wreck once its tumble settles.
//!
//! The player flies the ported tender from `shared/docking_pair.rs`. 200 m
//! ahead drifts a neutralized raider, the base fleet's armed corvette. THE
//! WRECK IS DISCLOSED FIXTURE STATE:
//!
//! | What | Fixture |
//! | - | - |
//! | `bridge`, the raider's one flight computer | killed by a lethal hit at start |
//! | spin | [`TUMBLE_DPS`] about a tilted axis, the rate a real computer kill left |
//! | drift | [`DRIFT`], kept: a dead hull coasts |
//! | `port_collar` | free |
//! | loose cargo canisters within 100 m | removed as the tumble is set |
//!
//! A ship root with no live flight computer settles its spin: the spin halves
//! every 8 s, so the tumble falls under the 5 deg/s capture limit in about
//! half a minute. The drift stays. The capture reads the motion of the two
//! port faces, so a hull matched to the wreck's centre is still too fast at a
//! port the tumble swings.
//!
//! What to do:
//!
//! 1. watch the tumble settle. The diagnostics layer (F3) shows the spin.
//! 2. put the wreck under the screen centre and HOLD the radar key until the
//!    lock takes.
//! 3. MATCH (Shift + X) takes the tender onto the wreck's drift.
//! 4. fly the docking sight to the collar and dock (D). The wreck is dead, so
//!    the helm is yours at capture.
//!
//! ```text
//! nix develop --command cargo run --example spin_dock_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk checks the tumble, waits for it to settle and checks the drift,
//! locks the wreck, MATCHes, puts the tender on the collar's berth at its
//! own matched velocity, presses DOCK and checks the dock and the helm.
//! `NOVA_CAPTURE=1` shoots the tumble and the settled berth with the sight
//! up.

#[expect(dead_code, reason = "this example flies the tender and moors no spar")]
#[path = "shared/docking_pair.rs"]
mod docking_pair;
#[path = "../screenshots/shared/kit.rs"]
mod kit;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, RigidBody, Rotation};
use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The wreck's scenario id and name.
const WRECK_ID: &str = "tumbling_raider";
const WRECK_NAME: &str = "RAIDER WRECK";
/// The catalog hull of the wreck.
const WRECK_HULL: &str = "raider";
/// The design id of the wreck's one flight computer.
const WRECK_COMPUTER: &str = "bridge";
/// Where the wreck drifts at start, meters.
const WRECK_POSITION: Meters3 = Meters3::new(0.0, 10.0, -200.0);

/// The wreck's spin at start, deg/s: the fastest a raider turned at the tick
/// its flight computer died, read in AI duels of shipped hulls killed
/// mid-fight (8.8, 46.1 and 55.1 deg/s; a patrol picket 64.8).
const TUMBLE_DPS: f32 = 55.0;
/// The wreck's drift, meters per second.
const DRIFT: MetersPerSecond3 = MetersPerSecond3::new(3.0, 0.0, 0.0);

/// Seconds of game time the dead bridge's fragments get to clear the hull
/// before the tumble is set, at least and at most.
const DEBRIS_CLEAR_SECS: f32 = 5.0;
const DEBRIS_WAIT_SECS: f32 = 60.0;
/// Engine units from the wreck's centre inside which a free body counts as
/// at the hull: 100 m, past the raider's reach.
const DEBRIS_REACH: f32 = 10.0;

/// The capture gate's spin limit, deg/s.
const GATE_DPS: f32 = 5.0;

/// The player's scenario id and ship name.
const PLAYER_ID: &str = "player_spaceship";
const PLAYER_NAME: &str = "Tender";

/// The intro line.
const INTRO: &str = "A dead raider tumbles: wait for it to settle, lock it, MATCH, then dock.";

fn main() -> bevy::app::AppExit {
    open_world_stage::isolate_the_profile("spin_dock_sandbox");
    let mut app = AppBuilder::new()
        .with_game_plugins(spin_dock_plugin)
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        PostUpdate,
        draw_the_wreck
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
        app.add_systems(Startup, hide_dev_overlays);
        app.add_systems(First, open_world_stage::hold_the_walk_clock);
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn spin_dock_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_observer(note_match);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(
        Update,
        stage_the_wreck.run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_void(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShipDesigns>,
) {
    let object = |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        })
    };
    let player = object(
        PLAYER_ID,
        PLAYER_NAME,
        Meters3::ZERO,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(docking_pair::tender()),
            ..default()
        },
    );
    let wreck = object(
        WRECK_ID,
        WRECK_NAME,
        WRECK_POSITION,
        SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            allegiance: Some(Allegiance::Enemy),
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, WRECK_HULL)),
            initial_velocity: DRIFT,
            ..default()
        },
    );
    let mut actions = vec![player, wreck];
    actions.extend(ThreePointRig::around("tender", Meters3::ZERO, 1.0).actions());
    actions.extend(ThreePointRig::around("wreck", WRECK_POSITION, 2.0).actions());
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A tumbling dead raider and a ported tender".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "spin_dock_sandbox".to_string(),
            "Spin Dock Sandbox".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// Where the staging stands. The walk reads it, because a walk predicate
/// reads the world but cannot query it.
#[derive(Resource, Default)]
struct Staging {
    /// The lethal hit on the wreck's computer landed.
    killed: bool,
    /// The virtual time the wreck was first seen with no live computer.
    dead_at: Option<f32>,
    /// The virtual time the tumble was set, once the computer was dead.
    spun_at: Option<f32>,
    /// The wreck's drift as the settle wait starts, engine units. Read
    /// there, after the tumble shot, not from [`DRIFT`]: the staging spins
    /// the hull up in one tick, and that kick moves the drift once (about
    /// 0.1 m/s in the walks). The settle after it must not move it.
    drift: Option<Vec3>,
    /// The player's ship engaged MATCH. Set by [`note_match`]: on a slow
    /// frame MATCH can engage and finish between two walk checks.
    matched_by_match: bool,
}

/// Note the player's ship engaging MATCH.
fn note_match(
    insert: On<Insert<Autopilot>>,
    q_player: Query<&Autopilot, With<PlayerSpaceshipMarker>>,
    mut staging: ResMut<Staging>,
) {
    if let Ok(autopilot) = q_player.get(insert.entity) {
        if matches!(autopilot.action, AutopilotAction::MatchTarget { .. }) {
            staging.matched_by_match = true;
        }
    }
}

/// The wreck root, once it has spawned.
fn wreck_root(world: &mut World) -> Option<Entity> {
    let mut roots = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    roots
        .iter(world)
        .find(|(_, id)| id.0 == WRECK_ID)
        .map(|(entity, _)| entity)
}

/// The section of `root` with design id `id`.
fn section_of(world: &mut World, root: Entity, id: &str) -> Option<Entity> {
    let mut sections = world.query_filtered::<(Entity, &EntityId, &ChildOf), With<SectionMarker>>();
    sections
        .iter(world)
        .find(|(_, found, child)| found.0 == id && child.parent() == root)
        .map(|(entity, ..)| entity)
}

/// Whether `root` has a live flight computer.
fn has_live_computer(world: &mut World, root: Entity) -> bool {
    let mut computers = world.query_filtered::<&ChildOf, (
        With<ControllerSectionMarker>,
        With<PDController>,
        Without<SectionInactiveMarker>,
    )>();
    computers.iter(world).any(|child| child.parent() == root)
}

/// Remove the loose cargo canisters within [`DEBRIS_REACH`] of `wreck`. In
/// the walks one stood by the dead hull's intake and the tumbling hull struck
/// it, which moved the drift the walk judges the settle by.
fn clear_canisters(world: &mut World, wreck: Entity) {
    let Some(at) = world.get::<Position>(wreck).map(|at| at.0) else {
        return;
    };
    let mut canisters = world.query_filtered::<(Entity, &Position), With<CargoCanister>>();
    let near: Vec<Entity> = canisters
        .iter(world)
        .filter(|(_, position)| position.0.distance(at) < DEBRIS_REACH)
        .map(|(entity, _)| entity)
        .collect();
    for canister in &near {
        world.entity_mut(*canister).despawn();
    }
    if !near.is_empty() {
        info!(
            "spin_dock_sandbox: removed {} loose canister(s) at the wreck",
            near.len()
        );
    }
}

/// The free bodies other than the player within [`DEBRIS_REACH`] of `wreck`.
fn bodies_near(world: &mut World, wreck: Entity) -> Vec<String> {
    let Some(at) = world.get::<Position>(wreck).map(|at| at.0) else {
        return Vec::new();
    };
    let mut bodies = world.query_filtered::<(Entity, &Position, &RigidBody, Option<&Name>), (Without<PlayerSpaceshipMarker>, Without<CargoCanister>)>();
    bodies
        .iter(world)
        .filter(|(entity, position, body, _)| {
            *entity != wreck && !body.is_static() && position.0.distance(at) < DEBRIS_REACH
        })
        .map(|(entity, position, _, name)| {
            format!(
                "{entity} {} at {:.0} m",
                name.map_or("unnamed", |name| name.as_str()),
                Meters3::from_engine(position.0 - at).get().length()
            )
        })
        .collect()
}

/// The length of the engine velocity `engine`, in meters per second.
fn speed(engine: Vec3) -> f32 {
    MetersPerSecond3::from_engine(engine).get().length()
}

/// The tumble axis: tilted off every hull axis, so the collar swings.
fn tumble() -> Vec3 {
    Vec3::new(0.35, 1.0, 0.25).normalize() * TUMBLE_DPS.to_radians()
}

/// Kill the wreck's computer through the production damage entry point,
/// then, once it is dead, set the tumble and the drift.
fn stage_the_wreck(world: &mut World) {
    if world.resource::<Staging>().spun_at.is_some() {
        return;
    }
    let Some(wreck) = wreck_root(world) else {
        return;
    };
    if !world.resource::<Staging>().killed {
        // Wait for the ship graph, as a fight does: the hit lands on a hull
        // whose sections are built and joined.
        let Some(computer) = section_of(world, wreck, WRECK_COMPUTER).filter(|section| {
            world.get::<Health>(*section).is_some() && world.get::<ConnectedTo>(*section).is_some()
        }) else {
            return;
        };
        world.trigger(HealthApplyDamage {
            entity: computer,
            source: None,
            amount: 1e6,
        });
        world.resource_mut::<Staging>().killed = true;
        return;
    }
    if has_live_computer(world, wreck) {
        return;
    }
    // The dead bridge leaves as a wreck piece, and loose bodies such as a
    // canister can stand at the hull: let them clear before it tumbles, or a
    // body the hull swings into moves its drift. Past `DEBRIS_WAIT_SECS` the
    // tumble is set anyway and the log names what is still there.
    let now = world.resource::<Time<Virtual>>().elapsed_secs();
    let dead_at = *world.resource_mut::<Staging>().dead_at.get_or_insert(now);
    let near = bodies_near(world, wreck);
    if now - dead_at < DEBRIS_CLEAR_SECS || (!near.is_empty() && now - dead_at < DEBRIS_WAIT_SECS) {
        return;
    }
    if !near.is_empty() {
        warn!("spin_dock_sandbox: bodies still at the wreck as it tumbles: {near:?}");
    }
    clear_canisters(world, wreck);
    world
        .entity_mut(wreck)
        .insert((AngularVelocity(tumble()), LinearVelocity(DRIFT.to_engine())));
    world.resource_mut::<Staging>().spun_at = Some(now);
    info!(
        "spin_dock_sandbox: the wreck's computer is dead; tumble {TUMBLE_DPS} deg/s, drift {:?} m/s",
        DRIFT
    );
}

/// The panel: the wreck's spin against the gate, and its drift.
fn draw_the_wreck(
    mut panel: ResMut<DiagnosticsPanel>,
    q_wreck: Query<(&EntityId, &AngularVelocity, &LinearVelocity), With<SpaceshipRootMarker>>,
    q_player: Query<Has<DockedShip>, With<PlayerSpaceshipMarker>>,
) {
    let Some((_, spin, drift)) = q_wreck.iter().find(|(id, ..)| id.0 == WRECK_ID) else {
        return;
    };
    let dps = spin.0.length().to_degrees();
    panel.line(format!(
        "Wreck spin: {dps:.1} deg/s ({})",
        if dps < GATE_DPS {
            "under the gate"
        } else {
            "settling"
        }
    ));
    panel.line(format!("Wreck drift: {:.1} m/s", speed(drift.0)));
    panel.detail(format!(
        "Capture gate: {GATE_DPS:.0} deg/s and 5 m/s between the port faces"
    ));
    if q_player.single().unwrap_or(false) {
        panel.hint("D", "undock");
    } else {
        panel.hint("Ctrl", "hold on the wreck to lock it");
        panel.hint("Shift + X", "MATCH");
        panel.hint("D", "dock");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS};

    use super::*;

    /// Seconds the walk gives the void to load and the wreck to be staged.
    const LOAD_SECS: f32 = 600.0;
    /// Seconds any later beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 120.0;
    /// Seconds the settle may take, real time: 35 s of sim time, slowed by
    /// a software-rendered GPU.
    const SETTLE_REAL_SECS: f32 = 900.0;
    /// Sim seconds by which the tumble must be under the gate.
    const SETTLE_BY_SECS: f32 = 35.0;
    /// Seconds the radar key is held: past the gesture and the dwell.
    const LOCK_HOLD: f32 = 3.0;
    /// m/s within which the tender's velocity counts as matched.
    const MATCHED: f32 = 0.2;
    /// Frames a check step holds.
    const SETTLE_FRAMES: u32 = 10;

    /// The frames the capture path writes.
    const TUMBLE_SHOT: &str = "spin-dock-sandbox-tumble.png";
    const SETTLED_SHOT: &str = "spin-dock-sandbox-settled.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn read_wreck(world: &World) -> Option<(Vec3, Vec3)> {
        world
            .try_query::<(&EntityId, &LinearVelocity, &AngularVelocity)>()
            .and_then(|mut query| {
                query
                    .iter(world)
                    .find(|(id, ..)| id.0 == WRECK_ID)
                    .map(|(_, v, w)| (v.0, w.0))
            })
    }

    fn wreck_dps(world: &World) -> f32 {
        read_wreck(world).map_or(f32::INFINITY, |(_, w)| w.length().to_degrees())
    }

    fn player_velocity(world: &World) -> Option<Vec3> {
        world
            .try_query_filtered::<&LinearVelocity, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next().map(|v| v.0))
    }

    /// The tumble is over the gate and the wreck is dead.
    fn assert_the_tumble(world: &mut World) {
        let wreck = wreck_root(world).expect("spin_dock_sandbox: the wreck is up");
        assert!(
            !has_live_computer(world, wreck),
            "spin_dock_sandbox: the wreck has no live flight computer"
        );
        assert!(
            world.get::<NeutralizedMarker>(wreck).is_some(),
            "spin_dock_sandbox: the wreck is neutralized"
        );
        let dps = wreck_dps(world);
        assert!(
            dps > GATE_DPS,
            "spin_dock_sandbox: the wreck tumbles over the gate at start: {dps} deg/s"
        );
        let (drift, _) = read_wreck(world).expect("spin_dock_sandbox: the wreck is up");
        info!(
            "spin_dock_sandbox: PASS the dead wreck tumbles at {dps:.1} deg/s, over the {GATE_DPS} deg/s gate, drifting {:.3} m/s",
            speed(drift)
        );
    }

    /// The tumble is under the gate by [`SETTLE_BY_SECS`] and the drift is
    /// the drift it was given.
    fn assert_the_settle(world: &mut World) {
        let spun_at = world
            .resource::<Staging>()
            .spun_at
            .expect("spin_dock_sandbox: the tumble was staged");
        let took = world.resource::<Time<Virtual>>().elapsed_secs() - spun_at;
        let dps = wreck_dps(world);
        assert!(
            dps < GATE_DPS && took <= SETTLE_BY_SECS,
            "spin_dock_sandbox: under {GATE_DPS} deg/s by {SETTLE_BY_SECS} s: {dps} deg/s after {took} s"
        );
        let (drift, _) = read_wreck(world).expect("spin_dock_sandbox: the wreck is up");
        let want = world
            .resource::<Staging>()
            .drift
            .expect("spin_dock_sandbox: the drift was read as the wait started");
        assert!(
            speed(drift - want) < 0.01,
            "spin_dock_sandbox: the settle leaves the drift: {drift} against {want}"
        );
        info!(
            "spin_dock_sandbox: PASS the tumble settled to {dps:.2} deg/s after {took:.1} s, drift {:.3} m/s unchanged",
            speed(drift)
        );
    }

    /// The radar hold locked the wreck.
    fn assert_the_lock(world: &mut World) {
        let wreck = wreck_root(world).expect("spin_dock_sandbox: the wreck is up");
        let mut ships = world.query_filtered::<&TravelLock, With<PlayerSpaceshipMarker>>();
        let locked = ships.iter(world).next().and_then(|lock| lock.0);
        assert_eq!(
            locked,
            Some(wreck),
            "spin_dock_sandbox: the radar hold locks the wreck"
        );
        info!("spin_dock_sandbox: PASS the wreck is locked");
    }

    fn match_seen(world: &World) -> bool {
        world
            .get_resource::<Staging>()
            .is_some_and(|staging| staging.matched_by_match)
    }

    fn assert_match_engaged(world: &mut World) {
        assert!(
            match_seen(world),
            "spin_dock_sandbox: Shift + X engages MATCH"
        );
        info!("spin_dock_sandbox: PASS MATCH engaged on the wreck");
    }

    fn matched(world: &World) -> bool {
        let (Some((drift, _)), Some(velocity)) = (read_wreck(world), player_velocity(world)) else {
            return false;
        };
        speed(velocity - drift) < MATCHED
    }

    fn assert_matched(world: &mut World) {
        let (drift, _) = read_wreck(world).expect("spin_dock_sandbox: the wreck is up");
        let velocity = player_velocity(world).expect("spin_dock_sandbox: a player");
        info!(
            "spin_dock_sandbox: PASS MATCH holds the wreck's drift: {:.3} m/s off",
            speed(velocity - drift)
        );
    }

    /// Put the tender on the collar's berth at its own matched velocity and
    /// with no spin, and point the mouse rig where the berth points it.
    /// Returns whether the tender already stood on the berth.
    ///
    /// Unlike `open_world_stage::berth`, the tender does NOT take the
    /// wreck's spin: it keeps the velocity MATCH gave it, so the capture
    /// judges the face speed the settled tumble leaves.
    fn berth_at_matched_velocity(world: &mut World) -> bool {
        let player = open_world_stage::the_player(world);
        let (Some(player), Some(wreck)) = (player, wreck_root(world)) else {
            return false;
        };
        let (Some(target_port), Some(own_port)) = (
            open_world_stage::outward_port(world, wreck),
            open_world_stage::free_ports(world, player)
                .into_iter()
                .next(),
        ) else {
            panic!("spin_dock_sandbox: the wreck or the tender has no free docking port");
        };
        let position = world.get::<Position>(wreck).expect("a wreck stands").0;
        let rotation = world.get::<Rotation>(wreck).expect("a wreck stands").0;
        let port = open_world_stage::local_pose(world, target_port, wreck);
        let own = open_world_stage::local_pose(world, own_port, player);
        let (at, turn) = parked_hull_pose((position, rotation), port, own);
        let stands = |found_at: Vec3, found_turn: Quat| {
            found_at.distance(at) < BERTH_TOLERANCE
                && found_turn.angle_between(turn) < BERTH_TOLERANCE
        };
        let settled = world
            .get::<Position>(player)
            .zip(world.get::<Rotation>(player))
            .is_some_and(|(found_at, found_turn)| stands(found_at.0, found_turn.0))
            && world.get::<GlobalTransform>(player).is_some_and(|shown| {
                let (_, shown_turn, shown_at) = shown.to_scale_rotation_translation();
                stands(shown_at, shown_turn)
            });
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            AngularVelocity(Vec3::ZERO),
        ));
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<PointRotationOutput>>()
            .iter(world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).insert((
                PointRotation {
                    initial_rotation: turn,
                },
                PointRotationOutput(turn),
            ));
        }
        settled
    }

    /// How close to its berth, in engine units and in radians, the tender
    /// stands for the berth to count as settled.
    const BERTH_TOLERANCE: f32 = 0.05;

    #[derive(Resource, Default)]
    struct Berthed(bool);

    /// The berth's face speed passes the gate, judged by the pair the
    /// capture itself reads.
    fn report_the_berth(world: &mut World) {
        let dps = wreck_dps(world);
        let (drift, spin) = read_wreck(world).expect("spin_dock_sandbox: the wreck is up");
        let velocity = player_velocity(world).expect("spin_dock_sandbox: a player");
        info!(
            "spin_dock_sandbox: berthed; wreck spin {dps:.2} deg/s ({spin}), centre speed {:.3} m/s",
            speed(velocity - drift)
        );
    }

    fn docked(world: &World) -> bool {
        world
            .try_query_filtered::<(), (With<PlayerSpaceshipMarker>, With<DockedShip>)>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    }

    /// The dock holds on the wreck and the player holds the helm.
    fn assert_the_dock(world: &mut World) {
        let player = open_world_stage::the_player(world).expect("spin_dock_sandbox: a player");
        let wreck = wreck_root(world).expect("spin_dock_sandbox: the wreck is up");
        let connection = world
            .get::<DockedShip>(player)
            .expect("spin_dock_sandbox: a docked player carries DockedShip")
            .connection;
        let connection = world
            .get::<DockingConnection>(connection)
            .expect("spin_dock_sandbox: the dock has its connection");
        assert!(
            [connection.first_ship, connection.second_ship].contains(&wreck),
            "spin_dock_sandbox: the dock holds on the wreck"
        );
        assert_eq!(
            connection.helm,
            DockedHelmType::Held(player),
            "spin_dock_sandbox: the dead wreck gives the player the helm"
        );
        info!("spin_dock_sandbox: PASS docked to the settled wreck, the helm with the player");
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("spin dock sandbox: load the void and stage the wreck")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.spun_at.is_some())
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("spin dock sandbox: check the tumble")
            .on_enter(assert_the_tumble)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("spin dock sandbox: shoot the tumble")
            .on_enter(|world: &mut World| shoot(world, TUMBLE_SHOT))
            .until(shot_written(TUMBLE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("spin dock sandbox: wait for the tumble to settle")
            .on_enter(|world: &mut World| {
                let (drift, _) = read_wreck(world).expect("spin_dock_sandbox: the wreck is up");
                world.resource_mut::<Staging>().drift = Some(drift);
            })
            .each(|world, _, frame| {
                if frame % 20 == 0 {
                    if let Some((drift, spin)) = read_wreck(world) {
                        info!(
                            "spin_dock_sandbox: settling {:.2} deg/s, drift {:.4} m/s",
                            spin.length().to_degrees(),
                            speed(drift)
                        );
                    }
                }
            })
            .until(when(|world| wreck_dps(world) < GATE_DPS))
            .deadline(SETTLE_REAL_SECS)
            .add()
            .step("spin dock sandbox: check the settle and the drift")
            .on_enter(assert_the_settle)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("spin dock sandbox: centre the wreck")
            .each(keep_centred_on(scenario_object(WRECK_ID)))
            .until(elapsed(1.0))
            .add()
            .step("spin dock sandbox: hold the radar onto the wreck")
            .on_enter(press_action("radar_hold"))
            .each(keep_centred_on(scenario_object(WRECK_ID)))
            .until(elapsed(LOCK_HOLD))
            .add()
            .step("spin dock sandbox: release the radar")
            .on_enter(release_action("radar_hold"))
            .until(elapsed(0.5))
            .add()
            .step("spin dock sandbox: check the lock")
            .on_enter(assert_the_lock)
            .until(frames(SETTLE_FRAMES))
            .add()
            // MATCH is the STOP key with the RCS modifier held.
            .step("spin dock sandbox: hold the RCS modifier")
            .on_enter(press_action("rcs_modifier"))
            .until(elapsed(0.3))
            .add()
            // Let go of the chord the frame MATCH engages: RCS input with the
            // modifier held ends MATCH, and the harness's cursor pin is mouse
            // input.
            .step("spin dock sandbox: MATCH")
            .on_enter(press_action("autopilot_stop"))
            .until(when(match_seen))
            .deadline(BEAT_SECS)
            .add()
            .step("spin dock sandbox: let go of the chord")
            .on_enter(release_action("autopilot_stop"))
            .on_enter(release_action("rcs_modifier"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("spin dock sandbox: MATCH flies the tender onto the drift")
            .on_enter(assert_match_engaged)
            .until(when(matched))
            .deadline(BEAT_SECS)
            .add()
            .step("spin dock sandbox: check the match")
            .on_enter(assert_matched)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("spin dock sandbox: put the tender on the collar's berth")
            .on_enter(|world: &mut World| {
                world.init_resource::<Berthed>();
            })
            .each(|world, _, _| {
                if berth_at_matched_velocity(world) {
                    world.resource_mut::<Berthed>().0 = true;
                }
            })
            .until(when(|world| {
                world
                    .get_resource::<Berthed>()
                    .is_some_and(|berthed| berthed.0)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("spin dock sandbox: report the berth")
            .on_enter(report_the_berth)
            .until(frames(2))
            .add()
            .step("spin dock sandbox: shoot the settled berth")
            .on_enter(|world: &mut World| shoot(world, SETTLED_SHOT))
            .until(shot_written(SETTLED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("spin dock sandbox: press DOCK")
            .on_enter(press_action("dock"))
            .until(elapsed(0.3))
            .add()
            .step("spin dock sandbox: release DOCK")
            .on_enter(release_action("dock"))
            .until(when(docked))
            .deadline(BEAT_SECS)
            .add()
            .step("spin dock sandbox: check the dock and the helm")
            .on_enter(assert_the_dock)
            .until(frames(SETTLE_FRAMES))
            .add()
    }
}
