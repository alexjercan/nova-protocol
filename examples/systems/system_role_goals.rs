//! system_role_goals: a bounty hunter's charter starts standing, stands six
//! hand-ins against its kill-list line while no station stands, meets both
//! lines once a station is bought, shows in the Map, and survives the leave
//! save and a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on seed 10,
//! picks `bounty_hunter`, and waits for the armed world and its first save.
//! The range checks the authored charter, then injects six completed
//! Neutralize jobs as DISCLOSED FIXTURE STATE: each valid job goes through
//! `Jobs::accept`, `mark_done` and `complete`, with posters in cells
//! (1, 0, 0), (2, 0, 0), (3, 0, 0), (9, 0, 0), (10, 0, 0) and (11, 0, 0),
//! each naming a distinct hull; this fixture replaces flying and fighting,
//! not the goal tracker. It checks the kill-list line reads six with the
//! shared station line still at zero, undocks, jumps to the origin, picks
//! the nearest living non-Enemy station outside the origin's cell, warms its
//! civilization to Ally by merits the way a player does, berths and docks,
//! sets the player's balance to 100,000,000 cr as a second piece of
//! DISCLOSED FIXTURE STATE so the station is affordable, and buys it through
//! the Station pane's Buy station button. It checks the
//! one-time play-clock stamp, reads the met charter in the Map's chart view,
//! leaves to the main menu,
//! reads the save through [`open_world`], then loads it again and checks the
//! live progress.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the charter opens standing with its lines at zero` | the bounty hunter's `A kill list` has `Stations owned 0 / 1` and `Bounties handed in 0 / 6`, with no stamp |
//! | 2 | `outcome: six neutralize hand-ins stand the kill list at the station line` | six completed Neutralize jobs on six distinct hulls yield `Stations owned 0 / 1` and `Bounties handed in 6 / 6`, not met, still no stamp |
//! | 3 | `outcome: buying a station meets the kill list and stamps the record once` | buying one station through the Station pane reads `1 / 1` and `6 / 6`, met, with a stamp inside the play clock that does not move across two more frames |
//! | 4 | `outcome: the map's chart view shows the charter met` | its OBJECTIVES block shows one `Goal reached at` row in place of the milestones and no milestone row |
//! | 5 | `outcome: the leave save keeps the met charter` | `open_world` reads the same met stamp from the leave save |
//! | 6 | `outcome: a load restores the met charter` | after Load and a frame, the same stamp and met progress return |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_role_goals --features debug
//! # look for: `role goals: PASS buying a station meets the kill list` and
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `role-goals-charter.png` of the chart view
//! with the met goal's OBJECTIVES block.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_role_goals")]
#[command(version = "1.0.0")]
#[command(
    about = "Start the bounty hunter, stand its kill list at six hand-ins, buy a station to \
             meet the charter, read it in the Map, and check it through a save and a Load. \
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
        app.add_systems(Update, walk::hold_the_walk_clock);
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
        nova_interface::{pane::InterfacePaneType, station::STATION_BUY_BUTTON},
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{desired_sectors, SectorRoot, WorldClock, WorldConfig};

    use super::*;

    const WORLD_NAME: &str = "Role Goals World";
    const STATION_SEED: u32 = 10;
    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const SEED_FIELD: &str = "World Seed Field";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const START_ROLE_BUTTON: &str = "Start Role Button: bounty_hunter";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const MAP_TAB: &str = "InterfaceTabMap";
    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";
    /// The disclosed fixture balance the player holds once the station is
    /// bought.
    const FIXTURE_CREDITS: u32 = 100_000_000;
    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    const SESSION_SECS: f32 = 240.0;
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The frame the capture path writes of the chart view's OBJECTIVES block.
    const CHARTER_SHOT: &str = "role-goals-charter.png";

    /// What the chosen station and the first met stamp were.
    #[derive(Resource, Default)]
    pub(super) struct GoalWatch {
        /// The chosen station's typed id.
        station: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The first stamp observed once the charter is met.
        met_at: Option<f64>,
    }

    /// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
    /// scenario load hands `Time<Virtual>` back at its default.
    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
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
        the_player(world).expect("role goals: exactly one player ship")
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

    fn armed_and_saved() -> Arc<Predicate> {
        when(|world| world_armed(world) && saved_and_idle(world))
    }

    fn set_field(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("role goals: no field named '{name}'"));
        value.0 = text.to_string();
    }

    fn progress_is_met(world: &World) -> bool {
        world
            .get_resource::<GoalProgress>()
            .is_some_and(GoalProgress::is_met)
            && world
                .get_resource::<GoalRecord>()
                .is_some_and(GoalRecord::is_met)
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

    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("role goals: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("role goals: release {label}"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    fn charter_jobs() -> Vec<Job> {
        [1, 2, 3, 9, 10, 11]
            .into_iter()
            .enumerate()
            .map(|(index, x)| {
                let cell = SectorCoord::new(x, 0, 0);
                let station = GeneratedBodyId::new(cell, "cluster_station", 0);
                Job {
                    id: JobId {
                        station: station.clone(),
                        slot: 0,
                        posting: 0,
                    },
                    kind: JobType::Neutralize {
                        hull: GeneratedBodyId::new(cell, "cluster_hull", index as u32),
                        hull_name: format!("Fixture target {index}"),
                    },
                    target: station,
                    reward_cr: 400,
                }
            })
            .collect()
    }

    /// Disclosed fixture state: complete six valid neutralize jobs through
    /// the player's production Jobs API, each naming a distinct hull.
    fn inject_completed_jobs(world: &mut World) {
        let fixtures = charter_jobs();
        let mut jobs = world.resource_mut::<Jobs>();
        assert!(
            jobs.is_empty(),
            "role goals: the new world starts with no jobs"
        );
        for job in fixtures {
            job.validate()
                .unwrap_or_else(|error| panic!("role goals: invalid fixture job: {error}"));
            let id = job.id.clone();
            jobs.accept(job);
            jobs.mark_done(&id);
            jobs.complete(&id);
        }
        jobs.validate()
            .unwrap_or_else(|error| panic!("role goals: invalid completed jobs: {error}"));
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

    /// The kill list's two lines at `stations` owned and `bounties` handed in.
    fn expected_lines(stations: u64, bounties: u64) -> Vec<GoalLine> {
        vec![
            GoalLine::new("Stations owned", stations, 1),
            GoalLine::new("Bounties handed in", bounties, 6),
        ]
    }

    /// Claim 1: the charter opens standing with both lines at zero.
    fn assert_standing_charter(world: &mut World) {
        let (title, lines, met) = read_progress(world);
        assert_eq!(title, "A kill list", "role goals: charter title");
        assert_eq!(
            lines,
            expected_lines(0, 0),
            "role goals: the charter opens with zero progress"
        );
        assert!(!met, "role goals: the charter is standing");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goals: the standing charter has no stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the charter opens standing with its lines at zero",
            serde_json::json!({
                "title": title,
                "lines": ["Stations owned 0 / 1", "Bounties handed in 0 / 6"],
                "met_at": None::<f64>,
            }),
        );
        info!("role goals: the kill list opens standing at zero");
    }

    /// Claim 2: six hand-ins stand the kill list at the station line.
    fn assert_hand_ins_stand_at_station_line(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(0, 6),
            "role goals: six hand-ins meet the kill-list line alone"
        );
        assert!(
            !met,
            "role goals: six bounties with no station standing do not meet the charter"
        );
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goals: no stamp before a station is owned"
        );
        nova_probe::probe_marker(
            world,
            "outcome: six neutralize hand-ins stand the kill list at the station line",
            serde_json::json!({ "lines": ["Stations owned 0 / 1", "Bounties handed in 6 / 6"] }),
        );
        info!("role goals: six hand-ins stand the kill list at the station line");
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
        wearing(world, world.resource::<GoalWatch>().station.as_ref()?)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("role goals: the station stands")
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
            .expect("role goals: the chosen station prices goods")
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
                "role goals: '{}' never warmed to Ally",
                civ.name()
            );
        }
        info!(
            "role goals: '{}' warmed to Ally after {merits} merit(s)",
            civ.name()
        );
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
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

    /// Pick the nearest station that is living, generated, not Enemy, and
    /// stands outside the origin's cell, where the walk stands after its
    /// jump.
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
                    && side.is_some_and(|side| side != Allegiance::Enemy)
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
                "role goals: the window on seed {STATION_SEED} holds no living non-Enemy \
                 station outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!("role goals: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<GoalWatch>();
        watch.station = Some(id);
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
        info!("role goals: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<GoalWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "role goals: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("role goals: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("role goals: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("role goals: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("role goals: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, moving and
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

    /// Disclosed fixture state: the player's balance set to
    /// [`FIXTURE_CREDITS`].
    fn set_fixture_credits(world: &mut World) {
        let player = player(world);
        world
            .get_mut::<ShipCredits>(player)
            .expect("role goals: the player has a balance")
            .0 = FIXTURE_CREDITS;
        info!("role goals: fixture sets the player's balance to {FIXTURE_CREDITS} cr");
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

    /// Open the Station pane and scroll its Buy station button into view.
    fn open_the_station_pane(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goals: open the Station pane",
            "InterfaceTabStation",
            the_interface_shows(Some(InterfacePaneType::Station)),
            SESSION_SECS,
        )
        .step("role goals: settle the station lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("role goals: scroll the Buy button into view")
        .each(move |world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !in_panel(world, STATION_BUY_BUTTON) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(|world| in_panel(world, STATION_BUY_BUTTON)))
        .deadline(SESSION_SECS)
        .add()
        .step("role goals: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    fn named_node(world: &mut World, name: &str) -> bool {
        world
            .query::<&Name>()
            .iter(world)
            .any(|named| named.as_str() == name)
    }

    /// Check the tracker after the bought station meets the kill list.
    fn assert_met_and_capture_stamp(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(1, 6),
            "role goals: the bought station meets the kill list"
        );
        assert!(met, "role goals: the kill-list charter is met");
        let clock = world.resource::<WorldClock>().play_seconds;
        let met_at = world
            .resource::<GoalRecord>()
            .met_at
            .expect("role goals: a met charter has a stamp");
        assert!(
            met_at.is_finite() && met_at > 0.0 && met_at <= clock,
            "role goals: stamp {met_at} is within the positive play clock {clock}"
        );
        world.resource_mut::<GoalWatch>().met_at = Some(met_at);
    }

    /// The met stamp is unchanged after two more frames; emit the named claim.
    fn assert_stamp_stable(world: &mut World) {
        let (_, lines, met) = read_progress(world);
        assert_eq!(
            lines,
            expected_lines(1, 6),
            "role goals: the kill-list charter stays met"
        );
        assert!(met, "role goals: the kill-list charter stays met");
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the first stamp was captured");
        let found = world.resource::<GoalRecord>().met_at;
        assert_eq!(
            found,
            Some(expected),
            "role goals: the met record is stamped only once"
        );
        let completed = world.resource::<Jobs>().completed().to_vec();
        assert_eq!(completed.len(), 6, "role goals: six jobs completed");
        assert!(
            completed
                .iter()
                .all(|job| matches!(&job.kind, JobType::Neutralize { .. })),
            "role goals: every fixture hand-in is Neutralize"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: buying a station meets the kill list and stamps the record once",
            serde_json::json!({ "lines": progress_lines, "met_at": expected }),
        );
        info!("role goals: PASS buying a station meets the kill list at {expected} s");
    }

    /// The chart view's OBJECTIVES block holds the reached goal's stamp row
    /// in place of the milestones.
    fn assert_map_charter(world: &mut World) {
        let shown = shown_texts(world);
        let met_row = shown
            .iter()
            .find(|text| text.starts_with("Goal reached at "))
            .cloned()
            .unwrap_or_else(|| panic!("role goals: the chart view shows its met stamp: {shown:?}"));
        assert!(
            named_node(world, "MapChartObjectiveMet"),
            "role goals: the OBJECTIVES block holds the met row"
        );
        assert!(
            !named_node(world, "MapChartObjectiveMilestone"),
            "role goals: a met role lists no milestone row: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view shows the charter met",
            serde_json::json!({ "stamp": met_row }),
        );
        info!("role goals: the Map chart view shows the met charter: {met_row}");
    }

    /// Read the leave save from disk and release its lock before the Load step.
    fn assert_leave_save(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the met stamp was captured");
        let root = nova_assets::storage::worlds_root()
            .expect("role goals: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("role goals: world name has a slug");
        let (_folder, lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
            world.resource::<GameShipDesigns>(),
        )
        .unwrap_or_else(|refusal| panic!("role goals: cannot open the leave save: {refusal}"));
        assert_eq!(
            state.goal.met_at,
            Some(expected),
            "role goals: leave save keeps the met stamp"
        );
        assert!(
            state.goal.is_met(),
            "role goals: leave save records the charter met"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save keeps the met charter",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "met_at": state.goal.met_at,
            }),
        );
        info!("role goals: leave save keeps the charter met at {expected} s");
        drop(lock);
    }

    fn assert_loaded_charter(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the met stamp was captured");
        let (title, lines, met) = read_progress(world);
        assert_eq!(title, "A kill list", "role goals: loaded title");
        assert_eq!(lines, expected_lines(1, 6), "role goals: loaded lines");
        assert!(met, "role goals: loaded progress is met");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goals: Load restores the original stamp"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: a load restores the met charter",
            serde_json::json!({ "title": title, "lines": progress_lines, "met_at": expected }),
        );
        info!("role goals: PASS Load restored the met charter at {expected} s");
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("role goals: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "role goals: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("role goals: name the world and enter seed 10")
            .on_enter(|world: &mut World| {
                set_field(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "role goals: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "role goals: pick bounty hunter",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "role goals: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goals: the world arms and saves")
            .until(armed_and_saved())
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: assert the standing charter")
            .on_enter(assert_standing_charter)
            .add()
            .step("role goals: inject disclosed fixture hand-ins")
            .on_enter(inject_completed_jobs)
            .until(frames(1))
            .add()
            .step("role goals: assert six hand-ins stand at the station line")
            .on_enter(assert_hand_ins_stand_at_station_line)
            .add()
            .step("role goals: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(and(
                window_is(|_| Some(SectorCoord::ORIGIN)),
                when(saved_and_idle),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("role goals: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<GoalWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: set the fixture balance")
            .on_enter(set_fixture_credits)
            .until(frames(1))
            .add()
            .step("role goals: warm the station's civilization to Ally")
            .on_enter(warm_to_ally)
            .until(station_is_ally())
            .deadline(SESSION_SECS)
            .add();
        let script = open_the_station_pane(script).click_named(
            "role goals: buy the station",
            STATION_BUY_BUTTON,
            when(|world| station_root(world).is_some_and(|station| owned(world, station))),
            SESSION_SECS,
        );
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goals: wait for the goal tracker")
            .until(when(progress_is_met))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: check the first met stamp")
            .on_enter(assert_met_and_capture_stamp)
            .add()
            .step("role goals: hold the stamp for two frames")
            .until(frames(2))
            .add()
            .step("role goals: assert the stamp is stable")
            .on_enter(assert_stamp_stable)
            .add();
        let script = tap(
            script,
            "interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goals: open Map pane",
            MAP_TAB,
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        );
        let script = tap(
            script,
            "chart key",
            "map_chart",
            a_text_reads(|text| text == "OBJECTIVES"),
        )
        .step("role goals: settle the objectives block")
        .until(frames(6))
        .add()
        .step("role goals: assert the Map charter")
        .on_enter(assert_map_charter)
        .add()
        .step("role goals: shoot the objectives block")
        .on_enter(|world: &mut World| shoot(world, CHARTER_SHOT))
        .until(shot_written(CHARTER_SHOT))
        .deadline(BEAT_SECS)
        .add();
        let script = tap(script, "interface key", "interface_toggle", unpaused())
            .step("role goals: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: release Escape")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "role goals: leave to main menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("role goals: assert the leave save")
            .on_enter(assert_leave_save)
            .add()
            .click_named(
                "role goals: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("role goals: world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "role goals: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("role goals: world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "role goals: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goals: wait for the loaded world")
            .until(when(world_armed))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: let the goal tracker run after Load")
            .until(and(when(progress_is_met), frames(1)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: assert the loaded charter")
            .on_enter(assert_loaded_charter)
            .add();
        script
    }
}
