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

/// The target's combat state, not only the geometry, decides a new dock: an
/// active Enemy, a ship answering fire and a fighting ally refuse it; a calm
/// ally or Neutral admits it, and any neutralized ship admits a boarding.
#[test]
fn a_fighting_or_enemy_target_refuses_a_dock_and_a_calm_or_neutralized_one_admits_it() {
    let other = Entity::from_raw_u32(999).unwrap();
    let cases: [(
        &str,
        Allegiance,
        Option<Entity>,
        AIBehaviorState,
        bool,
        bool,
    ); 6] = [
        (
            "active enemy",
            Allegiance::Enemy,
            None,
            AIBehaviorState::Patrol,
            false,
            false,
        ),
        (
            "neutralized enemy",
            Allegiance::Enemy,
            None,
            AIBehaviorState::Patrol,
            true,
            true,
        ),
        (
            "calm neutral",
            Allegiance::Neutral,
            None,
            AIBehaviorState::Patrol,
            false,
            true,
        ),
        (
            "answering neutral",
            Allegiance::Neutral,
            Some(other),
            AIBehaviorState::Patrol,
            false,
            false,
        ),
        (
            "fighting ally",
            Allegiance::Player,
            None,
            AIBehaviorState::Engage,
            false,
            false,
        ),
        (
            "calm ally",
            Allegiance::Player,
            None,
            AIBehaviorState::Patrol,
            false,
            true,
        ),
    ];
    for (case, side, answering, state, neutralized, admitted) in cases {
        let mut app = docking_app();
        let (first, _, second, _) = facing_pair(&mut app, NOMINAL_GAP);
        app.world_mut().entity_mut(first).insert(Allegiance::Player);
        let mut target = app.world_mut().entity_mut(second);
        target.insert((side, RetaliationTarget(answering), state));
        if neutralized {
            target.insert(NeutralizedMarker);
        }

        request_dock(&mut app, first, second);

        assert_eq!(
            connections(&mut app).len(),
            usize::from(admitted),
            "{case}: admitted {admitted}"
        );
    }
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
fn a_dock_between_a_pitched_and_a_yawed_hull_holds_the_capture_pose() {
    let mut app = docking_app();
    // Belly up against a hull yawed 12 deg: two rotations that do not
    // commute, so a joint frame composed in the wrong order is off by 24 deg.
    let first = spawn_hull(
        &mut app,
        Vec3::ZERO,
        Quat::from_rotation_x(std::f32::consts::PI),
    );
    let second = spawn_hull(
        &mut app,
        Vec3::Z * (1.0 + NOMINAL_GAP),
        Quat::from_rotation_y(12.0_f32.to_radians()),
    );
    let first_port = spawn_port(&mut app, first, "fore", Vec3::ZERO, Quat::IDENTITY);
    let second_port = spawn_port(&mut app, second, "fore", Vec3::ZERO, Quat::IDENTITY);
    settle(&mut app);

    let (p1, r1, p2, r2, anchor) = app
        .world_mut()
        .run_system_once(move |ports: DockingPorts| {
            let (p1, r1) = ports.body_pose(first).expect("the first hull is a body");
            let (p2, r2) = ports.body_pose(second).expect("the second hull is a body");
            let face1 = ports.pose(first_port, first).expect("a posed port").face;
            let face2 = ports.pose(second_port, second).expect("a posed port").face;
            (p1, r1, p2, r2, face1.midpoint(face2))
        })
        .expect("the capture pose reads");

    // Advance the app once so the new joint is processed before inspection.
    app.world_mut().trigger(DockingConnectionRequest {
        entity: first,
        target: second,
    });
    app.update();

    let world = app.world_mut();
    let joint = world
        .query::<&FixedJoint>()
        .single(world)
        .expect("one joint");
    let (frame1, frame2) = (
        joint.local_frame1().expect("frame 1 is local"),
        joint.local_frame2().expect("frame 2 is local"),
    );
    let anchor1 = p1 + r1 * Vec3::from(frame1.translation);
    let anchor2 = p2 + r2 * Vec3::from(frame2.translation);
    assert!(
        anchor1.distance(anchor) < 1e-4 && anchor2.distance(anchor) < 1e-4,
        "both anchors sit on the capture midpoint {anchor}: {anchor1} {anchor2}"
    );
    let basis1 = r1 * frame1.rotation;
    let basis2 = r2 * frame2.rotation;
    let error = basis1.angle_between(basis2).to_degrees();
    // f32 `angle_between` resolves about 0.04 deg next to identity.
    assert!(
        error < 0.1,
        "the joint holds the relative rotation the hulls met at, not one \
         {error} deg off it"
    );
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

/// Every live undock separation in the world.
fn separations(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&UndockSeparation>();
    q.iter(world).count()
}

/// Put `ship` at `at`, as a teleport does.
fn place(app: &mut App, ship: Entity, at: Vec3) {
    let mut ship = app.world_mut().entity_mut(ship);
    ship.insert(Position(at));
    ship.get_mut::<Transform>()
        .expect("a hull has a transform")
        .translation = at;
}

/// The shared drift of the undock fixture, engine units per second: enough
/// to move a body's box past avian's broad-phase margin within a few steps.
const DRIFT: f32 = 0.5;

#[test]
fn an_undocked_pair_stays_collision_free_until_its_hulls_separate() {
    let mut app = docking_app();
    let (first, _, second, _) = facing_pair(&mut app, 0.0);
    request_dock(&mut app, first, second);
    assert_eq!(connections(&mut app).len(), 1, "the touching pair docks");
    // A greeble on the second hull that reaches past its port face into the
    // first hull, as a skin plate does on a real hull.
    app.world_mut().spawn((
        ChildOf(second),
        Name::new("greeble"),
        Transform::from_xyz(0.0, 0.0, -0.55),
        Collider::cuboid(0.2, 0.2, 0.2),
        ColliderDensity(1.0),
    ));
    settle(&mut app);
    // The pair drifts together, as a docked pair in flight does: a body at
    // rest keeps its broad-phase box, so avian would never pair the greeble
    // with the first hull, fix or no fix.
    for ship in [first, second] {
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * DRIFT));
    }

    app.world_mut()
        .trigger(DockingReleaseRequest { entity: second });
    for _ in 0..30 {
        app.update();
    }
    assert!(connections(&mut app).is_empty(), "the dock is gone");
    assert_eq!(separations(&mut app), 1, "the overlapping pair stays apart");
    assert!(
        !island_linked_contact(&app, first, second),
        "the greeble holds no contact with the first hull"
    );
    let velocity =
        |app: &App, ship: Entity| app.world().get::<LinearVelocity>(ship).expect("a body").0;
    let shove = (velocity(&app, second) - velocity(&app, first)).length();
    assert!(
        shove < 1.0e-4,
        "the undock shoves the hulls apart at {shove}"
    );

    let at = |app: &App| app.world().get::<Position>(first).expect("a body").0;
    let clear = at(&app) + Vec3::NEG_Z * 4.0;
    place(&mut app, second, clear);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        separations(&mut app),
        0,
        "the separated pair is filtered no more"
    );

    let overlap = at(&app) + Vec3::NEG_Z;
    place(&mut app, second, overlap);
    for _ in 0..10 {
        app.update();
    }
    assert!(
        island_linked_contact(&app, first, second),
        "the greeble collides with the first hull again"
    );
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

/// The station branch of admission, pure: a living Enemy station refuses, a
/// living station on any other side admits, a derelict admits whatever its
/// side, and a station already holding its one connection refuses the next
/// hull before its side is read.
#[test]
fn a_station_admits_by_its_side_unless_it_is_dead_or_occupied() {
    assert_eq!(
        station_admission(Some(Allegiance::Enemy), false, false, false, false),
        Err(DockRefusalType::EnemyStation)
    );
    for side in [Some(Allegiance::Neutral), Some(Allegiance::Player), None] {
        assert_eq!(
            station_admission(side, false, false, false, false),
            Ok(()),
            "a living {side:?} station admits"
        );
    }
    assert_eq!(
        station_admission(Some(Allegiance::Enemy), true, false, false, true),
        Ok(()),
        "a derelict Enemy station is a hull to loot"
    );
    assert_eq!(
        station_admission(Some(Allegiance::Neutral), false, true, false, false),
        Err(DockRefusalType::StationOccupied)
    );
    assert_eq!(
        station_admission(Some(Allegiance::Enemy), true, true, false, false),
        Err(DockRefusalType::StationOccupied),
        "a derelict with every port held is full"
    );
}

/// A living station refuses a ship hostile to it; one the player owns
/// refuses nobody, on the Enemy side or hostile to the ship, but is still
/// full once every port is held.
#[test]
fn an_owned_station_refuses_nobody_and_a_hostile_one_refuses_the_ship() {
    assert_eq!(
        station_admission(Some(Allegiance::Player), false, false, false, true),
        Err(DockRefusalType::HostileStation),
        "a living Player station refuses an Enemy-flagged ship"
    );
    for (side, hostile) in [
        (Some(Allegiance::Player), true),
        (Some(Allegiance::Enemy), false),
        (Some(Allegiance::Neutral), true),
    ] {
        assert_eq!(
            station_admission(side, false, false, true, hostile),
            Ok(()),
            "an owned {side:?} station admits a ship hostile {hostile}"
        );
    }
    assert_eq!(
        station_admission(Some(Allegiance::Player), false, true, true, false),
        Err(DockRefusalType::StationOccupied),
        "an owned station with every port held is full"
    );
}

/// The helm a capture opens with, pure: a living station holds its pair, a
/// dead hull (station or ship) hands it to the docked player, and two living
/// ships start neutral.
#[test]
fn a_living_station_holds_the_helm_and_a_dead_hull_hands_it_to_the_player() {
    let station = Entity::from_raw_u32(7).unwrap();
    let player = Entity::from_raw_u32(8).unwrap();
    assert_eq!(
        helm_on_capture(DockPartnerType::LivingStation, station, Some(player)),
        DockedHelmType::Station(station)
    );
    assert_eq!(
        helm_on_capture(DockPartnerType::Dead, station, Some(player)),
        DockedHelmType::Held(player)
    );
    assert_eq!(
        helm_on_capture(DockPartnerType::Dead, Entity::PLACEHOLDER, Some(player)),
        DockedHelmType::Held(player),
        "a dead ship pair names no station and still hands the player the helm"
    );
    assert_eq!(
        helm_on_capture(DockPartnerType::Dead, station, None),
        DockedHelmType::Neutral,
        "a dead hull with no player in the pair is flown by no one in particular"
    );
    assert_eq!(
        helm_on_capture(DockPartnerType::Ship, Entity::PLACEHOLDER, Some(player)),
        DockedHelmType::Neutral
    );
    assert!(!helm_request_admitted(DockedHelmType::Station(station)));
    assert!(helm_request_admitted(DockedHelmType::Neutral));
    assert!(helm_request_admitted(DockedHelmType::Held(player)));
}

/// The station facts reach the one admission point: a facing pair with a
/// station root on the far end docks or not by the station rule (its side,
/// its death as a derelict or as a neutralized hull, the ship's side against
/// it and whether the player owns it). A station is a hub: it admits a hull
/// while any port is free, and refuses only once every port is held.
#[test]
fn a_station_root_is_admitted_through_the_station_rule() {
    let station_marker = || StationMarker;
    for (case, side, derelict, neutralized, owned, player_side, admitted) in [
        (
            "living enemy",
            Allegiance::Enemy,
            false,
            false,
            false,
            Allegiance::Player,
            false,
        ),
        (
            "living neutral",
            Allegiance::Neutral,
            false,
            false,
            false,
            Allegiance::Player,
            true,
        ),
        (
            "dead enemy",
            Allegiance::Enemy,
            true,
            false,
            false,
            Allegiance::Player,
            true,
        ),
        (
            "neutralized enemy",
            Allegiance::Enemy,
            false,
            true,
            false,
            Allegiance::Player,
            true,
        ),
        (
            "living player, enemy ship",
            Allegiance::Player,
            false,
            false,
            false,
            Allegiance::Enemy,
            false,
        ),
        (
            "owned player, enemy ship",
            Allegiance::Player,
            false,
            false,
            true,
            Allegiance::Enemy,
            true,
        ),
    ] {
        let mut app = docking_app();
        let (first, _, second, _) = facing_pair(&mut app, NOMINAL_GAP);
        app.world_mut()
            .entity_mut(first)
            .insert((PlayerSpaceshipMarker, player_side));
        let mut station = app.world_mut().entity_mut(second);
        station.insert((station_marker(), side));
        if derelict {
            station.insert(DerelictShipMarker);
        }
        if neutralized {
            station.insert(NeutralizedMarker);
        }
        if owned {
            station.insert(OwnedStationMarker);
        }

        request_dock(&mut app, first, second);

        assert_eq!(
            connections(&mut app).len(),
            usize::from(admitted),
            "{case}: admitted {admitted}"
        );
    }

    // A station with two ports: the first hull docks at the fore port, the
    // second at the aft one, and a third meets a full station.
    let mut app = docking_app();
    let (first, _, station, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(station)
        .insert((station_marker(), Allegiance::Neutral));
    spawn_port(
        &mut app,
        station,
        "aft",
        Vec3::Z,
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    let second = spawn_hull(
        &mut app,
        Vec3::NEG_Z * (3.0 + 2.0 * NOMINAL_GAP),
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    spawn_port(&mut app, second, "fore", Vec3::ZERO, Quat::IDENTITY);
    let third = spawn_hull(&mut app, Vec3::new(0.0, 4.0, 0.0), Quat::IDENTITY);
    spawn_port(&mut app, third, "fore", Vec3::ZERO, Quat::IDENTITY);
    settle(&mut app);
    let refusal = |app: &mut App, ship: Entity| {
        app.world_mut()
            .run_system_once(move |admission: DockAdmission| admission.refusal(ship, station))
            .expect("the admission runs")
    };

    request_dock(&mut app, first, station);
    assert_eq!(connections(&mut app).len(), 1);
    assert_eq!(
        refusal(&mut app, second),
        None,
        "a station with a free port admits the next hull"
    );
    request_dock(&mut app, second, station);
    assert_eq!(
        connections(&mut app).len(),
        2,
        "the second hull docks at the free port"
    );
    for ship in [first, second] {
        assert!(app.world().get::<DockedShip>(ship).is_some());
    }
    assert!(
        app.world().get::<DockedShip>(station).is_none(),
        "a station holds no ship record"
    );
    assert_eq!(
        app.world()
            .get::<StationDocks>(station)
            .expect("the station lists its docks")
            .len(),
        2
    );
    assert_eq!(
        refusal(&mut app, third),
        Some(DockRefusalType::StationOccupied),
        "every port is held"
    );
    request_dock(&mut app, third, station);
    assert_eq!(
        connections(&mut app).len(),
        2,
        "a full station admits no hull"
    );
}

/// Docked to a living station the station holds the helm and `HELM` is
/// refused; docked to a dead station the player holds it from the capture.
#[test]
fn a_station_dock_opens_on_the_station_helm_rule() {
    let drives = |app: &App, root: Entity| match app.world().get::<DockedShip>(root) {
        Some(ship) => ship.drives,
        None => {
            app.world()
                .get::<DockedStation>(root)
                .expect("the root is docked")
                .drives
        }
    };

    let mut app = docking_app();
    let (player, _, station, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSpaceshipMarker);
    app.world_mut()
        .entity_mut(station)
        .insert((StationMarker, Allegiance::Neutral));
    request_dock(&mut app, player, station);
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Station(station)
    );
    assert!(
        drives(&app, station) && !drives(&app, player),
        "the living station drives its pair"
    );
    app.world_mut()
        .trigger(DockingHelmRequest { entity: player });
    app.update();
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Station(station),
        "HELM is refused at a living station"
    );
    assert!(drives(&app, station) && !drives(&app, player));
    assert_eq!(joints(&mut app), 1);

    let mut app = docking_app();
    let (player, _, station, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSpaceshipMarker);
    app.world_mut().entity_mut(station).insert((
        StationMarker,
        Allegiance::Enemy,
        DerelictShipMarker,
    ));
    request_dock(&mut app, player, station);
    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(player));
    assert!(
        drives(&app, player) && !drives(&app, station),
        "the player drives the pair from the capture at a dead station"
    );
    app.world_mut()
        .trigger(DockingHelmRequest { entity: player });
    app.update();
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Neutral,
        "a dead station's helm is an ordinary held helm: HELM hands it back"
    );

    // A station neutralized in combat carries `NeutralizedMarker` and no
    // `DerelictShipMarker` until the scenario layer turns it derelict; the
    // helm rule and the admission read it as dead on their own.
    let mut app = docking_app();
    let (player, _, station, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSpaceshipMarker);
    app.world_mut().entity_mut(station).insert((
        StationMarker,
        Allegiance::Enemy,
        NeutralizedMarker,
    ));
    request_dock(&mut app, player, station);
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Held(player),
        "a neutralized station hands the player the helm at capture"
    );
    assert!(
        drives(&app, player) && !drives(&app, station),
        "the player drives the pair from the capture at a neutralized station"
    );
}

/// A ship neutralized in combat has nobody at its helm: the docked player
/// holds the pair from the capture, as at a dead station, and `HELM` hands
/// it back to neutral as on any held helm.
#[test]
fn a_neutralized_ship_hands_the_docked_player_the_helm_at_capture() {
    let mut app = docking_app();
    let (player, _, partner, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(player)
        .insert((PlayerSpaceshipMarker, Allegiance::Player));
    app.world_mut()
        .entity_mut(partner)
        .insert((Allegiance::Enemy, NeutralizedMarker));

    request_dock(&mut app, player, partner);

    assert_eq!(connections(&mut app)[0].helm, DockedHelmType::Held(player));
    assert!(
        app.world().get::<DockedShip>(player).unwrap().drives,
        "the player drives the pair from the capture"
    );
    assert!(
        !app.world().get::<DockedShip>(partner).unwrap().drives,
        "the neutralized partner is held"
    );
    app.world_mut()
        .trigger(DockingHelmRequest { entity: player });
    app.update();
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Neutral,
        "HELM hands a dead ship's helm back as it does any held helm"
    );
    assert_eq!(joints(&mut app), 1, "the dock holds");
}

/// A living station that dies with the player docked hands the player the
/// helm, as a dock made at a dead station would.
#[test]
fn a_station_that_dies_while_docked_hands_the_player_the_helm() {
    let mut app = docking_app();
    let (player, _, station, _) = facing_pair(&mut app, NOMINAL_GAP);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSpaceshipMarker);
    app.world_mut()
        .entity_mut(station)
        .insert((StationMarker, Allegiance::Neutral));
    request_dock(&mut app, player, station);
    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Station(station)
    );

    app.world_mut()
        .entity_mut(station)
        .insert(DerelictShipMarker);
    app.update();

    assert_eq!(
        connections(&mut app)[0].helm,
        DockedHelmType::Held(player),
        "the dead station's pair is the player's to fly"
    );
    assert!(
        app.world().get::<DockedShip>(player).unwrap().drives,
        "the player drives the pair"
    );
    assert_eq!(joints(&mut app), 1, "the dock holds");
}

/// The station's port destroyed, or switched off, under a docked player
/// releases the dock and frees the player's port.
#[test]
fn losing_the_station_port_while_docked_releases_the_dock() {
    for destroyed in [true, false] {
        let mut app = docking_app();
        let (player, player_port, station, station_port) = facing_pair(&mut app, NOMINAL_GAP);
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerSpaceshipMarker);
        app.world_mut()
            .entity_mut(station)
            .insert((StationMarker, Allegiance::Neutral));
        request_dock(&mut app, player, station);
        assert_eq!(connections(&mut app).len(), 1);

        if destroyed {
            app.world_mut().entity_mut(station_port).despawn();
        } else {
            app.world_mut()
                .entity_mut(station_port)
                .insert(SectionInactiveMarker);
            app.world_mut()
                .run_system_once(release_broken_docking_connections)
                .expect("the liveness pass runs");
        }
        app.update();

        assert!(
            connections(&mut app).is_empty(),
            "destroyed {destroyed}: the connection is gone"
        );
        assert_eq!(joints(&mut app), 0, "and the joint with it");
        assert!(
            app.world().get::<DockedShip>(player).is_none(),
            "the player flies free"
        );
        assert!(
            app.world().get::<DockedPort>(player_port).is_none(),
            "the player's port is free"
        );
        let hub = app.world().entity(station);
        assert!(
            !hub.contains::<StationDocks>()
                && !hub.contains::<DockedStation>()
                && !hub.contains::<DockedAssembly>(),
            "the station holds no dock once its last one is gone"
        );
    }
}

/// Whether any collider of `first` holds a touching contact with any collider
/// of `second` that avian has linked into a simulation island.
fn island_linked_contact(app: &App, first: Entity, second: Entity) -> bool {
    let world = app.world();
    let graph = world.resource::<ContactGraph>();
    let colliders = |body: Entity| {
        world
            .get::<RigidBodyColliders>(body)
            .expect("a settled body lists its colliders")
            .iter()
            .collect::<Vec<_>>()
    };
    let others = colliders(second);
    colliders(first).into_iter().any(|a| {
        others.iter().any(|&b| {
            graph
                .get(a, b)
                .is_some_and(|(edge, pair)| pair.is_touching() && edge.island.is_some())
        })
    })
}

#[test]
fn docking_two_touching_hulls_leaves_their_island_sound_for_the_next_contact() {
    let mut app = docking_app();
    let (first, _, second, _) = facing_pair(&mut app, 0.0);
    for _ in 0..10 {
        app.update();
    }
    assert!(
        island_linked_contact(&app, first, second),
        "the hulls touch face to face before the dock"
    );

    request_dock(&mut app, first, second);
    assert_eq!(connections(&mut app).len(), 1, "the touching pair docks");

    // A third hull meets the docked pair. Its contact joins the pair's island,
    // which reads the island's contact list.
    let third = spawn_hull(&mut app, Vec3::NEG_Z * 2.0, Quat::IDENTITY);
    for _ in 0..30 {
        app.update();
    }
    assert!(
        island_linked_contact(&app, second, third),
        "the third hull's contact is linked into the docked pair's island"
    );
}
