//! system_role_goal_yard: a scavenger's "A yard of your own" charter starts
//! standing, counts a bought station even empty, reaches met once every part
//! family is stocked, keeps reading met off the frozen ledger once the
//! station's cell retires, shows in the Map, and survives the leave save and
//! a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on seed 10,
//! picks `scavenger`, and waits for the window to stream in around the
//! session's starting station, where the start docks the player, and for the
//! first save. The walk undocks and jumps to the origin, the spawn point
//! before the docked start: stations are a civilization apart, so the
//! station it wants never streams in round the start. Once the window stands
//! round the origin it picks the nearest living Neutral station outside the
//! origin's cell, jumps beside it and berths and docks the way
//! [`system_station_own`] does. A
//! Neutral station still refuses a sale, so the walk warms its civilization
//! to Ally by merits ([`record_merit`], one per handed-in job,
//! [`MERIT_STEP`] merits per step) the way a player does by handing in
//! jobs, then buys it through the Station pane's Buy station button.
//!
//! DISCLOSED FIXTURE STATE: once bought, the player holds 100,000,000 cr, so
//! the station is affordable (same fixture as [`system_station_own`]). Once
//! docked and owned, for each of the seven ship-part families the walk finds
//! the lightest loaded [`GameSections`] section of that family by
//! non-empty [`BuildRecipe`] mass and [`StationStorage::add`]s every part of
//! its recipe, at its recipe count, into the bought station's storage: the
//! same store the Station pane's buy and sell panels read. A generated
//! station's store holds 1 t at advancement 0 (4 t at advancement 1), so the
//! lightest yard fits. The walk then undocks,
//! flies clear and jumps three cells out, past the active radius, so the
//! station's cell retires and freezes; it never jumps back, so every claim
//! from there on reads the yard off the frozen ledger, not a live station.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the yard charter opens standing with no owned station` | `GoalProgress` reads `A yard of your own`, `Stations owned 0 / 1` and `Part families stocked 0 / 7`, not met, with no `GoalRecord` stamp |
//! | 2 | `outcome: a bought station counts as the yard with nothing stocked` | after the buy, `Stations owned 1 / 1` and `Part families stocked 0 / 7`, not met, still no stamp |
//! | 3 | `outcome: stocking a recipe of every part family meets the yard and stamps the record once` | stocking a recipe of every family reads `1 / 1` and `7 / 7`, met, with a stamp inside the play clock that does not move across two more frames |
//! | 4 | `outcome: a frozen yard still reads met` | after the station's cell freezes, the frozen record is owned, its frozen storage still holds every stocked part, and the progress still reads `1 / 1`, `7 / 7`, met, with the same stamp |
//! | 5 | `outcome: the map's chart view shows the yard charter met` | its OBJECTIVES block shows one `Goal reached at` row in place of the milestones and no milestone row |
//! | 6 | `outcome: the leave save keeps the met yard charter` | `open_world` reads the same met stamp from the leave save |
//! | 7 | `outcome: a load restores the met yard charter` | after Load and a frame, the same stamp and met progress return |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_role_goal_yard --features debug
//! # look for: `role goal yard: PASS a load restores the met yard charter` and
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `role-goal-yard-charter.png` of the chart
//! view with the met OBJECTIVES block.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_role_goal_yard")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed as the scavenger: warm a living Neutral station's \
             civilization to Ally by merits, buy it, stock a recipe of every part family into \
             it, and watch the yard charter reach met, keep reading met off the frozen ledger \
             past a cell freeze, show in the Map, and survive a save and a Load. Autopilot-only \
             correctness range",
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
        app.add_plugins(walk::YardWatchPlugin);
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

    /// The seed the range plays: its window around the origin holds a living
    /// Neutral station outside the origin's cell, as [`system_station_own`]
    /// relies on.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Role Goal Yard World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const START_ROLE_BUTTON: &str = "Start Role Button: scavenger";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const MAP_TAB: &str = "InterfaceTabMap";

    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";

    /// The disclosed fixture balance of the player.
    const FIXTURE_CREDITS: u32 = 100_000_000;

    /// The PNGs the capture path shoots.
    const BUY_SHOT: &str = "role-goal-yard-buy.png";
    const CHARTER_SHOT: &str = "role-goal-yard-charter.png";

    /// How far from the station the player parks before the berth, and how
    /// far it flies clear after the undock.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);
    /// How many cells the crossing jumps: past the active radius, so the
    /// station's cell retires.
    const CROSSING_CELLS: i32 = 3;

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// The seven ship-part families a yard's parts line counts, mirrored
    /// here because `ShipPartFamilyType` is private to `nova_world_base`.
    const FAMILIES: [&str; 7] = [
        "Hull",
        "Controller",
        "Thruster",
        "Weapon",
        "CargoIntake",
        "Docking",
        "Mining",
    ];

    /// The family a section kind builds toward, by the same grouping
    /// `nova_world_base::ship_parts::ShipPartFamilyType::of` uses.
    fn family_of(kind: &SectionKind) -> &'static str {
        match kind {
            SectionKind::Hull(_) => "Hull",
            SectionKind::Controller(_) => "Controller",
            SectionKind::Thruster(_) => "Thruster",
            SectionKind::Turret(_) | SectionKind::Torpedo(_) | SectionKind::Railgun(_) => "Weapon",
            SectionKind::CargoIntake(_) => "CargoIntake",
            SectionKind::Docking(_) => "Docking",
            SectionKind::Mining(_) => "Mining",
        }
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct YardWatch {
        /// Every buy and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The cell the crossing landed in.
        crossed: Option<SectorCoord>,
        /// Every part, at its recipe count, the fixture stocked.
        stocked: Vec<(ItemDesignId, u32)>,
        /// The first stamp observed once the yard is met.
        met_at: Option<f64>,
    }

    impl YardWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct YardWatchPlugin;

    impl Plugin for YardWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<YardWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<YardWatch>| {
                let name = event.name();
                if !matches!(name, "onstationbought" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("role goal yard: {name} {data}");
                watch.events.push((name.to_string(), data));
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
        the_player(world).expect("role goal yard: exactly one player ship")
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

    fn station_root(world: &World) -> Option<Entity> {
        wearing(world, world.resource::<YardWatch>().id.as_ref()?)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("role goal yard: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<YardWatch>()
            .id
            .clone()
            .expect("role goal yard: the station is chosen")
    }

    fn owned(world: &World, station: Entity) -> bool {
        world.get::<OwnedStationMarker>(station).is_some()
    }

    /// The chosen station reads the player's own side: the world layer's
    /// `sync_civilization_sides` wrote its civilization's stance there.
    fn station_is_ally() -> Arc<Predicate> {
        when(|world| {
            station_root(world)
                .and_then(|station| world.get::<Allegiance>(station))
                .is_some_and(|allegiance| *allegiance == Allegiance::Player)
        })
    }

    /// Warm the chosen station's civilization to Ally the way a player
    /// does: one merit per handed-in job, [`MERIT_STEP`] merits per step.
    /// Nothing when it is Ally already.
    ///
    /// # Panics
    ///
    /// The civilization never reaches Ally within twice [`MERIT_STEP`]
    /// merits.
    fn warm_to_ally(world: &mut World) {
        let station = station(world);
        let civ = world
            .get::<TraderMarket>(station)
            .expect("role goal yard: the chosen station prices goods")
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
                "role goal yard: '{}' never warmed to Ally",
                civ.name()
            );
        }
        info!(
            "role goal yard: '{}' warmed to Ally after {merits} merit(s)",
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

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| {
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

    /// The buy reported the player's balance and the station.
    fn buy_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<YardWatch>();
            !watch.named("onstationbought").is_empty()
                && watch
                    .named("oncreditschanged")
                    .iter()
                    .any(|data| data["cause"] == "StationBuy")
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
    /// click lands on it and a shot shows it.
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
        info!("role goal yard: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<YardWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "role goal yard: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("role goal yard: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("role goal yard: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("role goal yard: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("role goal yard: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, moving and
    /// turning with the station, then ask to dock. Every frame until the
    /// dock takes, as [`system_station_own`] does.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
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

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("role goal yard: no field named '{name}'"));
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
            .step(format!("role goal yard: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("role goal yard: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the Station pane, scroll the Buy button into view, settle and
    /// shoot `shot`.
    fn shoot_the_block(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        shown: &'static str,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key {label}"),
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            &format!("role goal yard: open the Station pane {label}"),
            "InterfaceTabStation",
            the_interface_shows(Some(InterfacePaneType::Station)),
            SESSION_SECS,
        )
        .step(format!("role goal yard: settle the lists {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!(
            "role goal yard: scroll the station block into view {label}"
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
        .step(format!("role goal yard: settle the scroll {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!("role goal yard: shoot the block {label}"))
        .on_enter(move |world: &mut World| shoot(world, shot))
        .until(shot_written(shot))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
    }

    /// Pick the nearest station that is living, generated, on the Neutral
    /// side, and stands outside the origin's cell, where the walk stands
    /// after its jump.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let seen: Vec<(GeneratedBodyId, Option<Allegiance>, bool, Vec3)> = world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .map(|(id, position, derelict, side)| (id.clone(), side.copied(), derelict, position.0))
            .collect();
        let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = seen
            .into_iter()
            .filter(|(_, side, derelict, at)| {
                !derelict
                    && *side == Some(Allegiance::Neutral)
                    && SectorCoord::containing(Meters3::from_engine(*at), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(id, _, _, at)| {
                (
                    Meters3::from_engine(at).distance(Meters3::ZERO).get(),
                    id,
                    at,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, at)) = stations.into_iter().next() else {
            panic!(
                "role goal yard: the window on seed {STATION_SEED} holds no living Neutral \
                 station outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!("role goal yard: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<YardWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
    }

    /// Disclosed fixture state: the player's balance set to
    /// [`FIXTURE_CREDITS`].
    fn set_fixture_credits(world: &mut World) {
        let player = player(world);
        world
            .get_mut::<ShipCredits>(player)
            .expect("role goal yard: the player has a balance")
            .0 = FIXTURE_CREDITS;
        info!("role goal yard: fixture sets the player's balance to {FIXTURE_CREDITS} cr");
    }

    /// Jump the undocked player [`PARK_DISTANCE`] straight out from the
    /// station, at rest, as a pilot who flies off would stand.
    fn fly_clear(world: &mut World) {
        let player = player(world);
        let station = world
            .get::<Position>(station(world))
            .expect("a station stands")
            .0;
        let at = world.get::<Position>(player).expect("a body stands").0;
        let out = (at - station).normalize_or(Vec3::Y);
        world.entity_mut(player).insert((
            Position(at + out * PARK_DISTANCE.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("role goal yard: the player flies {PARK_DISTANCE:?} clear of the station");
    }

    /// Jump the player [`CROSSING_CELLS`] cells out along +X.
    fn jump_out(world: &mut World) {
        let (edge, _) = world_shape(world);
        let player = player(world);
        let offset = Vec3::X * Meters(edge.get() * CROSSING_CELLS as f32).to_engine();
        let at = world.get::<Position>(player).expect("a body stands").0 + offset;
        world.entity_mut(player).insert(Position(at));
        let cell = SectorCoord::containing(Meters3::from_engine(at), edge);
        info!("role goal yard: the player jumps to cell {cell}");
        world.resource_mut::<YardWatch>().crossed = Some(cell);
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

    /// The storage the station's record in `sectors` froze with; `None` when
    /// no record holds it as a station.
    fn frozen_storage<'a>(sectors: &'a FrozenSectors, id: &str) -> Option<&'a StationStorage> {
        sectors.iter().find_map(|(_, record)| {
            record
                .bodies()
                .iter()
                .find(|body| body.id() == Some(id))
                .and_then(|body| match body.body() {
                    FrozenBodyType::Station(station) => Some(station.storage()),
                    _ => None,
                })
        })
    }

    /// The station's cell is frozen behind the jump.
    fn station_cell_frozen() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<YardWatch>()
                .cell
                .is_some_and(|cell| world.resource::<FrozenSectors>().get(cell).is_some())
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

    /// The yard's two lines at `stations` of 1 and `families` of 7.
    fn expected_lines(stations: u64, families: u64) -> Vec<GoalLine> {
        vec![
            GoalLine::new("Stations owned", stations, 1),
            GoalLine::new("Part families stocked", families, 7),
        ]
    }

    /// Claim 1: the charter opens standing with no owned station.
    fn assert_standing_charter(world: &mut World) {
        let (title, lines, met) = read_progress(world);
        assert_eq!(title, "A yard of your own", "role goal yard: charter title");
        assert_eq!(
            lines,
            expected_lines(0, 0),
            "role goal yard: the charter opens with zero progress"
        );
        assert!(!met, "role goal yard: the charter is standing");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goal yard: the standing charter has no stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the yard charter opens standing with no owned station",
            serde_json::json!({
                "title": title,
                "lines": ["Stations owned 0 / 1", "Part families stocked 0 / 7"],
                "met_at": None::<f64>,
            }),
        );
        info!("role goal yard: the yard charter opens standing with no owned station");
    }

    /// Claim 2: a bought station counts as the yard, with nothing stocked.
    fn assert_bought_counts(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(1, 0),
            "role goal yard: a bought station counts, nothing stocked"
        );
        assert!(!met, "role goal yard: the charter still stands");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goal yard: no stamp before any family is stocked"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a bought station counts as the yard with nothing stocked",
            serde_json::json!({ "lines": ["Stations owned 1 / 1", "Part families stocked 0 / 7"] }),
        );
        info!("role goal yard: a bought station counts as the yard with nothing stocked");
    }

    /// A loaded section's recipe mass in grams: the sum of
    /// `items.design(item).stack_mass_g(count)` over its build parts.
    fn recipe_mass_g(items: &GameItems, section: &SectionConfig) -> u64 {
        section
            .base
            .build
            .parts
            .iter()
            .map(|(item, count)| items.design(item).stack_mass_g(*count))
            .sum()
    }

    /// Disclosed fixture state: for each of the seven ship-part families,
    /// stock the bought station with the lightest loaded section of that
    /// family by non-empty recipe mass, so the lightest yard fits the
    /// bought station's store.
    ///
    /// # Panics
    ///
    /// A family with no loaded section whose recipe takes parts.
    fn stock_every_family(world: &mut World) {
        let items = world.resource::<GameItems>().clone();
        let sections: Vec<SectionConfig> = world.resource::<GameSections>().0.clone();
        let station = station(world);
        let mut stocked: Vec<(ItemDesignId, u32)> = Vec::new();
        let mut total_g: u64 = 0;
        for family in FAMILIES {
            let (section, mass_g) = sections
                .iter()
                .filter(|section| {
                    family_of(&section.kind) == family && !section.base.build.parts.is_empty()
                })
                .map(|section| (section, recipe_mass_g(&items, section)))
                .min_by_key(|(_, mass_g)| *mass_g)
                .unwrap_or_else(|| {
                    panic!("role goal yard: no loaded '{family}' section with a non-empty recipe")
                });
            info!(
                "role goal yard: {family} picks '{}' at {mass_g} g",
                section.base.id
            );
            total_g += mass_g;
            let mut storage = world
                .get_mut::<StationStorage>(station)
                .expect("role goal yard: the bought station has storage");
            for (item, count) in &section.base.build.parts {
                storage.add(&items, item, *count);
                stocked.push((item.clone(), *count));
            }
        }
        world.resource_mut::<YardWatch>().stocked = stocked;
        info!("role goal yard: the fixture stocks the lightest recipe of every part family, {total_g} g total");
    }

    /// Check the tracker after it has observed every family stocked.
    fn assert_met_and_capture_stamp(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(1, 7),
            "role goal yard: all seven families are stocked"
        );
        assert!(met, "role goal yard: the yard charter is met");
        let clock = world.resource::<WorldClock>().play_seconds;
        let met_at = world
            .resource::<GoalRecord>()
            .met_at
            .expect("role goal yard: a met charter has a stamp");
        assert!(
            met_at.is_finite() && met_at > 0.0 && met_at <= clock,
            "role goal yard: stamp {met_at} is within the positive play clock {clock}"
        );
        world.resource_mut::<YardWatch>().met_at = Some(met_at);
    }

    /// Claim 3: the met stamp is unchanged after two more frames.
    fn assert_stamp_stable(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(1, 7),
            "role goal yard: the yard charter stays met"
        );
        assert!(met, "role goal yard: the yard charter stays met");
        let expected = world
            .resource::<YardWatch>()
            .met_at
            .expect("role goal yard: the first stamp was captured");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal yard: the met record is stamped only once"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: stocking a recipe of every part family meets the yard and stamps the record once",
            serde_json::json!({ "lines": progress_lines, "met_at": expected }),
        );
        info!("role goal yard: PASS stocking every family meets the yard at {expected} s");
    }

    /// Claim 4: a frozen yard still reads met, off the frozen ledger.
    fn assert_frozen_yard(world: &mut World) {
        let id = station_id(world).to_string();
        assert!(
            station_root(world).is_none(),
            "role goal yard: the station retired with its cell"
        );
        assert_eq!(
            frozen_owned(world.resource::<FrozenSectors>(), &id),
            Some(true),
            "role goal yard: the frozen station record is owned"
        );
        let stock = frozen_storage(world.resource::<FrozenSectors>(), &id)
            .unwrap_or_else(|| panic!("role goal yard: the frozen station carries its storage"));
        for (item, count) in &world.resource::<YardWatch>().stocked.clone() {
            assert!(
                stock.count(item) >= *count,
                "role goal yard: the frozen station still holds {count} of '{item}'"
            );
        }
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(1, 7),
            "role goal yard: the frozen yard still reads met"
        );
        assert!(met, "role goal yard: the frozen yard still reads met");
        let expected = world
            .resource::<YardWatch>()
            .met_at
            .expect("role goal yard: the stamp was captured");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal yard: the frozen yard keeps its stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a frozen yard still reads met",
            serde_json::json!({ "station": id, "met_at": expected }),
        );
        info!("role goal yard: a frozen yard still reads met");
    }

    /// Claim 5: the chart view renders the met charter block.
    fn assert_map_charter(world: &mut World) {
        let shown = shown_texts(world);
        let met_row = shown
            .iter()
            .find(|text| text.starts_with("Goal reached at "))
            .cloned()
            .unwrap_or_else(|| {
                panic!("role goal yard: the chart view shows its met stamp: {shown:?}")
            });
        assert!(
            named_node(world, "MapChartObjectiveMet"),
            "role goal yard: the OBJECTIVES block holds the met row"
        );
        assert!(
            !named_node(world, "MapChartObjectiveMilestone"),
            "role goal yard: a met role lists no milestone row: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view shows the yard charter met",
            serde_json::json!({ "block": "OBJECTIVES", "stamp": met_row }),
        );
        info!("role goal yard: the Map chart view shows the met yard charter: {met_row}");
    }

    /// Claim 6: read the leave save from disk and release its lock before
    /// the Load step.
    fn assert_leave_save(world: &mut World) {
        let expected = world
            .resource::<YardWatch>()
            .met_at
            .expect("role goal yard: the met stamp was captured");
        let root = nova_assets::storage::worlds_root()
            .expect("role goal yard: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("role goal yard: world name has a slug");
        let (_folder, lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("role goal yard: cannot open the leave save: {refusal}"));
        assert_eq!(
            state.goal.met_at,
            Some(expected),
            "role goal yard: leave save keeps the met stamp"
        );
        assert!(
            state.goal.is_met(),
            "role goal yard: leave save records the yard charter met"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save keeps the met yard charter",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "met_at": state.goal.met_at,
            }),
        );
        info!("role goal yard: leave save keeps the yard charter met at {expected} s");
        drop(lock);
    }

    /// Claim 7: a load restores the met charter and its stamp.
    fn assert_loaded_charter(world: &mut World) {
        let expected = world
            .resource::<YardWatch>()
            .met_at
            .expect("role goal yard: the met stamp was captured");
        let (title, lines, met) = read_progress(world);
        assert_eq!(title, "A yard of your own", "role goal yard: loaded title");
        assert_eq!(lines, expected_lines(1, 7), "role goal yard: loaded lines");
        assert!(met, "role goal yard: loaded progress is met");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goal yard: Load restores the original stamp"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: a load restores the met yard charter",
            serde_json::json!({ "title": title, "lines": progress_lines, "met_at": expected }),
        );
        info!("role goal yard: PASS a load restores the met yard charter");
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("role goal yard: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "role goal yard: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("role goal yard: name the world and enter seed 10")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "role goal yard: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "role goal yard: pick the scavenger",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "role goal yard: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goal yard: the window streams in around the start and the first save lands")
            .until(and(window_is(start_cell), when(saved_and_idle)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(and(
                window_is(|_| Some(SectorCoord::ORIGIN)),
                when(saved_and_idle),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: the yard charter opens standing")
            .on_enter(assert_standing_charter)
            .add()
            .step("role goal yard: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("role goal yard: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<YardWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: set the fixture balance")
            .on_enter(set_fixture_credits)
            .until(frames(1))
            .add()
            .step("role goal yard: warm the station's civilization to Ally")
            .on_enter(warm_to_ally)
            .until(station_is_ally())
            .deadline(SESSION_SECS)
            .add();
        let script = shoot_the_block(script, "to buy", STATION_BUY_BUTTON, BUY_SHOT).click_named(
            "role goal yard: buy the station",
            STATION_BUY_BUTTON,
            when(|world| station_root(world).is_some_and(|station| owned(world, station))),
            SESSION_SECS,
        );
        // The pane pushed its report under the interface pause; the tracker
        // drains it once the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal yard: the buy reports")
            .until(buy_reported())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: run one frame after the buy")
            .until(frames(1))
            .add()
            .step("role goal yard: a bought station counts, nothing stocked")
            .on_enter(assert_bought_counts)
            .add()
            .step("role goal yard: stock a recipe of every part family")
            .on_enter(stock_every_family)
            .until(frames(1))
            .add()
            .step("role goal yard: wait for the goal tracker")
            .until(when(progress_is_met))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal yard: check the first met stamp")
            .on_enter(assert_met_and_capture_stamp)
            .add()
            .step("role goal yard: hold the stamp for two frames")
            .until(frames(2))
            .add()
            .step("role goal yard: assert the stamp is stable")
            .on_enter(assert_stamp_stable)
            .add()
            .step("role goal yard: undock")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: fly clear of the station")
            .on_enter(fly_clear)
            .until(frames(2))
            .add()
            .step("role goal yard: jump three cells out")
            .on_enter(jump_out)
            .until(and(
                window_is(|world| world.resource::<YardWatch>().crossed),
                station_cell_frozen(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: assert the frozen yard")
            .on_enter(assert_frozen_yard)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goal yard: open the Map pane",
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
        .step("role goal yard: settle the charter block")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("role goal yard: assert the Map charter")
        .on_enter(assert_map_charter)
        .add()
        .step("role goal yard: shoot the charter block")
        .on_enter(|world: &mut World| shoot(world, CHARTER_SHOT))
        .until(shot_written(CHARTER_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add();
        let row = format!(
            "Load World Row: {}",
            world_slug(WORLD_NAME).expect("role goal yard: world name has a slug")
        );
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal yard: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal yard: release escape")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "role goal yard: leave to main menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("role goal yard: assert the leave save")
            .on_enter(assert_leave_save)
            .add()
            .click_named(
                "role goal yard: click Load",
                LOAD_BUTTON,
                ui_node_present(row.clone()),
                BEAT_SECS,
            )
            .click_named(
                "role goal yard: select the world",
                &row,
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "role goal yard: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goal yard: the loaded world arms")
            .until(when(world_armed))
            .deadline(SESSION_SECS)
            .add()
            .step("role goal yard: let the goal tracker run after Load")
            .until(and(when(progress_is_met), frames(1)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal yard: assert the loaded charter")
            .on_enter(assert_loaded_charter)
            .add()
    }
}
