//! system_role_goal_empire: an industrial's "A mining empire" charter starts
//! standing, counts each bought station against the stations line, reaches
//! met once two escorts are hired at the second station, keeps reading met
//! off the frozen ledger once both stations' cells retire, shows in the Map,
//! and survives the leave save and a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on seed 10,
//! picks `industrial`, and waits for the window to stream in around the
//! session's starting station, where the start docks the player, and for the
//! first save. The walk undocks and jumps to the origin, the spawn point
//! before the docked start: stations are a civilization apart, so no station
//! it wants streams in round the start. The active window around the origin
//! on this seed holds only one living, non-Enemy station, so the walk
//! scouts: it scans the current window for living, non-Enemy, non-derelict
//! stations outside the origin's cell, and
//! once it holds two with at least one that offers Refit and hires out two
//! escorts or more, it jumps a window's span along +X past the active
//! radius and scans again. The first distinct pair it completes, with the
//! hire-capable one as station B and any other as station A, ends the
//! scout. The player jumps beside each in turn and berths and docks the way
//! [`system_station_own`] does. A non-Enemy station a scout finds may still
//! stand short of Ally, which a sale refuses, so the walk warms each
//! station's civilization to Ally by merits ([`record_merit`], one per
//! handed-in job, [`MERIT_STEP`] merits per step) the way a player does by
//! handing in jobs, before it buys A through the Station pane's Buy station
//! button, flies to B, warms it the same way and buys it, then hires two
//! escorts at B through the Hire button the way [`system_station_hire`]
//! does.
//!
//! DISCLOSED FIXTURE STATE: once docked at A, the player holds 100,000,000
//! cr (same fixture as [`system_station_own`]), enough for both buys and
//! both hires. After the second hire the walk undocks, flies clear and jumps
//! three cells out, past the active radius, so both bought stations' cells
//! retire and freeze; it never jumps back, so every claim from there on
//! reads the empire off the frozen ledger, not a live station.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the empire charter opens standing with no station and no escort` | `GoalProgress` reads `A mining empire`, `Stations owned 0 / 2` and `Escorts in fleet 0 / 2`, not met, with no `GoalRecord` stamp |
//! | 2 | `outcome: two bought stations stand the charter at the escort line` | after both buys, `Stations owned 2 / 2` and `Escorts in fleet 0 / 2`, not met, still no stamp |
//! | 3 | `outcome: hiring the second escort meets the empire charter and stamps the record once` | the first hire reads `1 / 2` escorts, not met; the second reads `2 / 2` and `2 / 2`, met, with a stamp inside the play clock that does not move across two more frames |
//! | 4 | `outcome: a frozen station still counts toward the met empire charter` | after both stations' cells freeze, both frozen records are owned, both escorts still fly in the fleet, and the progress still reads `2 / 2`, `2 / 2`, met, with the same stamp |
//! | 5 | `outcome: the map's chart view shows the empire charter met` | its OBJECTIVES block shows one `Goal reached at` row in place of the milestones and no milestone row |
//! | 6 | `outcome: the leave save keeps the met empire charter` | `open_world` reads the same met stamp from the leave save |
//! | 7 | `outcome: a load restores the met empire charter` | after Load and a frame, the same stamp, met progress and both escorts in the fleet return |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_role_goal_empire --features debug
//! # look for: `role goal empire: PASS a load restores the met empire charter` and
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `role-goal-empire-charter.png` of the chart
//! view with the met OBJECTIVES block.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_role_goal_empire")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed as the industrial: warm two living non-Enemy stations' \
             civilizations to Ally by merits, buy both, hire two escorts at the second through \
             the Station pane, and watch the empire charter reach met, keep reading met off the \
             frozen ledger past a cell freeze, show in the Map, and survive a save and a Load. \
             Autopilot-only correctness range",
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
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        // No frame-time claim: the range holds markers, progress and stamps.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::EmpireWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::{pane::InterfacePaneType, station::STATION_BUY_BUTTON},
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBodyType, FrozenSectors, SectorRoot, WorldConfig,
    };

    use super::*;

    /// The seed the range plays: its window around the origin holds two
    /// living, non-Enemy stations outside the origin's cell, one of which
    /// hires out two escorts or more.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Role Goal Empire World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const START_ROLE_BUTTON: &str = "Start Role Button: industrial";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const MAP_TAB: &str = "InterfaceTabMap";

    /// The escort for hire the walk hires, twice: the first the station
    /// holds, which the second takes the place of once hired.
    const HIRE_OFFER: usize = 0;
    const HIRE_BUTTON: &str = "EscortHire-0";

    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";

    /// The disclosed fixture balance of the player.
    const FIXTURE_CREDITS: u32 = 100_000_000;

    /// The PNG the capture path shoots.
    const CHARTER_SHOT: &str = "role-goal-empire-charter.png";

    /// How far from a station the player parks before the berth, and how
    /// far it flies clear after each undock.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// How many cells the final crossing jumps: past the active radius, so
    /// both bought stations' cells retire.
    const CROSSING_CELLS: i32 = 3;

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds the scout gets to find a station pair across every hop.
    const SCOUT_DEADLINE_SECS: f32 = 900.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// A station the walk targets: its typed id, the cell it stands in, and
    /// where it stood when chosen, for the park point.
    #[derive(Clone, Debug)]
    struct TargetStation {
        id: GeneratedBodyId,
        cell: SectorCoord,
        position: Meters3,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct EmpireWatch {
        /// The first station the walk buys.
        a: Option<TargetStation>,
        /// The second station the walk buys and hires at.
        b: Option<TargetStation>,
        /// The two escorts hired at `b`, in hire order.
        escorts: Vec<GeneratedBodyId>,
        /// The cell the final crossing landed in.
        crossed: Option<SectorCoord>,
        /// The first stamp observed once the empire is met.
        met_at: Option<f64>,
        /// Every qualifying station the scout has found so far, across every
        /// window it has visited.
        discovered: Vec<Candidate>,
        /// The scout jumped and is waiting for the next window to settle.
        moving: bool,
        /// How many scouting hops the scout has taken.
        hops: u32,
    }

    impl EmpireWatch {
        fn station(&self, tag: &str) -> TargetStation {
            match tag {
                "a" => self.a.clone(),
                "b" => self.b.clone(),
                other => panic!("role goal empire: no such station tag '{other}'"),
            }
            .unwrap_or_else(|| panic!("role goal empire: station '{tag}' is chosen"))
        }
    }

    pub(super) struct EmpireWatchPlugin;

    impl Plugin for EmpireWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<EmpireWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>| {
                let name = event.name();
                if !matches!(
                    name,
                    "onstationbought" | "onescorthired" | "oncreditschanged"
                ) {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("role goal empire: {name} {data}");
            });
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

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("role goal empire: exactly one player ship")
    }

    fn world_armed(world: &World) -> bool {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && the_player(world).is_some()
    }

    fn saved_and_idle(world: &World) -> bool {
        world
            .get_resource::<WorldSaveSession>()
            .is_some_and(|session| {
                session.is_idle() && matches!(session.status(), WorldSaveStatus::Saved { .. })
            })
    }

    /// The entity wearing `id`, if one does.
    fn wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    fn owned(world: &World, station: Entity) -> bool {
        world.get::<OwnedStationMarker>(station).is_some()
    }

    /// The station tagged `tag` reads the player's own side: the world
    /// layer's `sync_civilization_sides` wrote its civilization's stance
    /// there.
    fn station_is_ally(tag: &'static str) -> Arc<Predicate> {
        when(move |world| {
            let id = world.resource::<EmpireWatch>().station(tag).id;
            wearing(world, &id)
                .and_then(|station| world.get::<Allegiance>(station))
                .is_some_and(|allegiance| *allegiance == Allegiance::Player)
        })
    }

    /// Warm the station tagged `tag`'s civilization to Ally the way a
    /// player does: one merit per handed-in job, [`MERIT_STEP`] merits per
    /// step. Nothing when it is Ally already.
    ///
    /// # Panics
    ///
    /// The civilization never reaches Ally within twice [`MERIT_STEP`]
    /// merits.
    fn warm_to_ally(world: &mut World, tag: &'static str) {
        let id = world.resource::<EmpireWatch>().station(tag).id;
        let station = wearing(world, &id).expect("role goal empire: the station stands");
        let civ = world
            .get::<TraderMarket>(station)
            .expect("role goal empire: the chosen station prices goods")
            .civilization;
        let now = world.resource::<WorldClock>().play_seconds;
        let seeded = seeded_stance(&civ);
        let mut stances = world.resource_mut::<CivilizationStances>();
        let mut merits = 0;
        while stances.effective(&civ, seeded, now) != StanceType::Ally {
            record_merit(&mut stances, civ, now);
            merits += 1;
            assert!(
                merits <= 2 * MERIT_STEP,
                "role goal empire: '{}' never warmed to Ally",
                civ.name()
            );
        }
        info!(
            "role goal empire: '{}' warmed to Ally after {merits} merit(s)",
            civ.name()
        );
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
            if !world_armed(world) {
                return false;
            }
            let Some(centre) = centre(world) else {
                return false;
            };
            let (_, radius) = world_shape(world);
            live_cells(world) == desired_sectors(centre, radius)
        })
    }

    /// The station tagged `tag` stands live, with its services settled.
    fn the_station_stands(tag: &'static str) -> Arc<Predicate> {
        when(move |world| {
            let id = world.resource::<EmpireWatch>().station(tag).id;
            wearing(world, &id).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<StationServices>(station).is_some()
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

    fn progress_is_met(world: &World) -> bool {
        world
            .get_resource::<GoalProgress>()
            .is_some_and(GoalProgress::is_met)
            && world
                .get_resource::<GoalRecord>()
                .is_some_and(GoalRecord::is_met)
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

    /// Whether the named node lies inside the Station panel's box, where a
    /// click lands on it.
    fn in_panel(world: &World, name: &str) -> bool {
        match (
            ui_node_rect(world, name),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
            _ => false,
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
        info!("role goal empire: the player jumps to {position:?}");
    }

    /// Jump the player to the station tagged `tag`'s side, [`PARK_DISTANCE`]
    /// from it toward the origin, in its own cell.
    fn jump_beside(world: &mut World, tag: &'static str) {
        let target = world.resource::<EmpireWatch>().station(tag);
        let toward_origin = (-target.position.get()).normalize_or_zero();
        let beside = Meters3(target.position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            target.cell,
            "role goal empire: the park point for station '{tag}' stands in its own cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("role goal empire: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("role goal empire: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("role goal empire: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("role goal empire: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station tagged `tag`'s collar,
    /// moving and turning with it, then ask to dock. Every frame until the
    /// dock takes, as [`system_station_own`] does.
    fn berth_and_dock(world: &mut World, tag: &'static str) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let id = world.resource::<EmpireWatch>().station(tag).id;
        let station = wearing(world, &id).expect("role goal empire: the station stands");
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let position = world.get::<Position>(station).expect("a station stands").0;
        let rotation = world.get::<Rotation>(station).expect("a station stands").0;
        let velocity = world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(station)
            .map_or(Vec3::ZERO, |w| w.0);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let (at, turn) =
            parked_hull_pose((position, rotation), (port_at, port_rot), (own_at, own_rot));
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        // Point the mouse rig where the berth points the hull, as a pilot who
        // flew in would. Left on the spawn heading, it turns the hull back
        // the moment it undocks and sweeps it through the station.
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

    /// Jump the undocked player [`PARK_DISTANCE`] straight out from the
    /// station tagged `tag`, at rest, as a pilot who flies off would stand.
    fn fly_clear(world: &mut World, tag: &'static str) {
        let id = world.resource::<EmpireWatch>().station(tag).id;
        let station = wearing(world, &id)
            .and_then(|station| world.get::<Position>(station))
            .map(|position| position.0)
            .unwrap_or_else(|| panic!("role goal empire: the station '{tag}' stands"));
        let player = player(world);
        let at = world.get::<Position>(player).expect("a body stands").0;
        let out = (at - station).normalize_or(Vec3::Y);
        world.entity_mut(player).insert((
            Position(at + out * PARK_DISTANCE.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("role goal empire: the player flies {PARK_DISTANCE:?} clear of station '{tag}'");
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("role goal empire: no field named '{name}'"));
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
            .step(format!("role goal empire: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("role goal empire: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the Station pane and let its lists settle.
    fn open_station_pane(
        script: AutopilotPlugin<GameStates>,
        label: &str,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key {label}"),
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            &format!("role goal empire: open the Station pane {label}"),
            "InterfaceTabStation",
            the_interface_shows(Some(InterfacePaneType::Station)),
            SESSION_SECS,
        )
        .step(format!("role goal empire: settle the lists {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    /// Scroll the Station panel under the wheel until `shown` lies inside
    /// its box, then let the scroll settle.
    fn scroll_into_view(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        shown: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!(
                "role goal empire: scroll the station block into view {label}"
            ))
            .each(move |world, _, frame| {
                keep_hovering_named(STATION_PANEL)(world);
                if frame % 2 == 1 && !in_panel(world, shown) {
                    scroll_lines(-1.0)(world);
                }
            })
            .until(when(move |world| in_panel(world, shown)))
            .deadline(SESSION_SECS)
            .add()
            .step(format!("role goal empire: settle the scroll {label}"))
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    /// A station the scout found: living, non-Enemy and non-derelict,
    /// outside the origin's cell, where the walk stands after its jump, with
    /// whether it offers Refit and hires out two escorts or more.
    #[derive(Clone, Debug)]
    struct Candidate {
        id: GeneratedBodyId,
        cell: SectorCoord,
        position: Meters3,
        hire_capable: bool,
    }

    /// How many scouting hops [`scout_step`] takes before it gives up.
    const MAX_SCOUT_HOPS: u32 = 12;

    /// The window the active radius streams, centred on wherever the player
    /// now stands, is fully materialized.
    fn window_settled_here(world: &World) -> bool {
        if !world_armed(world) {
            return false;
        }
        let (edge, radius) = world_shape(world);
        let Some(player) = the_player(world) else {
            return false;
        };
        let Some(at) = world.get::<Position>(player) else {
            return false;
        };
        let cell = SectorCoord::containing(Meters3::from_engine(at.0), edge);
        live_cells(world) == desired_sectors(cell, radius)
    }

    /// Record every living, non-Enemy, non-derelict station in the current
    /// window outside the origin's cell that the scout has not already
    /// discovered.
    fn scan_candidates(world: &mut World) {
        let (edge, _) = world_shape(world);
        let mut q_stations = world.query_filtered::<(
            Entity,
            &GeneratedBodyId,
            &Position,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
        ), With<StationMarker>>();
        let view: &World = world;
        let found: Vec<(GeneratedBodyId, Entity, Vec3)> = q_stations
            .iter(view)
            .filter(|(_, _, position, derelict, allegiance)| {
                !derelict
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(entity, id, position, ..)| (id.clone(), entity, position.0))
            .collect();
        for (id, entity, at) in found {
            if world
                .resource::<EmpireWatch>()
                .discovered
                .iter()
                .any(|candidate| candidate.id == id)
            {
                continue;
            }
            let hire_capable = station_offers(
                world.get::<StationServices>(entity),
                false,
                StationServiceType::Refit,
            ) && hires_offered(world, entity) >= 2;
            let position = Meters3::from_engine(at);
            let cell = SectorCoord::containing(position, edge);
            info!(
                "role goal empire: scout finds '{id}' in cell {cell}, hire_capable={hire_capable}"
            );
            world
                .resource_mut::<EmpireWatch>()
                .discovered
                .push(Candidate {
                    id,
                    cell,
                    position,
                    hire_capable,
                });
        }
    }

    /// Assign station A and station B once the discovered set holds two
    /// distinct stations with at least one hire-capable.
    fn try_finalize_stations(world: &mut World) {
        if world.resource::<EmpireWatch>().a.is_some() {
            return;
        }
        let discovered = world.resource::<EmpireWatch>().discovered.clone();
        let Some(b_index) = discovered.iter().position(|c| c.hire_capable) else {
            return;
        };
        let Some(a_index) = (0..discovered.len()).find(|&index| index != b_index) else {
            return;
        };
        let a = discovered[a_index].clone();
        let b = discovered[b_index].clone();
        info!(
            "role goal empire: chose A '{}' in cell {}; B '{}' in cell {}, hiring escorts",
            a.id, a.cell, b.id, b.cell
        );
        let mut watch = world.resource_mut::<EmpireWatch>();
        watch.a = Some(TargetStation {
            id: a.id,
            cell: a.cell,
            position: a.position,
        });
        watch.b = Some(TargetStation {
            id: b.id,
            cell: b.cell,
            position: b.position,
        });
    }

    /// Jump the player a window's span along +X, past the active radius of
    /// wherever it now stands, so the next window holds fresh cells.
    fn hop_further(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let span = edge.get() * (2 * radius + 3) as f32;
        let player = player(world);
        let at = world.get::<Position>(player).expect("a body stands").0;
        let offset = Vec3::X * Meters(span).to_engine();
        world.entity_mut(player).insert((
            Position(at + offset),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        let mut watch = world.resource_mut::<EmpireWatch>();
        watch.hops += 1;
        info!(
            "role goal empire: scout hop {} along +X, {span:.0} units",
            watch.hops
        );
    }

    /// One tick of the scout: wait for a window to settle, scan it, try to
    /// finalize the pair, and hop on if the pair is not yet found.
    ///
    /// # Panics
    ///
    /// No pair of stations, one hire-capable, turns up within
    /// [`MAX_SCOUT_HOPS`] hops.
    fn scout_step(world: &mut World) {
        if world.resource::<EmpireWatch>().moving {
            if window_settled_here(world) {
                world.resource_mut::<EmpireWatch>().moving = false;
            }
            return;
        }
        if !window_settled_here(world) {
            return;
        }
        scan_candidates(world);
        try_finalize_stations(world);
        if world.resource::<EmpireWatch>().a.is_some() {
            return;
        }
        let hops = world.resource::<EmpireWatch>().hops;
        assert!(
            hops < MAX_SCOUT_HOPS,
            "role goal empire: no pair of stations, one hire-capable, turned up within \
             {MAX_SCOUT_HOPS} scouting hops on seed {STATION_SEED}; discovered: {:?}",
            world.resource::<EmpireWatch>().discovered
        );
        hop_further(world);
        world.resource_mut::<EmpireWatch>().moving = true;
    }

    /// The escorts for hire `station` holds, in the order the list shows
    /// them.
    fn hire_hulls(world: &World, station: Entity) -> Vec<Entity> {
        world
            .get::<ParkedHulls>(station)
            .map_or_else(Vec::new, |parked| {
                parked
                    .iter()
                    .filter(|hull| {
                        matches!(
                            world.get::<ParkedHullType>(*hull),
                            Some(ParkedHullType::Hire { .. })
                        )
                    })
                    .collect()
            })
    }

    /// How many escorts for hire `station` holds or is about to.
    fn hires_offered(world: &World, station: Entity) -> usize {
        let pending = world
            .get::<PendingParkedHulls>(station)
            .map_or(0, |pending| {
                pending
                    .0
                    .iter()
                    .filter(|hull| match hull {
                        PendingParkedHull::Offer(offer) => offer.terms != HullOfferType::Sale,
                        PendingParkedHull::Frozen(frozen) => {
                            matches!(frozen.terms(), ParkedHullType::Hire { .. })
                        }
                    })
                    .count()
            });
        pending + hire_hulls(world, station).len()
    }

    /// The escort for hire at [`HIRE_OFFER`] of station B: the hull, its
    /// roster slot and its price.
    fn hire_offer(world: &World, station: Entity) -> (Entity, u32, u64) {
        let hull = *hire_hulls(world, station)
            .get(HIRE_OFFER)
            .expect("role goal empire: station B holds an escort for hire");
        let Some(&ParkedHullType::Hire { slot }) = world.get::<ParkedHullType>(hull) else {
            panic!("role goal empire: {hull:?} is not for hire");
        };
        let design = world
            .get::<ShipDesignSections>(hull)
            .expect("an escort for hire has its design");
        let price = escort_price_cr(world.resource::<GameItems>(), &design.0);
        (hull, slot, price)
    }

    /// Record the escort id the catalogue will hand the offer at
    /// [`HIRE_OFFER`] of station B, ahead of the click that hires it: the
    /// roster assigns the id to the offer's slot before the hire lands.
    fn record_pending_hire(world: &mut World) {
        let b_id = world.resource::<EmpireWatch>().station("b").id;
        let station = wearing(world, &b_id).expect("role goal empire: station B stands");
        let (hull, slot, price) = hire_offer(world, station);
        let id = escort_id(&b_id, slot);
        info!(
            "role goal empire: '{}' hires for {price} cr as '{id}'",
            world.get::<Name>(hull).map_or("", Name::as_str)
        );
        world.resource_mut::<EmpireWatch>().escorts.push(id);
    }

    /// Whether the Hire button lies inside the Station panel's box, where a
    /// click lands on it.
    fn hire_button_in_view(world: &World) -> bool {
        in_panel(world, HIRE_BUTTON)
    }

    /// The escort just recorded stands in the fleet.
    fn last_escort_in_fleet() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<EmpireWatch>()
                .escorts
                .last()
                .is_some_and(|id| wearing(world, id).is_some())
        })
    }

    /// The next escort stands at [`HIRE_OFFER`] once the first left station
    /// B, and its Hire button is in view.
    fn next_hire_offered() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<EmpireWatch>();
            let Some(first) = watch.escorts.first().cloned() else {
                return false;
            };
            let b_id = watch.station("b").id;
            let Some(station) = wearing(world, &b_id) else {
                return false;
            };
            hire_hulls(world, station)
                .get(HIRE_OFFER)
                .and_then(|hull| world.get::<ParkedHullType>(*hull))
                .is_some_and(|terms| match terms {
                    ParkedHullType::Hire { slot } => escort_id(&b_id, *slot) != first,
                    _ => false,
                })
                && hire_button_in_view(world)
        })
    }

    /// Every ship in the player's fleet.
    fn fleet(world: &World) -> Vec<Entity> {
        world
            .try_query_filtered::<Entity, With<PlayerFleetMarker>>()
            .map(|mut query| query.iter(world).collect())
            .unwrap_or_default()
    }

    /// Jump the player and its fleet [`CROSSING_CELLS`] cells out along +X,
    /// each keeping its velocity.
    fn jump_out(world: &mut World) {
        let (edge, _) = world_shape(world);
        let offset = Vec3::X * Meters(edge.get() * CROSSING_CELLS as f32).to_engine();
        let mut roots = vec![player(world)];
        roots.extend(fleet(world));
        for root in roots {
            let at = world.get::<Position>(root).expect("a body stands").0 + offset;
            world.entity_mut(root).insert(Position(at));
        }
        let cell = SectorCoord::containing(
            Meters3::from_engine(world.get::<Position>(player(world)).expect("stands").0),
            edge,
        );
        info!("role goal empire: the player and the fleet jump to cell {cell}");
        world.resource_mut::<EmpireWatch>().crossed = Some(cell);
    }

    /// Whether the station's record in `sectors` is owned; `None` when no
    /// record holds it as a station.
    fn frozen_owned(sectors: &FrozenSectors, id: &str) -> Option<bool> {
        sectors.iter().find_map(|(_, record)| {
            record
                .bodies()
                .iter()
                .find(|body| body.id() == Some(id))
                .and_then(|body| match body.body() {
                    FrozenBodyType::Station(station) => Some(station.owned()),
                    _ => None,
                })
        })
    }

    /// Both bought stations' cells are frozen behind the jump.
    fn both_cells_frozen() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<EmpireWatch>();
            [watch.station("a").cell, watch.station("b").cell]
                .into_iter()
                .all(|cell| world.resource::<FrozenSectors>().get(cell).is_some())
        })
    }

    /// Every UI text on screen.
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

    /// The charter's title and each line, with whether it reads met.
    fn read_progress(world: &World) -> (String, Vec<GoalLine>, bool) {
        let progress = world.resource::<GoalProgress>();
        (
            progress.title.clone(),
            progress.lines.clone(),
            progress.is_met(),
        )
    }

    /// The empire's two lines at `stations` and `escorts` of 2 each.
    fn expected_lines(stations: u64, escorts: u64) -> Vec<GoalLine> {
        vec![
            GoalLine::new("Stations owned", stations, 2),
            GoalLine::new("Escorts in fleet", escorts, 2),
        ]
    }

    /// Claim 1: the charter opens standing with no station and no escort.
    fn assert_standing_charter(world: &mut World) {
        let (title, lines, met) = read_progress(world);
        assert_eq!(title, "A mining empire", "role goal empire: charter title");
        assert_eq!(
            lines,
            expected_lines(0, 0),
            "role goal empire: the charter opens with zero progress"
        );
        assert!(!met, "role goal empire: the charter is standing");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goal empire: the standing charter has no stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the empire charter opens standing with no station and no escort",
            serde_json::json!({
                "title": title,
                "lines": ["Stations owned 0 / 2", "Escorts in fleet 0 / 2"],
                "met_at": None::<f64>,
            }),
        );
        info!("role goal empire: the empire charter opens standing with no station and no escort");
    }

    /// Claim 2: two bought stations stand the charter at the escort line.
    fn assert_two_bought(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(2, 0),
            "role goal empire: two bought stations count, no escort yet"
        );
        assert!(!met, "role goal empire: the charter still stands");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goal empire: no stamp before any escort is hired"
        );
        nova_probe::probe_marker(
            world,
            "outcome: two bought stations stand the charter at the escort line",
            serde_json::json!({ "lines": ["Stations owned 2 / 2", "Escorts in fleet 0 / 2"] }),
        );
        info!("role goal empire: two bought stations stand the charter at the escort line");
    }

    /// After the first hire: one escort counts, the charter still stands.
    fn assert_first_hire(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(2, 1),
            "role goal empire: the first hire counts one escort"
        );
        assert!(!met, "role goal empire: the charter still stands");
        info!("role goal empire: the first hire stands the charter at 1 / 2 escorts");
    }

    /// Claim 3: the second hire meets the empire charter and stamps the
    /// record once.
    fn assert_met_and_capture_stamp(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(2, 2),
            "role goal empire: both stations and both escorts count"
        );
        assert!(met, "role goal empire: the empire charter is met");
        let clock = world.resource::<WorldClock>().play_seconds;
        let met_at = world
            .resource::<GoalRecord>()
            .met_at
            .expect("role goal empire: a met charter has a stamp");
        assert!(
            met_at.is_finite() && met_at > 0.0 && met_at <= clock,
            "role goal empire: stamp {met_at} is within the positive play clock {clock}"
        );
        world.resource_mut::<EmpireWatch>().met_at = Some(met_at);
    }

    /// The met stamp is unchanged after two more frames.
    fn assert_stamp_stable(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(2, 2),
            "role goal empire: the empire charter stays met"
        );
        assert!(met, "role goal empire: the empire charter stays met");
        let expected = world
            .resource::<EmpireWatch>()
            .met_at
            .expect("role goal empire: the first stamp was captured");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal empire: the met record is stamped only once"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: hiring the second escort meets the empire charter and stamps the record once",
            serde_json::json!({ "lines": progress_lines, "met_at": expected }),
        );
        info!("role goal empire: PASS hiring the second escort meets the empire charter at {expected} s");
    }

    /// Claim 4: both frozen stations still count toward the met charter.
    fn assert_frozen_empire(world: &mut World) {
        let (ids, escorts) = {
            let watch = world.resource::<EmpireWatch>();
            (
                [watch.station("a").id, watch.station("b").id],
                watch.escorts.clone(),
            )
        };
        for id in &ids {
            assert!(
                wearing(world, id).is_none(),
                "role goal empire: station '{id}' retired with its cell"
            );
            assert_eq!(
                frozen_owned(world.resource::<FrozenSectors>(), &id.to_string()),
                Some(true),
                "role goal empire: the frozen station '{id}' record is owned"
            );
        }
        let watch_ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert_eq!(escorts.len(), 2, "role goal empire: two escorts were hired");
        for id in &escorts {
            assert!(
                wearing(world, id)
                    .is_some_and(|escort| world.get::<PlayerFleetMarker>(escort).is_some()),
                "role goal empire: escort '{id}' still flies in the fleet"
            );
        }
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(2, 2),
            "role goal empire: the frozen empire still reads met"
        );
        assert!(met, "role goal empire: the frozen empire still reads met");
        let expected = world
            .resource::<EmpireWatch>()
            .met_at
            .expect("role goal empire: the stamp was captured");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal empire: the frozen empire keeps its stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a frozen station still counts toward the met empire charter",
            serde_json::json!({ "stations": watch_ids, "met_at": expected }),
        );
        info!("role goal empire: a frozen station still counts toward the met empire charter");
    }

    /// Claim 5: the chart view renders the met charter block.
    fn assert_map_charter(world: &mut World) {
        let shown = shown_texts(world);
        let met_row = shown
            .iter()
            .find(|text| text.starts_with("Goal reached at "))
            .cloned()
            .unwrap_or_else(|| {
                panic!("role goal empire: the chart view shows its met stamp: {shown:?}")
            });
        assert!(
            named_node(world, "MapChartObjectiveMet"),
            "role goal empire: the OBJECTIVES block holds the met row"
        );
        assert!(
            !named_node(world, "MapChartObjectiveMilestone"),
            "role goal empire: a met role lists no milestone row: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view shows the empire charter met",
            serde_json::json!({ "block": "OBJECTIVES", "stamp": met_row }),
        );
        info!("role goal empire: the Map chart view shows the met empire charter: {met_row}");
    }

    /// Claim 6: read the leave save from disk and release its lock before
    /// the Load step.
    fn assert_leave_save(world: &mut World) {
        let expected = world
            .resource::<EmpireWatch>()
            .met_at
            .expect("role goal empire: the met stamp was captured");
        let root = nova_assets::storage::worlds_root()
            .expect("role goal empire: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("role goal empire: world name has a slug");
        let (_folder, lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| {
            panic!("role goal empire: cannot open the leave save: {refusal}")
        });
        assert_eq!(
            state.goal.met_at,
            Some(expected),
            "role goal empire: leave save keeps the met stamp"
        );
        assert!(
            state.goal.is_met(),
            "role goal empire: leave save records the empire charter met"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save keeps the met empire charter",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "met_at": state.goal.met_at,
            }),
        );
        info!("role goal empire: leave save keeps the empire charter met at {expected} s");
        drop(lock);
    }

    /// Claim 7: a load restores the met charter, its stamp and both escorts.
    fn assert_loaded_charter(world: &mut World) {
        let expected = world
            .resource::<EmpireWatch>()
            .met_at
            .expect("role goal empire: the met stamp was captured");
        let (title, lines, met) = read_progress(world);
        assert_eq!(title, "A mining empire", "role goal empire: loaded title");
        assert_eq!(
            lines,
            expected_lines(2, 2),
            "role goal empire: loaded lines"
        );
        assert!(met, "role goal empire: loaded progress is met");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal empire: Load restores the original stamp"
        );
        assert_eq!(
            fleet(world).len(),
            2,
            "role goal empire: both escorts are back in the fleet"
        );
        let escorts = world.resource::<EmpireWatch>().escorts.clone();
        for id in &escorts {
            assert!(
                wearing(world, id)
                    .is_some_and(|escort| world.get::<PlayerFleetMarker>(escort).is_some()),
                "role goal empire: escort '{id}' is back in the fleet"
            );
        }
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: a load restores the met empire charter",
            serde_json::json!({ "title": title, "lines": progress_lines, "met_at": expected }),
        );
        info!("role goal empire: PASS a load restores the met empire charter");
    }

    /// Jump to the station tagged `tag`, berth and dock.
    fn dock_at(
        script: AutopilotPlugin<GameStates>,
        tag: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("role goal empire: jump to station {tag}'s side"))
            .on_enter(move |world: &mut World| jump_beside(world, tag))
            .until(and(
                window_is(move |world| Some(world.resource::<EmpireWatch>().station(tag).cell)),
                the_station_stands(tag),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step(format!("role goal empire: berth and dock at {tag}"))
            .each(move |world, _, _| berth_and_dock(world, tag))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the Station pane, scroll the Buy button into view and buy the
    /// station tagged `tag`.
    fn buy_at(
        script: AutopilotPlugin<GameStates>,
        tag: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        let script = open_station_pane(script, &format!("to buy {tag}"));
        let script = scroll_into_view(script, &format!("to buy {tag}"), STATION_BUY_BUTTON);
        script.click_named(
            &format!("role goal empire: buy station {tag}"),
            STATION_BUY_BUTTON,
            when(move |world| {
                let id = world.resource::<EmpireWatch>().station(tag).id;
                wearing(world, &id).is_some_and(|station| owned(world, station))
            }),
            SESSION_SECS,
        )
    }

    /// Undock from the station tagged `tag` and fly clear of it. The caller
    /// closes the Station pane first if it is still open; this helper never
    /// toggles the interface, so it is safe to call with the pane already
    /// closed (as it is right after the second hire's goal-tracker wait).
    fn undock_and_clear(
        script: AutopilotPlugin<GameStates>,
        tag: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("role goal empire: undock from {tag}"))
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("role goal empire: fly clear of {tag}"))
            .on_enter(move |world: &mut World| fly_clear(world, tag))
            .until(frames(2))
            .add()
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("role goal empire: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "role goal empire: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("role goal empire: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "role goal empire: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "role goal empire: pick the industrial",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "role goal empire: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step(
                "role goal empire: the window streams in around the start and the first save lands",
            )
            .until(and(window_is(start_cell), when(saved_and_idle)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(and(
                window_is(|_| Some(SectorCoord::ORIGIN)),
                when(saved_and_idle),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: the empire charter opens standing")
            .on_enter(assert_standing_charter)
            .add()
            .step("role goal empire: scout for a station pair")
            .each(|world, _, _| scout_step(world))
            .until(when(|world| world.resource::<EmpireWatch>().a.is_some()))
            .deadline(SCOUT_DEADLINE_SECS)
            .add();
        let script = dock_at(script, "a")
            .step("role goal empire: set the fixture balance")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.get_mut::<ShipCredits>(player).expect("a balance").0 = FIXTURE_CREDITS;
                info!(
                    "role goal empire: fixture sets the player's balance to {FIXTURE_CREDITS} cr"
                );
            })
            .until(frames(1))
            .add()
            .step("role goal empire: warm station a's civilization to Ally")
            .on_enter(|world: &mut World| warm_to_ally(world, "a"))
            .until(station_is_ally("a"))
            .deadline(SESSION_SECS)
            .add();
        let script = buy_at(script, "a");
        let script = tap(script, "the interface key", "interface_toggle", unpaused());
        let script = undock_and_clear(script, "a");
        let script = dock_at(script, "b")
            .step("role goal empire: warm station b's civilization to Ally")
            .on_enter(|world: &mut World| warm_to_ally(world, "b"))
            .until(station_is_ally("b"))
            .deadline(SESSION_SECS)
            .add();
        let script = buy_at(script, "b");
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal empire: run one frame after both buys")
            .until(frames(1))
            .add()
            .step("role goal empire: two bought stations stand the charter")
            .on_enter(assert_two_bought)
            .add();
        let script = open_station_pane(script, "to hire");
        let script = scroll_into_view(script, "to hire", HIRE_BUTTON);
        let script = script
            .step("role goal empire: record the first hire")
            .on_enter(record_pending_hire)
            .until(frames(1))
            .add()
            .click_named(
                "role goal empire: hire the first escort",
                HIRE_BUTTON,
                last_escort_in_fleet(),
                SESSION_SECS,
            );
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal empire: the first hire reports")
            .until(frames(1))
            .add()
            .step("role goal empire: the first hire stands the charter")
            .on_enter(assert_first_hire)
            .add();
        let script = open_station_pane(script, "to hire the second");
        let script = scroll_into_view(script, "to hire the second", HIRE_BUTTON);
        let script = script
            .step("role goal empire: the next escort is offered")
            .until(next_hire_offered())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: record the second hire")
            .on_enter(record_pending_hire)
            .until(frames(1))
            .add()
            .click_named(
                "role goal empire: hire the second escort",
                HIRE_BUTTON,
                last_escort_in_fleet(),
                SESSION_SECS,
            );
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal empire: wait for the goal tracker")
            .until(when(progress_is_met))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal empire: check the first met stamp")
            .on_enter(assert_met_and_capture_stamp)
            .add()
            .step("role goal empire: hold the stamp for two frames")
            .until(frames(2))
            .add()
            .step("role goal empire: assert the stamp is stable")
            .on_enter(assert_stamp_stable)
            .add();
        let script = undock_and_clear(script, "b")
            .step("role goal empire: jump three cells out")
            .on_enter(jump_out)
            .until(and(
                window_is(|world| world.resource::<EmpireWatch>().crossed),
                both_cells_frozen(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: assert the frozen empire")
            .on_enter(assert_frozen_empire)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goal empire: open the Map pane",
            MAP_TAB,
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        );
        let script = tap(
            script,
            "the chart key",
            "map_chart",
            a_text_reads(|text| text.starts_with("Goal reached at ")),
        )
        .step("role goal empire: settle the charter block")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("role goal empire: assert the Map charter")
        .on_enter(assert_map_charter)
        .add()
        .step("role goal empire: shoot the charter block")
        .on_enter(|world: &mut World| shoot(world, CHARTER_SHOT))
        .until(shot_written(CHARTER_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add();
        let row = format!(
            "Load World Row: {}",
            world_slug(WORLD_NAME).expect("role goal empire: world name has a slug")
        );
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal empire: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal empire: release escape")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "role goal empire: leave to main menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("role goal empire: assert the leave save")
            .on_enter(assert_leave_save)
            .add()
            .click_named(
                "role goal empire: click Load",
                LOAD_BUTTON,
                ui_node_present(row.clone()),
                BEAT_SECS,
            )
            .click_named(
                "role goal empire: select the world",
                &row,
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "role goal empire: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goal empire: the loaded world arms")
            .until(when(world_armed))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal empire: let the goal tracker run after Load")
            .until(and(when(progress_is_met), frames(1)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal empire: assert the loaded charter")
            .on_enter(assert_loaded_charter)
            .add()
    }
}
