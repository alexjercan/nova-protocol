//! The port's contract: which pairs `DOCK` accepts, what one accepted pair
//! builds, and what takes it apart again.
//!
//! Real avian, because the thing docking produces is a joint between two
//! rigid bodies and a hand-built rig cannot stand in for one. Every fixture
//! here is two one-cell hulls facing each other along Z with their ports on
//! the facing ends, so a gap in these tests is a gap in engine units and the
//! authored `capture_distance` (10 m, one cell) is the number under test.

use avian3d::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use nova_events::prelude::EntityId;
use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

use super::*;
use crate::{
    flight::NovaFlightPlugin,
    sections::docking_section::connection::release_broken_docking_connections,
};

/// The face gap the fixture stands at unless a test says otherwise: half a
/// cell, comfortably inside the one-cell capture distance.
const NOMINAL_GAP: f32 = 0.5;

/// A headless app carrying docking and nothing else.
fn docking_app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins(DockingSectionPlugin { render: false });
    app.finish();
    app
}

/// One hull at `at`, facing down `rotation`, as a dynamic body. Its
/// `HullRadius` is the one-cell block's half diagonal, set here because the
/// section plugin that measures it is not in these apps.
fn spawn_hull(app: &mut App, at: Vec3, rotation: Quat) -> Entity {
    let ship = app
        .world_mut()
        .spawn((
            Name::new("hull"),
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            Transform::from_translation(at).with_rotation(rotation),
            HullRadius(0.87),
        ))
        .id();
    app.world_mut().spawn((
        ChildOf(ship),
        Name::new("hull block"),
        Transform::default(),
        Collider::cuboid(1.0, 1.0, 1.0),
        ColliderDensity(1.0),
    ));
    ship
}

/// One port on `ship`, mounted at `offset` and turned by `rotation` in the
/// hull's own frame. `id` is the design-local section id the tie-break reads.
fn spawn_port(app: &mut App, ship: Entity, id: &str, offset: Vec3, rotation: Quat) -> Entity {
    app.world_mut()
        .spawn((
            ChildOf(ship),
            Name::new(id.to_string()),
            EntityId(id.to_string()),
            Transform::from_translation(offset).with_rotation(rotation),
            docking_section(DockingSectionConfig::default()),
        ))
        .id()
}

/// The standard fixture: two hulls nose to nose down Z, one port each on the
/// facing end, `gap` engine units between the two retracted faces.
///
/// The ports sit on the hull origin and face local -Z, so the second hull is
/// turned about Y to point its port back at the first. Each face stands half
/// a cell off its own hull centre, which is why the hull separation carries
/// the two half-cells plus the gap.
fn facing_pair(app: &mut App, gap: f32) -> (Entity, Entity, Entity, Entity) {
    let first = spawn_hull(app, Vec3::ZERO, Quat::IDENTITY);
    let second = spawn_hull(
        app,
        Vec3::NEG_Z * (1.0 + gap),
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    let first_port = spawn_port(app, first, "fore", Vec3::ZERO, Quat::IDENTITY);
    let second_port = spawn_port(app, second, "fore", Vec3::ZERO, Quat::IDENTITY);
    settle(app);
    (first, first_port, second, second_port)
}

/// The best candidate between two ships, run as a one-off system.
fn candidate(app: &mut App, first: Entity, second: Entity) -> Option<DockingPair> {
    app.world_mut()
        .run_system_once(move |ports: DockingPorts| -> Option<DockingPair> {
            ports.best_candidate(first, second)
        })
        .expect("the candidate search runs")
}

/// Ask for a dock, then let the world act on it.
fn request_dock(app: &mut App, first: Entity, second: Entity) {
    app.world_mut().trigger(DockingConnectionRequest {
        entity: first,
        target: second,
    });
    settle(app);
}

/// Every live connection in the world.
fn connections(app: &mut App) -> Vec<DockingConnection> {
    let world = app.world_mut();
    let mut q = world.query::<&DockingConnection>();
    q.iter(world).copied().collect()
}

/// Every live fixed joint in the world.
fn joints(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&FixedJoint>();
    q.iter(world).count()
}

fn state(app: &App, port: Entity) -> DockingSectionState {
    *app.world()
        .get::<DockingSectionState>(port)
        .expect("a live port carries its sleeve state")
}

#[test]
fn a_facing_pair_inside_the_envelope_is_a_candidate() {
    let mut app = docking_app();
    let (first, first_port, second, second_port) = facing_pair(&mut app, NOMINAL_GAP);

    let found = candidate(&mut app, first, second).expect("a facing pair half a cell apart docks");

    assert_eq!(found.first_section, first_port);
    assert_eq!(found.second_section, second_port);
    assert!(
        (found.gap - NOMINAL_GAP).abs() < 1e-3,
        "the gap is measured between the two FACES, not the two origins: {}",
        found.gap
    );
}

#[test]
fn a_pair_beyond_the_face_gap_is_refused() {
    let mut app = docking_app();
    // One and a half cells: past the one-cell reach two sleeves can bridge.
    let (first, .., second, _) = facing_pair(&mut app, 1.5);

    assert!(
        candidate(&mut app, first, second).is_none(),
        "a pair further apart than the sleeves reach is not a candidate"
    );
}

#[test]
fn a_pair_whose_axes_are_not_opposed_is_refused() {
    let mut app = docking_app();
    let first = spawn_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
    // Turned a quarter, so its port faces across the gap rather than into it.
    let second = spawn_hull(
        &mut app,
        Vec3::NEG_Z * (1.0 + NOMINAL_GAP),
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
    );
    spawn_port(&mut app, first, "fore", Vec3::ZERO, Quat::IDENTITY);
    spawn_port(&mut app, second, "fore", Vec3::ZERO, Quat::IDENTITY);
    settle(&mut app);

    assert!(
        candidate(&mut app, first, second).is_none(),
        "ports must face each other within the capture cone"
    );
}

#[test]
fn roll_about_the_docking_axis_does_not_refuse_a_pair() {
    let mut app = docking_app();
    let first = spawn_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
    // A half turn to face back, then a third of a turn of ROLL about the
    // docking axis itself. A keyed port would refuse this; a cylinder does
    // not care, and the joint keeps whatever clocking it met.
    let second = spawn_hull(
        &mut app,
        Vec3::NEG_Z * (1.0 + NOMINAL_GAP),
        Quat::from_rotation_y(std::f32::consts::PI)
            * Quat::from_rotation_z(std::f32::consts::FRAC_PI_3),
    );
    spawn_port(&mut app, first, "fore", Vec3::ZERO, Quat::IDENTITY);
    spawn_port(&mut app, second, "fore", Vec3::ZERO, Quat::IDENTITY);
    settle(&mut app);

    assert!(
        candidate(&mut app, first, second).is_some(),
        "roll alone must not refuse a cylindrical port"
    );
}

#[test]
fn a_pair_closing_faster_than_the_limit_is_refused() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);

    // 8 m/s of closing rate against an authored 5 m/s ceiling.
    app.world_mut().entity_mut(first).insert(LinearVelocity(
        Vec3::NEG_Z * MetersPerSecond(8.0).to_engine(),
    ));
    settle(&mut app);

    assert!(
        candidate(&mut app, first, second).is_none(),
        "a pair closing above the safe rate is not a candidate"
    );

    // The same pair, drifting TOGETHER, is eligible again: what the gate
    // reads is relative motion, not speed through the world.
    app.world_mut().entity_mut(second).insert(LinearVelocity(
        Vec3::NEG_Z * MetersPerSecond(8.0).to_engine(),
    ));
    settle(&mut app);

    assert!(
        candidate(&mut app, first, second).is_some(),
        "two hulls holding station on each other dock at any shared speed"
    );
}

#[test]
fn a_reserved_port_is_not_a_candidate() {
    let mut app = docking_app();
    let (first, first_port, second, _) = facing_pair(&mut app, NOMINAL_GAP);

    let held = app.world_mut().spawn(Name::new("other connection")).id();
    app.world_mut()
        .entity_mut(first_port)
        .insert(DockedPort(held));

    assert!(
        candidate(&mut app, first, second).is_none(),
        "a port already spoken for is not on offer"
    );
}

#[test]
fn the_nearer_pair_wins_and_equal_pairs_break_on_section_id() {
    let mut app = docking_app();
    let first = spawn_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
    let second = spawn_hull(
        &mut app,
        Vec3::NEG_Z * (1.0 + NOMINAL_GAP),
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    // Two ports a cell apart across the hull's X, both facing the same way:
    // same gap, same alignment, nothing to separate them but their ids.
    let port_b = spawn_port(&mut app, first, "port_b", Vec3::X, Quat::IDENTITY);
    let port_a = spawn_port(&mut app, first, "port_a", Vec3::NEG_X, Quat::IDENTITY);
    // The partner sits opposite `port_a`, so both of the first hull's ports
    // are the same distance from it.
    spawn_port(&mut app, second, "fore", Vec3::X, Quat::IDENTITY);
    settle(&mut app);

    let found = candidate(&mut app, first, second).expect("a tied pair still resolves");

    assert_ne!(
        found.first_section, port_b,
        "the lower section id wins a tie, so iteration order cannot decide it"
    );
    assert_eq!(found.first_section, port_a);
}

#[test]
fn one_command_builds_one_connection_and_one_joint() {
    let mut app = docking_app();
    let (first, first_port, second, second_port) = facing_pair(&mut app, NOMINAL_GAP);

    request_dock(&mut app, first, second);

    let live = connections(&mut app);
    assert_eq!(live.len(), 1, "one command, one connection");
    assert_eq!(joints(&mut app), 1, "one connection, one root joint");
    assert_eq!(live[0].first_ship, first);
    assert_eq!(live[0].second_ship, second);

    for port in [first_port, second_port] {
        assert!(
            app.world().get::<DockedPort>(port).is_some(),
            "both ports are reserved by the connection"
        );
    }
    for ship in [first, second] {
        assert!(
            app.world().get::<DockedShip>(ship).is_some(),
            "both hulls know they are held"
        );
    }

    // A second command finds nothing left to offer: both ports are spoken
    // for, and one connection per hull is the whole of this baseline.
    request_dock(&mut app, first, second);
    assert_eq!(connections(&mut app).len(), 1, "no second connection");
}

#[test]
fn both_sleeves_reach_out_once_the_joint_holds() {
    let mut app = docking_app();
    let (first, first_port, second, second_port) = facing_pair(&mut app, NOMINAL_GAP);

    assert_eq!(state(&app, first_port), DockingSectionState::Retracted);

    request_dock(&mut app, first, second);

    // No authored track on this fixture, so each sleeve arrives the frame it
    // is told to leave: the state machine is what is under test, not a clock.
    for port in [first_port, second_port] {
        assert_eq!(
            state(&app, port),
            DockingSectionState::Extended,
            "a captured port reaches across"
        );
    }
}

#[test]
fn docking_never_zeroes_the_drift_the_two_hulls_share() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);

    let drift = Vec3::new(0.0, MetersPerSecond(30.0).to_engine(), 0.0);
    for ship in [first, second] {
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(drift));
    }
    settle(&mut app);

    request_dock(&mut app, first, second);

    let carried = app
        .world()
        .get::<LinearVelocity>(first)
        .expect("the hull keeps its velocity")
        .0;
    assert!(
        carried.y > drift.y * 0.5,
        "the pair keeps sailing: the joint constrains the two hulls to each \
         other, it does not stop them: {carried:?}"
    );
}

#[test]
fn dock_pressed_again_by_either_hull_takes_the_joint_away_and_stows_both_sleeves() {
    let mut app = docking_app();
    let (first, first_port, second, second_port) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);
    assert_eq!(connections(&mut app).len(), 1);

    // The SECOND hull asks to be free. Either endpoint may leave; nothing
    // here makes the ship that issued the command the one in charge.
    app.world_mut()
        .trigger(DockingReleaseRequest { entity: second });
    app.update();

    assert!(connections(&mut app).is_empty(), "the connection is gone");
    assert_eq!(joints(&mut app), 0, "and the joint with it");
    for port in [first_port, second_port] {
        assert!(
            app.world().get::<DockedPort>(port).is_none(),
            "both ports are free again"
        );
        assert_eq!(
            state(&app, port),
            DockingSectionState::Retracted,
            "both sleeves come home"
        );
    }
    for ship in [first, second] {
        assert!(
            app.world().get::<DockedShip>(ship).is_none(),
            "neither hull is held any more"
        );
    }
}

#[test]
fn a_release_request_from_a_hull_that_is_not_docked_does_nothing() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);

    let bystander = spawn_hull(&mut app, Vec3::X * 20.0, Quat::IDENTITY);
    app.world_mut()
        .trigger(DockingReleaseRequest { entity: bystander });
    app.update();

    assert_eq!(
        connections(&mut app).len(),
        1,
        "only the hulls a connection names can let it go"
    );
}

#[test]
fn a_dock_holds_through_burn_and_helm_because_the_verb_is_what_ends_it() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);

    // Everything the first cut read as "intent": a throttle held down, and a
    // helm swung a quarter turn - which is what a hand on the mouse writes
    // every frame it moves.
    app.world_mut()
        .entity_mut(second)
        .insert(FlightIntent { burn: 1.0 });
    for _ in 0..10 {
        app.update();
    }

    assert_eq!(
        connections(&mut app).len(),
        1,
        "a dock ends on the verb, not on a control that moved"
    );
}

#[test]
fn a_docked_helm_is_re_parked_on_the_attitude_the_hull_actually_has() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    let computers = [first, second].map(|ship| {
        app.world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("computer"),
                Transform::default(),
                ControllerSectionMarker,
                ControllerSectionRotationInput(Quat::IDENTITY),
            ))
            .id()
    });
    request_dock(&mut app, first, second);

    // The same order on both computers. The suppressed hull must drop it for
    // its live attitude; the driver's computer must keep it as its command.
    let stale = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    for computer in computers {
        **app
            .world_mut()
            .get_mut::<ControllerSectionRotationInput>(computer)
            .expect("the computer holds a command") = stale;
    }
    app.update();

    // Neutral: the player's hull is suppressed and the partner drives.
    let live = app
        .world()
        .get::<Rotation>(first)
        .expect("a docked hull still has an attitude")
        .0;
    let parked = app
        .world()
        .get::<ControllerSectionRotationInput>(computers[0])
        .expect("the computer holds a command")
        .0;
    assert!(
        parked.angle_between(live) < 1e-3,
        "the suppressed helm follows the hull: {parked:?} against {live:?}"
    );
    assert!(
        app.world()
            .get::<DockedShip>(first)
            .expect("the hull is held")
            .helm
            .angle_between(live)
            < 1e-3,
        "and the hull records the same attitude"
    );
    let driven = app
        .world()
        .get::<ControllerSectionRotationInput>(computers[1])
        .expect("the computer holds a command")
        .0;
    assert!(
        driven.angle_between(stale) < 1e-3,
        "the driver's command is its own, never parked: {driven:?}"
    );
}

#[test]
fn a_dock_starts_neutral_and_the_helm_toggle_never_touches_the_joint() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    request_dock(&mut app, first, second);

    let drives = |app: &App, ship: Entity| {
        app.world()
            .get::<DockedShip>(ship)
            .expect("the hull is docked")
            .drives
    };
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Neutral);
    assert!(
        drives(&app, second) && !drives(&app, first),
        "neutral: the partner drives the pair, the player does not"
    );

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(first));
    assert!(drives(&app, first) && !drives(&app, second));
    assert_eq!(joints(&mut app), 1, "taking the helm keeps the joint");

    // Only the player may hold a helm: the partner cannot take it back.
    app.world_mut()
        .trigger(DockingHelmRequest { entity: second });
    app.update();
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(first));

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Neutral);
    assert!(drives(&app, second) && !drives(&app, first));
    assert_eq!(joints(&mut app), 1, "handing the helm back keeps the joint");
    assert_eq!(connections(&mut app).len(), 1);
}

#[test]
fn only_the_driver_actuates_a_docked_pair() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    let engines = [first, second].map(|ship| {
        app.world_mut()
            .spawn((
                ChildOf(ship),
                ThrusterSectionMarker,
                ThrusterSectionInput(0.0),
            ))
            .id()
    });
    // Each hull's attitude loop asks for the opposite turn: the pair turns
    // the way the driver's loop asks, and the other loop is not applied.
    for (ship, torque) in [(first, Vec3::NEG_Y * 50.0), (second, Vec3::Y * 0.5)] {
        app.world_mut().spawn((
            ChildOf(ship),
            PDControllerOutput(torque),
            PDControllerTarget(ship),
        ));
    }
    request_dock(&mut app, first, second);

    for engine in engines {
        **app
            .world_mut()
            .get_mut::<ThrusterSectionInput>(engine)
            .expect("the engine has a throttle") = 1.0;
    }
    for _ in 0..10 {
        app.update();
    }

    let throttle =
        |app: &App, engine: Entity| **app.world().get::<ThrusterSectionInput>(engine).unwrap();
    assert_eq!(
        throttle(&app, engines[0]),
        0.0,
        "the suppressed player's throttle is cut"
    );
    assert_eq!(
        throttle(&app, engines[1]),
        1.0,
        "the driving partner's is not"
    );
    for ship in [first, second] {
        let spin = app.world().get::<AngularVelocity>(ship).unwrap().0;
        assert!(
            spin.y > 0.0,
            "both hulls turn the way the DRIVER asks: {ship:?} {spin:?}"
        );
    }

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    for engine in engines {
        **app
            .world_mut()
            .get_mut::<ThrusterSectionInput>(engine)
            .expect("the engine has a throttle") = 1.0;
    }
    app.update();
    assert_eq!(throttle(&app, engines[0]), 1.0, "the holder drives");
    assert_eq!(throttle(&app, engines[1]), 0.0, "and the partner is cut");
}

#[test]
fn an_engaged_maneuver_no_longer_releases_the_dock() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);

    // A docked partner flies the pair; its maneuver is how it does it.
    app.world_mut()
        .entity_mut(second)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    for _ in 0..10 {
        app.update();
    }

    assert_eq!(connections(&mut app).len(), 1, "the dock holds");
    assert_eq!(joints(&mut app), 1, "and the joint with it");
    assert!(
        app.world().get::<Autopilot>(second).is_some(),
        "and nothing took the maneuver away"
    );
}

#[test]
fn a_suppressed_partner_keeps_its_order_and_autopilot_until_released() {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((DockingSectionPlugin { render: false }, NovaFlightPlugin));
    app.finish();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    app.world_mut().spawn((
        ChildOf(second),
        ControllerSectionMarker,
        ControllerSectionRotationInput::default(),
        PDController {
            frequency: 4.0,
            damping_ratio: 4.0,
            max_angular_acceleration: 100.0,
            sustained_angular_speed: f32::INFINITY,
        },
        Transform::default(),
    ));
    let engine = app
        .world_mut()
        .spawn((
            ChildOf(second),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
            Transform::default(),
        ))
        .id();
    app.world_mut().entity_mut(second).insert((
        ShipHelmOrder::new(
            "job".to_string(),
            ShipOrderDirective::Move {
                position: Vec3::new(0.0, 0.0, -1000.0),
                arrival_standoff: None,
            },
        ),
        ShipOrderHelmAuthority,
        ShipOrderReports::default(),
    ));
    request_dock(&mut app, first, second);
    let engaged = app
        .world()
        .get::<Autopilot>(second)
        .map(|autopilot| autopilot.action)
        .expect("in neutral the partner's order flies the pair");

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    for _ in 0..30 {
        app.update();
    }

    assert_eq!(
        app.world()
            .get::<Autopilot>(second)
            .map(|autopilot| autopilot.action),
        Some(engaged),
        "the held partner's maneuver is frozen, not removed"
    );
    assert!(app.world().get::<ShipOrderEngaged>(second).is_some());
    assert!(
        app.world()
            .get::<ShipOrderReports>(second)
            .unwrap()
            .0
            .is_empty(),
        "a paused order neither completes nor fails"
    );
    assert_eq!(
        **app.world().get::<ThrusterSectionInput>(engine).unwrap(),
        0.0
    );

    app.world_mut()
        .trigger(DockingReleaseRequest { entity: first });
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<Autopilot>(second)
            .map(|autopilot| autopilot.action),
        Some(engaged),
        "released, the partner flies its order on"
    );
    assert!(app
        .world()
        .get::<ShipOrderReports>(second)
        .unwrap()
        .0
        .is_empty());
}

#[test]
fn release_and_port_destruction_clear_the_helm_and_the_assembly() {
    let mut app = docking_app();
    let (first, first_port, second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    request_dock(&mut app, first, second);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    for ship in [first, second] {
        assert!(app.world().get::<DockedAssembly>(ship).is_some());
    }

    app.world_mut()
        .trigger(DockingReleaseRequest { entity: first });
    app.update();
    for ship in [first, second] {
        assert!(app.world().get::<DockedShip>(ship).is_none());
        assert!(
            app.world().get::<DockedAssembly>(ship).is_none(),
            "a released hull flies on its own numbers"
        );
    }

    request_dock(&mut app, first, second);
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Neutral,
        "a re-dock starts neutral, whoever held the last one"
    );

    app.world_mut().entity_mut(first_port).despawn();
    app.update();
    assert!(connections(&mut app).is_empty());
    for ship in [first, second] {
        assert!(app.world().get::<DockedShip>(ship).is_none());
        assert!(app.world().get::<DockedAssembly>(ship).is_none());
    }
}

#[test]
fn a_held_helm_without_a_computer_applies_no_attitude_wrench() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    let engine = app
        .world_mut()
        .spawn((
            ChildOf(first),
            ThrusterSectionMarker,
            ThrusterSectionInput(0.0),
        ))
        .id();
    request_dock(&mut app, first, second);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();

    // Only the held partner has an attitude loop, and it is asking hard.
    app.world_mut().spawn((
        ChildOf(second),
        PDControllerOutput(Vec3::Y * 50.0),
        PDControllerTarget(second),
    ));
    **app
        .world_mut()
        .get_mut::<ThrusterSectionInput>(engine)
        .unwrap() = 1.0;
    for _ in 0..10 {
        app.update();
    }

    for ship in [first, second] {
        let spin = app.world().get::<AngularVelocity>(ship).unwrap().0;
        assert!(
            spin.length() < 1e-3,
            "no computer on the holder, no turn: {ship:?} {spin:?}"
        );
    }
    assert_eq!(
        **app.world().get::<ThrusterSectionInput>(engine).unwrap(),
        1.0,
        "the holder still has its drive"
    );
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(first));
}

#[test]
fn a_lost_player_marker_returns_the_helm_to_neutral() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    request_dock(&mut app, first, second);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(first));

    app.world_mut()
        .entity_mut(first)
        .remove::<PlayerSpaceshipMarker>();
    app.update();

    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Neutral);
    let drives = |ship: Entity| app.world().get::<DockedShip>(ship).unwrap().drives;
    assert!(
        drives(second) && !drives(first),
        "with no player aboard, the second ship drives"
    );
    assert_eq!(joints(&mut app), 1);
}

/// The fault the HUD reads as `HELM FAULT`: set by the refusal that
/// suppresses both roots, cleared by the first pass that measures the pair.
#[test]
fn an_unmeasurable_assembly_faults_both_roots_until_it_measures() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    let engines = [first, second].map(|ship| {
        app.world_mut()
            .spawn((
                ChildOf(ship),
                ThrusterSectionMarker,
                ThrusterSectionInput(0.0),
            ))
            .id()
    });
    request_dock(&mut app, first, second);
    assert!(app.world().get::<DockedShip>(second).unwrap().drives);

    // A hull with no measured reach cannot be planned for.
    app.world_mut().entity_mut(second).remove::<HullRadius>();
    for engine in engines {
        **app
            .world_mut()
            .get_mut::<ThrusterSectionInput>(engine)
            .unwrap() = 1.0;
    }
    app.update();

    for (ship, engine) in [first, second].into_iter().zip(engines) {
        assert!(!app.world().get::<DockedShip>(ship).unwrap().drives);
        assert!(app.world().get::<DockedAssembly>(ship).is_none());
        assert_eq!(
            **app.world().get::<ThrusterSectionInput>(engine).unwrap(),
            0.0
        );
    }
    let [connection] = connections(&mut app)[..] else {
        panic!("the dock is not let go");
    };
    assert!(connection.measurement_fault);

    app.world_mut().entity_mut(second).insert(HullRadius(0.87));
    app.update();

    let [connection] = connections(&mut app)[..] else {
        panic!("the dock is still held");
    };
    assert!(!connection.measurement_fault, "a measured pair clears it");
    assert!(app.world().get::<DockedShip>(second).unwrap().drives);
    assert!(!app.world().get::<DockedShip>(first).unwrap().drives);
}

#[test]
fn a_faulted_neutral_pair_refuses_the_helm_until_it_measures() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    request_dock(&mut app, first, second);
    app.world_mut().entity_mut(second).remove::<HullRadius>();
    app.update();
    assert!(connections(&mut app)[0].measurement_fault);

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    let [connection] = connections(&mut app)[..] else {
        panic!("a refused helm keeps the dock");
    };
    assert_eq!(connection.helm, DockedHelmType::Neutral, "H is refused");
    assert!(!app.world().get::<DockedShip>(first).unwrap().drives);

    app.world_mut().entity_mut(second).insert(HullRadius(0.87));
    app.update();
    assert!(!connections(&mut app)[0].measurement_fault);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Held(first),
        "a measured pair gives the helm back"
    );
    assert!(app.world().get::<DockedShip>(first).unwrap().drives);
}

#[test]
fn a_helm_held_through_a_fault_can_be_released_but_not_retaken() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    request_dock(&mut app, first, second);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(first));

    app.world_mut().entity_mut(second).remove::<HullRadius>();
    app.update();
    assert!(connections(&mut app)[0].measurement_fault);

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Neutral,
        "a faulted helm is still handed back"
    );

    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    let [connection] = connections(&mut app)[..] else {
        panic!("the dock is still held");
    };
    assert_eq!(connection.helm, DockedHelmType::Neutral, "and not retaken");
    assert!(connection.measurement_fault);
}

#[test]
fn a_re_dock_after_a_fault_starts_clear() {
    let mut app = docking_app();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);
    app.world_mut().entity_mut(second).remove::<HullRadius>();
    app.update();
    assert!(connections(&mut app)[0].measurement_fault);

    app.world_mut()
        .trigger(DockingReleaseRequest { entity: first });
    app.update();
    assert!(connections(&mut app).is_empty());

    // Checked before any tick: a measured pass would clear a fault anyway.
    app.world_mut().trigger(DockingConnectionRequest {
        entity: first,
        target: second,
    });
    app.world_mut().flush();
    let [connection] = connections(&mut app)[..] else {
        panic!("the pair docks again");
    };
    assert!(
        !connection.measurement_fault,
        "a new dock keeps no old fault"
    );
}

#[test]
fn docked_rcs_translates_the_pair_without_turning_it() {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((DockingSectionPlugin { render: false }, NovaFlightPlugin));
    app.finish();
    let (first, .., second, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(first)
        .insert(PlayerSpaceshipMarker);
    request_dock(&mut app, first, second);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: first });
    app.update();
    let start = [first, second].map(|ship| app.world().get::<Position>(ship).unwrap().0);

    // A sideways trim across the docking axis: pushed on the driver alone,
    // the joint would drag the partner and turn the pair.
    app.world_mut().get_mut::<RcsIntent>(first).unwrap().0 = Vec3::X;
    for _ in 0..20 {
        app.update();
    }

    let velocity = [first, second].map(|ship| app.world().get::<LinearVelocity>(ship).unwrap().0);
    assert!(velocity[0].x > 0.0, "the pair moves: {velocity:?}");
    assert!(
        velocity[0].abs_diff_eq(velocity[1], 1e-3 * velocity[0].length()),
        "both hulls move together: {velocity:?}"
    );
    for ship in [first, second] {
        let spin = app.world().get::<AngularVelocity>(ship).unwrap().0;
        assert!(spin.length() < 1e-3, "and neither turns: {ship:?} {spin:?}");
    }
    let end = [first, second].map(|ship| app.world().get::<Position>(ship).unwrap().0);
    assert!(
        ((end[0] - end[1]) - (start[0] - start[1])).length() < 1e-3,
        "the joint does not stretch"
    );
}

#[test]
fn a_destroyed_port_frees_its_partner() {
    let mut app = docking_app();
    let (first, first_port, second, second_port) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);

    app.world_mut().entity_mut(first_port).despawn();
    app.update();

    assert!(connections(&mut app).is_empty(), "the connection is gone");
    assert_eq!(joints(&mut app), 0, "and the joint with it");
    assert!(
        app.world().get::<DockedPort>(second_port).is_none(),
        "the surviving port is free"
    );
    assert_eq!(
        state(&app, second_port),
        DockingSectionState::Retracted,
        "and its sleeve comes home"
    );
}

#[test]
fn a_destroyed_hull_releases_the_ship_that_outlived_it() {
    let mut app = docking_app();
    let (first, .., second, second_port) = facing_pair(&mut app, NOMINAL_GAP);
    request_dock(&mut app, first, second);

    app.world_mut().entity_mut(first).despawn();
    app.world_mut()
        .run_system_once(release_broken_docking_connections)
        .expect("the liveness pass runs");
    app.update();

    assert!(connections(&mut app).is_empty());
    assert!(
        app.world().get::<DockedShip>(second).is_none(),
        "the surviving hull flies free"
    );
    assert!(app.world().get::<DockedPort>(second_port).is_none());
}

#[test]
fn a_pair_grades_against_the_stricter_of_its_two_ports() {
    let generous = DockingSectionConfig {
        capture_distance: Meters(20.0),
        capture_angle: 40.0,
        maximum_relative_speed: MetersPerSecond(9.0),
        maximum_relative_angular_speed: 9.0,
        ..DockingSectionConfig::default()
    };
    let delicate = DockingSectionConfig {
        capture_distance: Meters(4.0),
        capture_angle: 5.0,
        maximum_relative_speed: MetersPerSecond(1.0),
        maximum_relative_angular_speed: 1.0,
        ..DockingSectionConfig::default()
    };

    let strictest = generous.envelope().strictest(delicate.envelope());

    assert_eq!(strictest.capture_distance, Meters(4.0).to_engine());
    assert_eq!(strictest.capture_angle, 5.0_f32.to_radians());
    assert_eq!(
        strictest.maximum_relative_speed,
        MetersPerSecond(1.0).to_engine()
    );
    assert_eq!(
        strictest.maximum_relative_angular_speed,
        1.0_f32.to_radians()
    );
}
