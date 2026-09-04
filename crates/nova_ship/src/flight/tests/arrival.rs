//! One arrival model, one target category at a time.
//!
//! ```text
//! centre distance = target radius + mover radius + navigation margin
//! ```
//!
//! Engine units, like the rest of this tree: a distance here is a world unit
//! (10 m). The rig's sections carry no `SectionMarker`, so `publish_hull_radius`
//! writes nothing for these hulls and a test's authored [`HullRadius`] is the
//! only mover size in play - which is what makes the arithmetic below exact
//! rather than mesh-dependent.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::prelude::*;

/// The mover in every test below: a hull whose outer face is 8 u from its
/// centre. Big enough that leaving it out of the arrival would be visible.
const MOVER_RADIUS: f32 = 8.0;

/// Where the goal sits. Far enough that the leg is a real flip-and-burn.
const GOAL: Vec3 = Vec3::new(0.0, 0.0, -300.0);

/// A ship that publishes its own size, engaged on `action`, one tick in.
fn engaged_ship(app: &mut App, action: AutopilotAction) -> Entity {
    let (ship, _, _) = spawn_ship(app);
    app.world_mut()
        .entity_mut(ship)
        .insert(HullRadius(MOVER_RADIUS));
    settle(app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(action));
    app.update();
    ship
}

/// What the leg says it will do: the centre distance it parks at, and the gap
/// it publishes to the chip.
fn plan(app: &App, ship: Entity, goal: Vec3) -> (f32, f32) {
    let telemetry = app
        .world()
        .get::<ManeuverTelemetry>(ship)
        .expect("an engaged translation leg publishes telemetry");
    (telemetry.park_point.distance(goal), telemetry.distance)
}

/// The global margin, so a test states the model rather than a literal.
fn margin(app: &App) -> f32 {
    app.world().resource::<FlightSettings>().arrival_standoff
}

#[test]
fn an_unsized_mark_gets_no_radius_but_the_hull_still_gets_its_own() {
    // GotoPos: a mark has no size, so the leg rests one margin off THIS
    // hull's face. Before the mover counted, it parked its origin there and
    // an authored zero would have put the mark inside the ship.
    let mut app = flight_app();
    let ship = engaged_ship(&mut app, AutopilotAction::GotoPos { position: GOAL });

    let (park, _) = plan(&app, ship, GOAL);
    assert!(
        (park - (MOVER_RADIUS + margin(&app))).abs() < 1e-2,
        "a mark contributes no radius, the hull contributes its own, got {park}"
    );
}

#[test]
fn a_beacon_parks_off_the_orb_it_publishes() {
    // The category the goal names first: a nav beacon publishes its own
    // radius (20 m of orb), so the leg stops off its FACE - and the chip
    // reads the gap between the two surfaces, not a centre distance.
    let mut app = flight_app();
    let beacon_radius = 2.0;
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(GOAL),
            GlobalTransform::from(Transform::from_translation(GOAL)),
            BodyRadius(beacon_radius),
        ))
        .id();
    let ship = engaged_ship(&mut app, AutopilotAction::Goto { target });

    let (park, gap) = plan(&app, ship, GOAL);
    assert!(
        (park - (beacon_radius + MOVER_RADIUS + margin(&app))).abs() < 1e-2,
        "beacon radius + hull radius + margin, got {park}"
    );
    let centre = (GOAL - position_of(&app, ship)).length();
    assert!(
        (gap - (centre - beacon_radius - MOVER_RADIUS)).abs() < 1e-2,
        "the published distance is hull face to orb face, got {gap}"
    );
}

#[test]
fn a_ship_target_parks_off_the_hull_it_publishes() {
    // The other category that resolved nothing before: a ship is sized by
    // its live sections, not by BodyRadius - which additionally means "a
    // solid body patrol legs steer around", and a hull is not that.
    let mut app = flight_app();
    let target_hull = 6.0;
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(GOAL),
            GlobalTransform::from(Transform::from_translation(GOAL)),
            HullRadius(target_hull),
        ))
        .id();
    let ship = engaged_ship(&mut app, AutopilotAction::Goto { target });

    let (park, _) = plan(&app, ship, GOAL);
    assert!(
        (park - (target_hull + MOVER_RADIUS + margin(&app))).abs() < 1e-2,
        "target hull + mover hull + margin, got {park}"
    );
}

#[test]
fn an_asteroid_without_a_well_parks_off_its_measured_surface() {
    // A rock with a derived collider radius and no gravity at all. Both
    // radii count; neither is a well.
    let mut app = flight_app();
    let rock = 30.0;
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(GOAL),
            GlobalTransform::from(Transform::from_translation(GOAL)),
            BodyRadius(rock),
        ))
        .id();
    let ship = engaged_ship(&mut app, AutopilotAction::Goto { target });

    let (park, _) = plan(&app, ship, GOAL);
    assert!(
        (park - (rock + MOVER_RADIUS + margin(&app))).abs() < 1e-2,
        "rock radius + hull radius + margin, got {park}"
    );
}

#[test]
fn an_authored_anchor_flies_the_margin_its_ship_was_given() {
    // An authored anchor is an unsized entity - the scenario's own
    // `SpawnScenarioObject` marker - and the ship carries the order's
    // margin. Zero is legal, and means this hull's FACE on the mark.
    let mut app = flight_app();
    let anchor = app
        .world_mut()
        .spawn((
            Transform::from_translation(GOAL),
            GlobalTransform::from(Transform::from_translation(GOAL)),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert((HullRadius(MOVER_RADIUS), FlightArrivalStandoff(0.0)));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: anchor }));
    app.update();

    let (park, _) = plan(&app, ship, GOAL);
    assert!(
        (park - MOVER_RADIUS).abs() < 1e-2,
        "a zero margin parks the hull's face on the mark, got {park}"
    );
}

#[test]
fn a_zero_margin_means_the_same_thing_on_a_mark_as_on_a_body() {
    // The authored zero is one rule, not a special case per target kind:
    // face on the surface, whatever publishes the surface.
    let mut app = flight_app();
    let rock = 30.0;
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(GOAL),
            GlobalTransform::from(Transform::from_translation(GOAL)),
            BodyRadius(rock),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert((HullRadius(MOVER_RADIUS), FlightArrivalStandoff(0.0)));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    app.update();

    let (park, _) = plan(&app, ship, GOAL);
    assert!(
        (park - (rock + MOVER_RADIUS)).abs() < 1e-2,
        "hull face on the rock's surface, got {park}"
    );
}

/// A body big enough that the two safe distances used to cross: with a 200 u
/// radius the old arrival parked at 250 u while ORBIT's own band starts at
/// 301.5 u, so the handoff burned the ship 51 u back OUT to a ring it was
/// never told to fly. The arrival now floors itself at that band.
#[test]
fn a_big_well_parks_on_a_ring_the_orbit_band_already_accepts() {
    let mut app = flight_app();
    let gravity = GravitySettings::default();
    let body_radius = 200.0;
    let well = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            Position(Vec3::ZERO),
            nova_gameplay::gravity::GravityWell::from_mass(100_000.0, body_radius, &gravity),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut().entity_mut(ship).insert((
        HullRadius(MOVER_RADIUS),
        Transform::from_xyz(0.0, 0.0, 900.0),
    ));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));
    app.update();

    let settings = app.world().resource::<FlightSettings>().clone();
    let band_floor = settings.orbit_clearance_factor * (body_radius + gravity.surface_margin);
    let unfloored = body_radius + MOVER_RADIUS + settings.arrival_standoff;
    assert!(
        unfloored < band_floor,
        "the fixture must actually straddle the crossing ({unfloored} vs {band_floor})"
    );
    let (park, _) = plan(&app, ship, Vec3::ZERO);
    assert!(
        (park - band_floor).abs() < 1e-2,
        "the arrival floors at the orbit band, got {park}"
    );
}

/// The handoff half of the same defect, from the position it actually bites:
/// a ship already sitting on the band floor. The old arrival called that 51 u
/// too far out, burned inward to its own 250 u park, and the handoff clamped it
/// straight back to the floor - an in-out hunt the ship could not settle. One
/// floor, and the leg is simply already home.
#[test]
fn a_big_well_accepts_the_ring_the_ship_is_already_on() {
    let mut app = flight_app();
    let gravity = GravitySettings::default();
    let body_radius = 200.0;
    let well = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            Position(Vec3::ZERO),
            nova_gameplay::gravity::GravityWell::from_mass(100_000.0, body_radius, &gravity),
        ))
        .id();
    let band_floor = app
        .world()
        .resource::<FlightSettings>()
        .orbit_clearance_factor
        * (body_radius + gravity.surface_margin);
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut().entity_mut(ship).insert((
        HullRadius(MOVER_RADIUS),
        Transform::from_xyz(0.0, 0.0, band_floor),
    ));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));

    let mut plan_radius = None;
    let mut closest = f32::MAX;
    for _ in 0..400 {
        app.update();
        closest = closest.min(position_of(&app, ship).length());
        if let Some(Autopilot {
            action: AutopilotAction::Orbit {
                plan: Some(plan), ..
            },
            ..
        }) = app.world().get::<Autopilot>(ship)
        {
            plan_radius = Some(plan.radius);
            break;
        }
    }
    let ring = plan_radius.expect("a GOTO at a well body parks, and does so promptly");
    assert!(
        (ring - band_floor).abs() < 1e-2,
        "the ring is the floor the leg already parked on, got {ring}"
    );
    assert!(
        closest > band_floor - 2.0,
        "the ship must not dive inward to an arrival the band then rejects, got {closest}"
    );
}

/// The ORBIT park used to read the GLOBAL margin while the arrival read the
/// ship's own: a hull authored to park wide was handed a ring planned on
/// somebody else's number.
#[test]
fn the_orbit_park_spends_the_ships_own_margin() {
    let mut app = flight_app();
    let gravity = GravitySettings::default();
    // Big enough to carry a 400 u SOI, so a wide margin still lands inside
    // the band rather than being clamped by its ceiling.
    let body_radius = 80.0;
    let well = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            Position(Vec3::ZERO),
            nova_gameplay::gravity::GravityWell::from_mass(40_000.0, body_radius, &gravity),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    // Parked inside the envelope already, so the first tick is the handoff
    // and the ring is planned from the leg's intent, not from a flown-in
    // position. The ship's own margin is 4x the global one.
    let ship_margin = 200.0;
    app.world_mut().entity_mut(ship).insert((
        HullRadius(MOVER_RADIUS),
        FlightArrivalStandoff(ship_margin),
        Transform::from_xyz(0.0, 0.0, 150.0),
    ));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));

    let mut plan_radius = None;
    for _ in 0..600 {
        app.update();
        if let Some(Autopilot {
            action: AutopilotAction::Orbit {
                plan: Some(plan), ..
            },
            ..
        }) = app.world().get::<Autopilot>(ship)
        {
            plan_radius = Some(plan.radius);
            break;
        }
    }
    let ring = plan_radius.expect("the handoff fires in budget");
    let expected = body_radius + MOVER_RADIUS + ship_margin;
    assert!(
        (ring - expected).abs() < 2.0,
        "the park spends the ship's own margin ({expected}u ring), got {ring}"
    );
}
