//! system_role_goal_flagship: a military start's flagship charter opens at
//! the patrol picket's own hull value, a bought hull worth the Line Warship
//! meets it, the Map shows it met, and the leave save and a Load keep it.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on seed 10,
//! picks `military`, and waits for the armed world and its first save. The
//! range checks the authored charter against the patrol picket's own hull
//! value and the Line Warship's recipe value, both recomputed live off the
//! loaded catalog, never hardcoded. The window streams in around the
//! session's starting station, where the start docks the player; the walk
//! undocks and jumps to the origin, the spawn point before the docked start,
//! because stations are a civilization apart and the station it wants never
//! streams in round the start. Once the window stands round the origin it
//! picks the nearest living, non-Enemy station that offers Refit outside the
//! origin's cell, as DISCLOSED FIXTURE STATE: every
//! hull the station parks, for sale or for hire, is despawned, and a single
//! Sale offer of a "Fixture Line Warship" (the loaded `line_warship` design,
//! inline) is handed to the station in its place. Once the fixture is
//! parked and docked, the player berths and docks, both balances are set to
//! 100,000,000 cr (also disclosed fixture state), and a Station pane buy
//! swaps the player's hull for it. The range checks the one-time play-clock
//! stamp, reads the met charter in the Map's chart view, leaves to the main
//! menu, reads the save through [`open_world`], then loads it again and
//! checks the live progress.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the flagship charter opens standing at the picket's value` | the charter reads the live picket's hull value against the Line Warship's recipe value, both recomputed, the picket short, the goal short of met and no stamp |
//! | 2 | `outcome: buying a hull worth the line warship meets the flagship charter and stamps the record once` | buying the fixture lifts the player's live hull value to the Line Warship's, meets the charter, and the record keeps its first stamp across two more frames |
//! | 3 | `outcome: the map's chart view shows the flagship charter met` | its OBJECTIVES block shows one `Met at` row in place of the milestones and no milestone row |
//! | 4 | `outcome: the leave save keeps the met flagship charter` | `open_world` reads the same met stamp from the leave save |
//! | 5 | `outcome: a load restores the met flagship charter` | after Load and a frame, the same stamp and met progress return |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_role_goal_flagship --features debug
//! # look for: `role goal flagship: PASS a load restores the met flagship charter` and
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `role-goal-flagship-charter.png` of the chart
//! view with the met OBJECTIVES block.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_role_goal_flagship")]
#[command(version = "1.0.0")]
#[command(
    about = "Start the military role, meet its flagship charter by buying a hull worth the Line \
             Warship, read it in the Map, and check it through a save and a Load. \
             Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

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

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<walk::GoalWatch>();
        app.add_plugins(walk::script());
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::pane::InterfacePaneType,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{desired_sectors, SectorRoot, WorldClock, WorldConfig};

    use super::*;

    const WORLD_NAME: &str = "Role Goal Flagship World";
    const STATION_SEED: u32 = 10;
    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const SEED_FIELD: &str = "World Seed Field";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const START_ROLE_BUTTON: &str = "Start Role Button: military";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const MAP_TAB: &str = "InterfaceTabMap";

    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";

    const SESSION_SECS: f32 = 240.0;
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// How long the fixture offer is given to park and dock before the range
    /// gives up. Not the docking owner's deadline: if this fires, read
    /// `park_pending_hulls`'s log for a port-fit warning before touching
    /// anything.
    const FIXTURE_DEADLINE_SECS: f32 = 60.0;
    /// The frame the capture path writes of the chart view's OBJECTIVES block.
    const CHARTER_SHOT: &str = "role-goal-flagship-charter.png";

    /// The military role's flagship goal hull
    /// (`start_roles.rs`'s `RoleGoalType::Flagship { hull: LINE_WARSHIP_SHIP_ID }`).
    const LINE_WARSHIP_HULL_ID: &str = "line_warship";
    /// The disclosed fixture hull parked in place of the station's generated
    /// offers.
    const FIXTURE_HULL_NAME: &str = "Fixture Line Warship";
    /// The disclosed fixture balance of the player and of the station: well
    /// over any generated hull's full price.
    const FIXTURE_CREDITS: u32 = 100_000_000;

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// What the run recorded between beats.
    #[derive(Resource, Default)]
    pub(super) struct GoalWatch {
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The Line Warship's recipe value, recomputed once.
        need_cr: Option<u64>,
        /// The player's docking connection before the buy.
        before_connection: Option<Entity>,
        /// The Buy button the next click aims at.
        button: Option<String>,
        /// The first stamp observed after the fixture meets the charter.
        met_at: Option<f64>,
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player_stands(world: &World) -> bool {
        let Some(mut players) = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        else {
            return false;
        };
        let mut players = players.iter(world);
        players.next().is_some() && players.next().is_none()
    }

    fn world_armed(world: &World) -> bool {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && player_stands(world)
    }

    fn saved_and_idle(world: &World) -> bool {
        world
            .get_resource::<WorldSaveSession>()
            .is_some_and(|session| {
                session.is_idle() && matches!(session.status(), WorldSaveStatus::Saved { .. })
            })
    }

    fn armed_and_saved() -> Arc<Predicate> {
        when(|world| world_armed(world) && saved_and_idle(world))
    }

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("role goal flagship: exactly one player ship")
    }

    /// The entity wearing the chosen station's id, if one does.
    fn station_root(world: &World) -> Option<Entity> {
        let id = world.resource::<GoalWatch>().id.clone()?;
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("role goal flagship: the station stands")
    }

    /// Whether `ship` is docked with `station`.
    fn docked_with(world: &World, ship: Entity, station: Entity) -> bool {
        world.get::<DockedShip>(ship).is_some_and(|docked| {
            world
                .get::<DockingConnection>(docked.connection)
                .is_some_and(|connection| connection.joins(station))
        })
    }

    /// Every hull `station` lists, in park order: the docked hulls and the
    /// escorts for hire coasting beside it.
    fn listed_hulls(world: &World, station: Entity) -> Vec<Entity> {
        world
            .get::<ParkedHulls>(station)
            .map_or_else(Vec::new, |parked| parked.iter().collect())
    }

    /// The hulls docked at `station` for sale or owned, in park order: the
    /// Station pane's hull rows.
    fn parked_hulls(world: &World, station: Entity) -> Vec<Entity> {
        listed_hulls(world, station)
            .into_iter()
            .filter(|hull| {
                !matches!(
                    world.get::<ParkedHullType>(*hull),
                    Some(ParkedHullType::Hire { .. })
                )
            })
            .collect()
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

    /// The station stands with its hulls parked and every one docked with it.
    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| {
                let parked = parked_hulls(world, station);
                world.get::<Position>(station).is_some()
                    && world.get::<PendingParkedHulls>(station).is_none()
                    && !parked.is_empty()
                    && parked.iter().all(|hull| docked_with(world, *hull, station))
            })
        })
    }

    /// A fixture hull named [`FIXTURE_HULL_NAME`], listed for sale, is
    /// docked with the station: the disclosed offer parked and took.
    fn fixture_hull_docked() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| {
                world.get::<PendingParkedHulls>(station).is_none()
                    && listed_hulls(world, station).into_iter().any(|hull| {
                        world.get::<Name>(hull).map(Name::as_str) == Some(FIXTURE_HULL_NAME)
                            && world.get::<ParkedHullType>(hull).copied()
                                == Some(ParkedHullType::Sale)
                            && docked_with(world, hull, station)
                    })
            })
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

    /// The buy's swap took and the player redocked through a new connection.
    fn swapped_and_redocked() -> Arc<Predicate> {
        when(|world| {
            let Some(before) = world.resource::<GoalWatch>().before_connection else {
                return false;
            };
            let (Some(player), Some(station)) = (the_player(world), station_root(world)) else {
                return false;
            };
            world
                .get::<DockedShip>(player)
                .is_some_and(|docked| docked.connection != before)
                && docked_with(world, player, station)
        })
    }

    /// The player's ship took the fixture's name: the swap itself happened,
    /// ahead of the redock, which needs the sim running again.
    fn swap_happened() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| {
                world.get::<Name>(player).map(Name::as_str) == Some(FIXTURE_HULL_NAME)
            })
        })
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

    fn progress_is_met(world: &World) -> bool {
        world
            .get_resource::<GoalProgress>()
            .is_some_and(GoalProgress::is_met)
            && world
                .get_resource::<GoalRecord>()
                .is_some_and(GoalRecord::is_met)
    }

    /// Whether a node carries the `Name` `name`.
    fn named_node(world: &mut World, name: &str) -> bool {
        world
            .query::<&Name>()
            .iter(world)
            .any(|named| named.as_str() == name)
    }

    fn shown_texts(world: &mut World) -> Vec<String> {
        world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect()
    }

    fn a_text_reads(check: impl Fn(&str) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Text>()
                .is_some_and(|mut query| query.iter(world).any(|text| check(&text.0)))
        })
    }

    fn set_field(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("role goal flagship: no field named '{name}'"));
        value.0 = text.to_string();
    }

    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("role goal flagship: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("role goal flagship: release {label}"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Pick the nearest station that is living, not on the Enemy side,
    /// offers Refit, parks hulls for sale, and stands outside the origin's
    /// cell, where the walk stands after its jump.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                Option<&StationServices>,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
                Has<PendingParkedHulls>,
                Has<ParkedHulls>,
            ), With<StationMarker>>()
            .iter(world)
            .filter(
                |(_, position, services, derelict, allegiance, pending, parked)| {
                    station_offers(*services, *derelict, StationServiceType::Refit)
                        && (*pending || *parked)
                        && *allegiance != Some(&Allegiance::Enemy)
                        && SectorCoord::containing(Meters3::from_engine(position.0), edge)
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
        let Some((distance, id, at)) = stations.into_iter().next() else {
            panic!(
                "role goal flagship: the window on seed {STATION_SEED} holds no living, \
                 non-Enemy station with Refit and parked hulls outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!("role goal flagship: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<GoalWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
    }

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("role goal flagship: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<GoalWatch>();
        let position = watch
            .position
            .expect("role goal flagship: the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch
                .cell
                .expect("role goal flagship: the station is chosen"),
            "role goal flagship: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// The live docking ports mounted on `root`.
    fn ports_of(world: &mut World, root: Entity) -> Vec<Entity> {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (With<DockingSectionMarker>, Without<SectionInactiveMarker>)>(
            )
            .iter(world)
            .collect();
        ports
            .into_iter()
            .filter(|port| mounted_on(world, *port, root))
            .collect()
    }

    fn mounted_on(world: &World, section: Entity, root: Entity) -> bool {
        let mut at = section;
        while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
            if parent == root {
                return true;
            }
            at = parent;
        }
        false
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("role goal flagship: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("role goal flagship: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The free, live docking ports mounted on `root`, lowest first.
    fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
        let mut ports: Vec<Entity> = ports_of(world, root)
            .into_iter()
            .filter(|port| world.get::<DockedPort>(*port).is_none())
            .collect();
        ports.sort();
        ports
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("role goal flagship: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull.
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
            .unwrap_or_else(|| panic!("role goal flagship: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's free port, moving and
    /// turning with the station, then ask to dock. Every frame until the
    /// dock takes.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let position = world
            .get::<Position>(station)
            .expect("role goal flagship: a station stands")
            .0;
        let rotation = world
            .get::<Rotation>(station)
            .expect("role goal flagship: a station stands")
            .0;
        let velocity = world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(station)
            .map_or(Vec3::ZERO, |w| w.0);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let station_face_rot = rotation * port_rot;
        let station_face = position + rotation * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
        let at = station_face + station_axis * BERTH_GAP - face_offset;
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
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
            entity: player,
            target: station,
        });
    }

    /// Despawn every hull the chosen station lists, for sale or for hire:
    /// DISCLOSED FIXTURE STATE clearing the way for the fixture offer.
    fn despawn_parked_hulls(world: &mut World) {
        let station = station(world);
        let hulls = listed_hulls(world, station);
        let ports = ports_of(world, station).len();
        info!(
            "role goal flagship: despawning {} parked hulls ({ports} docking ports total)",
            hulls.len()
        );
        for hull in hulls {
            world.entity_mut(hull).despawn();
        }
    }

    /// Hand the station a single Sale offer of the loaded `line_warship`
    /// design, named [`FIXTURE_HULL_NAME`]: DISCLOSED FIXTURE STATE.
    fn insert_fixture_offer(world: &mut World) {
        let station = station(world);
        let hull_id = ShipDesignId::from(LINE_WARSHIP_HULL_ID);
        let design = world
            .resource::<GameShipDesigns>()
            .get_design(&hull_id)
            .unwrap_or_else(|| {
                panic!("role goal flagship: no loaded ship design '{LINE_WARSHIP_HULL_ID}'")
            })
            .design
            .clone();
        world
            .entity_mut(station)
            .insert(PendingParkedHulls(vec![PendingParkedHull::Offer(
                StationHullOffer {
                    name: FIXTURE_HULL_NAME.to_string(),
                    terms: HullOfferType::Sale,
                    design,
                },
            )]));
        info!("role goal flagship: the station takes on the fixture '{FIXTURE_HULL_NAME}'");
    }

    /// The row index of the fixture hull among the Station pane's rows (Sale
    /// and Owned, in park order): names its Buy button.
    fn fixture_row(world: &World, station: Entity) -> usize {
        parked_hulls(world, station)
            .into_iter()
            .enumerate()
            .find(|(_, hull)| world.get::<Name>(*hull).map(Name::as_str) == Some(FIXTURE_HULL_NAME))
            .map(|(index, _)| index)
            .unwrap_or_else(|| panic!("role goal flagship: the fixture hull is not parked"))
    }

    /// Disclosed fixture state: both balances set to [`FIXTURE_CREDITS`].
    /// Then aim the next click at the fixture's Buy button and record the
    /// player's docking connection before the buy.
    fn record_before_buy(world: &mut World) {
        let player = player(world);
        let station = station(world);
        world
            .get_mut::<ShipCredits>(player)
            .expect("role goal flagship: a balance")
            .0 = FIXTURE_CREDITS;
        world
            .get_mut::<ShipCredits>(station)
            .expect("role goal flagship: a balance")
            .0 = FIXTURE_CREDITS;
        info!("role goal flagship: fixture sets both balances to {FIXTURE_CREDITS} cr");
        let row = fixture_row(world, station);
        let connection = world
            .get::<DockedShip>(player)
            .expect("role goal flagship: the player is docked")
            .connection;
        let mut watch = world.resource_mut::<GoalWatch>();
        watch.button = Some(format!("HullBuy-{row}"));
        watch.before_connection = Some(connection);
        info!("role goal flagship: row {row} is the fixture; the buy aims at it");
    }

    fn the_button(world: &World) -> String {
        world
            .resource::<GoalWatch>()
            .button
            .clone()
            .expect("role goal flagship: a Buy button is aimed at")
    }

    /// Whether the aimed Buy button lies inside the Station panel's box,
    /// where a click lands on it.
    fn buy_button_in_view(world: &World) -> bool {
        match (
            ui_node_rect(world, &the_button(world)),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(button), Some(panel)) => {
                button.min.y >= panel.min.y && button.max.y <= panel.max.y
            }
            _ => false,
        }
    }

    /// Open the interface on the Station pane and scroll the aimed Buy
    /// button into view.
    fn open_the_hull_list(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goal flagship: open the Station pane",
            "InterfaceTabStation",
            the_interface_shows(Some(InterfacePaneType::Station)),
            SESSION_SECS,
        )
        .step("role goal flagship: settle the lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("role goal flagship: scroll the hull list into view")
        .each(|world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !buy_button_in_view(world) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(buy_button_in_view))
        .deadline(SESSION_SECS)
        .add()
        .step("role goal flagship: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    /// Click the Buy button [`GoalWatch::button`] names, then wait for the
    /// swap itself: a board or buy swaps the design in the click's own
    /// command, ahead of the redock, which needs the sim running again.
    fn click_the_button(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("role goal flagship: the Buy button is up")
            .until(when(|world| {
                ui_node_rect(world, &the_button(world)).is_some()
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal flagship: aim at the Buy button")
            .on_enter(|world: &mut World| hover_named(the_button(world))(world))
            .each(|world, _, _| keep_hovering_named(the_button(world))(world))
            .until(when(|world| pointer_over_node(the_button(world))(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal flagship: press the Buy button")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step("role goal flagship: release the Buy button")
            .on_enter(release_mouse(MouseButton::Left))
            .until(swap_happened())
            .deadline(SESSION_SECS)
            .add()
    }

    /// The Line Warship's recipe value at ask, recomputed off the loaded
    /// catalog: never hardcoded.
    fn flagship_need_cr(world: &World) -> u64 {
        let designs = world.resource::<GameShipDesigns>();
        let sections = world.resource::<GameSections>();
        let (resolved, errors) = resolve_ship_design(
            &ShipDesignSource::prototype(LINE_WARSHIP_HULL_ID),
            designs,
            sections,
        );
        assert!(
            errors.is_empty(),
            "role goal flagship: the line warship design resolves: {errors:?}"
        );
        hull_price_cr(
            world.resource::<GameItems>(),
            resolved
                .sections
                .iter()
                .map(|section| &section.config.base.build),
        )
    }

    /// The player's live hull value at ask, read the way the goal tracker
    /// does: every immediate child's [`SectionBuildConfig`].
    fn player_hull_value_cr(world: &World) -> u64 {
        let player = player(world);
        let children = world
            .get::<Children>(player)
            .expect("role goal flagship: the player has children");
        hull_price_cr(
            world.resource::<GameItems>(),
            children
                .iter()
                .filter_map(|child| world.get::<SectionBuildConfig>(child))
                .map(|config| &config.0.base.build),
        )
    }

    /// Claim 1: the charter opens standing at the picket's own value.
    fn assert_opening_charter(world: &mut World) {
        let need = flagship_need_cr(world);
        let have = player_hull_value_cr(world);
        assert!(
            have < need,
            "role goal flagship: the picket ({have} cr) is worth less than the line warship \
             ({need} cr)"
        );
        assert!(
            need > 0,
            "role goal flagship: the line warship has a positive value"
        );
        let progress = world.resource::<GoalProgress>();
        assert_eq!(
            progress.title, "The Line Warship",
            "role goal flagship: charter title"
        );
        assert_eq!(
            progress.lines,
            vec![GoalLine::new("Hull value, cr", have, need)],
            "role goal flagship: charter lines"
        );
        assert!(
            !progress.is_met(),
            "role goal flagship: the charter is standing"
        );
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goal flagship: the standing charter has no stamp"
        );
        world.resource_mut::<GoalWatch>().need_cr = Some(need);
        nova_probe::probe_marker(
            world,
            "outcome: the flagship charter opens standing at the picket's value",
            serde_json::json!({ "have": have, "need": need }),
        );
        info!("role goal flagship: the flagship charter opens at {have} / {need} cr");
    }

    /// Check the tracker right after it has observed the fixture buy.
    fn assert_met_and_capture_stamp(world: &mut World) {
        let need = world
            .resource::<GoalWatch>()
            .need_cr
            .expect("role goal flagship: the need value was captured");
        let have = player_hull_value_cr(world);
        assert_eq!(
            have, need,
            "role goal flagship: the bought hull is worth the line warship"
        );
        let progress = world.resource::<GoalProgress>();
        assert_eq!(
            progress.title, "The Line Warship",
            "role goal flagship: charter title"
        );
        assert_eq!(
            progress.lines,
            vec![GoalLine::new("Hull value, cr", need, need)],
            "role goal flagship: charter lines"
        );
        assert!(
            progress.is_met(),
            "role goal flagship: the flagship charter is met"
        );
        let clock = world.resource::<WorldClock>().play_seconds;
        let met_at = world
            .resource::<GoalRecord>()
            .met_at
            .expect("role goal flagship: a met charter has a stamp");
        assert!(
            met_at.is_finite() && met_at > 0.0 && met_at <= clock,
            "role goal flagship: stamp {met_at} is within the positive play clock {clock}"
        );
        world.resource_mut::<GoalWatch>().met_at = Some(met_at);
    }

    /// The met stamp is unchanged after two more frames; emit claim 2.
    fn assert_stamp_stable(world: &mut World) {
        let need = world
            .resource::<GoalWatch>()
            .need_cr
            .expect("role goal flagship: the need value was captured");
        let (title, lines, met) = {
            let progress = world.resource::<GoalProgress>();
            (
                progress.title.clone(),
                progress.lines.clone(),
                progress.is_met(),
            )
        };
        assert_eq!(
            title, "The Line Warship",
            "role goal flagship: charter title"
        );
        assert_eq!(
            lines,
            vec![GoalLine::new("Hull value, cr", need, need)],
            "role goal flagship: charter lines stay met"
        );
        assert!(met, "role goal flagship: the flagship charter stays met");
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goal flagship: the first stamp was captured");
        let found = world.resource::<GoalRecord>().met_at;
        assert_eq!(
            found,
            Some(expected),
            "role goal flagship: the met record is stamped only once"
        );
        nova_probe::probe_marker(
            world,
            "outcome: buying a hull worth the line warship meets the flagship charter and stamps the record once",
            serde_json::json!({ "have": need, "need": need, "met_at": expected }),
        );
        info!(
            "role goal flagship: PASS buying a hull worth the line warship meets the flagship \
             charter at {expected} s"
        );
    }

    /// Claim 3: the chart view renders the met charter block.
    fn assert_map_charter(world: &mut World) {
        let shown = shown_texts(world);
        let met_row = shown
            .iter()
            .find(|text| text.starts_with("Met at "))
            .cloned()
            .unwrap_or_else(|| {
                panic!("role goal flagship: the chart view shows its met stamp: {shown:?}")
            });
        assert!(
            named_node(world, "MapChartObjectiveMet"),
            "role goal flagship: the OBJECTIVES block holds the met row"
        );
        assert!(
            !named_node(world, "MapChartObjectiveMilestone"),
            "role goal flagship: a met role lists no milestone row: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view shows the flagship charter met",
            serde_json::json!({ "block": "OBJECTIVES", "stamp": met_row }),
        );
        info!("role goal flagship: the Map chart view shows the met flagship charter: {met_row}");
    }

    /// Claim 4: read the leave save from disk and release its lock.
    fn assert_leave_save(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goal flagship: the met stamp was captured");
        let root = nova_assets::storage::worlds_root()
            .expect("role goal flagship: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("role goal flagship: world name has a slug");
        let (_folder, lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| {
            panic!("role goal flagship: cannot open the leave save: {refusal}")
        });
        assert_eq!(
            state.goal.met_at,
            Some(expected),
            "role goal flagship: leave save keeps the met stamp"
        );
        assert!(
            state.goal.is_met(),
            "role goal flagship: leave save records the charter met"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save keeps the met flagship charter",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "met_at": state.goal.met_at,
            }),
        );
        info!("role goal flagship: leave save keeps the flagship charter met at {expected} s");
        drop(lock);
    }

    /// Claim 5: a load restores the met charter.
    fn assert_loaded_charter(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goal flagship: the met stamp was captured");
        let need = world
            .resource::<GoalWatch>()
            .need_cr
            .expect("role goal flagship: the need value was captured");
        let (title, lines, met) = {
            let progress = world.resource::<GoalProgress>();
            (
                progress.title.clone(),
                progress.lines.clone(),
                progress.is_met(),
            )
        };
        assert_eq!(
            title, "The Line Warship",
            "role goal flagship: loaded title"
        );
        assert_eq!(
            lines,
            vec![GoalLine::new("Hull value, cr", need, need)],
            "role goal flagship: loaded lines"
        );
        assert!(met, "role goal flagship: loaded progress is met");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal flagship: Load restores the original stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a load restores the met flagship charter",
            serde_json::json!({ "title": title, "need": need, "met_at": expected }),
        );
        info!("role goal flagship: PASS a load restores the met flagship charter at {expected} s");
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("role goal flagship: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "role goal flagship: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("role goal flagship: name the world and enter seed 10")
            .on_enter(|world: &mut World| {
                set_field(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "role goal flagship: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "role goal flagship: pick military",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "role goal flagship: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goal flagship: the world arms and saves")
            .until(armed_and_saved())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: assert the opening charter")
            .on_enter(assert_opening_charter)
            .add()
            .step("role goal flagship: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("role goal flagship: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<GoalWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: despawn the station's parked hulls")
            .on_enter(despawn_parked_hulls)
            .until(frames(1))
            .add()
            .step("role goal flagship: hand the station the fixture line warship")
            .on_enter(insert_fixture_offer)
            .until(fixture_hull_docked())
            .deadline(FIXTURE_DEADLINE_SECS)
            .add()
            .step("role goal flagship: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: settle the dock")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("role goal flagship: set the fixture balances and aim the buy")
            .on_enter(record_before_buy)
            .until(frames(1))
            .add();
        let script = open_the_hull_list(script);
        let script = click_the_button(script);
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal flagship: the bought hull redocks")
            .until(swapped_and_redocked())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: wait for the flagship charter to meet")
            .until(when(progress_is_met))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal flagship: check the first met stamp")
            .on_enter(assert_met_and_capture_stamp)
            .add()
            .step("role goal flagship: hold the stamp for two frames")
            .until(frames(2))
            .add()
            .step("role goal flagship: assert the stamp is stable")
            .on_enter(assert_stamp_stable)
            .add();
        let script = tap(
            script,
            "interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goal flagship: open Map pane",
            MAP_TAB,
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        );
        let script = tap(
            script,
            "chart key",
            "map_chart",
            a_text_reads(|text| text.starts_with("Met at ")),
        )
        .step("role goal flagship: settle the charter block")
        .until(frames(6))
        .add()
        .step("role goal flagship: assert the Map charter")
        .on_enter(assert_map_charter)
        .add()
        .step("role goal flagship: shoot the charter block")
        .on_enter(|world: &mut World| shoot(world, CHARTER_SHOT))
        .until(shot_written(CHARTER_SHOT))
        .deadline(BEAT_SECS)
        .add();
        let script = tap(script, "interface key", "interface_toggle", unpaused())
            .step("role goal flagship: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal flagship: release Escape")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "role goal flagship: leave to main menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("role goal flagship: assert the leave save")
            .on_enter(assert_leave_save)
            .add()
            .click_named(
                "role goal flagship: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("role goal flagship: world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "role goal flagship: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("role goal flagship: world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "role goal flagship: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goal flagship: wait for the loaded world")
            .until(when(world_armed))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal flagship: let the goal tracker run after Load")
            .until(and(when(progress_is_met), frames(1)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal flagship: assert the loaded charter")
            .on_enter(assert_loaded_charter)
            .add();
        script
    }
}
