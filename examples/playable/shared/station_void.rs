//! Staging for the hand-played examples that start docked at an authored
//! station in a void: the scenario objects, the void, the dock, and reads of
//! the Inventory pane's draft form.
//!
//! Include it with `#[path = "shared/station_void.rs"] mod station_void;`
//! beside `mod open_world_stage;`, whose `berth` puts the hull on the berth.
//! A void loads in seconds, where a generated world takes minutes on a
//! software-rendered GPU.

#![allow(
    dead_code,
    reason = "each example uses part of the shared staging, and an include compiles it per example"
)]

use avian3d::prelude::{LinearVelocity, Position};
use bevy::{prelude::*, ui::InteractionDisabled};
use nova_protocol::{nova_interface::pane::InterfacePaneType, prelude::*};

use super::open_world_stage;

/// The scenario id of the hull the player flies.
pub const PLAYER_ID: &str = "player_spaceship";

/// A station is turned a quarter turn about +Y, so the waystation's collar,
/// which stands off its -X flank, faces +Z.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// The `Name` of the Inventory draft form's price text.
pub const DRAFT_PRICE: &str = "InventoryDraftPrice";
/// The `Name` of the Inventory draft form's Confirm button.
pub const DRAFT_CONFIRM: &str = "InventoryDraftConfirm";

/// A living Neutral station of `design` at `position`.
pub fn station(
    id: &str,
    name: &str,
    position: Meters3,
    design: StationDesignSource,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::from_rotation_y(STATION_TURN),
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design,
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    }
}

/// The shipped waystation, living and Neutral, at `position`.
pub fn waystation(id: &str, name: &str, position: Meters3) -> ScenarioObjectConfig {
    station(
        id,
        name,
        position,
        StationDesignSource::Prototype(BLOCK_WAYSTATION_STATION_ID.into()),
    )
}

/// Where the tender of `shared/docking_pair.rs` stands docked at a waystation
/// at `position`: the turned collar's face stands 2.5 cells along +Z from the
/// station origin and the tender's bow port face 2.5 cells ahead of its own,
/// [`BERTH_GAP`] apart. A player spawned here is on the berth already, so the
/// staging moves nothing.
pub fn tender_berth(position: Meters3) -> Meters3 {
    position + Meters3::from_engine(Vec3::new(0.0, 0.0, 5.0 + BERTH_GAP))
}

/// The player's ship `name` at `position`, flying `ship`.
pub fn player(name: &str, position: Meters3, ship: SpaceshipConfig) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }
}

/// A hold stock from `(item id, count)` pairs. A count of zero is left out:
/// a fixture table states it, and a stock refuses an empty stack.
pub fn stock(stacks: &[(&str, u32)]) -> ShipInventoryStock {
    ShipInventoryStock::new(
        stacks
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|&(id, count)| (id.into(), count)),
    )
}

/// The void `id`: `objects`, and one three-point light rig around each
/// point of `lit` with its offsets multiplied by the point's scale, because
/// the engine spawns no light.
pub fn void(
    id: &str,
    name: &str,
    objects: Vec<ScenarioObjectConfig>,
    lit: &[(Meters3, f32)],
    game_assets: &GameAssets,
) -> ScenarioConfig {
    let mut actions: Vec<EventActionConfig> = objects
        .into_iter()
        .map(EventActionConfig::SpawnScenarioObject)
        .collect();
    for (index, (at, scale)) in lit.iter().enumerate() {
        actions.extend(ThreePointRig::around(&format!("rig_{index}"), *at, *scale).actions());
    }
    ScenarioConfig {
        description: name.to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            id.to_string(),
            name.to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The station root with scenario id `id`, once it has spawned.
pub fn station_root(world: &mut World, id: &str) -> Option<Entity> {
    let mut stations = world.query_filtered::<(Entity, &EntityId), With<StationMarker>>();
    stations
        .iter(world)
        .find(|(_, found)| found.0 == id)
        .map(|(entity, _)| entity)
}

/// The player's ship root, once it has spawned.
pub fn player_root(world: &mut World) -> Option<Entity> {
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    players.iter(world).next()
}

/// The station the player is docked with, among the living stations.
pub fn docked_station(world: &mut World) -> Option<Entity> {
    let player = player_root(world)?;
    let connection = world.get::<DockedShip>(player)?.connection;
    let connection = world.get::<DockingConnection>(connection)?;
    [connection.first_ship, connection.second_ship]
        .into_iter()
        .find(|ship| world.get::<StationMarker>(*ship).is_some())
}

/// Whether the player is docked with the station `id`.
pub fn docked_at(world: &mut World, id: &str) -> bool {
    let station = station_root(world, id);
    station.is_some() && docked_station(world) == station
}

/// Put the player on the berth of the station `id` and, once the hull stands
/// there, ask to dock. Call it each frame; it returns `true` once the dock
/// holds.
pub fn berth_and_dock_at(world: &mut World, id: &str) -> bool {
    if docked_at(world, id) {
        return true;
    }
    let (Some(player), Some(station)) = (player_root(world), station_root(world, id)) else {
        return false;
    };
    // A port is not up on the frame its hull spawns.
    if world.get::<Position>(station).is_none()
        || open_world_stage::free_ports(world, player).is_empty()
        || open_world_stage::outward_port(world, station).is_none()
    {
        return false;
    }
    if open_world_stage::berth(world, player, station) {
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }
    false
}

/// Assert that the docked pair of `player` and `station` stands still. A
/// dock made at a stale pose throws both hulls at hundreds of meters per
/// second.
///
/// # Panics
///
/// On a pair that moves faster than 1 m/s.
pub fn assert_at_rest(world: &mut World, player: Entity, station: Entity) {
    let pose = |world: &World, entity: Entity| {
        (
            world
                .get::<Position>(entity)
                .expect("station void: a hull has a physics position")
                .0,
            world
                .get::<LinearVelocity>(entity)
                .map_or(Vec3::ZERO, |velocity| velocity.0),
        )
    };
    let (player_at, player_velocity) = pose(world, player);
    let (station_at, station_velocity) = pose(world, station);
    let gap = Meters::from_engine(player_at.distance(station_at));
    let speed = Meters::from_engine(player_velocity.length().max(station_velocity.length()));
    info!(
        "station void: the docked pair stands {:.1} m apart, origin to origin, at {:.2} m/s",
        gap.0, speed.0
    );
    assert!(
        speed.0 < 1.0,
        "station void: the docked pair must stand still: {:.2} m/s, {:.1} m apart",
        speed.0,
        gap.0
    );
}

/// Show `pane` of the interface, as Tab and the pane's tab do.
pub fn open_pane(world: &mut World, pane: InterfacePaneType) {
    *world.resource_mut::<InterfacePaneType>() = pane;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
}

/// Whether the interface is up.
pub fn interface_up(pause: Option<&State<PauseStates>>) -> bool {
    pause.is_some_and(|pause| *pause.get() == PauseStates::Interface)
}

/// The text of the UI node named `name`, if it is up.
pub fn node_text(world: &mut World, name: &str) -> Option<String> {
    let mut texts = world.query::<(&Name, &Text)>();
    texts
        .iter(world)
        .find(|(found, _)| found.as_str() == name)
        .map(|(_, text)| text.0.clone())
}

/// Whether the UI node named `name` is disabled, if it is up.
pub fn node_disabled(world: &mut World, name: &str) -> Option<bool> {
    let mut nodes = world.query::<(&Name, Has<InteractionDisabled>)>();
    nodes
        .iter(world)
        .find(|(found, _)| found.as_str() == name)
        .map(|(_, disabled)| disabled)
}
