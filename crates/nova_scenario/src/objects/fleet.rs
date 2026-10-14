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

/// Daylight (world units, 50 m) a hired escort spawns with to every hull and
/// sized body, measured on their reach spheres.
const ESCORT_SPAWN_CLEARANCE: f32 = 5.0;

/// How many places out along the side away from the station a hire tries
/// before it gives up: each is one escort's width further out.
const ESCORT_SPAWN_TRIES: u32 = 64;

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

/// Where a hired escort of reach `reach` spawns.
///
/// `leader` is the hiring ship's anchor and `leader_arm` its arm, `away` the
/// unit direction from the station to it and `up` its up axis. The first
/// place is beside the leader on the `away` side, clear of it by
/// [`AI_FOLLOW_GAP`], moved along the fleet `side` (in a frame whose -Z is
/// `away`, as [`FLEET_SIDES`] reads) by its own reach plus half the gap, so
/// two slots never share a place. While that place leaves less than
/// [`ESCORT_SPAWN_CLEARANCE`] between the escort's reach and any of
/// `occupied` (centre and radius of every hull and sized body), it moves one
/// escort's width further along `away`.
///
/// # Panics
///
/// When [`ESCORT_SPAWN_TRIES`] places are all taken: a hire must never put
/// one hull inside another.
fn escort_spawn_position(
    leader: Vec3,
    leader_arm: f32,
    away: Vec3,
    up: Vec3,
    side: Vec3,
    reach: f32,
    occupied: &[(Vec3, f32)],
) -> Vec3 {
    let up = up
        .reject_from(away)
        .try_normalize()
        .unwrap_or_else(|| away.any_orthonormal_vector());
    let frame = Mat3::from_cols(away.cross(up), up, -away);
    let first = leader
        + away * (leader_arm + reach + AI_FOLLOW_GAP)
        + frame * side * (reach + AI_FOLLOW_GAP * 0.5);
    let step = 2.0 * reach + ESCORT_SPAWN_CLEARANCE;
    (0..ESCORT_SPAWN_TRIES)
        .map(|try_index| first + away * step * try_index as f32)
        .find(|place| {
            occupied.iter().all(|&(centre, radius)| {
                place.distance(centre) >= radius + reach + ESCORT_SPAWN_CLEARANCE
            })
        })
        .unwrap_or_else(|| {
            panic!(
                "escort_spawn_position: no clear place in {ESCORT_SPAWN_TRIES} tries out from \
                 {leader} along {away}"
            )
        })
}

/// Hire a station's offer into the player's fleet, as a Ship pane hire asks
/// once the hire rule moved the credits.
///
/// The offer leaves the catalogue. The escort spawns with the player's
/// rotation and the station's velocity, beside the player on the side away
/// from the station, at the place [`escort_spawn_position`] finds clear of
/// every hull (the player, the station, the rest of the fleet, traffic) and
/// sized body: Player-sided, an AI with an [`ESCORT_LEASH`] leash, an empty
/// hold and no credits.
///
/// # Panics
///
/// On a station with no catalogue or no such offer, an offer for sale, or an
/// escort id that is not the offer's slot: the Ship pane raises the request
/// from the same catalogue the frame it plans it. On no clear place, see
/// [`escort_spawn_position`].
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
    // Every hull, the station and the player among them, and every sized
    // body, as the follow order rounds them.
    q_hull: Query<
        (&Transform, Option<&ComputedCenterOfMass>, &HullRadius),
        With<SpaceshipRootMarker>,
    >,
    q_body: Query<(&Transform, &BodyRadius), (Without<SpaceshipRootMarker>, Without<BeaconMarker>)>,
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
    let occupied: Vec<(Vec3, f32)> = q_hull
        .iter()
        .map(|(transform, com, radius)| (live_structure_anchor(transform, com), **radius))
        .chain(
            q_body
                .iter()
                .map(|(transform, radius)| (transform.translation, **radius)),
        )
        .collect();
    let position = escort_spawn_position(
        leader_anchor,
        leader_arm.map_or(0.0, |arm| **arm),
        away,
        leader_transform.rotation * Vec3::Y,
        side,
        design_reach(offer),
        &occupied,
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Two escorts hired one after the other from a ship docked at a station
    /// stand apart, each clear of the other, the ship and the station, and
    /// a place taken by a hull moves the escort further out instead of into
    /// it.
    #[test]
    fn hired_escorts_spawn_apart_and_clear_of_every_hull() {
        let (leader, leader_arm) = (Vec3::ZERO, 3.0);
        let station = (Vec3::new(-30.0, 0.0, 0.0), 25.0);
        let away = Vec3::X;
        let mut occupied = vec![(leader, leader_arm), station];
        let clear = |place: Vec3, reach: f32, occupied: &[(Vec3, f32)]| {
            occupied.iter().all(|&(centre, radius)| {
                place.distance(centre) >= radius + reach + ESCORT_SPAWN_CLEARANCE
            })
        };

        let first = escort_spawn_position(
            leader,
            leader_arm,
            away,
            Vec3::Y,
            FLEET_SIDES[0],
            4.0,
            &occupied,
        );
        assert!(
            clear(first, 4.0, &occupied),
            "the first escort stands clear: {first}"
        );
        occupied.push((first, 4.0));
        let second = escort_spawn_position(
            leader,
            leader_arm,
            away,
            Vec3::Y,
            FLEET_SIDES[1],
            4.0,
            &occupied,
        );
        assert!(
            clear(second, 4.0, &occupied),
            "the second escort stands clear of the first, the ship and the station: {second}"
        );

        // A hull parked on the first place: the escort moves out past it.
        let parked = vec![(leader, leader_arm), station, (first, 6.0)];
        let moved = escort_spawn_position(
            leader,
            leader_arm,
            away,
            Vec3::Y,
            FLEET_SIDES[0],
            4.0,
            &parked,
        );
        assert!(
            clear(moved, 4.0, &parked) && (moved - first).dot(away) > 0.0,
            "a taken place moves the escort further out along the way from the station: {moved}"
        );
    }
}
