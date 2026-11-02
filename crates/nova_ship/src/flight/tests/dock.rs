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

use super::{super::dock_corridor::DockCorridors, support::*};
use crate::prelude::*;

/// The station's drift, engine units per second (5 m/s along +X).
const STATION_VELOCITY: Vec3 = Vec3::new(0.5, 0.0, 0.0);

/// Fixed ticks the approach may take before the test gives up on it: 200 s,
/// well past the approach's own liveness deadline.
const APPROACH_FRAMES: usize = 12_000;

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
/// whole pull, it hunted on its drive with its port axis swinging up to
/// 130 deg off and overran its deadline 27 u out.
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

/// A pylon standing in the only corridor refuses the approach before the hull
/// moves: the sweep names the pylon, and the order fails on its first engaged
/// tick with the hull where it started and nothing touched.
#[test]
fn a_dock_order_whose_only_corridor_is_blocked_fails_and_names_the_blocker() {
    let mut app = dock_flight_app();
    let (station, _) = slab_station(&mut app, true, Vec3::ZERO);
    let offset = Vec3::new(0.0, -9.0, 0.0);
    let (ship, _) = placed_visitor(&mut app, station, offset, Quat::IDENTITY);

    let blocker = app
        .world_mut()
        .run_system_once(move |corridors: DockCorridors| {
            let pair = corridors.ports.ranked_pairs(ship, station)[0];
            corridors.plan(ship, &pair, 0.0).err().flatten()
        })
        .expect("the sweep runs");
    assert_eq!(
        blocker.and_then(|blocker| blocker.id),
        Some("pylon".to_string()),
        "the sweep names the pylon"
    );

    order_dock(&mut app, ship, station);
    let mut frames = 0;
    while app.world().get::<ShipOrderReported>(ship).is_none() && frames < 10 {
        app.update();
        frames += 1;
        assert!(touching(&app, ship, station).is_empty());
    }
    assert_eq!(
        report_outcomes(&app, ship),
        vec![ShipOrderOutcome::Failed],
        "the order fails once, after {frames} ticks"
    );
    assert!(frames <= 3, "the refusal took {frames} ticks");
    let moved = (position_of(&app, ship) - position_of(&app, station) - offset).length();
    assert!(
        moved < 0.01,
        "the refused hull moved {moved} u off its start"
    );
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
