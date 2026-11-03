//! The DOCK order: a hull flies one of its ports onto a free port of a moving
//! station and is captured through the real connection request.

use std::{
    collections::HashSet,
    f32::consts::{FRAC_PI_2, PI},
};

use avian3d::prelude::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::EntityId;
use nova_gameplay::{prelude::*, test_support::settle};

use super::{
    super::{
        autopilot::dock_turns,
        dock_corridor::{forecast_blocker, DockCorridors},
        dock_forecast::{station_frame_sweep, FieldBody},
        state::DOCK_BLOCKED_SECS,
    },
    support::*,
};
use crate::prelude::*;

/// The station's drift, engine units per second (5 m/s along +X).
const STATION_VELOCITY: Vec3 = Vec3::new(0.5, 0.0, 0.0);

/// Fixed ticks the approach may take before the test gives up on it: 200 s,
/// well past the approach's own liveness deadline.
const APPROACH_FRAMES: usize = 12_000;

/// Fixed ticks an approach round an orbiting station may take: 500 s, past
/// the 422 s liveness deadline of its 300 u start.
const ORBIT_APPROACH_FRAMES: usize = 30_000;

/// The containment radius of a one-cell hull about its centre: half the cube's
/// diagonal. This rig runs no section plugin to publish it.
const CELL_ENVELOPE: f32 = 0.866;

/// One port on `root`, at the root's origin, facing `rotation * -Z`.
fn spawn_port(app: &mut App, root: Entity, id: &str, rotation: Quat) -> Entity {
    app.world_mut()
        .spawn((
            ChildOf(root),
            Name::new(id.to_string()),
            EntityId(id.to_string()),
            Transform::from_rotation(rotation),
            docking_section(DockingSectionConfig::default()),
        ))
        .id()
}

/// A one-cell hull at `at`, drifting at `velocity`.
fn spawn_block(app: &mut App, at: Vec3, velocity: Vec3, density: f32) -> Entity {
    let root = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            Transform::from_translation(at),
            LinearVelocity(velocity),
        ))
        .id();
    app.world_mut().spawn((
        ChildOf(root),
        Transform::default(),
        Collider::cuboid(1.0, 1.0, 1.0),
        ColliderDensity(density),
    ));
    root
}

/// The moving station with two ports, its +Z port already held by a docked
/// tender. Returns the station, the held port and the free +Y port.
fn station_with_one_free_port(app: &mut App) -> (Entity, Entity, Entity) {
    let station = spawn_block(app, Vec3::ZERO, STATION_VELOCITY, 100.0);
    app.world_mut()
        .entity_mut(station)
        .insert((StationMarker, HullEnvelopeRadius(CELL_ENVELOPE)));
    let held = spawn_port(app, station, "top", Quat::from_rotation_y(PI));
    let free = spawn_port(app, station, "dorsal", Quat::from_rotation_x(FRAC_PI_2));
    // The tender's port faces -Z from its origin, half a cell off the
    // station's +Z face.
    let tender = spawn_block(app, Vec3::new(0.0, 0.0, 1.5), STATION_VELOCITY, 1.0);
    spawn_port(app, tender, "fore", Quat::IDENTITY);
    settle(app);
    app.world_mut().trigger(DockingConnectionRequest {
        entity: tender,
        target: station,
    });
    settle(app);
    assert!(
        app.world().get::<DockedPort>(held).is_some(),
        "the tender holds the station's +Z port"
    );
    (station, held, free)
}

/// The flight rig's ship with RCS and a port on the side of its middle cell,
/// facing +X: off the nose axis, so meeting a port that faces +Y takes a roll
/// about the nose that pointing the nose never asks for.
fn visitor(app: &mut App, at: Vec3) -> (Entity, Entity) {
    let (ship, ..) = spawn_ship(app);
    app.world_mut()
        .entity_mut(ship)
        .insert((Transform::from_translation(at), RcsIntent::default()));
    let port = spawn_port(app, ship, "side", Quat::from_rotation_y(-FRAC_PI_2));
    settle(app);
    (ship, port)
}

/// A dock order flies a side-port hull from rest onto the free port of a
/// station drifting past it: it skips the nearer port a tender holds, rolls
/// its port onto the free one, and is captured through
/// `DockingConnectionRequest`. One connection on the selected pair, one
/// completion, and the order keeps the helm.
#[test]
fn a_dock_order_flies_a_side_port_hull_onto_a_moving_stations_free_port() {
    let mut app = dock_flight_app();
    let (station, held, free) = station_with_one_free_port(&mut app);
    // At rest while the station drifts, and nearer the held port's face
    // (0, 0, 0.5) than the free one's (0, 0.5, 0).
    let (ship, port) = visitor(&mut app, Vec3::new(0.0, 10.0, 20.0));
    let start = position_of(&app, ship).distance(position_of(&app, station));
    app.world_mut().entity_mut(ship).insert((
        ShipHelmOrder::new("visit".to_string(), ShipOrderDirective::Dock { station }),
        ShipOrderHelmAuthority,
        ShipOrderReports::default(),
    ));

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
    }

    let reports: Vec<_> = app
        .world()
        .get::<ShipOrderReports>(ship)
        .unwrap()
        .0
        .iter()
        .map(|report| report.outcome)
        .collect();
    assert_eq!(
        reports,
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks from {start} u out"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (
            connection.first_ship,
            connection.first_section,
            connection.second_ship,
            connection.second_section,
        ),
        (ship, port, station, free),
        "the capture joins the selected pair, not the tender's port {held:?}"
    );
    assert!(app.world().get::<ShipOrderHelmAuthority>(ship).is_some());
    assert!(app.world().get::<Autopilot>(ship).is_none());
}

/// A dock order captures at a station in a circular orbit 200 u from a
/// strong well, where the pull (1.05 u/s^2) is over the RCS gravity gate. The
/// visitor co-orbits from 28 u off. Both bodies fall in the well, so the
/// approach trims on the RCS against the tidal difference only. Gated on the
/// whole pull, the hull hunts on its drive and overruns its deadline.
#[test]
fn a_dock_order_captures_at_a_station_orbiting_a_strong_well() {
    const RADIUS: f32 = 200.0;
    let mut app = dock_orbit_app();
    let well = spawn_strong_well(&mut app);
    let mu = app
        .world()
        .get::<nova_gameplay::gravity::GravityWell>(well)
        .unwrap()
        .mu;
    let at = Vec3::new(RADIUS, 0.0, 0.0);
    let orbit = Vec3::Z * nova_gameplay::gravity::circular_orbit_speed(mu, RADIUS);
    let station = spawn_block(&mut app, at, orbit, 100.0);
    app.world_mut()
        .entity_mut(station)
        .insert((StationMarker, HullEnvelopeRadius(CELL_ENVELOPE)));
    let free = spawn_port(
        &mut app,
        station,
        "dorsal",
        Quat::from_rotation_x(FRAC_PI_2),
    );
    settle(&mut app);
    let offset = Vec3::new(0.0, 20.0, 20.0);
    let (ship, port) = visitor(&mut app, at + offset);
    // The settles let the well pull both bodies: start them again on the
    // orbit, together.
    for (body, position) in [(station, at), (ship, at + offset)] {
        app.world_mut().entity_mut(body).insert((
            Transform::from_translation(position),
            Position(position),
            LinearVelocity(orbit),
            AngularVelocity::ZERO,
        ));
    }
    app.world_mut().entity_mut(ship).insert((
        ShipHelmOrder::new("visit".to_string(), ShipOrderDirective::Dock { station }),
        ShipOrderHelmAuthority,
        ShipOrderReports::default(),
    ));

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
    }

    let reports: Vec<_> = app
        .world()
        .get::<ShipOrderReports>(ship)
        .unwrap()
        .0
        .iter()
        .map(|report| report.outcome)
        .collect();
    assert_eq!(
        reports,
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, free),
        "the capture joins the visitor's port to the station's"
    );
    let radius = position_of(&app, station).length();
    assert!(
        (radius - RADIUS).abs() < 0.05 * RADIUS,
        "the station still orbits near {RADIUS} u, at {radius} u"
    );
}

/// The moving station with two free ports facing +Y: `forward` stands 0.3 u
/// further out than `flush` and 0.3 u to its side, so a hull resting at
/// `flush`'s berth gap is nearer `forward`'s face than `flush`'s. Returns the
/// station, `forward` and `flush`.
fn station_with_a_forward_port(app: &mut App) -> (Entity, Entity, Entity) {
    let station = spawn_block(app, Vec3::ZERO, STATION_VELOCITY, 100.0);
    app.world_mut()
        .entity_mut(station)
        .insert((StationMarker, HullEnvelopeRadius(CELL_ENVELOPE)));
    let dorsal = Quat::from_rotation_x(FRAC_PI_2);
    let flush = spawn_port(app, station, "flush", dorsal);
    let forward = spawn_port(app, station, "forward", dorsal);
    app.world_mut()
        .entity_mut(forward)
        .insert(Transform::from_xyz(0.3, 0.3, 0.0).with_rotation(dorsal));
    (station, forward, flush)
}

/// The visitor drifting with the station at `offset` from it, rolled by
/// `rotation`. Spawned clear of the station, then placed after its settle.
fn placed_visitor(
    app: &mut App,
    station: Entity,
    offset: Vec3,
    rotation: Quat,
) -> (Entity, Entity) {
    let (ship, port) = visitor(app, position_of(app, station) + offset);
    let at = position_of(app, station) + offset;
    app.world_mut().entity_mut(ship).insert((
        Transform::from_translation(at).with_rotation(rotation),
        Position(at),
        Rotation(rotation),
        LinearVelocity(STATION_VELOCITY),
        AngularVelocity::ZERO,
    ));
    (ship, port)
}

/// Install `ShipOrderDirective::Dock` on `ship` as already engaged on
/// `action`, the way a long approach leaves it.
fn engage_dock(app: &mut App, ship: Entity, station: Entity, action: AutopilotAction) {
    app.world_mut().entity_mut(ship).insert((
        ShipHelmOrder::new("visit".to_string(), ShipOrderDirective::Dock { station }),
        ShipOrderHelmAuthority,
        ShipOrderReports::default(),
        ShipOrderEngaged,
        Autopilot::engage(action),
    ));
}

/// A selection already made, the way a long approach leaves it: the pair, a
/// corridor entry 5 u out and the gate's own 0.3 u tube. These tests seed it
/// to prove what is done with a selection; the sweep that makes one is
/// proved by the corridor tests.
fn seeded_plan(own_port: Entity, station_port: Entity) -> DockPlan {
    DockPlan {
        own_port,
        station_port,
        entry: 5.0,
        lateral: 0.3,
    }
}

fn report_outcomes(app: &App, ship: Entity) -> Vec<ShipOrderOutcome> {
    app.world()
        .get::<ShipOrderReports>(ship)
        .unwrap()
        .0
        .iter()
        .map(|report| report.outcome)
        .collect()
}

/// A hull holding its selected pair inside the capture envelope, while a
/// different pair at the same station ranks best, is captured on the pair
/// `DockingConnectionRequest` picks, not left to overrun its deadline.
#[test]
fn a_dock_order_captures_on_the_best_eligible_pair_when_it_is_not_the_selected_one() {
    let mut app = dock_flight_app();
    let (station, forward, flush) = station_with_a_forward_port(&mut app);
    // The side port faces -Y one berth gap above `flush`'s face (0, 0.5, 0).
    let (ship, port) = placed_visitor(
        &mut app,
        station,
        Vec3::new(0.0, 2.0, 0.0),
        Quat::from_rotation_z(-FRAC_PI_2),
    );
    engage_dock(
        &mut app,
        ship,
        station,
        AutopilotAction::DockApproach {
            station,
            plan: Some(seeded_plan(port, flush)),
            deadline: None,
            settled: false,
            engaged_s: 0.0,
            blocked_s: None,
        },
    );

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (
            connection.first_ship,
            connection.first_section,
            connection.second_ship,
            connection.second_section,
        ),
        (ship, port, station, forward),
        "the capture joins the best eligible pair, not the selected {flush:?}"
    );
}

/// A nearer free pair does not replace the selected one or re-arm its
/// deadline: the approach overruns on the first deadline and the order fails.
#[test]
fn a_nearer_free_pair_does_not_rearm_a_dock_approachs_deadline() {
    let mut app = dock_flight_app();
    let (station, _forward, flush) = station_with_a_forward_port(&mut app);
    let (ship, port) = placed_visitor(
        &mut app,
        station,
        Vec3::new(0.0, 10.0, 20.0),
        Quat::IDENTITY,
    );
    let armed = 1.0;
    engage_dock(
        &mut app,
        ship,
        station,
        AutopilotAction::DockApproach {
            station,
            plan: Some(seeded_plan(port, flush)),
            deadline: Some(armed),
            settled: true,
            engaged_s: 0.0,
            blocked_s: None,
        },
    );

    let mut left = armed;
    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
        let Some(autopilot) = app.world().get::<Autopilot>(ship) else {
            continue;
        };
        let AutopilotAction::DockApproach { plan, deadline, .. } = autopilot.action else {
            panic!("the approach is the only maneuver: {:?}", autopilot.action);
        };
        assert_eq!(
            plan,
            Some(seeded_plan(port, flush)),
            "the selection holds, tick {frames}"
        );
        let now = deadline.expect("the armed deadline stays armed");
        assert!(
            now <= left,
            "the deadline re-armed at tick {frames}: {left} -> {now}"
        );
        left = now;
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Failed],
        "the approach overruns once, after {frames} ticks"
    );
}

/// A hull ordered to dock while 12 u/s off the station's velocity arms its
/// deadline on the first tick, brakes past where it planned from and re-arms
/// once, on the first tick it flies within 2.6 u/s of the station's velocity,
/// from the leg it plans there. It docks; the deadline rose that once.
#[test]
fn a_dock_approach_armed_off_the_stations_velocity_rearms_once_when_it_settles() {
    let mut app = dock_flight_app();
    let (station, _forward, _flush) = station_with_a_forward_port(&mut app);
    let (ship, _port) = placed_visitor(
        &mut app,
        station,
        Vec3::new(0.0, 10.0, 20.0),
        Quat::IDENTITY,
    );
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(STATION_VELOCITY + Vec3::new(12.0, 0.0, 0.0)));
    engage_dock(
        &mut app,
        ship,
        station,
        AutopilotAction::DockApproach {
            station,
            plan: None,
            deadline: None,
            settled: false,
            engaged_s: 0.0,
            blocked_s: None,
        },
    );

    let mut last = None;
    let mut rises = Vec::new();
    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
        let Some(autopilot) = app.world().get::<Autopilot>(ship) else {
            continue;
        };
        let AutopilotAction::DockApproach {
            deadline, settled, ..
        } = autopilot.action
        else {
            panic!("the approach is the only maneuver: {:?}", autopilot.action);
        };
        if let (Some(before), Some(now)) = (last, deadline) {
            if now > before {
                let speed = (app.world().get::<LinearVelocity>(ship).unwrap().0
                    - app.world().get::<LinearVelocity>(station).unwrap().0)
                    .length();
                rises.push((frames, settled, speed));
            }
        }
        last = deadline;
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the approach docks, after {frames} ticks"
    );
    assert_eq!(rises.len(), 1, "the deadline rises once: {rises:?}");
    let (_, settled, speed) = rises[0];
    assert!(
        settled && speed <= 2.6,
        "it rises when the hull settles: {rises:?}"
    );
}

/// Two hulls ordered to dock at one station do not fly their approaches at
/// once: the second waits at the station's velocity with no deadline armed
/// until the first docks.
#[test]
fn a_second_dock_approach_to_a_station_waits_until_the_first_ends() {
    let mut app = dock_flight_app();
    let (station, _forward, _flush) = station_with_a_forward_port(&mut app);
    let (first, _) = placed_visitor(
        &mut app,
        station,
        Vec3::new(0.0, 10.0, 20.0),
        Quat::IDENTITY,
    );
    let (second, _) = placed_visitor(
        &mut app,
        station,
        Vec3::new(0.0, 10.0, -20.0),
        Quat::IDENTITY,
    );
    let approach = AutopilotAction::DockApproach {
        station,
        plan: None,
        deadline: None,
        settled: false,
        engaged_s: 0.0,
        blocked_s: None,
    };
    engage_dock(&mut app, first, station, approach);
    engage_dock(&mut app, second, station, approach);
    // Equal claims go to the lower entity bits.
    let (first, second) = if first.to_bits() < second.to_bits() {
        (first, second)
    } else {
        (second, first)
    };

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(first).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
        if app.world().get::<DockedShip>(first).is_some() {
            continue;
        }
        let waiting = app
            .world()
            .get::<Autopilot>(second)
            .map(|autopilot| autopilot.action);
        assert!(
            matches!(
                waiting,
                Some(AutopilotAction::DockApproach { deadline: None, .. })
            ),
            "the second approach waits unarmed at tick {frames}: {waiting:?}"
        );
        let drift = (app.world().get::<LinearVelocity>(second).unwrap().0
            - app.world().get::<LinearVelocity>(station).unwrap().0)
            .length();
        assert!(
            drift < 0.05,
            "the second hull holds the station's velocity at tick {frames}: {drift} u/s"
        );
    }

    assert_eq!(
        report_outcomes(&app, first),
        vec![ShipOrderOutcome::Complete],
        "the first approach docks, after {frames} ticks"
    );
}

/// A selected pair whose station port another hull holds is replaced by the
/// free pair, and only that replacement re-arms the deadline: an approach
/// armed with one second left still docks on the free port.
#[test]
fn a_dock_approach_on_a_held_port_retargets_to_the_free_one() {
    let mut app = dock_flight_app();
    let (station, held, free) = station_with_one_free_port(&mut app);
    let (ship, port) = visitor(&mut app, Vec3::new(0.0, 10.0, 20.0));
    engage_dock(
        &mut app,
        ship,
        station,
        AutopilotAction::DockApproach {
            station,
            plan: Some(seeded_plan(port, held)),
            deadline: Some(1.0),
            settled: true,
            engaged_s: 0.0,
            blocked_s: None,
        },
    );

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, free),
        "the capture joins the free port, not the held {held:?}"
    );
}

/// The containment radius of the 5x1x5 slab with its pylon about the centre
/// of mass, rounded up.
const SLAB_ENVELOPE: f32 = 3.6;

/// A 5x1x5 cell slab centred `at` and drifting at [`STATION_VELOCITY`], with
/// one free port on its top face facing +Y. With `pylon`, one more cell stands
/// 2.5 u over the port, in the only corridor. Returns the station and its port.
fn slab_station(app: &mut App, pylon: bool, at: Vec3) -> (Entity, Entity) {
    let station = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            StationMarker,
            HullEnvelopeRadius(SLAB_ENVELOPE),
            RigidBody::Dynamic,
            Transform::from_translation(at),
            LinearVelocity(STATION_VELOCITY),
        ))
        .id();
    let mut cells: Vec<(String, Vec3)> = (-2..=2)
        .flat_map(|x| {
            (-2..=2).map(move |z| (format!("slab_{x}_{z}"), Vec3::new(x as f32, 0.0, z as f32)))
        })
        .collect();
    if pylon {
        cells.push(("pylon".to_string(), Vec3::new(0.0, 3.0, 0.0)));
    }
    for (id, at) in cells {
        app.world_mut().spawn((
            ChildOf(station),
            Name::new(id.clone()),
            EntityId(id),
            Transform::from_translation(at),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(100.0),
        ));
    }
    let port = spawn_port(app, station, "dorsal", Quat::from_rotation_x(FRAC_PI_2));
    settle(app);
    (station, port)
}

/// Every pair of a `ship` collider and a `station` collider touching now.
fn touching(app: &App, ship: Entity, station: Entity) -> HashSet<(Entity, Entity)> {
    let body = |collider: Entity| app.world().get::<ColliderOf>(collider).map(|of| of.body);
    app.world()
        .resource::<ContactGraph>()
        .iter_active_touching()
        .filter_map(|pair| match (body(pair.collider1), body(pair.collider2)) {
            (Some(a), Some(b)) if a == ship && b == station => {
                Some((pair.collider1, pair.collider2))
            }
            (Some(a), Some(b)) if a == station && b == ship => {
                Some((pair.collider2, pair.collider1))
            }
            _ => None,
        })
        .collect()
}

/// Install `ShipOrderDirective::Dock` on `ship`, not yet engaged.
fn order_dock(app: &mut App, ship: Entity, station: Entity) {
    app.world_mut().entity_mut(ship).insert((
        ShipHelmOrder::new("visit".to_string(), ShipOrderDirective::Dock { station }),
        ShipOrderHelmAuthority,
        ShipOrderReports::default(),
    ));
}

/// A hull that starts under the slab, behind its only port, routes round the
/// station to the corridor entry and closes down the corridor: captured on
/// that port, with no collider of it ever touching the station. A straight
/// approach flew into the slab's underside and pressed there until its
/// deadline ran out.
#[test]
fn a_dock_order_from_behind_the_station_routes_round_it_and_captures_with_no_contact() {
    docks_from_behind_the_slab(Vec3::ZERO);
}

/// The same approach 250 km out, where the f32 position quantum is 0.002 u: the
/// final close still captures. A close that decayed with the gap stalled
/// there short of the gate, its commanded step per tick under half a quantum,
/// until its deadline ran out.
#[test]
fn a_dock_order_far_from_the_origin_still_closes_to_capture() {
    docks_from_behind_the_slab(Vec3::new(25000.0, 25000.0, 0.0));
}

/// Order a hull under a slab centred `at` to dock, and assert it captures on
/// the slab's port with no contact on any tick.
fn docks_from_behind_the_slab(at: Vec3) {
    let mut app = dock_flight_app();
    let (station, station_port) = slab_station(&mut app, false, at);
    let (ship, port) = placed_visitor(&mut app, station, Vec3::new(0.0, -9.0, 0.0), Quat::IDENTITY);
    order_dock(&mut app, ship, station);

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
        let contacts = touching(&app, ship, station);
        assert!(
            contacts.is_empty(),
            "the hull touches the station at tick {frames}: {contacts:?}"
        );
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, station_port),
        "the capture joins the hull's port to the slab's"
    );
}

/// The same approach from 300 u under a slab orbiting a strong well at
/// 12 u/s, close to a generated station's orbit, captures with no contact.
/// Two faults each failed it on its deadline. A detour corner held as a fixed
/// world point fell behind the station, and the hull chased it round the
/// orbit. A trim against the tidal pull emptied the RCS magazine, then spent
/// each tick's refill at once, so the hull held at the corridor entry with the
/// drive and the RCS trading the goal and the axes never inside the gate.
#[test]
fn a_dock_order_from_far_behind_an_orbiting_station_routes_round_it_and_captures() {
    const RADIUS: f32 = 300.0;
    let mut app = dock_orbit_app();
    let well = spawn_strong_well(&mut app);
    let mu = app
        .world()
        .get::<nova_gameplay::gravity::GravityWell>(well)
        .unwrap()
        .mu;
    let at = Vec3::new(RADIUS, 0.0, 0.0);
    let orbit = Vec3::Z * nova_gameplay::gravity::circular_orbit_speed(mu, RADIUS);
    let (station, station_port) = slab_station(&mut app, false, at);
    let offset = Vec3::new(0.0, -300.0, 0.0);
    let (ship, port) = placed_visitor(&mut app, station, offset, Quat::IDENTITY);
    // The settles let the well pull both bodies: start them again on the
    // orbit, together.
    for (body, position) in [(station, at), (ship, at + offset)] {
        app.world_mut().entity_mut(body).insert((
            Transform::from_translation(position),
            Position(position),
            LinearVelocity(orbit),
            AngularVelocity::ZERO,
        ));
    }
    order_dock(&mut app, ship, station);

    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < ORBIT_APPROACH_FRAMES {
        app.update();
        frames += 1;
        let contacts = touching(&app, ship, station);
        assert!(
            contacts.is_empty(),
            "the hull touches the station at tick {frames}: {contacts:?}"
        );
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, station_port),
        "the capture joins the hull's port to the slab's"
    );
}

/// A station on a circular orbit 148 u out about an 88.5 u planet (period
/// 72 s), 60 u over its surface, with its only port 60 degrees out of the
/// orbit plane, is docked with no contact with the planet. In the station's
/// frame the planet circles it at 12.9 u/s. Leg A once planned round the
/// planet's world snapshot, so the hull was dragged onto the planet's surface:
/// its collider touched the planet's 3947 ticks in, with the centre of mass
/// 89.5 u from the planet's centre.
#[test]
fn a_dock_order_at_a_station_orbiting_close_to_a_planet_captures_without_touching_it() {
    const RADIUS: f32 = 88.5;
    const MU: f32 = 24_700.0;
    const ORBIT: f32 = 148.0;
    let mut app = dock_orbit_app();
    let planet = spawn_planet(&mut app, RADIUS, MU);
    let at = Vec3::new(ORBIT, 0.0, 0.0);
    let velocity = Vec3::Z * nova_gameplay::gravity::circular_orbit_speed(MU, ORBIT);
    let (station, station_port) = slab_station(&mut app, false, at);
    // The orbit's normal is -Y: the port leans 60 degrees out of the plane
    // toward it and 30 degrees in-plane toward the planet.
    let axis = Vec3::new(-0.5, -(60.0_f32).to_radians().sin(), 0.0).normalize();
    let tilt = Quat::from_rotation_arc(Vec3::Y, axis);
    // Past the planet on the far side of the orbit, so the straight leg to
    // the corridor entry crosses the planet.
    let offset = Vec3::new(-280.0, 0.0, 0.0);
    let (ship, port) = placed_visitor(&mut app, station, offset, Quat::IDENTITY);
    // The slab's colliders take the tilt in a settle: the corridor sweep
    // reads them, not the root.
    app.world_mut().entity_mut(station).insert((
        Transform::from_translation(at).with_rotation(tilt),
        Position(at),
        Rotation(tilt),
    ));
    settle(&mut app);
    // The settles let the well pull both bodies: start them again on the
    // orbit, together.
    app.world_mut().entity_mut(station).insert((
        Transform::from_translation(at).with_rotation(tilt),
        Position(at),
        Rotation(tilt),
        LinearVelocity(velocity),
        AngularVelocity::ZERO,
    ));
    app.world_mut().entity_mut(ship).insert((
        Transform::from_translation(at + offset),
        Position(at + offset),
        LinearVelocity(velocity),
        AngularVelocity::ZERO,
    ));
    order_dock(&mut app, ship, station);

    let mut frames = 0;
    let mut closest = f32::MAX;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < ORBIT_APPROACH_FRAMES {
        app.update();
        frames += 1;
        closest = closest.min(position_of(&app, ship).length());
        let contacts = touching(&app, ship, station);
        assert!(
            contacts.is_empty(),
            "the hull touches the station at tick {frames}: {contacts:?}"
        );
        let contacts = touching(&app, ship, planet);
        assert!(
            contacts.is_empty(),
            "the hull touches the planet at tick {frames}, {closest} u from its centre: {contacts:?}"
        );
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, station_port),
        "the capture joins the hull's port to the slab's"
    );
}

/// A hull that takes its Dock order inside the ring an orbiting station's
/// planet sweeps in the station's frame leaves the ring and docks with no
/// contact. The station orbits 148 u out about an 88.5 u planet (72 s); the
/// hull starts 170 u from it in the orbit plane, at its velocity, where the
/// planet comes round in about 47 s. Every leg from inside the ring was
/// blocked, so the corner hopped each refresh and the 2 u/s closing cap held
/// the hull there until the planet struck it (seed 26 of
/// `system_station_visits`).
#[test]
fn a_dock_order_taken_inside_a_planets_swept_ring_leaves_it_and_captures_without_touching_it() {
    const RADIUS: f32 = 88.5;
    const MU: f32 = 24_700.0;
    const ORBIT: f32 = 148.0;
    let mut app = dock_orbit_app();
    let planet = spawn_planet(&mut app, RADIUS, MU);
    let at = Vec3::new(ORBIT, 0.0, 0.0);
    let velocity = Vec3::Z * nova_gameplay::gravity::circular_orbit_speed(MU, ORBIT);
    let (station, station_port) = slab_station(&mut app, false, at);
    let axis = Vec3::new(-0.5, -(60.0_f32).to_radians().sin(), 0.0).normalize();
    let tilt = Quat::from_rotation_arc(Vec3::Y, axis);
    let offset = Vec3::new(0.0, 0.0, 170.0);
    let (ship, port) = placed_visitor(&mut app, station, offset, Quat::IDENTITY);
    app.world_mut().entity_mut(station).insert((
        Transform::from_translation(at).with_rotation(tilt),
        Position(at),
        Rotation(tilt),
    ));
    settle(&mut app);
    app.world_mut().entity_mut(station).insert((
        Transform::from_translation(at).with_rotation(tilt),
        Position(at),
        Rotation(tilt),
        LinearVelocity(velocity),
        AngularVelocity::ZERO,
    ));
    app.world_mut().entity_mut(ship).insert((
        Transform::from_translation(at + offset),
        Position(at + offset),
        LinearVelocity(velocity),
        AngularVelocity::ZERO,
    ));
    order_dock(&mut app, ship, station);

    let mut frames = 0;
    let mut closest = f32::MAX;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < ORBIT_APPROACH_FRAMES {
        app.update();
        frames += 1;
        closest = closest.min(position_of(&app, ship).length());
        let contacts = touching(&app, ship, station);
        assert!(
            contacts.is_empty(),
            "the hull touches the station at tick {frames}: {contacts:?}"
        );
        let contacts = touching(&app, ship, planet);
        assert!(
            contacts.is_empty(),
            "the hull touches the planet at tick {frames}, {closest} u from its centre: {contacts:?}"
        );
    }

    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, station_port),
        "the capture joins the hull's port to the slab's"
    );
}

/// A DOCK approach to a station on a 72 s orbit 148 u out about an 88.5 u
/// planet spends no RCS before its port reaches the corridor, and captures.
/// The hull holds against the tidal pull all the way in; trimmed on the RCS
/// through leg A, the 30 u/s magazine ran dry before the entry (seed 26 of
/// `system_station_visits`), and with the RCS refused the hull turned its
/// drive onto each error and swung its port past the gates until its deadline.
#[test]
fn a_dock_approach_to_an_orbiting_station_keeps_its_rcs_for_the_corridor() {
    const RADIUS: f32 = 88.5;
    const MU: f32 = 24_700.0;
    const ORBIT: f32 = 148.0;
    let mut app = dock_orbit_app();
    spawn_planet(&mut app, RADIUS, MU);
    let at = Vec3::new(ORBIT, 0.0, 0.0);
    let velocity = Vec3::Z * nova_gameplay::gravity::circular_orbit_speed(MU, ORBIT);
    let (station, station_port) = slab_station(&mut app, false, at);
    let axis = Vec3::new(-0.5, -(60.0_f32).to_radians().sin(), 0.0).normalize();
    let tilt = Quat::from_rotation_arc(Vec3::Y, axis);
    let offset = Vec3::new(-280.0, 0.0, 0.0);
    let (ship, port) = placed_visitor(&mut app, station, offset, Quat::IDENTITY);
    app.world_mut().entity_mut(station).insert((
        Transform::from_translation(at).with_rotation(tilt),
        Position(at),
        Rotation(tilt),
    ));
    settle(&mut app);
    app.world_mut().entity_mut(station).insert((
        Transform::from_translation(at).with_rotation(tilt),
        Position(at),
        Rotation(tilt),
        LinearVelocity(velocity),
        AngularVelocity::ZERO,
    ));
    app.world_mut().entity_mut(ship).insert((
        Transform::from_translation(at + offset),
        Position(at + offset),
        LinearVelocity(velocity),
        AngularVelocity::ZERO,
        RcsBudget::default(),
    ));
    order_dock(&mut app, ship, station);

    let mut frames = 0;
    let mut first_spend = None;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < ORBIT_APPROACH_FRAMES {
        app.update();
        frames += 1;
        if first_spend.is_some() {
            continue;
        }
        let spent = app
            .world()
            .get::<RcsBudget>(ship)
            .expect("a root budget")
            .spent;
        if spent > 0.0 {
            let Some(Autopilot {
                action:
                    AutopilotAction::DockApproach {
                        plan: Some(plan), ..
                    },
                ..
            }) = app.world().get::<Autopilot>(ship)
            else {
                panic!("the hull spends RCS at tick {frames} with no DOCK pair planned");
            };
            let gap = app
                .world()
                .get::<GlobalTransform>(port)
                .expect("a port pose")
                .translation()
                .distance(
                    app.world()
                        .get::<GlobalTransform>(station_port)
                        .expect("a station port pose")
                        .translation(),
                );
            first_spend = Some((frames, gap, plan.entry + plan.lateral));
        }
    }

    let (tick, gap, corridor) = first_spend.expect("the close spends RCS");
    assert!(
        gap <= corridor + 2.0,
        "the hull first spends RCS at tick {tick}, {gap} u from the station port, outside the \
         {corridor} u corridor"
    );
    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
}

/// A station on a 72 s orbit 148 u out about an 88.5 u planet sees the planet
/// circle it in its own frame. The forecast keeps that ring: a corridor 70 u
/// out along the orbit's track is refused on the planet, though the planet
/// stands 164 u from it now, and a corridor along the orbit's normal is not.
#[test]
fn a_planet_an_orbiting_station_circles_refuses_a_corridor_its_path_crosses() {
    const RADIUS: f32 = 88.5;
    const MU: f32 = 24_700.0;
    const ORBIT: f32 = 148.0;
    let planet = Entity::from_raw_u32(7).expect("a valid index");
    let gravity = GravitySettings::default();
    let wells = [(
        planet,
        Vec3::ZERO,
        nova_gameplay::gravity::GravityWell::from_mass(MU, RADIUS, &gravity),
    )];
    let field = [FieldBody {
        entity: planet,
        center: Vec3::ZERO,
        radius: RADIUS,
        velocity: None,
        feels_wells: false,
    }];
    let station = Vec3::new(ORBIT, 0.0, 0.0);
    let velocity = Vec3::Z * nova_gameplay::gravity::circular_orbit_speed(MU, ORBIT);
    let forecast: Vec<_> = station_frame_sweep(station, velocity, &field, &wells, &gravity, 80.0)
        .into_iter()
        .map(|sphere| (sphere.body, station + sphere.offset, sphere.radius))
        .collect();
    let corridor = |axis: Vec3, field: &[(Entity, Vec3, f32)]| {
        forecast_blocker(station + axis * 70.0, station + axis * 5.0, 0.0, field)
    };

    assert_eq!(
        corridor(Vec3::Z, &[(planet, Vec3::ZERO, RADIUS)]),
        None,
        "the planet where it stands now is clear of the corridor along the orbit's track"
    );
    assert_eq!(
        corridor(Vec3::Z, &forecast).map(|(body, _)| body),
        Some(planet),
        "the planet's path crosses the corridor along the orbit's track"
    );
    assert_eq!(
        corridor(Vec3::Y, &forecast),
        None,
        "the planet's path stays clear of the corridor along the orbit's normal"
    );
}

/// Seconds of fixed time the rig has flown.
fn flown_s(app: &App) -> f32 {
    app.world().resource::<Time<Fixed>>().elapsed_secs()
}

/// Run the rig for `secs` of fixed time, calling `each` after every update.
fn fly_for(app: &mut App, secs: f32, mut each: impl FnMut(&App)) {
    let until = flown_s(app) + secs;
    while flown_s(app) < until {
        app.update();
        each(app);
    }
}

/// A pylon standing in the only corridor holds the approach before the hull
/// moves: the hull waits where it started, nothing touched, and the order
/// fails once the approach has held for its bound with no clear corridor.
#[test]
fn a_dock_order_whose_only_corridor_stays_blocked_holds_unmoved_then_fails_and_names_the_blocker() {
    let mut app = dock_flight_app();
    let (station, _) = slab_station(&mut app, true, Vec3::ZERO);
    let offset = Vec3::new(0.0, -9.0, 0.0);
    let (ship, _) = placed_visitor(&mut app, station, offset, Quat::IDENTITY);

    let blocker = app
        .world_mut()
        .run_system_once(move |corridors: DockCorridors| {
            let pair = corridors.ports.ranked_pairs(ship, station)[0];
            corridors.plan(ship, &pair, 0.0, &[]).err().flatten()
        })
        .expect("the sweep runs");
    assert_eq!(
        blocker.and_then(|blocker| blocker.id),
        Some("pylon".to_string()),
        "the sweep names the pylon"
    );

    order_dock(&mut app, ship, station);
    let start = flown_s(&app);
    while app.world().get::<ShipOrderReported>(ship).is_none()
        && flown_s(&app) - start < DOCK_BLOCKED_SECS + 2.0
    {
        app.update();
        assert!(touching(&app, ship, station).is_empty());
        let moved = (position_of(&app, ship) - position_of(&app, station) - offset).length();
        assert!(
            moved < 0.05,
            "the holding hull moved {moved} u off its start at {} s",
            flown_s(&app) - start
        );
    }
    let held = flown_s(&app) - start;
    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Failed],
        "the order fails once, after {held} s"
    );
    assert!(
        held >= DOCK_BLOCKED_SECS - 0.1,
        "the approach held {held} s, short of its {DOCK_BLOCKED_SECS} s bound"
    );
}

/// A hull that parks in a flying approach's corridor drops its pair on the
/// next forecast refresh; the approach holds, touching nothing, and docks once
/// the hull leaves. Cast only at selection, the corridor was flown into
/// whatever entered it later; refused on a parked hull, the order failed for
/// good where the hull would soon leave (a player's order behind a docked
/// visitor, seed 29).
#[test]
fn a_hull_that_parks_in_a_flying_approachs_corridor_holds_it_until_the_hull_leaves() {
    let mut app = dock_flight_app();
    let (station, station_port) = slab_station(&mut app, false, Vec3::ZERO);
    let (ship, port) = placed_visitor(&mut app, station, Vec3::new(0.0, -9.0, 0.0), Quat::IDENTITY);
    order_dock(&mut app, ship, station);
    let selected = |app: &App| {
        matches!(
            app.world()
                .get::<Autopilot>(ship)
                .map(|autopilot| autopilot.action),
            Some(AutopilotAction::DockApproach { plan: Some(_), .. })
        )
    };
    let holding = |app: &App| {
        matches!(
            app.world()
                .get::<Autopilot>(ship)
                .map(|autopilot| autopilot.action),
            Some(AutopilotAction::DockApproach {
                plan: None,
                blocked_s: Some(_),
                ..
            })
        )
    };
    let mut frames = 0;
    while !selected(&app) && frames < 10 {
        app.update();
        frames += 1;
    }
    assert!(selected(&app), "the clear corridor is selected at once");
    fly_for(&mut app, 2.0, |_| {});

    let above = position_of(&app, station) + Vec3::new(0.0, 3.0, 0.0);
    let parked = spawn_block(&mut app, above, STATION_VELOCITY, 1.0);
    let parked_at = flown_s(&app);
    while !holding(&app) && flown_s(&app) - parked_at < 2.5 {
        app.update();
    }
    assert!(
        holding(&app),
        "the approach drops its pair within a refresh of the hull parking: {:?}",
        app.world()
            .get::<Autopilot>(ship)
            .map(|autopilot| autopilot.action)
    );
    fly_for(&mut app, 30.0, |app| {
        assert!(holding(app), "the approach holds while the hull stays");
        assert!(touching(app, ship, station).is_empty());
        assert!(touching(app, ship, parked).is_empty());
    });

    app.world_mut().entity_mut(parked).despawn();
    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
        let contacts = touching(&app, ship, station);
        assert!(
            contacts.is_empty(),
            "the hull touches the station at tick {frames}: {contacts:?}"
        );
    }
    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once the hull has left, after {frames} ticks"
    );
    let docked = *app
        .world()
        .get::<DockedShip>(ship)
        .expect("the captured ship is docked");
    let connection = *app
        .world()
        .get::<DockingConnection>(docked.connection)
        .expect("the dock names its connection");
    assert_eq!(
        (connection.first_section, connection.second_section),
        (port, station_port)
    );
}

/// A beam of 13 heavy cells along X, its centre of mass at the root, with one
/// free port on the +X end cell facing -Z, drifting with the rig's station.
fn beam_station(app: &mut App) -> (Entity, Entity) {
    const HALF: i32 = 6;
    let station = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            StationMarker,
            HullEnvelopeRadius(HALF as f32 + 1.0),
            RigidBody::Dynamic,
            Transform::default(),
            LinearVelocity(STATION_VELOCITY),
        ))
        .id();
    for x in -HALF..=HALF {
        app.world_mut().spawn((
            ChildOf(station),
            Name::new(format!("cell_{x}")),
            EntityId(format!("cell_{x}")),
            Transform::from_xyz(x as f32, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(100.0),
        ));
    }
    let port = spawn_port(app, station, "end", Quat::IDENTITY);
    app.world_mut()
        .entity_mut(port)
        .insert(Transform::from_xyz(HALF as f32, 0.0, 0.0));
    settle(app);
    (station, port)
}

/// A dock order captures at a station spinning 2 deg/s, about its port's axis
/// and about an axis across it, with no contact. Matched to the centre of
/// mass's velocity alone, the hull lagged the turning entry and the port by
/// their speed over the 0.3/s closing gain, 1.5 to 4.3 u off the axis, outside
/// the 0.3 u gate, and held at the entry until its deadline ran out.
#[test]
fn a_dock_order_captures_at_a_spinning_station() {
    for axis in [Vec3::Z, Vec3::Y] {
        let mut app = dock_flight_app();
        let (station, station_port) = beam_station(&mut app);
        let at = position_of(&app, station) + Vec3::new(6.0, 3.0, -20.0);
        let (ship, port) = visitor(&mut app, at);
        app.world_mut()
            .entity_mut(station)
            .insert(AngularVelocity(axis * 2.0_f32.to_radians()));
        order_dock(&mut app, ship, station);

        let mut frames = 0;
        while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
            app.update();
            frames += 1;
            let contacts = touching(&app, ship, station);
            assert!(
                contacts.is_empty(),
                "spin about {axis}: the hull touches the station at tick {frames}: {contacts:?}"
            );
        }
        assert_eq!(
            report_outcomes(&app, ship),
            vec![ShipOrderOutcome::Complete],
            "spin about {axis}: the order completes once, after {frames} ticks"
        );
        let docked = *app
            .world()
            .get::<DockedShip>(ship)
            .expect("the captured ship is docked");
        let connection = *app
            .world()
            .get::<DockingConnection>(docked.connection)
            .expect("the dock names its connection");
        assert_eq!(
            (connection.first_section, connection.second_section),
            (port, station_port),
            "spin about {axis}"
        );
    }
}

/// A station's turn goes to an armed approach, else to the longest engaged,
/// whatever the entity bits: a visitor bound after the player was ordered in
/// never takes the turn from it.
#[test]
fn a_later_dock_approach_never_takes_a_stations_turn_from_one_already_waiting() {
    let station = Entity::from_raw_u32(1).expect("a valid index");
    let approach = |engaged_s: f32, deadline: Option<f32>| AutopilotAction::DockApproach {
        station,
        plan: None,
        deadline,
        settled: false,
        engaged_s,
        blocked_s: None,
    };
    // Both index orders, so the entity bits favour each hull once.
    for (early, late) in [(2, 90), (90, 2)] {
        let early = Entity::from_raw_u32(early).expect("a valid index");
        let late = Entity::from_raw_u32(late).expect("a valid index");
        let armed = Entity::from_raw_u32(50).expect("a valid index");
        let waiting = [(early, approach(120.0, None)), (late, approach(4.0, None))];
        assert_eq!(
            dock_turns(waiting.into_iter()).get(&station),
            Some(&early),
            "the longest engaged takes the turn over the entity bits ({early:?}, {late:?})"
        );
        let flying = [
            (early, approach(120.0, None)),
            (late, approach(4.0, None)),
            (armed, approach(1.0, Some(300.0))),
        ];
        assert_eq!(
            dock_turns(flying.into_iter()).get(&station),
            Some(&armed),
            "an armed approach keeps the turn"
        );
    }
}

/// The containment radius of [`walled_station`] about its centre of mass,
/// rounded up.
const WALLED_ENVELOPE: f32 = 8.5;

/// A slab with a wall standing on its +X edge, a heavy keel hung well off its
/// -X edge so the centre of mass sits level with the berth and far to the
/// port's -X side, and one port facing +Y 1.2 u in from the slab's centre
/// toward -X. Static. Returns the station and its port.
///
/// From a hull beside the port, a corner pushed out of the centre of mass
/// points almost straight at the wall.
fn walled_station(app: &mut App) -> (Entity, Entity) {
    let station = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            StationMarker,
            HullEnvelopeRadius(WALLED_ENVELOPE),
            RigidBody::Dynamic,
            Transform::default(),
        ))
        .id();
    let mut cells: Vec<(String, Vec3, f32)> = Vec::new();
    for z in -2..=2 {
        for x in -2..=2 {
            cells.push((
                format!("slab_{x}_{z}"),
                Vec3::new(x as f32, 0.0, z as f32),
                100.0,
            ));
        }
        for y in 1..=3 {
            let at = Vec3::new(2.0, y as f32, z as f32);
            cells.push((format!("wall_{y}_{z}"), at, 100.0));
            let at = Vec3::new(-5.0, y as f32, z as f32);
            cells.push((format!("keel_{y}_{z}"), at, 5000.0));
        }
    }
    for (id, at, density) in cells {
        app.world_mut().spawn((
            ChildOf(station),
            Name::new(id.clone()),
            EntityId(id),
            Transform::from_translation(at),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(density),
        ));
    }
    let port = spawn_port(app, station, "dorsal", Quat::from_rotation_x(FRAC_PI_2));
    app.world_mut().entity_mut(port).insert(
        Transform::from_xyz(-1.2, 0.0, 0.0).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    );
    settle(app);
    (station, port)
}

/// The face centre and outward axis of `port`, mounted directly on `root`, in
/// avian's pose.
fn port_face(app: &App, root: Entity, port: Entity) -> (Vec3, Vec3) {
    let position = app.world().get::<Position>(root).unwrap().0;
    let rotation = app.world().get::<Rotation>(root).unwrap().0;
    let mount = app.world().get::<Transform>(port).unwrap();
    let face =
        position + rotation * (mount.translation + mount.rotation * Vec3::new(0.0, 0.0, -0.5));
    (face, rotation * mount.rotation * Vec3::NEG_Z)
}

/// A hull inside the swept corridor that has drifted off the axis past the
/// final leg's gate, but not out of the tube, backs out along the axis and
/// closes again: its port never leaves the tube and nothing touches the
/// station. Routed by the leg planner from inside the station's bubble, it
/// cornered out of the centre of mass and into the wall beside the port.
#[test]
fn a_dock_approach_that_drifts_off_axis_inside_the_corridor_backs_out_along_it() {
    let mut app = dock_flight_app();
    let (station, station_port) = walled_station(&mut app);
    // The port faces -Y with its face 1.5 u above the station port's face and
    // 0.5 u toward the wall: in the tube, outside the 0.3 u gate.
    let (ship, port) = placed_visitor(
        &mut app,
        station,
        Vec3::new(-0.7, 2.5, 0.0),
        Quat::from_rotation_z(-FRAC_PI_2),
    );
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::ZERO));
    // The rig's controller holds the attitude it spawned with; hold the placed
    // one instead, or the hull swings its port off the axis before any
    // approach flies.
    let mut inputs = app
        .world_mut()
        .query::<(&ChildOf, &mut ControllerSectionRotationInput)>();
    for (&ChildOf(parent), mut input) in inputs.iter_mut(app.world_mut()) {
        if parent == ship {
            **input = Quat::from_rotation_z(-FRAC_PI_2);
        }
    }
    order_dock(&mut app, ship, station);

    let mut frames = 0;
    let mut planned = None;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < APPROACH_FRAMES {
        app.update();
        frames += 1;
        let contacts = touching(&app, ship, station);
        assert!(
            contacts.is_empty(),
            "the hull touches the station at tick {frames}: {contacts:?}"
        );
        if let Some(AutopilotAction::DockApproach {
            plan: Some(plan), ..
        }) = app
            .world()
            .get::<Autopilot>(ship)
            .map(|autopilot| autopilot.action)
        {
            planned = Some(plan);
        }
        let Some(plan) = planned else {
            continue;
        };
        let (station_face, axis) = port_face(&app, station, station_port);
        let (face, _) = port_face(&app, ship, port);
        let from_face = face - station_face;
        let along = from_face.dot(axis);
        let lateral = (from_face - axis * along).length();
        assert!(
            along >= 0.0 && along <= plan.entry + 0.3 && lateral < plan.lateral,
            "the port left the tube at tick {frames}: {along} u along, {lateral} u off \
             (entry {}, tube {})",
            plan.entry,
            plan.lateral
        );
    }

    let plan = planned.expect("the approach planned a corridor");
    assert!(
        plan.lateral > 0.5,
        "the start sits inside the tube: {}",
        plan.lateral
    );
    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Complete],
        "the order completes once, after {frames} ticks"
    );
}
