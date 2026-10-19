//! The walk the four located-job ranges share: New Game on a pinned seed as
//! the start role whose boards lean to the wanted kind, stream the window,
//! find a living station whose board posts a job of that kind, dock there,
//! accept it off the Jobs tab, read the hand-in refused as not done,
//! undock, do the job's work in the world, see the open world mark it done,
//! dock again and hand it in for the reward.
//!
//! Include with `#[path = "shared/located_jobs.rs"] mod located_jobs;` and
//! build the app with [`LocatedJobWalkPlugin`] and [`script`] for one
//! [`LocatedKindType`]. `NOVA_JOB_SEED=<u32>` plays another world seed than
//! the kind's pinned one, to hunt a seed whose window posts the kind. The
//! neutralize range plants its hostile: generated hostiles stand with their
//! own Enemy civilization, so no scanned seed put one in the cube of a
//! living friendly station near the spawn; the walk re-sides one intact
//! generated ship within one cell of the spawn to Enemy before any board
//! posts, and the spawn cell's board names it as it would any hostile. Each range asserts the same
//! four claims for its kind:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a <kind> job is posted and accepted` | a living station's board posts the kind; after the Accept click the job is held and its bodies, if the kind puts any in the world, stand under the poster's cell root |
//! | 2 | `outcome: the hand-in is refused until the <kind> job is done` | docked at the poster with the job held, Your jobs greys the Hand in button with the not-done reason in its tooltip |
//! | 3 | `outcome: the world marks the <kind> job done` | after the work the job is listed done while still held |
//! | 4 | `outcome: the hand-in pays the <kind> job` | docked again, the Hand in click pays the reward from the station, takes nothing out of the hold and lists the job completed |

use std::{collections::BTreeSet, sync::Arc};

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::{prelude::*, ui::InteractionDisabled};
use nova_protocol::{
    nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
    nova_interface::pane::InterfacePaneType,
    prelude::*,
};
use nova_ui::widget::{TextFieldValue, Tooltip};
use nova_world::prelude::{desired_sectors, SectorRoot, SectorShipConditionType, WorldConfig};

/// The four `outcome:` markers a range emits, held as literals in the
/// including example so the probe roster test reads them beside the range.
#[derive(Clone, Copy, Debug)]
pub struct LocatedJobMarkers {
    /// Claim 1: the job is posted and the accept holds it and spawns its bodies.
    pub accepted: &'static str,
    /// Claim 2: the hand-in is refused while the job is not done.
    pub refused: &'static str,
    /// Claim 3: the world marks the job done.
    pub done: &'static str,
    /// Claim 4: the hand-in pays the job.
    pub paid: &'static str,
}

/// Which located kind a range walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(dead_code, reason = "each including example constructs one variant")]
pub enum LocatedKindType {
    /// Take a stack off a wreck and undock with it.
    Recover,
    /// See a hauler to its mark.
    Escort,
    /// Reach a caller: dock with a hull in need, or neutralize a raider.
    Distress,
    /// Neutralize a named hostile hull.
    Neutralize,
}

impl LocatedKindType {
    /// The word the markers and the log lines use.
    pub const fn label(self) -> &'static str {
        match self {
            LocatedKindType::Recover => "recover",
            LocatedKindType::Escort => "escort",
            LocatedKindType::Distress => "distress",
            LocatedKindType::Neutralize => "neutralize",
        }
    }

    /// The start role whose boards lean to the kind and whose hull can do
    /// it. Escort plays the military start: the Trade starts that lean to
    /// it fly unarmed hulls, and the starting station's board hides a job
    /// the hull cannot do.
    pub const fn role(self) -> &'static str {
        match self {
            LocatedKindType::Recover => "scavenger",
            LocatedKindType::Escort => "military",
            LocatedKindType::Distress => "bounty_hunter",
            LocatedKindType::Neutralize => "military",
        }
    }

    /// The world seed the range plays: `NOVA_JOB_SEED` when set, else one
    /// whose window around the docked start holds a living station posting
    /// the kind where the Jobs pane lists it.
    pub fn seed(self) -> u32 {
        if let Ok(seed) = std::env::var("NOVA_JOB_SEED") {
            return seed
                .parse()
                .unwrap_or_else(|error| panic!("NOVA_JOB_SEED '{seed}' is not a u32: {error}"));
        }
        self.pinned_seed()
    }

    const fn pinned_seed(self) -> u32 {
        match self {
            LocatedKindType::Recover => 15,
            LocatedKindType::Escort => 2,
            LocatedKindType::Distress => 16,
            LocatedKindType::Neutralize => 16,
        }
    }

    /// Whether `kind` is this kind.
    pub const fn wants(self, kind: &JobType) -> bool {
        matches!(
            (self, kind),
            (LocatedKindType::Recover, JobType::Recover { .. })
                | (LocatedKindType::Escort, JobType::Escort { .. })
                | (LocatedKindType::Distress, JobType::Distress { .. })
                | (LocatedKindType::Neutralize, JobType::Neutralize { .. })
        )
    }
}

const NEW_GAME_BUTTON: &str = "New Game Button";
const WORLD_NAME_FIELD: &str = "World Name Field";
const WORLD_NAME: &str = "Located Jobs World";
const SEED_FIELD: &str = "World Seed Field";
const CREATE_WORLD_BUTTON: &str = "Create World Button";
const WORLD_NEXT_BUTTON: &str = "World Next Button";
const JOBS_TAB: &str = "InterfaceTabJobs";

/// How far from a hull the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);
/// The gap the berth leaves between the two port faces, engine units.
const BERTH_GAP: f32 = 0.5;
/// Where a port's face sits in its own frame, as the docking verb reads it.
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);
/// How far short of its mark the walk sets the hauler down, engine units,
/// so the leg ends in frames rather than minutes on a software GPU.
const HAULER_DROP_SHORT: f32 = 15.0;

/// Seconds a load, a stream or a beat gets on a software-rendered GPU.
const SESSION_SECS: f32 = 240.0;
/// Seconds the work in the world gets.
const WORK_SECS: f32 = 480.0;
/// Seconds a click gets.
const BEAT_SECS: f32 = 60.0;
/// Frames a pane gets to draw.
const SETTLE_FRAMES: u32 = 6;
/// The virtual clock's clamp on the software rasterizer.
const WALK_CLOCK_SECS: f32 = 1.0;

/// What the run chose and recorded between beats.
#[derive(Resource, Debug)]
pub struct LocatedJobWatch {
    /// The kind the range walks.
    kind: LocatedKindType,
    /// The markers the range emits.
    markers: LocatedJobMarkers,
    /// The posting station's typed id, the cell it stands in and where it
    /// stood when chosen.
    station: Option<(GeneratedBodyId, SectorCoord, Meters3)>,
    /// The job the walk accepts, and its row on the posted list.
    job: Option<(usize, Job)>,
    /// Whether the work's one-shot world edit landed.
    worked: bool,
    /// The hostile the neutralize walk planted.
    planted: Option<GeneratedBodyId>,
    /// The player's and the station's balances before the hand-in.
    before_cr: Option<(u32, u32)>,
}

impl LocatedJobWatch {
    fn new(kind: LocatedKindType, markers: LocatedJobMarkers) -> Self {
        Self {
            kind,
            markers,
            station: None,
            job: None,
            worked: false,
            planted: None,
            before_cr: None,
        }
    }
}

/// The walk's record and its clock hold.
pub struct LocatedJobWalkPlugin(pub LocatedKindType, pub LocatedJobMarkers);

impl Plugin for LocatedJobWalkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LocatedJobWatch::new(self.0, self.1));
        app.add_systems(Update, hold_the_walk_clock);
        if self.0 == LocatedKindType::Neutralize {
            // Every frame from boot: the spawn cell's board can post while
            // the world still loads, before the walk's first step runs.
            app.add_systems(Update, plant_a_hostile);
        }
    }
}

/// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
/// scenario load hands `Time<Virtual>` back at its default.
fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
    let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
    if time.max_delta() != clamp {
        time.set_max_delta(clamp);
    }
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

fn kind(world: &World) -> LocatedKindType {
    world.resource::<LocatedJobWatch>().kind
}

fn tag(world: &World) -> &'static str {
    kind(world).label()
}

fn markers(world: &World) -> LocatedJobMarkers {
    world.resource::<LocatedJobWatch>().markers
}

/// The one player ship, if exactly one stands.
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The player's cell, once the player stands in an armed world: the start
/// docks the player at its starting station, so the spawn's cell is the
/// station's, not the origin.
fn player_cell(world: &World) -> Option<SectorCoord> {
    let edge = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()?
        .sector_edge;
    let at = world.get::<Position>(the_player(world)?)?.0;
    Some(SectorCoord::containing(Meters3::from_engine(at), edge))
}

fn player(world: &World) -> Entity {
    the_player(world).expect("located jobs: exactly one player ship")
}

/// The entity wearing the generated id `id`, if one does.
fn body_wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
    query
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

fn station_root(world: &World) -> Option<Entity> {
    let (id, ..) = world.resource::<LocatedJobWatch>().station.clone()?;
    body_wearing(world, &id)
}

fn station(world: &World) -> Entity {
    station_root(world).expect("located jobs: the station stands")
}

fn the_job(world: &World) -> (usize, Job) {
    world
        .resource::<LocatedJobWatch>()
        .job
        .clone()
        .expect("located jobs: the job is chosen")
}

/// The body the held job is about: the wreck, the hauler, the caller or the
/// hull.
fn located_body(world: &World) -> GeneratedBodyId {
    let (_, job) = the_job(world);
    job.located_body()
        .cloned()
        .expect("located jobs: a located job names a body")
}

fn world_shape(world: &World) -> (Meters, i32) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
    (config.sector_edge, config.active_radius)
}

fn live_cells(world: &World) -> BTreeSet<SectorCoord> {
    world
        .try_query::<&SectorRoot>()
        .map(|mut query| query.iter(world).map(|root| root.0).collect())
        .unwrap_or_default()
}

/// Advance once the world is armed around one player ship and the live set
/// is exactly the window around `centre`.
fn window_is(
    centre: impl Fn(&World) -> Option<SectorCoord> + Send + Sync + 'static,
) -> Arc<Predicate> {
    when(move |world| {
        if !world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
            || the_player(world).is_none()
        {
            return false;
        }
        let Some(centre) = centre(world) else {
            return false;
        };
        let (_, radius) = world_shape(world);
        live_cells(world) == desired_sectors(centre, radius)
    })
}

fn the_job_is_chosen() -> Arc<Predicate> {
    when(|world| world.resource::<LocatedJobWatch>().job.is_some())
}

fn the_job_is_held() -> Arc<Predicate> {
    when(|world| {
        world
            .resource::<LocatedJobWatch>()
            .job
            .as_ref()
            .is_some_and(|(_, job)| world.resource::<Jobs>().holds(&job.id))
    })
}

fn the_job_is_done() -> Arc<Predicate> {
    when(|world| {
        world
            .resource::<LocatedJobWatch>()
            .job
            .as_ref()
            .is_some_and(|(_, job)| world.resource::<Jobs>().is_done(&job.id))
    })
}

fn the_job_is_completed() -> Arc<Predicate> {
    when(|world| {
        world
            .resource::<LocatedJobWatch>()
            .job
            .as_ref()
            .is_some_and(|(_, job)| world.resource::<Jobs>().is_completed(&job.id))
    })
}

/// Whether the player is docked, and with `target` when one is named.
fn docked_with(world: &World, target: Option<Entity>) -> bool {
    the_player(world).is_some_and(|player| {
        world.get::<DockedShip>(player).is_some_and(|docked| {
            target.is_none_or(|target| {
                world
                    .get::<DockingConnection>(docked.connection)
                    .is_some_and(|connection| {
                        connection.first_ship == target || connection.second_ship == target
                    })
            })
        })
    })
}

fn player_docked_at_the_station() -> Arc<Predicate> {
    when(|world| docked_with(world, station_root(world)))
}

fn player_docked() -> Arc<Predicate> {
    when(|world| docked_with(world, None))
}

fn player_free() -> Arc<Predicate> {
    when(|world| the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none()))
}

fn the_interface_shows(pane: Option<InterfacePaneType>) -> Arc<Predicate> {
    when(move |world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && pane.is_none_or(|pane| {
                world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
            })
    })
}

fn unpaused() -> Arc<Predicate> {
    when(|world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
    })
}

/// Every UI text on screen.
fn texts(world: &mut World) -> Vec<String> {
    world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect()
}

/// The index and texts of the Jobs pane's posted row `JobRowPosted-<i>`
/// whose title cell reads `title`.
fn posted_row(world: &mut World, title: &str) -> Option<(usize, Vec<String>)> {
    let rows: Vec<(usize, Entity)> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter_map(|(entity, name)| {
            let index = name.as_str().strip_prefix("JobRowPosted-")?.parse().ok()?;
            Some((index, entity))
        })
        .collect();
    rows.into_iter().find_map(|(index, row)| {
        let mut texts = Vec::new();
        let mut stack = vec![row];
        while let Some(entity) = stack.pop() {
            let entity = world.entity(entity);
            if let Some(text) = entity.get::<Text>() {
                texts.push(text.0.clone());
            }
            if let Some(children) = entity.get::<Children>() {
                stack.extend(children.iter());
            }
        }
        texts
            .iter()
            .any(|text| text == title)
            .then_some((index, texts))
    })
}

fn ui_node_named(name: &'static str) -> Arc<Predicate> {
    when(move |world| {
        world
            .try_query::<&Name>()
            .is_some_and(|mut query| query.iter(world).any(|found| found.as_str() == name))
    })
}

/// Teleport the player to `position`, at rest.
fn park_player(world: &mut World, position: Meters3) {
    let player = player(world);
    world.entity_mut(player).insert((
        Position(position.to_engine()),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
    info!("{} job: the player jumps to {position:?}", tag(world));
}

/// Where the player parks beside the body `root`: [`PARK_DISTANCE`] from it
/// toward the world origin.
fn park_beside(world: &mut World, root: Entity) {
    let at = world
        .get::<Position>(root)
        .map(|position| Meters3::from_engine(position.0))
        .expect("located jobs: the body stands");
    let toward_origin = (-at.get()).normalize_or(Vec3::X);
    park_player(
        world,
        Meters3(at.get() + toward_origin * PARK_DISTANCE.get()),
    );
}

fn park_beside_the_station(world: &mut World) {
    let station = station(world);
    park_beside(world, station);
}

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let transform = world
            .get::<Transform>(at)
            .expect("located jobs: a mounted section has a transform");
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        at = world
            .get::<ChildOf>(at)
            .expect("located jobs: a section's chain reaches its root")
            .parent();
    }
    (position, rotation)
}

/// Whether `entity`'s mount chain reaches `root`.
fn mounted_on(world: &World, entity: Entity, root: Entity) -> bool {
    let mut at = entity;
    while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
        if parent == root {
            return true;
        }
        at = parent;
    }
    false
}

/// The free, live docking ports mounted on `root`, lowest first.
fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
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
        .filter(|port| mounted_on(world, *port, root))
        .collect();
    ports.sort();
    ports
}

/// The lowest free, live docking port mounted on `root`.
fn free_port(world: &mut World, root: Entity) -> Entity {
    free_ports(world, root)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("located jobs: {root:?} has no free docking port"))
}

/// The free port of `target` that faces most squarely away from its root,
/// so the berth sets the player outside the hull.
fn outward_port(world: &mut World, target: Entity) -> Entity {
    free_ports(world, target)
        .into_iter()
        .map(|port| {
            let (at, rotation) = local_pose(world, port, target);
            let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
            (port, facing)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(port, _)| port)
        .unwrap_or_else(|| panic!("located jobs: {target:?} has no free docking port"))
}

/// Put the player on the berth at `target`'s collar and ask to dock, every
/// frame until the dock takes.
fn berth_and_dock(world: &mut World, target: Entity) {
    let player = player(world);
    if world.get::<DockedShip>(player).is_some() {
        return;
    }
    let target_port = outward_port(world, target);
    let player_port = free_port(world, player);
    let body = |entity: Entity| {
        (
            world
                .get::<Position>(entity)
                .expect("a hull has a position")
                .0,
            world
                .get::<Rotation>(entity)
                .expect("a hull has a rotation")
                .0,
            world
                .get::<LinearVelocity>(entity)
                .map_or(Vec3::ZERO, |v| v.0),
            world
                .get::<AngularVelocity>(entity)
                .map_or(Vec3::ZERO, |w| w.0),
        )
    };
    let (target_at, target_rot, target_velocity, target_spin) = body(target);
    let (port_at, port_rot) = local_pose(world, target_port, target);
    let target_face_rot = target_rot * port_rot;
    let target_face = target_at + target_rot * port_at + target_face_rot * PORT_FACE;
    let target_axis = target_face_rot * Vec3::NEG_Z;
    let (own_at, own_rot) = local_pose(world, player_port, player);
    let rotation = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -target_axis);
    let face_offset = rotation * own_at + rotation * own_rot * PORT_FACE;
    let position = target_face + target_axis * BERTH_GAP - face_offset;
    world.entity_mut(player).insert((
        Position(position),
        Rotation(rotation),
        LinearVelocity(target_velocity),
        AngularVelocity(target_spin),
    ));
    world.trigger(DockingConnectionRequest {
        entity: player,
        target,
    });
}

fn berth_and_dock_at_the_station(world: &mut World) {
    let station = station(world);
    berth_and_dock(world, station);
}

/// Activate the UI node named `name`, as a click on it would.
fn activate_named(world: &mut World, name: &str) {
    let target = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("located jobs: no node named '{name}'"));
    world.trigger(bevy::ui_widgets::Activate { entity: target });
}

fn set_field_text(world: &mut World, name: &str, text: &str) {
    let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
    let (_, mut value) = fields
        .iter_mut(world)
        .find(|(found, _)| found.as_str() == name)
        .unwrap_or_else(|| panic!("located jobs: no field named '{name}'"));
    value.0 = text.to_string();
}

/// Every living, non-Enemy station board in the window, nearest the origin
/// first: the id, the cell, the position, the jobs and whether it is the
/// starting station's board.
fn posted_boards(
    world: &mut World,
) -> Vec<(GeneratedBodyId, SectorCoord, Meters3, Vec<Job>, bool)> {
    let (edge, _) = world_shape(world);
    let mut boards: Vec<(GeneratedBodyId, SectorCoord, Meters3, Vec<Job>, bool)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            &JobBoard,
            Has<StarterBoardMarker>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(|(.., derelict, allegiance)| !derelict && *allegiance != Some(&Allegiance::Enemy))
        .map(|(id, position, board, starter, ..)| {
            let position = Meters3::from_engine(position.0);
            (
                id.clone(),
                SectorCoord::containing(position, edge),
                position,
                board.0.clone(),
                starter,
            )
        })
        .collect();
    boards.sort_by(|a, b| {
        a.2.distance(Meters3::ZERO)
            .get()
            .total_cmp(&b.2.distance(Meters3::ZERO).get())
            .then_with(|| a.0.cmp(&b.0))
    });
    boards
}

/// Whether the player's hold takes the whole stack a recovery asks for.
fn stack_fits(world: &World, job: &Job) -> bool {
    let JobType::Recover { item, count, .. } = &job.kind else {
        return true;
    };
    let items = world.resource::<GameItems>();
    the_player(world)
        .and_then(|player| world.get::<ShipInventory>(player))
        .is_some_and(|hold| {
            items.design(item).stack_mass_g(*count) <= u64::from(hold.free_g(items))
        })
}

/// How far above the start's spawn point the neutralize walk plants its
/// hostile: in the start's cell, clear of the starting station and the start
/// role's ship.
const PLANT_ABOVE: Meters = Meters(4_000.0);

/// Where the neutralize walk plants its hostile: [`PLANT_ABOVE`] the
/// session's spawn point.
fn plant_at(world: &World) -> Meters3 {
    let spawn = world.resource::<OpenWorldSession>().start.spawn;
    Meters3(spawn.get() + Vec3::Y * PLANT_ABOVE.get())
}

/// Plant one intact Armored hull on the Enemy side in the spawn's cell, once,
/// before any board posts: the spawn's cell is in the cube of every station
/// within one cell of it, so such a station's board names the hull as a
/// hostile. The hull is laid out and stocked as the open world lays out and
/// stocks a raider, for the civilization in reach of the spawn; its crew
/// is a non-combatant so the walk's docking beats are not shot at.
fn plant_a_hostile(world: &mut World) {
    if world.resource::<LocatedJobWatch>().planted.is_some()
        || !world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
    {
        return;
    }
    let Some(home) = player_cell(world) else {
        return;
    };
    let Some(origin) = world
        .query::<(Entity, &SectorRoot)>()
        .iter(world)
        .find(|(_, root)| root.0 == home)
        .map(|(entity, _)| entity)
    else {
        return;
    };
    let Some(items) = world.get_resource::<GameItems>().cloned() else {
        return;
    };
    let id = GeneratedBodyId::new(home, "walk_hostile", 0);
    let plant_at = plant_at(world);
    let (name, design, stock) = {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        let key = |aspect: &[u8]| {
            Fnv32::new()
                .write(&config.seed.to_le_bytes())
                .write(aspect)
                .write(id.to_string().as_bytes())
                .finish()
        };
        let parts = config.generator.parts();
        let civilization = CivilizationField::new(config.seed)
            .in_reach(plant_at)
            .unwrap_or_else(|fault| panic!("neutralize job: the plant: {fault}"))
            .into_iter()
            .max_by(|a, b| a.selection_weight.total_cmp(&b.selection_weight))
            .expect("neutralize job: a civilization reaches the spawn")
            .civilization;
        let request = ShipLayoutRequest {
            seed: key(b"walk_hostile_hull"),
            civilization: civilization.id,
            role: ShipRoleType::Armored,
            advancement: civilization.advancement,
        };
        let layout = generate_ship(parts, request)
            .unwrap_or_else(|failure| panic!("neutralize job: the plant: {failure}"));
        let stock = ship_stock(
            parts,
            &items,
            &layout.design,
            ShipRoleType::Armored,
            SectorShipConditionType::Intact,
            civilization.advancement,
            key(b"walk_hostile_stock"),
        )
        .expect("neutralize job: the plant's hold fits its role's lightest item");
        (
            format!("{} raider, planted", civilization.id.name()),
            layout.design,
            stock,
        )
    };
    world.spawn((
        base_scenario_object(&BaseScenarioObjectConfig {
            id: id.to_string(),
            name,
            position: plant_at,
            rotation: Quat::IDENTITY,
        }),
        id.clone(),
        spaceship_scenario_object(SpaceshipConfig {
            design: ShipDesignSource::Inline(design),
            controller: SpaceshipController::AI(AIControllerConfig {
                non_combatant: true,
                ..AIControllerConfig::default()
            }),
            initial_velocity: MetersPerSecond3::ZERO,
            allegiance: Some(Allegiance::Enemy),
            capabilities: ShipCapabilities::default(),
            inventory: stock,
            lootable: false,
            credits: 0,
        }),
        ChildOf(origin),
    ));
    info!("neutralize job: the walk plants the hostile '{id}' at {plant_at:?}");
    world.resource_mut::<LocatedJobWatch>().planted = Some(id);
}

/// Pick the nearest living, non-Enemy station whose board posts a job of
/// the walked kind, and that job; nothing while no board does, so the step
/// keeps looking as boards post.
fn choose_the_job(world: &mut World) {
    let kind = kind(world);
    let boards = posted_boards(world);
    // The Jobs pane leaves a job the hull cannot do off the starting
    // station's board, so the walk cannot accept one there.
    let player = player(world);
    let mut fit = JobHullFit {
        hold_capacity_g: world
            .get::<ShipInventory>(player)
            .map_or(0, ShipInventory::capacity_g),
        has_clamp: false,
        has_weapon: false,
    };
    for (&ChildOf(root), clamp, turret, bay, railgun) in world
        .query_filtered::<(
            &ChildOf,
            Has<DockingSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Has<RailgunSectionMarker>,
        ), (With<SectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
    {
        if root == player {
            fit.has_clamp |= clamp;
            fit.has_weapon |= turret || bay || railgun;
        }
    }
    let items = world.resource::<GameItems>();
    let found = boards
        .iter()
        .find_map(|(id, cell, position, board, starter)| {
            board
                .iter()
                .enumerate()
                .find(|(_, job)| {
                    kind.wants(&job.kind)
                        && stack_fits(world, job)
                        && (!starter || plan_job_hull_fit(job, items, fit).is_ok())
                })
                .map(|(index, job)| (id.clone(), *cell, *position, index, job.clone()))
        });
    let Some((id, cell, position, index, job)) = found else {
        return;
    };
    let items = world.resource::<GameItems>().clone();
    info!(
        "{} job: '{id}' in cell {cell} posts row {index}: {} for {} cr",
        kind.label(),
        job.title(&items),
        job.reward_cr
    );
    let mut watch = world.resource_mut::<LocatedJobWatch>();
    watch.station = Some((id, cell, position));
    watch.job = Some((index, job));
}

/// Every station board in the window, for the log when none posts the kind.
fn describe_the_boards(world: &World) -> String {
    let items = world.resource::<GameItems>();
    let Some(mut boards) =
        world.try_query_filtered::<(&GeneratedBodyId, &JobBoard), With<StationMarker>>()
    else {
        return "no boards".to_string();
    };
    boards
        .iter(world)
        .map(|(id, board)| {
            format!(
                "{id}: {}",
                board
                    .0
                    .iter()
                    .map(|job| job.title(items))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Press and release a key action, one frame each.
fn tap(
    script: AutopilotPlugin<GameStates>,
    label: &str,
    action: &'static str,
    landed: Arc<Predicate>,
) -> AutopilotPlugin<GameStates> {
    script
        .step(format!("located jobs: press {label}"))
        .on_enter(press_action(action))
        .until(frames(1))
        .add()
        .step(format!("located jobs: let {label} up"))
        .on_enter(release_action(action))
        .until(landed)
        .deadline(SESSION_SECS)
        .add()
}

/// Open the interface and its Jobs tab.
fn open_the_jobs_tab(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
    tap(
        script,
        "the interface key",
        "interface_toggle",
        the_interface_shows(None),
    )
    .click_named(
        "located jobs: open the Jobs tab",
        JOBS_TAB,
        the_interface_shows(Some(InterfacePaneType::Jobs)),
        BEAT_SECS,
    )
    .step("located jobs: settle the Jobs tab")
    .until(frames(SETTLE_FRAMES))
    .add()
}

/// Claim 1: the accept holds the job and its bodies stand.
fn assert_the_accept(world: &mut World) {
    let tag = tag(world);
    let (_, job) = the_job(world);
    let items = world.resource::<GameItems>().clone();
    let jobs = world.resource::<Jobs>().clone();
    assert_eq!(jobs.held(), &[job.clone()], "{tag} job: the job is held");
    assert!(
        !jobs.is_done(&job.id),
        "{tag} job: the job is not done at the accept"
    );
    let station = station(world);
    let root = world
        .get::<ChildOf>(station)
        .expect("a generated station stands under its cell root")
        .parent();
    let mut bodies = Vec::new();
    for id in job.spawned_bodies() {
        let entity = body_wearing(world, &id)
            .unwrap_or_else(|| panic!("{tag} job: the accept put '{id}' in the world"));
        assert_eq!(
            world.get::<ChildOf>(entity).map(ChildOf::parent),
            Some(root),
            "{tag} job: '{id}' stands under the poster's cell root"
        );
        bodies.push(id.to_string());
    }
    match &job.kind {
        JobType::Escort { hauler, cargo_cr } => {
            let entity = body_wearing(world, hauler).expect("the hauler stands");
            assert!(
                world.get::<ShipHelmOrder>(entity).is_none(),
                "{tag} job: the hauler waits with no helm order while the player is docked"
            );
            assert!(
                jobs.legs().any(|(id, leg)| *id == job.id && !leg.started),
                "{tag} job: the hauler's leg is laid and not started"
            );
            assert_eq!(
                job.raider_ids().len() as u32,
                escort_raiders(*cargo_cr),
                "{tag} job: the raiders scale with the cargo"
            );
        }
        JobType::Distress { caller, genuine } => {
            let entity = body_wearing(world, caller).expect("the caller stands");
            assert_eq!(
                world.get::<DerelictShipMarker>(entity).is_some(),
                *genuine,
                "{tag} job: a genuine caller drifts, a trap is a raider"
            );
        }
        JobType::Recover {
            wreck, item, count, ..
        } => {
            let entity = body_wearing(world, wreck).expect("the wreck stands");
            let aboard = world
                .get::<ShipInventory>(entity)
                .map_or(0, |hold| hold.count(item));
            assert!(
                aboard >= *count,
                "{tag} job: the wreck carries {aboard} of the {count} {item} asked"
            );
        }
        JobType::Neutralize { hull, .. } => {
            let entity = body_wearing(world, hull).expect("the hull stands");
            assert!(
                world.get::<NeutralizedMarker>(entity).is_none(),
                "{tag} job: the hull is not neutralized yet"
            );
        }
        JobType::Procure { .. } => panic!("{tag} job: a located job was chosen"),
    }
    let title = job.title(&items);
    let shown = texts(world);
    assert!(
        shown.contains(&"Your jobs".to_string()),
        "{tag} job: the Your jobs header shows: {shown:?}"
    );
    nova_probe::probe_marker(
        world,
        markers(world).accepted,
        serde_json::json!({ "job": job.id.to_string(), "title": title, "bodies": bodies }),
    );
    info!("{tag} job: holds '{}' ({title}); bodies {bodies:?}", job.id);
}

/// Claim 2: Your jobs greys the Hand in button with the not-done reason in
/// its tooltip.
fn assert_the_refusal(world: &mut World) {
    let tag = tag(world);
    let (_, job) = the_job(world);
    let items = world.resource::<GameItems>().clone();
    let title = job.title(&items);
    let refusal = format!("{title}: not done yet");
    let (disabled, tooltip) = world
        .query::<(&Name, Has<InteractionDisabled>, Option<&Tooltip>)>()
        .iter(world)
        .find(|(name, ..)| name.as_str() == "JobHandIn-0")
        .map(|(_, disabled, tooltip)| (disabled, tooltip.map(|tooltip| tooltip.0.clone())))
        .unwrap_or_else(|| panic!("{tag} job: Your jobs shows a Hand in button for the held job"));
    assert!(
        disabled,
        "{tag} job: the Hand in button is greyed while the job is not done"
    );
    assert_eq!(
        tooltip.as_deref(),
        Some(refusal.as_str()),
        "{tag} job: the greyed Hand in button says why"
    );
    nova_probe::probe_marker(
        world,
        markers(world).refused,
        serde_json::json!({ "job": job.id.to_string(), "refusal": refusal }),
    );
    info!("{tag} job: the hand-in reads '{refusal}'");
}

/// Spend every weapon, controller and thruster section mounted on `root`
/// to zero health, as a scripted hit does, so the hull neutralizes.
fn disable_the_hull(world: &mut World, root: Entity) {
    let sections: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<Health>,
            Or<(
                With<TurretSectionMarker>,
                With<TorpedoSectionMarker>,
                With<RailgunSectionMarker>,
                With<ControllerSectionMarker>,
                With<ThrusterSectionMarker>,
            )>,
        )>()
        .iter(world)
        .filter(|section| mounted_on(world, *section, root))
        .collect();
    for section in sections {
        let left = world.get::<Health>(section).expect("queried").current;
        if left > 0.0 {
            world.trigger(HealthApplyDamage {
                entity: section,
                source: None,
                amount: left,
            });
        }
    }
}

/// One frame of the kind's work in the world, until the job is done.
fn do_the_work(world: &mut World) {
    let kind = kind(world);
    let (_, job) = the_job(world);
    let player = player(world);
    let body = located_body(world);
    let Some(target) = body_wearing(world, &body) else {
        return;
    };
    match (kind, &job.kind) {
        (LocatedKindType::Recover, JobType::Recover { item, count, .. }) => {
            let worked = world.resource::<LocatedJobWatch>().worked;
            if !docked_with(world, Some(target)) && !worked {
                berth_and_dock(world, target);
            } else if !worked {
                let items = world.resource::<GameItems>().clone();
                let aboard = world
                    .get::<ShipInventory>(target)
                    .map_or(0, |hold| hold.count(item));
                world
                    .get_mut::<ShipInventory>(target)
                    .expect("the wreck has a hold")
                    .remove(item, aboard);
                world
                    .get_mut::<ShipInventory>(player)
                    .expect("the player has a hold")
                    .add(&items, item, aboard);
                world.resource_mut::<LocatedJobWatch>().worked = true;
                info!("recover job: took {aboard} {item} off '{body}' ({count} asked)");
            } else if docked_with(world, Some(target)) {
                world.trigger(DockingReleaseRequest { entity: player });
            }
        }
        (LocatedKindType::Escort, JobType::Escort { .. }) => {
            if world.resource::<LocatedJobWatch>().worked
                || world.get::<ShipOrderEngaged>(target).is_none()
            {
                return;
            }
            let Some(ShipOrderDirective::Move { position, .. }) = world
                .get::<ShipHelmOrder>(target)
                .map(|order| order.directive.clone())
            else {
                panic!("escort job: the hauler flies a Move order");
            };
            let at = world.get::<Position>(target).expect("the hauler stands").0;
            let approach = (position - at).normalize_or(Vec3::X);
            world.entity_mut(target).insert((
                Position(position - approach * HAULER_DROP_SHORT),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            world.resource_mut::<LocatedJobWatch>().worked = true;
            info!(
                "escort job: set the hauler '{body}' down {HAULER_DROP_SHORT} units short of its mark"
            );
        }
        (LocatedKindType::Distress, JobType::Distress { genuine, .. }) => {
            if *genuine {
                berth_and_dock(world, target);
            } else {
                disable_the_hull(world, target);
            }
        }
        (LocatedKindType::Neutralize, JobType::Neutralize { .. }) => {
            disable_the_hull(world, target);
        }
        (kind, other) => panic!("{} job: the chosen job is {other:?}", kind.label()),
    }
}

/// Claim 3: the world marked the job done while it is still held.
fn assert_the_done(world: &mut World) {
    let tag = tag(world);
    let (_, job) = the_job(world);
    let jobs = world.resource::<Jobs>().clone();
    assert!(jobs.holds(&job.id), "{tag} job: the done job is still held");
    assert_eq!(jobs.done(), &[job.id.clone()], "{tag} job: the job is done");
    assert!(
        jobs.completed().is_empty(),
        "{tag} job: nothing is completed before the hand-in"
    );
    let body = located_body(world);
    let standing = body_wearing(world, &body).is_some();
    nova_probe::probe_marker(
        world,
        markers(world).done,
        serde_json::json!({ "job": job.id.to_string(), "body": body.to_string(), "body_stands": standing }),
    );
    info!(
        "{tag} job: '{}' is done; '{body}' stands: {standing}",
        job.id
    );
}

/// Note both balances before the hand-in.
fn note_the_balances(world: &mut World) {
    let (_, job) = the_job(world);
    let player = player(world);
    let station = station(world);
    let own_cr = world.get::<ShipCredits>(player).expect("credits").0;
    let station_cr = world.get::<ShipCredits>(station).expect("credits").0;
    assert!(
        station_cr >= job.reward_cr,
        "{} job: the station holds {station_cr} cr, the job pays {}",
        tag(world),
        job.reward_cr
    );
    world.resource_mut::<LocatedJobWatch>().before_cr = Some((own_cr, station_cr));
}

/// Claim 4: the hand-in paid the reward, took nothing and completed the job.
fn assert_the_hand_in(world: &mut World) {
    let tag = tag(world);
    let (_, job) = the_job(world);
    let (own_before, station_before) = world
        .resource::<LocatedJobWatch>()
        .before_cr
        .expect("recorded");
    let player = player(world);
    let station = station(world);
    let jobs = world.resource::<Jobs>().clone();
    assert!(jobs.held().is_empty(), "{tag} job: nothing is held");
    assert!(jobs.done().is_empty(), "{tag} job: nothing is done");
    let completed: Vec<&JobId> = jobs.completed().iter().map(|done| &done.id).collect();
    assert_eq!(completed, [&job.id], "{tag} job: the job is completed");
    let own_cr = world.get::<ShipCredits>(player).expect("credits").0;
    let station_cr = world.get::<ShipCredits>(station).expect("credits").0;
    assert_eq!(
        own_cr,
        own_before + job.reward_cr,
        "{tag} job: the player was paid"
    );
    assert_eq!(
        station_cr,
        station_before - job.reward_cr,
        "{tag} job: the station paid"
    );
    let buttons: Vec<String> = world
        .query::<&Name>()
        .iter(world)
        .map(|name| name.to_string())
        .filter(|name| name.starts_with("JobHandIn-"))
        .collect();
    assert!(
        buttons.is_empty(),
        "{tag} job: no Hand in button remains after the hand-in: {buttons:?}"
    );
    nova_probe::probe_marker(
        world,
        markers(world).paid,
        serde_json::json!({
            "job": job.id.to_string(),
            "reward_cr": job.reward_cr,
            "player_cr": own_cr,
            "station_cr": station_cr,
        }),
    );
    info!("{tag} job: PASS the hand-in paid {} cr", job.reward_cr);
}

/// Distress-only: the caller wears the objective marker tag
/// `mark_held_job_targets` gives the held job's located body.
fn the_caller_wears_its_marker() -> Arc<Predicate> {
    when(|world| {
        let body = located_body(world);
        body_wearing(world, &body)
            .is_some_and(|caller| world.get::<ObjectiveMarkerTarget>(caller).is_some())
    })
}

/// Distress-only: the HUD has grown the caller's marker chip.
fn a_chip_targets_the_caller() -> Arc<Predicate> {
    when(|world| {
        let body = located_body(world);
        let Some(caller) = body_wearing(world, &body) else {
            return false;
        };
        world
            .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|target| **target == caller))
    })
}

/// Distress-only claim: the caller carries the in-flight marker chip while
/// the player holds off it undocked.
fn assert_the_in_flight_marker(world: &mut World) {
    let (_, job) = the_job(world);
    let body = located_body(world);
    let caller =
        body_wearing(world, &body).unwrap_or_else(|| panic!("distress job: no '{body}' stands"));
    let jobs = world.resource::<Jobs>().clone();
    let code = jobs
        .mark_code(&job.id)
        .expect("distress job: the job is held");
    let label = format!("{code} Answer");
    let marker = world
        .get::<ObjectiveMarkerTarget>(caller)
        .unwrap_or_else(|| panic!("distress job: '{body}' wears no objective marker"));
    assert_eq!(
        marker.label, label,
        "distress job: the caller's marker reads '{label}'"
    );
    let chips = world
        .query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
        .iter(world)
        .filter(|&target| **target == caller)
        .count();
    assert_eq!(
        chips, 1,
        "distress job: exactly one chip targets the caller"
    );
    assert!(
        world.get::<DockedShip>(player(world)).is_none(),
        "distress job: the player is undocked"
    );
    info!("distress job: PASS the caller wears the in-flight marker '{label}'");
}

/// The walk for `kind`. Every beat waits on a world value.
pub fn script(kind: LocatedKindType) -> AutopilotPlugin<GameStates> {
    let tag = kind.label();
    let board_shot = format!("job-{tag}-board.png");
    let done_shot = format!("job-{tag}-done.png");
    let role_button = format!("Start Role Button: {}", kind.role());
    let script = AutopilotPlugin::<GameStates>::new()
        .step("located jobs: reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "located jobs: click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(WORLD_NEXT_BUTTON),
            BEAT_SECS,
        )
        .step("located jobs: name the world and enter the seed")
        .on_enter(move |world: &mut World| {
            set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
            set_field_text(world, SEED_FIELD, &kind.seed().to_string());
        })
        .until(frames(2))
        .add()
        .click_named(
            "located jobs: click Next",
            WORLD_NEXT_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_SECS,
        )
        .click_named(
            "located jobs: pick the start role",
            &role_button,
            pointer_released(),
            BEAT_SECS,
        )
        .click_named(
            "located jobs: click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("located jobs: the window streams in around the start")
        .until(window_is(player_cell))
        .deadline(SESSION_SECS)
        .add()
        .step("located jobs: the start docks the player at its starting station")
        .until(player_docked())
        .deadline(SESSION_SECS)
        .add()
        .step("located jobs: undock from the starting station")
        .on_enter(|world: &mut World| {
            let player = the_player(world).expect("located jobs: one player ship");
            world.trigger(DockingReleaseRequest { entity: player });
        })
        .until(player_free())
        .deadline(SESSION_SECS)
        .add()
        .step(format!("{tag} job: find a board posting the kind"))
        .each(|world, _, _| choose_the_job(world))
        .until(the_job_is_chosen())
        .diagnose(describe_the_boards)
        .deadline(SESSION_SECS)
        .add()
        .step("located jobs: jump to the station's side")
        .on_enter(park_beside_the_station)
        .until(window_is(|world| {
            world
                .resource::<LocatedJobWatch>()
                .station
                .as_ref()
                .map(|(_, cell, _)| *cell)
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("located jobs: berth and dock")
        .each(|world, _, _| berth_and_dock_at_the_station(world))
        .until(player_docked_at_the_station())
        .deadline(SESSION_SECS)
        .add();
    let board_shot_name = board_shot.clone();
    let script = open_the_jobs_tab(script)
        .step("located jobs: the board shows")
        .until(ui_node_present("JobAccept-0"))
        .deadline(BEAT_SECS)
        .add()
        .step("located jobs: click Accept")
        .on_enter(|world: &mut World| {
            let (_, job) = the_job(world);
            let title = job.title(world.resource::<GameItems>());
            let (row, _) = posted_row(world, &title)
                .unwrap_or_else(|| panic!("located jobs: the Jobs tab lists no row '{title}'"));
            activate_named(world, &format!("JobAccept-{row}"));
        })
        .until(and(the_job_is_held(), frames(SETTLE_FRAMES)))
        .deadline(BEAT_SECS)
        .add()
        .step(format!("{tag} job: assert the accept"))
        .on_enter(assert_the_accept)
        .add()
        .step(format!("{tag} job: assert the hand-in refusal"))
        .on_enter(assert_the_refusal)
        .add()
        .step("located jobs: shoot the board")
        .on_enter(move |world: &mut World| shoot(world, &board_shot_name))
        .until(shot_written(board_shot))
        .deadline(BEAT_SECS)
        .add();
    let script = tap(script, "the interface key", "interface_toggle", unpaused())
        .step("located jobs: undock")
        .on_enter(|world: &mut World| {
            let player = player(world);
            world.trigger(DockingReleaseRequest { entity: player });
        })
        .until(and(player_free(), frames(SETTLE_FRAMES)))
        .deadline(SESSION_SECS)
        .add();
    let script = if kind == LocatedKindType::Distress {
        let inflight_shot = format!("job-{tag}-inflight.png");
        let inflight_shot_name = inflight_shot.clone();
        script
            .step("distress job: park in flight off the caller")
            .on_enter(|world: &mut World| {
                // The chip pins to the screen edge when the caller is off
                // screen, so the shot proves it whichever way the view faces.
                let body = located_body(world);
                let caller = body_wearing(world, &body)
                    .unwrap_or_else(|| panic!("distress job: no '{body}' stands"));
                park_beside(world, caller);
            })
            .until(and(
                and(the_caller_wears_its_marker(), a_chip_targets_the_caller()),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step("distress job: assert the in-flight marker")
            .on_enter(assert_the_in_flight_marker)
            .add()
            .step("distress job: shoot the caller in flight")
            .on_enter(move |world: &mut World| shoot(world, &inflight_shot_name))
            .until(shot_written(inflight_shot))
            .deadline(BEAT_SECS)
            .add()
    } else {
        script
    };
    let script = script
        .step(format!("{tag} job: do the work"))
        .each(|world, _, _| do_the_work(world))
        .until(the_job_is_done())
        .deadline(WORK_SECS)
        .add()
        .step(format!("{tag} job: assert the done"))
        .on_enter(assert_the_done)
        .add()
        .step("located jobs: jump back to the station's side")
        .on_enter(|world: &mut World| {
            let player = player(world);
            if world.get::<DockedShip>(player).is_some() {
                world.trigger(DockingReleaseRequest { entity: player });
            }
        })
        .until(and(player_free(), frames(SETTLE_FRAMES)))
        .deadline(SESSION_SECS)
        .add()
        .step("located jobs: park beside the station")
        .on_enter(park_beside_the_station)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("located jobs: berth and dock again")
        .each(|world, _, _| berth_and_dock_at_the_station(world))
        .until(player_docked_at_the_station())
        .deadline(SESSION_SECS)
        .add()
        .step("located jobs: note the balances")
        .on_enter(note_the_balances)
        .add();
    let done_shot_name = done_shot.clone();
    open_the_jobs_tab(script)
        .step("located jobs: the Hand in button shows")
        .until(ui_node_named("JobHandIn-0"))
        .deadline(BEAT_SECS)
        .add()
        .step("located jobs: click Hand in")
        .on_enter(|world: &mut World| activate_named(world, "JobHandIn-0"))
        .until(and(the_job_is_completed(), frames(SETTLE_FRAMES)))
        .deadline(BEAT_SECS)
        .add()
        .step(format!("{tag} job: assert the hand-in"))
        .on_enter(assert_the_hand_in)
        .add()
        .step("located jobs: shoot the Jobs tab after the hand-in")
        .on_enter(move |world: &mut World| shoot(world, &done_shot_name))
        .until(shot_written(done_shot))
        .deadline(BEAT_SECS)
        .add()
}
