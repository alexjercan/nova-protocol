//! The MATCH TARGET action: the RCS alone takes the hull's velocity onto a
//! body's root velocity, then lets go. It never turns the hull, lights the
//! drive, closes on the target, or docks, and it says why when it cannot
//! finish.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{
    prelude::{PlayerSpaceshipMarker, SectionInactiveMarker},
    test_support::settle,
};

use super::support::*;
use crate::prelude::*;

/// Where the harness hands every [`MatchRefusal`] the autopilot wrote, in
/// order. Messages live two updates; the collector runs every frame so a test
/// can read them after the run.
#[derive(Resource, Default)]
struct Refusals(Vec<MatchRefusalType>);

fn collect_refusals(mut reader: MessageReader<MatchRefusal>, mut seen: ResMut<Refusals>) {
    seen.0
        .extend(reader.read().map(|MatchRefusal(refusal)| *refusal));
}

fn refusals_of(app: &App) -> Vec<MatchRefusalType> {
    app.world().resource::<Refusals>().0.clone()
}

/// A flight harness whose refusals are collected.
fn match_app() -> App {
    let mut app = flight_app();
    app.init_resource::<Refusals>();
    app.add_systems(Last, collect_refusals);
    app
}

/// A far body coasting at `velocity`: a real dynamic body, so the match reads
/// the same `LinearVelocity` it reads off any root.
fn spawn_target(app: &mut App, position: Vec3, velocity: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::from_translation(position),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
            LinearVelocity(velocity),
        ))
        .id()
}

/// A player-marked RCS hull drifting at `velocity` with a turned attitude,
/// and the controller's rotation command parked on that attitude as the pilot
/// would have left it.
fn spawn_player_hull(app: &mut App, velocity: Vec3, attitude: Quat) -> (Entity, Entity) {
    let (ship, controller) = spawn_rcs_ship(app);
    app.world_mut().entity_mut(ship).insert((
        PlayerSpaceshipMarker,
        LinearVelocity(velocity),
        Rotation(attitude),
    ));
    app.world_mut()
        .entity_mut(controller)
        .insert(ControllerSectionRotationInput(attitude));
    (ship, controller)
}

fn engage_match(app: &mut App, ship: Entity, target: Entity) {
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::MatchTarget { target }));
}

fn budget_of(app: &App, ship: Entity) -> RcsBudget {
    *app.world().get::<RcsBudget>(ship).unwrap()
}

/// Every engine on `ship` is at zero input right now.
fn engines_dark(app: &mut App, ship: Entity) -> bool {
    let mut engines = app.world_mut().query::<(&ThrusterSectionInput, &ChildOf)>();
    engines
        .iter(app.world())
        .filter(|(_, parent)| parent.parent() == ship)
        .all(|(input, _)| **input == 0.0)
}

/// MATCH takes the hull onto the target's velocity on the RCS alone: the
/// engines never light, the hull keeps its attitude, the magazine pays what
/// was shed, the hull does not steer toward the target, and the player is told
/// it completed.
#[test]
fn match_brings_the_relative_speed_to_zero_on_the_rcs_alone() {
    let mut app = match_app();
    let attitude = Quat::from_rotation_y(1.0) * Quat::from_rotation_x(0.4);
    let (ship, _) = spawn_player_hull(&mut app, Vec3::new(-1.5, 2.0, 1.0), attitude);
    let target_position = Vec3::new(3000.0, 200.0, -4000.0);
    let target = spawn_target(&mut app, target_position, Vec3::new(3.0, 0.0, -2.0));
    settle(&mut app);

    let ship_start = position_of(&app, ship);
    let target_start = position_of(&app, target);
    let attitude_start = *app.world().get::<Rotation>(ship).unwrap();
    let initial = velocity_of(&app, ship) - velocity_of(&app, target);
    assert!(
        initial.length() > 1.0,
        "the rig needs a real relative velocity to shed, got {initial}"
    );
    let clock_start = app.world().resource::<Time<Fixed>>().elapsed_secs();
    engage_match(&mut app, ship, target);

    let mut spent_at_end = None;
    for _ in 0..600 {
        app.update();
        assert!(
            engines_dark(&mut app, ship),
            "MATCH is RCS only: no engine input may ever leave zero"
        );
        if app.world().get::<Autopilot>(ship).is_none() {
            spent_at_end = Some(budget_of(&app, ship).spent);
            break;
        }
    }
    let spent = spent_at_end.expect("the match should finish inside the frame budget");

    let relative = velocity_of(&app, ship) - velocity_of(&app, target);
    assert!(
        relative.length() <= 0.05 + 1e-4,
        "the match should leave at most the release speed, {} u/s remain",
        relative.length()
    );
    let completed = app.world().get::<PlayerAutopilotCompleted>(ship);
    assert!(
        matches!(
            completed.map(|completed| completed.action),
            Some(AutopilotAction::MatchTarget { target: done }) if done == target
        ),
        "the player is told the match completed, got {:?}",
        completed.map(|completed| completed.action)
    );
    assert!(
        refusals_of(&app).is_empty(),
        "a completed match refuses nothing, got {:?}",
        refusals_of(&app)
    );

    let attitude_end = *app.world().get::<Rotation>(ship).unwrap();
    let turned = attitude_start.angle_between(attitude_end).to_degrees();
    assert!(
        turned < 1.0,
        "the match never turns the hull, but it swung {turned} deg"
    );

    let shed = (initial - relative).length();
    assert!(
        spent > 0.0 && (spent - shed).abs() < 0.02,
        "the hull's own magazine paid the {shed} u/s it shed, spent {spent}"
    );

    // No automatic approach: the pair only drifted apart or together by the
    // relative velocity it started with, along that velocity, and never docked.
    let elapsed = app.world().resource::<Time<Fixed>>().elapsed_secs() - clock_start;
    let drift = (position_of(&app, ship) - ship_start) - (position_of(&app, target) - target_start);
    assert!(
        drift.length() <= initial.length() * elapsed + 0.01,
        "the relative drift {} u exceeds what {} u/s over {elapsed} s allows",
        drift.length(),
        initial.length()
    );
    let across = drift - drift.project_onto(initial);
    assert!(
        across.length() < 0.05,
        "the hull must not steer toward the target, but it moved {} u across the relative velocity",
        across.length()
    );
    let mut connections = app.world_mut().query::<&DockingConnection>();
    assert_eq!(
        connections.iter(app.world()).count(),
        0,
        "MATCH never docks"
    );
}

/// An empty magazine ends the match with the reason and no completion. The
/// main drive does not take over, so the relative velocity is left as it was.
#[test]
fn match_on_an_empty_magazine_disengages_without_the_drive() {
    let mut app = match_app();
    let velocity = Vec3::new(-1.5, 2.0, 1.0);
    let (ship, _) = spawn_player_hull(&mut app, velocity, Quat::from_rotation_y(0.7));
    let target = spawn_target(
        &mut app,
        Vec3::new(0.0, 0.0, -3000.0),
        Vec3::new(3.0, 0.0, -2.0),
    );
    settle(&mut app);

    let capacity = budget_of(&app, ship).capacity;
    app.world_mut().entity_mut(ship).insert(RcsBudget {
        spent: capacity,
        ..default()
    });
    engage_match(&mut app, ship, target);
    let before = velocity_of(&app, ship);

    for _ in 0..30 {
        app.update();
        assert!(
            engines_dark(&mut app, ship),
            "an empty magazine does not hand the match to the drive"
        );
    }

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "an empty magazine ends the match"
    );
    assert!(
        app.world().get::<PlayerAutopilotCompleted>(ship).is_none(),
        "a match that never matched is not completed"
    );
    assert_eq!(
        refusals_of(&app),
        vec![MatchRefusalType::RcsEmpty],
        "the player hears once that the magazine is empty"
    );
    let drift = (velocity_of(&app, ship) - before).length();
    assert!(
        drift < 1e-4,
        "with no RCS and no drive the velocity stays put, but it moved {drift} u/s"
    );
}

/// The target going away mid-match ends it with a refusal, not a completion,
/// and the hull stops pushing at once.
#[test]
fn match_ends_when_its_target_is_gone() {
    let mut app = match_app();
    let (ship, _) = spawn_player_hull(
        &mut app,
        Vec3::new(-3.0, 2.0, 1.0),
        Quat::from_rotation_y(0.7),
    );
    let target = spawn_target(
        &mut app,
        Vec3::new(0.0, 0.0, -3000.0),
        Vec3::new(3.0, 0.0, -4.0),
    );
    settle(&mut app);

    engage_match(&mut app, ship, target);
    run(&mut app, 10);
    assert!(
        app.world().get::<Autopilot>(ship).is_some(),
        "the match is still shedding speed before the target goes"
    );
    assert!(
        budget_of(&app, ship).spent > 0.0,
        "the RCS was pushing before the target went"
    );

    app.world_mut().entity_mut(target).despawn();
    run(&mut app, 5);
    let held = velocity_of(&app, ship);

    for _ in 0..30 {
        app.update();
        assert!(
            engines_dark(&mut app, ship),
            "a lost target does not light the drive"
        );
    }

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a match with no target ends"
    );
    assert!(
        app.world().get::<PlayerAutopilotCompleted>(ship).is_none(),
        "a lost target is not a completed match"
    );
    assert_eq!(
        refusals_of(&app),
        vec![MatchRefusalType::TargetLost],
        "the player hears the target is lost"
    );
    let moved = (velocity_of(&app, ship) - held).length();
    assert!(
        moved < 1e-4,
        "the hull stops pushing once the target is gone, but its velocity moved {moved} u/s"
    );
}

/// A flight computer lost mid-match ends it with a refusal the player sees,
/// not a silent drop.
#[test]
fn match_without_a_flight_computer_ends_with_a_refusal() {
    let mut app = match_app();
    let (ship, controller) = spawn_player_hull(
        &mut app,
        Vec3::new(-3.0, 2.0, 1.0),
        Quat::from_rotation_y(0.7),
    );
    let target = spawn_target(
        &mut app,
        Vec3::new(0.0, 0.0, -3000.0),
        Vec3::new(3.0, 0.0, -4.0),
    );
    settle(&mut app);

    engage_match(&mut app, ship, target);
    run(&mut app, 5);
    app.world_mut()
        .entity_mut(controller)
        .insert(SectionInactiveMarker);
    run(&mut app, 5);

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "no live flight computer, no MATCH"
    );
    assert!(
        app.world().get::<PlayerAutopilotCompleted>(ship).is_none(),
        "a match cut short is not completed"
    );
    assert_eq!(
        refusals_of(&app),
        vec![MatchRefusalType::NoFlightComputer],
        "the player hears the flight computer is lost"
    );
}

/// A MATCH whose target docks to the hull completes at once. The two roots
/// of a spinning pair differ in velocity by the spin, so a match that kept
/// flying would push the pair until the magazine is empty.
#[test]
fn match_on_a_docked_target_completes_without_a_push() {
    let mut app = match_app();
    let (ship, _) = spawn_player_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
    let target = spawn_target(&mut app, Vec3::X * 100.0, Vec3::new(0.0, 1.0, 0.0));
    settle(&mut app);
    let connection = app
        .world_mut()
        .spawn(DockingConnection {
            first_ship: ship,
            first_section: Entity::PLACEHOLDER,
            second_ship: target,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    let world = app.world();
    let assembly = DockedAssembly {
        mass: world.get::<ComputedMass>(ship).unwrap().value()
            + world.get::<ComputedMass>(target).unwrap().value(),
        center_of_mass: Vec3::X * 50.0,
        linear_velocity: Vec3::ZERO,
        inertia: *world.get::<ComputedAngularInertia>(ship).unwrap(),
        reach: 60.0,
    };
    app.world_mut().entity_mut(ship).insert((
        DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: true,
        },
        assembly,
    ));

    engage_match(&mut app, ship, target);
    run(&mut app, 2);

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a match on the docked partner ends"
    );
    assert!(
        matches!(
            app.world()
                .get::<PlayerAutopilotCompleted>(ship)
                .map(|completed| completed.action),
            Some(AutopilotAction::MatchTarget { .. })
        ),
        "the joint holds the match, so it completes"
    );
    assert_eq!(budget_of(&app, ship).spent, 0.0, "the pair is never pushed");
    assert!(refusals_of(&app).is_empty(), "nothing is refused");
}
