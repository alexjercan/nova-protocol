//! system_station_round_trip: the stations epic's whole loop on a generated
//! New Game. The player loses a section, docks at a generated station,
//! rebuilds the section from parts and credits, undocks, leaves the sector,
//! comes back, and after a save and a Load the station is where and as the
//! player left it.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the session's starting station, where the start docks the player.
//! The walk undocks and jumps to the origin, the spawn point before the
//! docked start: stations are a civilization apart, so the station it wants
//! never streams in round the start. Once the window stands round the origin
//! it picks the nearest generated station that is living, not on the Enemy
//! side, offers Rebuild, and stands outside the origin's cell. The walk picks
//! the scavenger start role. THE LOSS AND THE PARTS ARE DISCLOSED FIXTURE
//! STATE: the salvage hauler's `pdc_dorsal` is set
//! to zero health with the depletion marker, the state the damage path leaves
//! a dead section in, and the PDC recipe's parts are added to the hold, so
//! the rebuild takes every part from the hold and costs the recipe's work.
//! Once docked, three section panels are put straight into the station's
//! storage, so the round trip has storage to keep.
//! The player jumps beside the station and is put on a berth at its collar
//! each frame until the dock takes; the walk never flies the approach
//! (`system_station_docking` does).
//!
//! The rebuild goes through the Ship pane's Rebuild button, the code path a
//! player clicks. The player undocks, jumps away until the station's cell
//! retires, and jumps back until it thaws. Then the player leaves to the main
//! menu, the leave save is read back off disk through [`open_world`], and the
//! world is loaded again in this process through the Load screen.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the lost section is rebuilt from its recipe at a generated station` | the section is live at full health; the hold lost exactly the recipe's parts, the player paid exactly the recipe's credits, the station earned them, and `OnRebuilt` fired once |
//! | 2 | `outcome: the retired cell's record holds the station as left` | the cell is not live, the station entity is gone, and the record holds the station under its id with the stock and storage it was left with, at the pose its motion carried it to |
//! | 3 | `outcome: the thawed station is the station as left` | a new entity wears the same id at the record's pose, with the same stock, storage, balance and live section count |
//! | 4 | `outcome: the leave save holds the station as left` | the leave save holds the station in its cell with the same stock and storage, and the player with the rebuilt section |
//! | 5 | `outcome: a loaded world gives back the station and the rebuilt section` | after Load the station wears its id at the saved pose with the same stock, storage, balance and live section count, and the player's section is live at full health |
//!
//! ONE SUBJECT: the done-when chain, end to end. Each link has its own range:
//! the rebuild rule and the code it keeps in `system_station_rebuild`, the
//! storage pane in `system_station_storage`, the generated station's retire
//! and thaw in `system_generated_stations`, the chart in `system_world_chart`.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=2400 \
//!   cargo run --example system_station_round_trip --features debug
//! # look for: `station round trip: PASS the loaded world holds the station as left`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_round_trip")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: lose a section, dock at a living generated station, \
             rebuild it from the Ship pane, undock, retire and thaw the station's cell, then check \
             the station and the section through a save and a Load. Autopilot-only correctness \
             range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds counts, credits, ids and poses.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::RoundTripWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::{pane::InterfacePaneType, ship::SectionCode},
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBody, FrozenBodyType, FrozenSector, FrozenSectors, SectorRoot,
        WorldConfig,
    };

    use super::*;

    /// The seed the range plays: `system_station_storage`'s seed, whose window
    /// around the origin holds a living Neutral station outside the origin's
    /// cell.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Round Trip World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the scavenger's salvage hauler carries the
    /// point-defense mount the walk loses and the hold its recipe parts go in.
    const START_ROLE_BUTTON: &str = "Start Role Button: scavenger";
    const START_ROLE: &str = "scavenger";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// The salvage hauler section the player loses and rebuilds: its one PDC,
    /// a leaf whose loss severs nothing else.
    const LOST_SECTION: &str = "pdc_dorsal";
    /// Its design code, the Rebuild button's name: the hauler's only PDC.
    const LOST_CODE: &str = "PDC-1";

    /// The disclosed fixture stack put in the station's storage while docked.
    const STORED_ITEM: &str = "SectionPanel";
    const STORED_COUNT: u32 = 3;

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// How far a station may stand from a pose it was read at, in meters, on
    /// top of what its own velocity carries it over the seconds between.
    const POSE_TOLERANCE_M: f32 = 1.0;
    /// How far its attitude may turn, in radians, on top of its own spin.
    const ATTITUDE_TOLERANCE_RAD: f32 = 0.05;

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Ship pane gets to build its panel and list.
    const SETTLE_FRAMES: u32 = 10;
    /// The Ship pane's side panel, which scrolls under the wheel.
    const SHIP_PANEL: &str = "ShipPanel";
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// The station as the player left it: what the retire, the thaw, the
    /// save and the Load must give back.
    #[derive(Clone, Debug, PartialEq)]
    struct StationState {
        stock: BTreeMap<String, u32>,
        storage: BTreeMap<String, u32>,
        credits: u32,
        sections: usize,
    }

    /// A station's pose and motion at a moment of virtual time.
    #[derive(Clone, Copy, Debug)]
    struct Pose {
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        spin: Vec3,
        at_secs: f32,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct RoundTripWatch {
        /// Every rebuild and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// Virtual seconds at which each station entity was added.
        added_at: BTreeMap<Entity, f32>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// The cell the retire jump lands in.
        away: Option<SectorCoord>,
        /// Where it stood when chosen, for the park points.
        position: Option<Meters3>,
        /// The entity that wore the id before the retire.
        before: Option<Entity>,
        /// The lost section's design code, the Rebuild button's name.
        code: Option<String>,
        /// The player's balance and recipe-part counts, and the station's
        /// balance, right before the rebuild.
        before_rebuild: Option<(u32, BTreeMap<String, u32>, u32)>,
        /// The station as left, after the undock.
        left: Option<StationState>,
        /// Its pose as left.
        left_pose: Option<Pose>,
        /// The retired record's pose, which the thaw must stand at.
        record_pose: Option<Pose>,
        /// The leave save's pose, which the Load must stand at.
        saved_pose: Option<Pose>,
    }

    impl RoundTripWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct RoundTripWatchPlugin;

    impl Plugin for RoundTripWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<RoundTripWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<RoundTripWatch>| {
                let name = event.name();
                if !matches!(name, "onrebuilt" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station round trip: {name} {data}");
                watch.events.push((name.to_string(), data));
            });
            app.add_observer(
                |add: On<Add, StationMarker>,
                 time: Res<Time<Virtual>>,
                 mut watch: ResMut<RoundTripWatch>| {
                    watch.added_at.insert(add.entity, time.elapsed_secs());
                },
            );
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

    fn now(world: &World) -> f32 {
        world.resource::<Time<Virtual>>().elapsed_secs()
    }

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("station round trip: exactly one player ship")
    }

    /// The entity wearing the chosen station's id, if one does.
    fn station_root(world: &World) -> Option<Entity> {
        let id = world.resource::<RoundTripWatch>().id.clone()?;
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station round trip: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<RoundTripWatch>()
            .id
            .clone()
            .expect("station round trip: the station is chosen")
    }

    /// The live section `id` of `root`, or `None` while it has none.
    fn section_of(world: &World, root: Entity, id: &str) -> Option<Entity> {
        world
            .get::<Children>(root)?
            .iter()
            .find(|child| world.get::<EntityId>(*child).is_some_and(|eid| eid.0 == id))
    }

    fn full_health(world: &World, section: Entity) -> bool {
        world
            .get::<Health>(section)
            .is_some_and(|health| health.max > 0.0 && health.current == health.max)
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station round trip: every ship carries a balance")
            .0
    }

    fn hold_map(inventory: &ShipInventory) -> BTreeMap<String, u32> {
        inventory
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect()
    }

    fn storage_map(storage: &StationStorage) -> BTreeMap<String, u32> {
        storage
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect()
    }

    /// The lost section's recipe, from the player ship's design.
    fn recipe(world: &World) -> BuildRecipe {
        world
            .get::<ShipDesignSections>(player(world))
            .expect("station round trip: the player ship has a design")
            .get(LOST_SECTION)
            .unwrap_or_else(|| panic!("station round trip: the design has no '{LOST_SECTION}'"))
            .config
            .base
            .build
            .clone()
    }

    fn live_sections(world: &World, root: Entity) -> usize {
        world.get::<Children>(root).map_or(0, |children| {
            children
                .iter()
                .filter(|child| world.get::<SectionMarker>(*child).is_some())
                .count()
        })
    }

    fn station_state(world: &World) -> StationState {
        let station = station(world);
        StationState {
            stock: hold_map(
                world
                    .get::<ShipInventory>(station)
                    .expect("station round trip: a station carries a hold"),
            ),
            storage: storage_map(
                world
                    .get::<StationStorage>(station)
                    .expect("station round trip: a station root carries StationStorage"),
            ),
            credits: credits(world, station),
            sections: live_sections(world, station),
        }
    }

    fn pose_of(world: &World, entity: Entity) -> Pose {
        Pose {
            position: world
                .get::<Position>(entity)
                .expect("station round trip: a station has a physics position")
                .0,
            rotation: world
                .get::<Rotation>(entity)
                .expect("station round trip: a station has a physics rotation")
                .0,
            velocity: world
                .get::<LinearVelocity>(entity)
                .map_or(Vec3::ZERO, |v| v.0),
            spin: world
                .get::<AngularVelocity>(entity)
                .map_or(Vec3::ZERO, |w| w.0),
            at_secs: now(world),
        }
    }

    fn record_pose(body: &FrozenBody, at_secs: f32) -> Pose {
        let transform = body.transform();
        let (velocity, spin) = body.motion().unwrap_or((Vec3::ZERO, Vec3::ZERO));
        Pose {
            position: transform.translation,
            rotation: transform.rotation,
            velocity,
            spin,
            at_secs,
        }
    }

    /// Assert `seen` stands within the tolerances of `reference`, widened by
    /// what `reference`'s motion carries it over `secs` virtual seconds.
    /// Returns the distance in meters and the turn in radians.
    fn assert_pose_near(label: &str, reference: Pose, seen: Pose, secs: f32) -> (f32, f32) {
        let moved_m = Meters3::from_engine(seen.position)
            .distance(Meters3::from_engine(reference.position))
            .get();
        let drift_m = Meters3::from_engine(reference.velocity * secs.max(0.0))
            .distance(Meters3::ZERO)
            .get();
        let turned = seen.rotation.angle_between(reference.rotation);
        let spun = reference.spin.length() * secs.max(0.0);
        assert!(
            moved_m <= POSE_TOLERANCE_M + drift_m,
            "station round trip: {label}: the station stands {moved_m:.2} m from the reference, \
             past {POSE_TOLERANCE_M} m plus {drift_m:.2} m of its own motion over {secs:.2} s"
        );
        assert!(
            turned <= ATTITUDE_TOLERANCE_RAD + spun,
            "station round trip: {label}: the station turned {turned:.4} rad, past \
             {ATTITUDE_TOLERANCE_RAD} rad plus {spun:.4} rad of its own spin over {secs:.2} s"
        );
        (moved_m, turned)
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
    }

    /// The cell of the session's starting station, where the start docks the
    /// player, once the session and the world config stand.
    fn start_cell(world: &World) -> Option<SectorCoord> {
        let edge = world
            .get_resource::<WorldConfig<NovaLayeredWorld>>()?
            .sector_edge;
        let start = world.get_resource::<OpenWorldSession>()?.start.position;
        Some(SectorCoord::containing(start, edge))
    }

    fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
        world
            .try_query::<&SectorRoot>()
            .map(|mut query| query.iter(world).map(|root| root.0).collect())
            .unwrap_or_default()
    }

    /// Advance once the world is armed around one player ship and the live
    /// set is exactly the window around `centre`.
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

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
        })
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    fn section_coded() -> Arc<Predicate> {
        when(|world| {
            the_player(world)
                .and_then(|player| section_of(world, player, LOST_SECTION))
                .is_some_and(|section| world.get::<SectionCode>(section).is_some())
        })
    }

    fn section_lost() -> Arc<Predicate> {
        when(|world| {
            the_player(world)
                .is_some_and(|player| section_of(world, player, LOST_SECTION).is_none())
        })
    }

    fn section_live() -> Arc<Predicate> {
        when(|world| {
            the_player(world)
                .is_some_and(|player| section_of(world, player, LOST_SECTION).is_some())
        })
    }

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    /// Whether the named node lies inside the Ship panel's box, where a
    /// click lands on it.
    fn in_panel(world: &World, name: &str) -> bool {
        match (ui_node_rect(world, name), ui_node_rect(world, SHIP_PANEL)) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
            _ => false,
        }
    }

    /// Whether the named node sits below the Ship panel's bottom edge.
    fn below_panel(world: &World, name: &str) -> bool {
        match (ui_node_rect(world, name), ui_node_rect(world, SHIP_PANEL)) {
            (Some(node), Some(panel)) => node.max.y > panel.max.y,
            _ => true,
        }
    }

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("station round trip: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<RoundTripWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "station round trip: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station round trip: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station round trip: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
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

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("station round trip: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull. Which port is
    /// lowest changes with entity ids, and a station's lowest port can face
    /// into its own structure, where a berth crushes both hulls.
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
            .unwrap_or_else(|| panic!("station round trip: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, moving and
    /// turning with the station, then ask to dock. Every frame until the dock
    /// takes, as `system_station_storage` does.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let pose = pose_of(world, station);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let station_face_rot = pose.rotation * port_rot;
        let station_face = pose.position + pose.rotation * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let rotation = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = rotation * own_at + rotation * own_rot * PORT_FACE;
        let position = station_face + station_axis * BERTH_GAP - face_offset;
        world.entity_mut(player).insert((
            Position(position),
            Rotation(rotation),
            LinearVelocity(pose.velocity),
            AngularVelocity(pose.spin),
        ));
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("station round trip: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station round trip: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station round trip: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Pick the nearest station that is living, not on the Enemy side,
    /// offers Rebuild, and stands outside the origin's cell, where the walk
    /// stands after its jump; choose the retire jump.
    fn choose_the_station(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let mut stations: Vec<(f32, GeneratedBodyId, Entity)> = world
            .query_filtered::<(
                Entity,
                &GeneratedBodyId,
                &Position,
                Option<&StationServices>,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .filter(|(_, _, position, services, derelict, allegiance)| {
                station_offers(*services, *derelict, StationServiceType::Rebuild)
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(entity, id, position, ..)| {
                (
                    Meters3::from_engine(position.0)
                        .distance(Meters3::ZERO)
                        .get(),
                    id.clone(),
                    entity,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, entity)) = stations.into_iter().next() else {
            panic!(
                "station round trip: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station with Rebuild outside the origin's cell"
            );
        };
        let pose = pose_of(world, entity);
        let position = Meters3::from_engine(pose.position);
        let cell = SectorCoord::containing(position, edge);
        let axes = [cell.x, cell.y, cell.z];
        let (axis, _) = axes
            .iter()
            .enumerate()
            .max_by_key(|(_, index)| index.abs())
            .expect("three axes");
        let mut away = [0; 3];
        away[axis] = axes[axis] - axes[axis].signum() * (radius + 1);
        let away = SectorCoord::new(away[0], away[1], away[2]);
        assert!(
            !desired_sectors(away, radius).contains(&cell),
            "the window around {away} leaves {cell} out"
        );
        let name = world
            .get::<Name>(entity)
            .map_or_else(|| entity.to_string(), ToString::to_string);
        info!(
            "station round trip: chose '{id}' ({name}) in cell {cell}, {distance:.0} m out, \
             {} live sections, moving at {:.3} m/s",
            live_sections(world, entity),
            Meters3::from_engine(pose.velocity)
                .distance(Meters3::ZERO)
                .get()
        );
        let mut watch = world.resource_mut::<RoundTripWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.away = Some(away);
        watch.position = Some(position);
        watch.before = Some(entity);
    }

    /// Disclosed fixture state: add the recipe's parts to the hold, record
    /// the section's code, and kill the section the way the damage path
    /// leaves a dead one.
    fn lose_the_section(world: &mut World) {
        assert_eq!(
            world.resource::<OpenWorldSession>().role.as_str(),
            START_ROLE,
            "station round trip: Create must start the picked role"
        );
        let player = player(world);
        let recipe = recipe(world);
        let items = world.resource::<GameItems>().clone();
        {
            let mut hold = world
                .get_mut::<ShipInventory>(player)
                .expect("station round trip: the player carries a hold");
            for (item, &count) in &recipe.parts {
                hold.add(&items, item, count);
            }
        }
        let section = section_of(world, player, LOST_SECTION)
            .unwrap_or_else(|| panic!("station round trip: '{LOST_SECTION}' is live at the start"));
        let code = world
            .get::<SectionCode>(section)
            .expect("station round trip: the pane coded the section")
            .0
            .clone();
        assert_eq!(
            code, LOST_CODE,
            "station round trip: '{LOST_SECTION}' codes {LOST_CODE}"
        );
        info!(
            "station round trip: fixture adds {:?} to the hold; '{LOST_SECTION}' ({code}) is lost, \
             its recipe {} cr",
            recipe.parts, recipe.credits
        );
        world.resource_mut::<RoundTripWatch>().code = Some(code);
        world
            .get_mut::<Health>(section)
            .expect("station round trip: the section has health")
            .current = 0.0;
        world.entity_mut(section).insert(HealthZeroMarker);
    }

    fn record_before_rebuild(world: &mut World) {
        let player = player(world);
        let recipe = recipe(world);
        let hold = world
            .get::<ShipInventory>(player)
            .expect("the player carries a hold");
        let parts: BTreeMap<String, u32> = recipe
            .parts
            .keys()
            .map(|item| (item.to_string(), hold.count(item)))
            .collect();
        let before = (
            credits(world, player),
            parts,
            credits(world, station(world)),
        );
        world.resource_mut::<RoundTripWatch>().before_rebuild = Some(before);

        // Disclosed fixture state: a stack in the station's storage, so the
        // retire, the thaw, the save and the Load have storage to keep.
        // `system_station_storage` stores through the pane.
        let station = station(world);
        let items = world.resource::<GameItems>().clone();
        world
            .get_mut::<StationStorage>(station)
            .expect("station round trip: a station root carries StationStorage")
            .add(&items, &STORED_ITEM.into(), STORED_COUNT);
        info!("station round trip: fixture stores {STORED_COUNT} {STORED_ITEM} at the station");
    }

    fn rebuild_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<RoundTripWatch>();
            !watch.named("onrebuilt").is_empty() && watch.named("oncreditschanged").len() >= 2
        })
    }

    /// Claim 1: the rebuilt section, the parts and credits by the recipe,
    /// and the one `OnRebuilt`; then undock.
    fn assert_the_rebuild(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let recipe = recipe(world);
        let section = section_of(world, player, LOST_SECTION)
            .expect("station round trip: the section is live after the rebuild");
        assert!(
            full_health(world, section),
            "station round trip: the rebuilt section is whole: {:?}",
            world.get::<Health>(section)
        );
        let watch = world.resource::<RoundTripWatch>();
        let code = watch.code.clone().expect("coded");
        let (player_cr, parts_before, station_cr) = watch
            .before_rebuild
            .clone()
            .expect("recorded before the rebuild");
        let hold = world
            .get::<ShipInventory>(player)
            .expect("the player carries a hold");
        for (item, &count) in &recipe.parts {
            assert_eq!(
                hold.count(item),
                parts_before[&item.to_string()] - count,
                "station round trip: the rebuild took the recipe's {count} {item} from the hold"
            );
        }
        assert_eq!(
            credits(world, player),
            player_cr - recipe.credits,
            "station round trip: the player paid the recipe's credits"
        );
        assert_eq!(
            credits(world, station),
            station_cr + recipe.credits,
            "station round trip: the station earned the recipe's credits"
        );
        let rebuilt = watch.named("onrebuilt");
        assert_eq!(rebuilt.len(), 1, "station round trip: one OnRebuilt");
        let event = rebuilt[0].clone();
        assert_eq!(event["section"], code.as_str());
        assert_eq!(event["price_cr"], recipe.credits);
        let parts: BTreeMap<String, u32> = recipe
            .parts
            .iter()
            .map(|(item, count)| (item.to_string(), *count))
            .collect();
        assert_eq!(event["parts"], serde_json::json!(parts));
        info!(
            "station round trip: rebuilt {LOST_SECTION} as {code} for {} cr from {parts:?}",
            recipe.credits
        );
        nova_probe::probe_marker(
            world,
            "outcome: the lost section is rebuilt from its recipe at a generated station",
            serde_json::json!({
                "station": station_id(world).to_string(),
                "code": code,
                "price_cr": recipe.credits,
                "parts": parts,
            }),
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// Record the station as the player leaves it.
    fn record_the_station_as_left(world: &mut World) {
        let state = station_state(world);
        let pose = pose_of(world, station(world));
        info!("station round trip: left the station as {state:?} at {pose:?}");
        let mut watch = world.resource_mut::<RoundTripWatch>();
        watch.left = Some(state);
        watch.left_pose = Some(pose);
    }

    fn the_station_record(
        sector: Option<&FrozenSector>,
        id: &str,
        cell: SectorCoord,
    ) -> FrozenBody {
        match sector {
            Some(FrozenSector::Visited(bodies)) => bodies
                .iter()
                .find(|body| body.id() == Some(id))
                .cloned()
                .unwrap_or_else(|| panic!("station round trip: {cell}'s record has no '{id}'")),
            other => panic!("station round trip: {cell} froze as {other:?}, not a visit"),
        }
    }

    /// The stock and storage a frozen station record holds.
    fn record_state(body: &FrozenBody) -> (BTreeMap<String, u32>, BTreeMap<String, u32>) {
        match body.body() {
            FrozenBodyType::Station(station) => (
                hold_map(station.ship().inventory()),
                storage_map(station.storage()),
            ),
            other => panic!("station round trip: the station froze as {other:?}"),
        }
    }

    /// Claim 2: the retired record holds the station as left.
    fn assert_the_retire(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<RoundTripWatch>();
        let cell = watch.cell.expect("chosen");
        let before = watch.before.expect("chosen");
        let left = watch.left.clone().expect("left");
        let left_pose = watch.left_pose.expect("left");
        assert!(
            !live_cells(world).contains(&cell),
            "station round trip: the station's cell {cell} is retired"
        );
        assert!(
            world.get_entity(before).is_err(),
            "station round trip: the retire despawned the station"
        );
        let body = the_station_record(world.resource::<FrozenSectors>().get(cell), &id, cell);
        let (stock, storage) = record_state(&body);
        assert_eq!(
            stock, left.stock,
            "station round trip: the record keeps the stock"
        );
        assert_eq!(
            storage, left.storage,
            "station round trip: the record keeps the storage"
        );
        let at = now(world);
        let pose = record_pose(&body, at);
        let (moved_m, turned) =
            assert_pose_near("the record", left_pose, pose, at - left_pose.at_secs);
        nova_probe::probe_marker(
            world,
            "outcome: the retired cell's record holds the station as left",
            serde_json::json!({
                "id": id,
                "cell": cell.to_string(),
                "stock": stock,
                "storage": storage,
                "moved_m": moved_m,
                "turned_rad": turned,
            }),
        );
        info!(
            "station round trip: sector {cell} retired with '{id}' frozen {moved_m:.2} m from \
             where it was left"
        );
        world.resource_mut::<RoundTripWatch>().record_pose = Some(pose);
    }

    /// The station entity's pose read now, against `reference` moved by its
    /// motion since the entity was added.
    fn assert_spawned_at(world: &World, label: &str, reference: Pose) -> (f32, f32) {
        let station = station(world);
        let pose = pose_of(world, station);
        let added = world
            .resource::<RoundTripWatch>()
            .added_at
            .get(&station)
            .copied()
            .unwrap_or_else(|| panic!("station round trip: {station:?} was added unseen"));
        assert_pose_near(label, reference, pose, pose.at_secs - added)
    }

    /// Claim 3: the thawed station is the station as left.
    fn assert_the_thaw(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<RoundTripWatch>();
        let before = watch.before.expect("chosen");
        let left = watch.left.clone().expect("left");
        let record = watch.record_pose.expect("retired");
        let thawed = station(world);
        assert_ne!(
            thawed, before,
            "station round trip: the thaw is a new entity"
        );
        let (moved_m, turned) = assert_spawned_at(world, "the thaw", record);
        let state = station_state(world);
        assert_eq!(
            state, left,
            "station round trip: the thaw is the station as left"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the thawed station is the station as left",
            serde_json::json!({
                "id": id,
                "credits": state.credits,
                "sections": state.sections,
                "moved_m": moved_m,
                "turned_rad": turned,
            }),
        );
        info!(
            "station round trip: '{id}' thawed {moved_m:.2} m from its record as left: {state:?}"
        );
    }

    /// Claim 4: the leave save holds the station as left and the player with
    /// the rebuilt section.
    fn assert_the_save(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<RoundTripWatch>();
        let cell = watch.cell.expect("chosen");
        let left = watch.left.clone().expect("left");
        let root = nova_assets::storage::worlds_root()
            .expect("station round trip: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| {
            panic!("station round trip: cannot open the leave save: {refusal}")
        });
        let body = the_station_record(state.sectors.get(cell), &id, cell);
        let (stock, storage) = record_state(&body);
        assert_eq!(
            stock, left.stock,
            "station round trip: the save keeps the stock"
        );
        assert_eq!(
            storage, left.storage,
            "station round trip: the save keeps the storage"
        );
        assert!(
            state.player.ship.has_section(LOST_SECTION),
            "station round trip: the saved player carries '{LOST_SECTION}'"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the station as left",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "stock": stock,
                "storage": storage,
            }),
        );
        info!(
            "station round trip: save generation {} (format {}) holds '{id}' as left",
            header.generation, header.format
        );
        world.resource_mut::<RoundTripWatch>().saved_pose = Some(record_pose(&body, 0.0));
    }

    /// Claim 5: the loaded world gives back the station as left and the
    /// rebuilt section.
    fn assert_the_load(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<RoundTripWatch>();
        let left = watch.left.clone().expect("left");
        let saved = watch.saved_pose.expect("saved");
        let (moved_m, turned) = assert_spawned_at(world, "the Load", saved);
        let state = station_state(world);
        assert_eq!(
            state, left,
            "station round trip: the Load is the station as left"
        );
        let player = player(world);
        let section = section_of(world, player, LOST_SECTION)
            .expect("station round trip: the loaded player has the rebuilt section");
        assert!(
            full_health(world, section),
            "station round trip: the loaded section is whole: {:?}",
            world.get::<Health>(section)
        );
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world gives back the station and the rebuilt section",
            serde_json::json!({
                "id": id,
                "credits": state.credits,
                "stock": state.stock,
                "storage": state.storage,
                "sections": state.sections,
                "moved_m": moved_m,
                "turned_rad": turned,
            }),
        );
        info!(
            "station round trip: PASS the loaded world holds the station as left ({moved_m:.2} m \
             from the save) and {LOST_SECTION} whole"
        );
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station round trip: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station round trip: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("station round trip: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station round trip: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "station round trip: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "station round trip: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station round trip: the window streams in around the start")
            .until(and(window_is(start_cell), section_coded()))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(and(
                window_is(|_| Some(SectorCoord::ORIGIN)),
                section_coded(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("station round trip: lose the section")
            .on_enter(lose_the_section)
            .until(section_lost())
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<RoundTripWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: record the balances")
            .on_enter(record_before_rebuild)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            when(|world| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            }),
        )
        .click_named(
            "station round trip: open the Ship pane",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        .step("station round trip: settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The scavenger's section tree is long, so the Rebuild row starts
        // below the panel: the wheel over the panel scrolls it into view.
        .step("station round trip: scroll the Rebuild button into view")
        .each(|world, _, frame| {
            let name = format!("ShipRebuild-{LOST_CODE}");
            keep_hovering_named(SHIP_PANEL)(world);
            if frame % 2 == 1 && !in_panel(world, &name) {
                let lines = if below_panel(world, &name) { -1.0 } else { 1.0 };
                scroll_lines(lines)(world);
            }
        })
        .until(when(|world| {
            in_panel(world, &format!("ShipRebuild-{LOST_CODE}"))
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("station round trip: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "station round trip: rebuild the section",
            &format!("ShipRebuild-{LOST_CODE}"),
            section_live(),
            SESSION_SECS,
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them once the clock runs again.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station round trip: the rebuild reports")
            .until(rebuild_reported())
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: assert the rebuild, and undock")
            .on_enter(assert_the_rebuild)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: record the station as left")
            .on_enter(record_the_station_as_left)
            .add()
            .step("station round trip: jump away until the station's cell retires")
            .on_enter(|world: &mut World| {
                let (edge, _) = world_shape(world);
                let away = world.resource::<RoundTripWatch>().away.expect("chosen");
                park_player(world, away.centre(edge));
            })
            .until(window_is(|world| world.resource::<RoundTripWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: assert the retired record")
            .on_enter(assert_the_retire)
            .add()
            .step("station round trip: jump back to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<RoundTripWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: assert the thaw")
            .on_enter(assert_the_thaw)
            .add()
            .step("station round trip: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("station round trip: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "station round trip: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("station round trip: assert the leave save")
            .on_enter(assert_the_save)
            .add()
            .click_named(
                "station round trip: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "station round trip: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "station round trip: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station round trip: the loaded window streams the station back")
            .until(and(
                window_is(|world| world.resource::<RoundTripWatch>().cell),
                and(the_station_stands(), section_live()),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station round trip: assert the loaded world")
            .on_enter(assert_the_load)
            .add()
    }
}
