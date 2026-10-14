//! Staging for the hand-played open-world examples: start New Game as the
//! Create World modal does, wait for the window to stream in, jump the
//! player, and berth it at a generated station until the dock takes.
//!
//! Include it with `#[path = "shared/open_world_stage.rs"] mod
//! open_world_stage;`. [`isolate_the_profile`] runs first in `main`;
//! [`OpenWorldStartPlugin`] starts the world; the rest are `&mut World`
//! helpers an example's own phase machine calls. Nothing here saves: the
//! session has no save folder, as `station_docked_sandbox` has none.

#![allow(
    dead_code,
    reason = "each example uses part of the shared staging, and an include compiles it per example"
)]

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_protocol::prelude::*;
use nova_world::prelude::{desired_sectors, SectorRoot, WorldConfig};

/// The gap a berth leaves between the two port faces, engine units: half a
/// cell, inside the 10 m capture distance.
const BERTH_GAP: f32 = 0.5;

/// Where a port's face sits in its own frame, as the docking verb reads it.
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

/// Point this run's settings and saves at a folder of its own under
/// `target/example-profiles/`, so no player's saved world is in the way and
/// nothing this run does touches it. Call it first in `main`.
pub fn isolate_the_profile(crate_name: &str) {
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{crate_name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );
}

/// Starts New Game on `seed` as start role `role` the first time the main
/// menu comes up, with no save folder. A later Back to Main Menu stays on
/// the menu.
pub struct OpenWorldStartPlugin {
    /// The world seed.
    pub seed: u32,
    /// The start role id.
    pub role: String,
}

/// The start this run asked for, and whether it started.
#[derive(Resource, Clone, Debug)]
pub struct OpenWorldStart {
    /// The world seed.
    pub seed: u32,
    /// The start role id.
    pub role: String,
    /// New Game was started.
    pub started: bool,
}

impl Plugin for OpenWorldStartPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(OpenWorldStart {
            seed: self.seed,
            role: self.role.clone(),
            started: false,
        });
        app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    }
}

/// Start New Game as the Create World modal does, once.
///
/// # Panics
///
/// On a role no loaded mod authors: the list of known roles is in the
/// message.
fn start_the_world(
    mut commands: Commands,
    mut start: ResMut<OpenWorldStart>,
    roles: Res<GameStartRoles>,
    mut mode: ResMut<GameMode>,
    mut pick: ResMut<NewGameScenario>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if start.started {
        return;
    }
    let role = StartRoleDesignId::from(start.role.as_str());
    if roles.get_role(&role).is_none() {
        let known: Vec<&str> = roles.0.iter().map(|role| role.id.as_str()).collect();
        panic!("open world stage: no start role '{role}'; the roles are {known:?}");
    }
    commands.insert_resource(OpenWorldSession {
        seed: start.seed,
        role,
    });
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    start.started = true;
    info!(
        "open world stage: New Game on seed {} as '{}', no save folder",
        start.seed, start.role
    );
}

/// The one player ship, if exactly one stands.
pub fn the_player(world: &mut World) -> Option<Entity> {
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The world's cell edge, once the world is armed.
pub fn sector_edge(world: &World) -> Option<Meters> {
    world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()
        .map(|config| config.sector_edge)
}

/// The cell `position` (engine units) stands in.
pub fn cell_of(world: &World, position: Vec3) -> Option<SectorCoord> {
    sector_edge(world).map(|edge| SectorCoord::containing(Meters3::from_engine(position), edge))
}

/// Whether the world is armed around one player ship and the live set is
/// exactly the window around `centre`.
pub fn window_is(world: &mut World, centre: SectorCoord) -> bool {
    let Some(radius) = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()
        .map(|config| config.active_radius)
    else {
        return false;
    };
    if the_player(world).is_none() {
        return false;
    }
    let live: std::collections::BTreeSet<SectorCoord> = world
        .query::<&SectorRoot>()
        .iter(world)
        .map(|root| root.0)
        .collect();
    live == desired_sectors(centre, radius)
}

/// Whether the window around the player's own cell has streamed in.
pub fn window_settled(world: &mut World) -> bool {
    let Some(player) = the_player(world) else {
        return false;
    };
    let Some(cell) = world
        .get::<Position>(player)
        .and_then(|position| cell_of(world, position.0))
    else {
        return false;
    };
    window_is(world, cell)
}

/// One generated station as the chooser sees it.
pub struct StationCandidate<'a> {
    /// Its generated id.
    pub id: &'a GeneratedBodyId,
    /// Where it stands, engine units.
    pub position: Vec3,
    /// What it offers, if anything.
    pub services: Option<&'a StationServices>,
    /// It is a dead station.
    pub derelict: bool,
    /// Its side.
    pub allegiance: Option<&'a Allegiance>,
    /// Its escort catalogue.
    pub offers: Option<&'a HullOffers>,
    /// It parks hulls for sale, or is about to.
    pub sells_hulls: bool,
}

impl StationCandidate<'_> {
    /// A living station that offers `service`.
    pub fn offers_service(&self, service: StationServiceType) -> bool {
        station_offers(self.services, self.derelict, service)
    }

    /// Not on the Enemy side.
    pub fn not_enemy(&self) -> bool {
        self.allegiance != Some(&Allegiance::Enemy)
    }
}

/// The chosen station: its id, its cell and where it stood.
#[derive(Clone, Debug)]
pub struct ChosenStation {
    /// Its generated id.
    pub id: GeneratedBodyId,
    /// The cell it stands in.
    pub cell: SectorCoord,
    /// Where it stood when chosen.
    pub position: Meters3,
}

/// The station nearest the origin that `accept` takes, outside the spawn's
/// cell, with ties broken by id.
pub fn nearest_station(
    world: &mut World,
    accept: impl Fn(&StationCandidate<'_>) -> bool,
) -> Option<ChosenStation> {
    let edge = sector_edge(world)?;
    let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
            Option<&HullOffers>,
            Has<ParkedHulls>,
            Has<PendingParkedHulls>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(
            |(id, position, services, derelict, allegiance, offers, parked, pending)| {
                accept(&StationCandidate {
                    id,
                    position: position.0,
                    services: *services,
                    derelict: *derelict,
                    allegiance: *allegiance,
                    offers: *offers,
                    sells_hulls: *parked || *pending,
                }) && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                    != SectorCoord::ORIGIN
            },
        )
        .map(|(id, position, ..)| {
            (
                Meters3::from_engine(position.0)
                    .distance(Meters3::ZERO)
                    .get(),
                id.clone(),
                position.0,
            )
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let (distance, id, at) = stations.into_iter().next()?;
    let position = Meters3::from_engine(at);
    let cell = SectorCoord::containing(position, edge);
    info!("open world stage: chose station '{id}' in cell {cell}, {distance:.0} m out");
    Some(ChosenStation { id, cell, position })
}

/// The entity wearing generated id `id`, if one does.
pub fn body_by_id(world: &mut World, id: &GeneratedBodyId) -> Option<Entity> {
    world
        .query::<(Entity, &GeneratedBodyId)>()
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// Jump `ship` to `at`, at rest.
pub fn jump_to(world: &mut World, ship: Entity, at: Meters3) {
    world.entity_mut(ship).insert((
        Position(at.to_engine()),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
}

/// Jump `ship` `distance` from `target` toward the origin, at rest, so the
/// target's window streams in around it.
pub fn park_beside(world: &mut World, ship: Entity, target: Meters3, distance: Meters) {
    let toward_origin = (-target.get()).normalize_or(Vec3::X);
    jump_to(
        world,
        ship,
        Meters3(target.get() + toward_origin * distance.get()),
    );
}

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
pub fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let transform = world
            .get::<Transform>(at)
            .expect("open world stage: a mounted section has a transform");
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        at = world
            .get::<ChildOf>(at)
            .expect("open world stage: a section's chain reaches its root")
            .parent();
    }
    (position, rotation)
}

/// The free, live docking ports mounted on `root`, lowest first.
pub fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
    let ports: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .collect();
    let mut ports: Vec<Entity> = ports
        .into_iter()
        .filter(|port| {
            let mut at = *port;
            while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                if parent == root {
                    return true;
                }
                at = parent;
            }
            false
        })
        .collect();
    ports.sort();
    ports
}

/// The free port of `target` that faces most squarely away from its root, so
/// a berth sets the ship outside the hull. The lowest port can face into the
/// station, where a berth crushes both hulls.
pub fn outward_port(world: &mut World, target: Entity) -> Option<Entity> {
    free_ports(world, target)
        .into_iter()
        .map(|port| {
            let (at, rotation) = local_pose(world, port, target);
            let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
            (port, facing)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(port, _)| port)
}

/// Put `ship` on the berth at `target`'s outward port, moving and turning
/// with it, point the mouse rig where the berth points the hull, and ask to
/// dock. Call it each frame until the dock takes.
///
/// # Panics
///
/// On a target or a ship with no free docking port.
pub fn berth_and_dock(world: &mut World, ship: Entity, target: Entity) {
    let (Some(target_port), Some(own_port)) = (
        outward_port(world, target),
        free_ports(world, ship).into_iter().next(),
    ) else {
        panic!("open world stage: the target or the ship has no free docking port");
    };
    let position = world.get::<Position>(target).expect("a target stands").0;
    let rotation = world.get::<Rotation>(target).expect("a target stands").0;
    let velocity = world
        .get::<LinearVelocity>(target)
        .map_or(Vec3::ZERO, |v| v.0);
    let spin = world
        .get::<AngularVelocity>(target)
        .map_or(Vec3::ZERO, |w| w.0);
    let (port_at, port_rot) = local_pose(world, target_port, target);
    let target_face_rot = rotation * port_rot;
    let target_face = position + rotation * port_at + target_face_rot * PORT_FACE;
    let target_axis = target_face_rot * Vec3::NEG_Z;
    let (own_at, own_rot) = local_pose(world, own_port, ship);
    let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -target_axis);
    let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
    let at = target_face + target_axis * BERTH_GAP - face_offset;
    world.entity_mut(ship).insert((
        Position(at),
        Rotation(turn),
        LinearVelocity(velocity),
        AngularVelocity(spin),
    ));
    // Left on the spawn heading, the rig turns the hull back through the
    // station the moment it undocks.
    let rigs: Vec<Entity> = world
        .query_filtered::<Entity, With<PointRotationOutput>>()
        .iter(world)
        .collect();
    for rig in rigs {
        world.entity_mut(rig).insert((
            PointRotation {
                initial_rotation: turn,
            },
            PointRotationOutput(turn),
        ));
    }
    world.trigger(DockingConnectionRequest {
        entity: ship,
        target,
    });
}

/// Where a station dock stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StationDockPhaseType {
    /// The window around the spawn is streaming in.
    #[default]
    Streaming,
    /// The player jumped beside the station; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked.
    Docked,
}

/// A dock at the nearest generated station `accept` takes, driven one phase
/// per call by [`advance_station_dock`].
#[derive(Clone, Debug, Default)]
pub struct StationDock {
    /// Where the dock stands.
    pub phase: StationDockPhaseType,
    /// The chosen station.
    pub station: Option<ChosenStation>,
}

/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);

/// Advance `dock` one step: choose the station once the spawn's window is in,
/// park beside it, wait for its window and for its parked hulls to dock, then
/// berth the player each call until the dock takes. Returns the player and
/// the station once docked.
///
/// # Panics
///
/// When the spawn's window holds no station `accept` takes outside the
/// spawn's cell: `what` names the station the example needs.
pub fn advance_station_dock(
    world: &mut World,
    dock: &mut StationDock,
    what: &str,
    accept: impl Fn(&StationCandidate<'_>) -> bool,
) -> Option<(Entity, Entity)> {
    match dock.phase {
        StationDockPhaseType::Streaming => {
            if window_is(world, SectorCoord::ORIGIN) {
                let chosen = nearest_station(world, accept).unwrap_or_else(|| {
                    panic!("open world stage: the spawn's window holds no {what}")
                });
                let player = the_player(world)?;
                park_beside(world, player, chosen.position, PARK_DISTANCE);
                dock.station = Some(chosen);
                dock.phase = StationDockPhaseType::Jumped;
            }
            None
        }
        StationDockPhaseType::Jumped => {
            let chosen = dock.station.clone()?;
            // Every hull the station parks takes its port first, so the
            // berth takes the port left free.
            let stands = body_by_id(world, &chosen.id).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && world.get::<ParkedHulls>(station).is_none_or(|parked| {
                        parked
                            .iter()
                            .all(|hull| world.get::<DockedShip>(hull).is_some())
                    })
            });
            if window_is(world, chosen.cell) && stands {
                dock.phase = StationDockPhaseType::Docking;
            }
            None
        }
        StationDockPhaseType::Docking => {
            let chosen = dock.station.clone()?;
            let player = the_player(world)?;
            let station = body_by_id(world, &chosen.id)?;
            if world.get::<DockedShip>(player).is_some() {
                dock.phase = StationDockPhaseType::Docked;
                Some((player, station))
            } else {
                berth_and_dock(world, player, station);
                None
            }
        }
        StationDockPhaseType::Docked => {
            let chosen = dock.station.clone()?;
            Some((the_player(world)?, body_by_id(world, &chosen.id)?))
        }
    }
}

/// Hold the virtual clock's clamp every frame under the software rasterizer:
/// its frames are seconds long, and a scenario load hands `Time<Virtual>`
/// back at its default. Add it only under `NOVA_AUTOPILOT`.
pub fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
    let clamp = std::time::Duration::from_secs(1);
    if time.max_delta() != clamp {
        time.set_max_delta(clamp);
    }
}
