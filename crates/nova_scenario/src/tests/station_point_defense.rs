//! A living station defends itself against torpedoes as a ship does: its
//! spawn makes it an AI hull, so the one point-defense owner
//! (`nova_ship`'s `update_turret_point_defense`) assigns its PDC turret and
//! the AI gun feed aims it. A hostile torpedo and a torpedo from the ship a
//! Neutral station answers are engaged; an own-side torpedo is taken.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use nova_events::prelude::{EntityId, MetersPerSecond3};
use nova_gameplay::{prelude::*, test_support::settle};
use nova_ship::prelude::*;

use super::station_derelict::{design, unfinished_station_app};
use crate::prelude::*;

/// The station harness with the AI and point-defense input plugins, which
/// production adds with the ship layer.
fn app() -> App {
    let mut app = unfinished_station_app();
    app.add_plugins((
        SpaceshipAIInputPlugin,
        SpaceshipPointDefensePlugin { render: false },
    ));
    app.finish();
    app
}

/// A living station of `allegiance` at the origin, settled, and its gun.
fn station(app: &mut App, allegiance: Allegiance) -> (Entity, Entity) {
    let station = {
        let mut commands = app.world_mut().commands();
        let mut entity = commands.spawn((Transform::default(), EntityId::new("outpost")));
        station_scenario_object(
            &mut entity,
            &design(),
            &StationConfig {
                design: StationDesignSource::Inline(design()),
                allegiance,
                derelict: false,
                initial_velocity: MetersPerSecond3::ZERO,
            },
        );
        entity.id()
    };
    settle(app);
    settle(app);
    let gun = app
        .world()
        .get::<Children>(station)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<EntityId>(*child)
                .is_some_and(|id| id.0 == "gun")
        })
        .expect("the station has its gun");
    (station, gun)
}

/// A ship root of `side` at a fixed point 80 units off, clear of the
/// station's point-defense envelope.
fn ship(app: &mut App, side: Allegiance) -> Entity {
    app.world_mut()
        .spawn((
            SpaceshipRootMarker,
            side,
            Transform::from_xyz(80.0, 0.0, 0.0),
        ))
        .id()
}

/// A torpedo `owner` launched with `side`, committed on `station` and
/// closing on it from above the gun's mount.
fn torpedo(app: &mut App, station: Entity, owner: Entity, side: Allegiance) -> Entity {
    let position = Vec3::new(0.0, 10.0, -60.0);
    app.world_mut()
        .spawn((
            TorpedoProjectileMarker,
            TorpedoTargetChosen,
            TorpedoTargetEntity(station),
            ProjectileOwner(owner),
            side,
            Transform::from_translation(position),
            LinearVelocity(-position.normalize() * 30.0),
        ))
        .id()
}

/// What the gun is defending against, and what the AI gun feed aims it at.
fn engaged(app: &App, gun: Entity) -> (Option<Entity>, Option<Entity>) {
    (
        **app.world().get::<TurretDefenseTarget>(gun).unwrap(),
        **app.world().get::<TurretSectionTargetEntity>(gun).unwrap(),
    )
}

/// Whether the gun leaves `torpedo` alone: not assigned to it, not aimed at
/// it.
fn takes(app: &App, gun: Entity, torpedo: Entity) -> bool {
    let (defending, aimed) = engaged(app, gun);
    defending.is_none() && aimed != Some(torpedo)
}

#[test]
fn a_station_shoots_down_a_hostile_torpedo_and_takes_an_own_side_one() {
    let mut app = app();
    let (station, gun) = station(&mut app, Allegiance::Player);

    let wing = ship(&mut app, Allegiance::Player);
    let own = torpedo(&mut app, station, wing, Allegiance::Player);
    settle(&mut app);
    assert!(
        takes(&app, gun, own),
        "an own-side torpedo is taken, as a ship takes one: {:?}",
        engaged(&app, gun)
    );

    let raider = ship(&mut app, Allegiance::Enemy);
    let hostile = torpedo(&mut app, station, raider, Allegiance::Enemy);
    settle(&mut app);
    assert_eq!(
        engaged(&app, gun),
        (Some(hostile), Some(hostile)),
        "the hostile torpedo is assigned to the gun and aimed at"
    );
}

#[test]
fn a_neutral_station_shoots_down_the_torpedoes_of_the_ship_it_answers() {
    let mut app = app();
    let (station, gun) = station(&mut app, Allegiance::Neutral);
    let shooter = ship(&mut app, Allegiance::Player);
    let bystander = ship(&mut app, Allegiance::Enemy);

    let answered = torpedo(&mut app, station, shooter, Allegiance::Player);
    settle(&mut app);
    assert!(
        takes(&app, gun, answered),
        "a calm Neutral station is hostile to nobody's torpedo: {:?}",
        engaged(&app, gun)
    );

    let round = app
        .world_mut()
        .spawn((ProjectileOwner(shooter), Allegiance::Player))
        .id();
    app.world_mut().trigger(HealthApplyDamage {
        entity: station,
        source: Some(round),
        amount: NEUTRAL_RETALIATION_THRESHOLD,
    });
    settle(&mut app);
    assert_eq!(
        app.world().get::<RetaliationTarget>(station),
        Some(&RetaliationTarget(Some(shooter))),
        "the hit station answers the ship that hit it"
    );
    assert_eq!(
        engaged(&app, gun),
        (Some(answered), Some(answered)),
        "the answered ship's torpedo is assigned to the gun and aimed at"
    );

    app.world_mut().despawn(answered);
    let unprovoked = torpedo(&mut app, station, bystander, Allegiance::Enemy);
    settle(&mut app);
    assert!(
        takes(&app, gun, unprovoked),
        "a ship the station does not answer stays Neutral to it: {:?}",
        engaged(&app, gun)
    );
}
