//! The player's fleet: escorts hired at a station, which follow the player
//! ship from sector to sector.
//!
//! [`hire_station_escort`] answers an [`EscortHireRequest`] once the hire
//! rule moved the credits: the escort for hire coasting beside the station
//! leaves it and spawns again where it coasted as the player's, with
//! [`PlayerFleetMarker`] and an [`AIFollow`] order on the player.
//!
//! A fleet escort belongs to no sector. The streaming layer never adopts or
//! freezes it, and the world save keeps it in the player's record as a
//! [`FleetEscort`] ([`freeze_fleet`]); a Load thaws it beside the player
//! ([`thaw_fleet_escort`]). [`release_neutralized_escort`] takes a
//! neutralized escort out of the fleet for good: the wreck is then a sector's
//! body like any other.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::{
    actions::{base_scenario_object, BaseScenarioObjectConfig},
    loader::prelude::ScenarioScopedMarker,
    objects::spaceship::prelude::{
        freeze_ship, thaw_ship, AIControllerConfig, FrozenShip, SpaceshipController,
    },
};

/// The fleet record and its freeze and thaw, and the fleet plugin.
pub mod prelude {
    pub use super::{freeze_fleet, thaw_fleet_escort, FleetEscort, FleetPlugin, ESCORT_LEASH};
}

/// How far (3 km) an escort chases a hostile from its leader before it breaks
/// off and returns to its slot.
pub const ESCORT_LEASH: Meters = Meters(3_000.0);

/// One fleet escort as the world save keeps it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FleetEscort {
    /// The escort's scenario id.
    pub id: EntityId,
    /// The name it shows.
    pub name: String,
    /// Its pose, in engine units, from its physics pose.
    pub transform: Transform,
    /// Its linear and angular velocity, in engine units.
    pub motion: (Vec3, Vec3),
    /// The fleet slot it holds, one of [`FLEET_SIDES`].
    pub side: Vec3,
    /// The ship itself.
    pub ship: FrozenShip,
}

/// Registers the hire and the release.
pub struct FleetPlugin;

impl Plugin for FleetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayerFleetMarker>();
        app.add_observer(hire_station_escort);
        app.add_observer(release_neutralized_escort);
    }
}

/// Freeze every fleet escort, in id order.
///
/// # Errors
///
/// The escort's label and its [`UnsettledBody`] while one cannot freeze yet,
/// see [`freeze_ship`].
///
/// # Panics
///
/// On an escort with no id, no name or no follow order: the hire and the
/// thaw give it all three, and a neutralized escort has left the fleet.
pub fn freeze_fleet(world: &mut World) -> Result<Vec<FleetEscort>, (String, UnsettledBody)> {
    let escorts: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerFleetMarker>>()
        .iter(world)
        .collect();
    let mut fleet = Vec::with_capacity(escorts.len());
    for escort in escorts {
        let body = world.entity(escort);
        let id = body
            .get::<EntityId>()
            .cloned()
            .unwrap_or_else(|| panic!("freeze_fleet: escort {escort} has no scenario id"));
        let name = body
            .get::<Name>()
            .unwrap_or_else(|| panic!("freeze_fleet: escort '{}' has no name", id.0))
            .to_string();
        let side = body
            .get::<AIFollow>()
            .unwrap_or_else(|| panic!("freeze_fleet: escort '{}' has no follow order", id.0))
            .side;
        let mut transform = *body
            .get::<Transform>()
            .unwrap_or_else(|| panic!("freeze_fleet: escort '{}' has no transform", id.0));
        // The physics pose, not the interpolated transform that trails it.
        if let Some((position, rotation)) = body.get::<Position>().zip(body.get::<Rotation>()) {
            transform.translation = position.0;
            transform.rotation = rotation.0;
        }
        let motion = (
            body.get::<LinearVelocity>()
                .map_or(Vec3::ZERO, |linear| linear.0),
            body.get::<AngularVelocity>()
                .map_or(Vec3::ZERO, |angular| angular.0),
        );
        let ship =
            freeze_ship(world, escort).map_err(|reason| (format!("escort '{}'", id.0), reason))?;
        fleet.push(FleetEscort {
            id,
            name,
            transform,
            motion,
            side,
            ship,
        });
    }
    fleet.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    Ok(fleet)
}

/// Spawn the saved escort `escort` where and as it was saved, in the fleet of
/// `leader`, following it on its side.
pub fn thaw_fleet_escort(commands: &mut Commands, escort: FleetEscort, leader: Entity) {
    let FleetEscort {
        id,
        name,
        transform,
        motion: (linear, angular),
        side,
        ship,
    } = escort;
    let mut entity = commands.spawn((
        ScenarioScopedMarker,
        Name::new(name),
        transform,
        GlobalTransform::from(transform),
        Visibility::Visible,
    ));
    if let Ok(generated) = id.0.parse::<GeneratedBodyId>() {
        entity.insert(generated);
    }
    entity.insert(id);
    // The pose before the thaw, as the player's: the body's `Rotation` reads
    // the root `Transform` when the ship is added. The motion after: the
    // spawn bundle seeds zero velocity.
    thaw_ship(&mut entity, ship);
    entity.insert((
        LinearVelocity(linear),
        AngularVelocity(angular),
        PlayerFleetMarker,
        AIFollow { leader, side },
    ));
}

/// Hire an escort for hire of a station into the player's fleet, as a
/// Station pane hire asks once the hire rule moved the credits. The hire
/// needs the whole world, so it runs as a queued command
/// ([`make_hired_escort`]).
fn hire_station_escort(hire: On<EscortHireRequest>, mut commands: Commands) {
    let hire = hire.event().clone();
    commands.queue(move |world: &mut World| make_hired_escort(world, hire));
}

/// The hire an [`EscortHireRequest`] asks for.
///
/// The escort for hire freezes and despawns, and its record spawns again
/// where it coasted, at its velocity, under the escort's id and name:
/// Player-sided, every capability on, an AI with an [`ESCORT_LEASH`] leash,
/// its hold and damage as they were.
///
/// # Panics
///
/// On a hull that is not an escort for hire of the station, an escort id
/// that is not its roster slot, or a hull mid-process: the Station pane
/// raises the request for a hull it lists for hire that frame.
fn make_hired_escort(world: &mut World, hire: EscortHireRequest) {
    let EscortHireRequest {
        entity: leader,
        station,
        hull,
        escort,
        side,
    } = hire;
    assert_eq!(
        world.get::<ParkedHull>(hull).map(|parked| parked.0),
        Some(station),
        "hire_station_escort: {hull:?} is not held by station {station:?}"
    );
    let Some(&ParkedHullType::Hire { slot }) = world.get::<ParkedHullType>(hull) else {
        panic!("hire_station_escort: {hull:?} of station {station:?} is not for hire");
    };
    assert_eq!(
        escort.index, slot,
        "hire_station_escort: escort '{escort}' is not roster slot {slot}"
    );
    let body = world.entity(hull);
    let name = body
        .get::<Name>()
        .map_or_else(String::new, ToString::to_string);
    let position = body.get::<Position>().map_or_else(
        || {
            body.get::<Transform>()
                .map_or(Vec3::ZERO, |t| t.translation)
        },
        |p| p.0,
    );
    let rotation = body.get::<Rotation>().map_or_else(
        || {
            body.get::<Transform>()
                .map_or(Quat::IDENTITY, |t| t.rotation)
        },
        |r| r.0,
    );
    let velocity = body.get::<LinearVelocity>().copied().unwrap_or_default();
    let spin = body.get::<AngularVelocity>().copied().unwrap_or_default();
    let ship = freeze_ship(world, hull)
        .unwrap_or_else(|reason| panic!("hire_station_escort: '{name}' cannot freeze: {reason}"));
    world.entity_mut(hull).despawn();
    info!("hire_station_escort: '{escort}' ({name}) joins the fleet of {leader:?}");
    let mut commands = world.commands();
    let mut entity = commands.spawn((
        base_scenario_object(&BaseScenarioObjectConfig {
            id: escort.to_string(),
            name,
            // Engine boundary: a scenario pose counts meters.
            position: Meters3::from_engine(position),
            rotation,
        }),
        escort,
    ));
    // The pose before the thaw: the body's `Rotation` reads the root
    // `Transform` when the ship is added. The motion after: the spawn bundle
    // seeds zero velocity.
    thaw_ship(
        &mut entity,
        ship.hired(SpaceshipController::AI(AIControllerConfig {
            leash: Some(ESCORT_LEASH),
            ..AIControllerConfig::default()
        })),
    );
    entity.insert((velocity, spin, PlayerFleetMarker, AIFollow { leader, side }));
    world.flush();
}

/// Take a neutralized escort out of the fleet for good: it stops following,
/// and without [`PlayerFleetMarker`] it is no longer the player's
/// ([`OwnedShipMarker`]) and the streaming layer gives the wreck to
/// the cell it drifts in, and the save keeps it there.
fn release_neutralized_escort(
    neutralized: On<Add, NeutralizedMarker>,
    mut commands: Commands,
    q_escort: Query<&EntityId, With<PlayerFleetMarker>>,
) {
    let Ok(id) = q_escort.get(neutralized.entity) else {
        return;
    };
    info!(
        "release_neutralized_escort: '{}' is lost from the fleet",
        id.0
    );
    commands
        .entity(neutralized.entity)
        .try_remove::<(PlayerFleetMarker, OwnedShipMarker, AIFollow)>();
}
