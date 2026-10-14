//! The player's fleet: escorts hired at a station, which follow the player
//! ship from sector to sector.
//!
//! [`hire_station_escort`] answers an [`EscortHireRequest`] once the hire
//! rule moved the credits: it takes the offer off the station's catalogue and
//! spawns the escort beside the player, on the side away from the station,
//! with [`PlayerFleetMarker`] and an [`AIFollow`] order on the player.
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
    objects::{
        ship_design::prelude::ShipDesignSource,
        spaceship::prelude::{
            freeze_ship, thaw_ship, AIControllerConfig, FrozenShip, SpaceshipConfig,
            SpaceshipController,
        },
        station_hulls::prelude::StationHullCatalogue,
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

/// The reach (world units) of a design from its root: its furthest section
/// centre plus one cell's half-diagonal and change.
fn design_reach(offer: &HullOffer) -> f32 {
    offer
        .sections
        .iter()
        .map(|section| section.position.length())
        .fold(0.0, f32::max)
        + 1.0
}

/// Hire a station's offer into the player's fleet, as a Ship pane hire asks
/// once the hire rule moved the credits.
///
/// The offer leaves the catalogue. The escort spawns with the player's
/// rotation and the station's velocity, beside the player on the side away
/// from the station, clear of both hulls by [`AI_FOLLOW_GAP`]: Player-sided,
/// an AI with an [`ESCORT_LEASH`] leash, an empty hold and no credits.
///
/// # Panics
///
/// On a station with no catalogue or no such offer, an offer for sale, or an
/// escort id that is not the offer's slot: the Ship pane raises the request
/// from the same catalogue the frame it plans it.
fn hire_station_escort(
    hire: On<EscortHireRequest>,
    mut commands: Commands,
    q_station: Query<(
        &StationHullCatalogue,
        &HullOffers,
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&LinearVelocity>,
    )>,
    q_leader: Query<(
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&HullRadius>,
    )>,
) {
    let EscortHireRequest {
        entity: leader,
        station,
        offer: index,
        ref escort,
        side,
    } = *hire;
    let Ok((catalogue, offers, station_transform, station_com, station_velocity)) =
        q_station.get(station)
    else {
        panic!("hire_station_escort: {station:?} is not a station with a hull catalogue");
    };
    let (Some(entry), Some(offer)) = (catalogue.0.get(index), offers.0.get(index)) else {
        panic!("hire_station_escort: station {station:?} has no offer {index}");
    };
    let HullOfferType::Hire { slot } = entry.terms else {
        panic!(
            "hire_station_escort: offer {index} of station {station:?} is for sale, not for hire"
        );
    };
    assert_eq!(
        escort.index, slot,
        "hire_station_escort: escort '{escort}' is not roster slot {slot}"
    );
    let Ok((leader_transform, leader_com, leader_arm)) = q_leader.get(leader) else {
        panic!("hire_station_escort: the hiring ship {leader:?} is gone");
    };
    let leader_anchor = live_structure_anchor(leader_transform, leader_com);
    let away = (leader_anchor - live_structure_anchor(station_transform, station_com))
        .try_normalize()
        .unwrap_or(leader_transform.rotation * side);
    let reach = leader_arm.map_or(0.0, |arm| **arm) + design_reach(offer) + AI_FOLLOW_GAP;
    let position = leader_anchor + away * reach;
    let velocity = station_velocity.map_or(Vec3::ZERO, |velocity| **velocity);

    let mut remaining = catalogue.0.clone();
    let entry = remaining.remove(index);
    commands
        .entity(station)
        .insert(StationHullCatalogue(remaining));
    info!(
        "hire_station_escort: '{escort}' ({}) joins the fleet of {leader:?}",
        entry.name
    );
    commands
        .spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: escort.to_string(),
                name: entry.name,
                // Engine boundary: a scenario pose counts meters.
                position: Meters3::from_engine(position),
                rotation: leader_transform.rotation,
            }),
            escort.clone(),
            spaceship_hire_config(entry.design, velocity),
        ))
        .insert((PlayerFleetMarker, AIFollow { leader, side }));
}

/// The spawn of a hired escort: the offer's design, inline, driven by the AI
/// with an [`ESCORT_LEASH`] leash, on the player's side.
fn spaceship_hire_config(
    design: crate::objects::ship_design::prelude::ShipDesign,
    velocity: Vec3,
) -> impl Bundle {
    crate::objects::spaceship::prelude::spaceship_scenario_object(SpaceshipConfig {
        design: ShipDesignSource::Inline(design),
        controller: SpaceshipController::AI(AIControllerConfig {
            leash: Some(ESCORT_LEASH),
            ..AIControllerConfig::default()
        }),
        initial_velocity: MetersPerSecond3::from_engine(velocity),
        allegiance: Some(Allegiance::Player),
        capabilities: ShipCapabilities::default(),
        inventory: ShipInventoryStock::default(),
        lootable: false,
        credits: 0,
    })
}

/// Take a neutralized escort out of the fleet for good: it stops following,
/// and without [`PlayerFleetMarker`] the streaming layer gives the wreck to
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
        .try_remove::<(PlayerFleetMarker, AIFollow)>();
}
