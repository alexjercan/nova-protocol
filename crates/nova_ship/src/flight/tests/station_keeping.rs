//! Station keeping: a living station kicked off its orbit post rides back on
//! its RCS and idles there; a station under an order or dead is left to
//! whoever moves it.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::prelude::*;

/// The well of `spawn_strong_well` and the ring the station is posted on.
const MU: f32 = 43_350.0;
const RADIUS: f32 = 140.0;
/// The tangential kick, as a fraction of orbital speed: what a ram leaves
/// behind, and a four percent swing of the radius, well past the post
/// deadband.
const KICK_FRACTION: f32 = 0.01;
/// Fixed ticks in one lap of the ring at 64 Hz: a 50 s period.
const LAP_TICKS: usize = 3200;
/// How far past the post deadband the return may overshoot while settling
/// onto the ring, engine units.
const POST_OVERSHOOT: f32 = 0.25;

/// A station root on the ring at circular speed, with a hull to carry mass,
/// posted on the orbit it rides. Returns (station, velocity).
fn spawn_posted_station(app: &mut App) -> (Entity, Vec3) {
    let normal = Vec3::Y;
    let position = Vec3::X * RADIUS;
    let velocity = normal.cross(position.normalize()) * circular_orbit_speed(MU, RADIUS);
    let station = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::from_translation(position),
            LinearVelocity(velocity),
            SpaceshipRootMarker,
            StationMarker,
            FlightIntent::default(),
            StationPost::Orbit {
                centre: Vec3::ZERO,
                radius: RADIUS,
                normal,
            },
        ))
        .id();
    app.world_mut().spawn((
        ChildOf(station),
        Name::new("hull"),
        Transform::default(),
        Collider::cuboid(2.0, 2.0, 2.0),
        ColliderDensity(1.0),
    ));
    settle(app);
    (station, velocity)
}

fn radius_error(app: &App, station: Entity) -> f32 {
    (position_of(app, station).length() - RADIUS).abs()
}

fn budget_of(app: &App, station: Entity) -> RcsBudget {
    *app.world().get::<RcsBudget>(station).unwrap()
}

/// The post is held, not the kick: shoved along its orbit by a percent of
/// its speed the station swings off the ring, rides back inside the post
/// deadband on its RCS, then holds the ring for a lap within the deadband
/// plus the return's overshoot with its magazine never emptied and idle
/// long enough to refill.
#[test]
fn a_kicked_station_rides_back_onto_its_orbit_post_and_idles_its_rcs() {
    let mut app = orbit_app();
    spawn_strong_well(&mut app);
    let (station, velocity) = spawn_posted_station(&mut app);
    let settings = app.world().resource::<FlightSettings>().clone();

    run(&mut app, 64);
    assert!(
        radius_error(&app, station) < settings.station_post_deadband,
        "posted on its ring the station rides it: {} u off",
        radius_error(&app, station)
    );

    app.world_mut()
        .get_mut::<LinearVelocity>(station)
        .unwrap()
        .0 = velocity * (1.0 + KICK_FRACTION);
    let mut peak = 0.0f32;
    let mut back_at = None;
    let mut spent_max = 0.0f32;
    for tick in 0..2 * LAP_TICKS {
        app.update();
        let error = radius_error(&app, station);
        peak = peak.max(error);
        let budget = budget_of(&app, station);
        spent_max = spent_max.max(budget.spent);
        assert!(
            !budget.is_empty(&settings),
            "tick {tick}: the return emptied the magazine ({} u/s spent)",
            budget.spent
        );
        if peak > settings.station_post_deadband
            && error < settings.station_post_deadband
            && budget.idle >= settings.rcs_recovery_delay
        {
            back_at = Some(tick);
            break;
        }
    }
    assert!(
        peak > 4.0 * settings.station_post_deadband,
        "the kick swung the station off its ring: peak {peak} u"
    );
    assert!(
        back_at.is_some(),
        "the station never rode back inside its post deadband and idled: peak {peak} u, now {} u off",
        radius_error(&app, station)
    );
    assert!(spent_max > 0.0, "the return spent RCS");

    let bound = settings.station_post_deadband + POST_OVERSHOOT;
    let mut worst = 0.0f32;
    let mut idled = false;
    for _ in 0..LAP_TICKS {
        app.update();
        worst = worst.max(radius_error(&app, station));
        let budget = budget_of(&app, station);
        idled |= budget.idle >= settings.rcs_recovery_delay;
        assert!(!budget.is_empty(&settings), "the hold emptied the magazine");
    }
    assert!(
        worst <= bound,
        "over a lap the held station strayed {worst} u off its ring (bound {bound} u)"
    );
    assert!(
        idled,
        "the held station never idled its RCS long enough to refill"
    );
    println!(
        "station keeping: kick peak {peak:.3} u, back after {} ticks, spent at most {spent_max:.4} u/s, \
         then within {worst:.3} u for a lap",
        back_at.unwrap_or_default()
    );
}

/// Intentional motion wins: the same kick on a station under a scenario
/// order, or on a dead one, is left alone - the radius walks off and no RCS
/// is spent.
#[test]
fn an_ordered_or_dead_station_is_not_pushed_back_onto_its_post() {
    let hold_offs: [(&str, fn(&mut EntityWorldMut)); 2] = [
        ("ordered", |entity| {
            entity.insert(ShipOrderHelmAuthority);
        }),
        ("dead", |entity| {
            entity.insert(DerelictShipMarker);
        }),
    ];
    for (case, hold_off) in hold_offs {
        let mut app = orbit_app();
        spawn_strong_well(&mut app);
        let (station, velocity) = spawn_posted_station(&mut app);
        let settings = app.world().resource::<FlightSettings>().clone();
        hold_off(&mut app.world_mut().entity_mut(station));
        app.world_mut()
            .get_mut::<LinearVelocity>(station)
            .unwrap()
            .0 = velocity * (1.0 + KICK_FRACTION);

        let mut peak = 0.0f32;
        for _ in 0..LAP_TICKS {
            app.update();
            peak = peak.max(radius_error(&app, station));
        }
        assert!(
            peak > 4.0 * settings.station_post_deadband,
            "{case}: the kick must swing the untouched station off its ring, peak {peak} u"
        );
        assert_eq!(
            budget_of(&app, station).spent,
            0.0,
            "{case}: the keeper spent RCS on a station it does not own"
        );
        assert!(
            app.world()
                .get::<RcsIntent>(station)
                .is_none_or(|intent| intent.0 == Vec3::ZERO),
            "{case}: a command was written for a station the keeper does not own"
        );
    }
}
