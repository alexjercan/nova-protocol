//! system_start_roles: every start role starts, docks, and comes back from
//! its save, driven the way a player drives New Game; every role on seed 10
//! must stream the same generated stations.
//!
//! Boots the shipped app (via [`editor_app`]) and, for each role in the
//! loaded catalogue, in catalogue order: clicks New Game, names the world
//! after the role, enters seed 10, clicks Next, picks the role and clicks
//! Create. The world's first save lands; the spawned ship must be the role's
//! hull with the role's kit and every weapon bound to its key, and its design
//! must pass the post-build checks. The start docks the player at the
//! session's starting station, the living Ally station nearest the origin;
//! the walk waits for that dock, checks the docked station is the one the
//! session names, accepts every job but an escort off the station's board
//! as the Jobs pane's Accept does, stamps the first one done and completed
//! IN PLACE (disclosed fixture state: the save must carry a completed job
//! beside the held ones), undocks and waits for the save. THE SHIP IS THEN
//! MOVED ONE CELL ALONG +X (a disclosed fixture move in place of the flight):
//! the crossing retires the cells it leaves and writes the crossing save, and
//! the walk reads the records that save holds. It then jettisons one unit of
//! the kit's first item through its cargo intake, as the Inventory pane's
//! Jettison does, and waits for the canister to leave the door. THE CANISTER
//! IS THEN PUT IN THE INTAKE'S TRIGGER each frame, moving with the ship: a
//! disclosed fixture move in place of flying the ship onto it. The intake
//! must take it, and the hold must be the kit again. The walk then changes
//! its balance by one
//! credit. THAT CREDIT IS DISCLOSED FIXTURE STATE: a change the save never
//! saw, so the reload can show it came from disk. Load last save reopens the
//! world from disk: the session's role, the hull, the kit and the bindings
//! must be back, and the extra credit gone; once the window streams in, the
//! chart's stations, the jobs, the boards, the play clock and the retired
//! cells must be the crossing save's. Back to Main Menu leaves through the
//! leave save, and the next role starts.
//!
//! Seed 10's start is the same allied station for every role, so every role
//! docks at a generated station of the same world.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: create starts the picked start role` | the session Create inserted names the picked role, the first save landed, and the charter progress is the role's goal or empty for a role with none |
//! | 2 | `outcome: the start role spawns its hull with its kit and its weapons bound` | the player's design sections are the hull's, its credits and hold are the kit's, every turret is on the trigger, every railgun on R, every bay on F, every miner on V |
//! | 3 | `outcome: the start hull passes the post-build checks` | `ship_build_fault` finds no fault in the live design |
//! | 4 | `outcome: the start hull docks at a generated station` | the start docks the player at the session's starting station, a living Ally one, and the walk undocks |
//! | 5 | `outcome: the start hull jettisons and takes an item back in` | one kit unit planned by `plan_item_jettison` leaves the intake as a canister, the intake takes it back, and the hold is the kit again |
//! | 6 | `outcome: the start role survives a save and a load` | after Load last save the session's role, hull, kit and bindings are the role's, and the unsaved credit is gone |
//! | 7 | `outcome: load last save keeps the saved records` | after Load last save the chart's stations, the held and completed jobs, the posted boards and the cells the crossing retired are the crossing save's, and the play clock is past the crossing's |
//! | 8 | `outcome: every start role started` | the walk started every role in the catalogue |
//! | 9 | `outcome: every start role streams the same stations from one seed` | every role's sorted live station ids on seed 10 are non-empty and equal |
//!
//! Each role's hull price and kit value at ask are logged for review, not
//! asserted.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_start_roles --features debug
//! # look for: `start roles: <role> starts <hull>: hull <n> cr, kit <n> cr`,
//! #           `start roles: <role> docked at <station>`,
//! #           `start roles: <role> jettisoned and took back one <item>`,
//! #           `start roles: <role> came back from its save`,
//! #           `start roles: <role> kept its records through the load`,
//! #           `start roles: PASS every start role started`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_start_roles")]
#[command(version = "1.0.0")]
#[command(
    about = "Every start role through New Game: its hull, kit and bindings, the post-build checks, a dock at a generated station, and Load last save. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings and saved worlds, set before the app reads
    // them: Create makes one world per role, and a player's worlds or an
    // earlier run's may not be in its way.
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
        // No frame-time claim: this range holds spawns and saves, not
        // milliseconds.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<walk::RolesWatch>();
        app.add_observer(walk::count_ejected);
        app.add_observer(walk::count_taken);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_input::prelude::InputSource;
    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate},
        nova_interface::prelude::JobAcceptCommand,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, CurrentSector, FrozenSectors, SectorRoot, WorldConfig,
    };

    use super::*;

    /// The roles the walk starts, in the base catalogue's order. The walk
    /// asserts the loaded catalogue holds exactly these, so a new role needs
    /// a line here.
    const ROLES: [&str; 6] = [
        "industrial",
        "scavenger",
        "transport",
        "bounty_hunter",
        "military",
        "pirate",
    ];

    /// The seed every role's world plays: its start docks every role at the
    /// same living Ally station.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const SEED_FIELD: &str = "World Seed Field";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const RETRY_BUTTON: &str = "Pause Retry Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";

    /// Seconds a load, a stream, a save or a beat gets on a software-rendered
    /// GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;

    /// What the walk carries between beats.
    #[derive(Resource, Default, Debug)]
    pub(super) struct RolesWatch {
        /// The player ship before Load last save, so the reload's ship can be
        /// told apart.
        before: Option<Entity>,
        /// The starting station the start docked the role at.
        station: Option<Entity>,
        /// The roles that came back from their save.
        started: Vec<String>,
        /// The player's cargo intake while the jettison beat runs.
        intake: Option<Entity>,
        /// The canisters that stood before the jettison: the world's own,
        /// such as a strip's yield, which the walk leaves alone.
        strays: Vec<Entity>,
        /// The canister the jettison dropped, once found.
        canister: Option<Entity>,
        /// Canisters that intake dropped.
        ejected: usize,
        /// Canisters that intake took.
        taken: usize,
        /// Sorted generated station ids streamed for each role on the pinned
        /// seed.
        stations_by_role: BTreeMap<String, Vec<GeneratedBodyId>>,
        /// How many board offers the walk asked to accept.
        accepting: usize,
        /// The save generation before the crossing, so the crossing save
        /// can be told from the earlier ones.
        generation_before: u64,
        /// The cell the crossing moves the player into.
        next: Option<SectorCoord>,
        /// The play clock when the crossing was made: the crossing save's
        /// clock is at or past it.
        clock_at_crossing: f64,
        /// What the crossing save holds, read off the live records once it
        /// is on disk.
        saved: Option<SavedRecords>,
    }

    /// The records a Load must bring back, as they stood when the crossing
    /// save was on disk. The jobs and the start station's board stand still
    /// from the save to the Load; the chart and the other boards may grow as
    /// the new window's stations come into sensor range, so those are
    /// checked for what the save held, not for equality.
    #[derive(Debug)]
    struct SavedRecords {
        /// The start station, found while docked.
        start: GeneratedBodyId,
        /// The stations the chart knew, the start station among them.
        stations: Vec<GeneratedBodyId>,
        jobs: Jobs,
        /// Every board; the start station's is the one compared whole.
        postings: JobPostings,
        /// The cells the crossing retired. Off the new window, their records
        /// stay in the ledger through the reload; the window's own are
        /// consumed as its cells materialize.
        retired: BTreeSet<SectorCoord>,
    }

    /// Count a canister the watched intake dropped.
    pub(super) fn count_ejected(ejected: On<CargoCanisterEjected>, mut watch: ResMut<RolesWatch>) {
        if watch.intake == Some(ejected.entity) {
            watch.ejected += 1;
        }
    }

    /// Count a canister the watched intake took.
    pub(super) fn count_taken(taken: On<CargoCanisterTaken>, mut watch: ResMut<RolesWatch>) {
        if watch.intake == Some(taken.entity) {
            watch.taken += 1;
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
        the_player(world).expect("start roles: exactly one player ship")
    }

    fn role_of(world: &World, id: &str) -> StartRole {
        world
            .resource::<GameStartRoles>()
            .get_role(&id.into())
            .unwrap_or_else(|| panic!("start roles: the catalogue has no role '{id}'"))
            .clone()
    }

    fn set_field(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("start roles: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Advance once the world is armed around one player ship other than the
    /// one recorded before Load last save, and its session has a save on
    /// disk with nothing wanted or writing.
    fn armed_and_saved() -> Arc<Predicate> {
        when(|world| {
            let before = world.resource::<RolesWatch>().before;
            world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
                && the_player(world).is_some_and(|player| Some(player) != before)
                && saved_and_idle(world)
        })
    }

    fn saved_and_idle(world: &World) -> bool {
        world
            .get_resource::<WorldSaveSession>()
            .is_some_and(|session| {
                session.is_idle() && matches!(session.status(), WorldSaveStatus::Saved { .. })
            })
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free_and_saved() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
                && saved_and_idle(world)
        })
    }

    /// The full desired window around the player's cell is live, so query
    /// timing cannot change a role's recorded station set and every saved
    /// window cell has been thawed before the records are compared.
    fn window_streamed() -> Arc<Predicate> {
        when(|world| {
            let Some(config) = world.get_resource::<WorldConfig<NovaLayeredWorld>>() else {
                return false;
            };
            let Some(at) = the_player(world).and_then(|player| world.get::<Position>(player))
            else {
                return false;
            };
            let home = SectorCoord::containing(Meters3::from_engine(at.0), config.sector_edge);
            let live: BTreeSet<SectorCoord> = world
                .try_query::<&SectorRoot>()
                .map(|mut roots| roots.iter(world).map(|root| root.0).collect())
                .unwrap_or_default();
            live == desired_sectors(home, config.active_radius)
        })
    }

    /// The generation of the session's last save, once it is on disk and
    /// nothing is wanted or writing.
    fn saved_generation(world: &World) -> Option<u64> {
        world
            .get_resource::<WorldSaveSession>()
            .and_then(|session| match session.status() {
                WorldSaveStatus::Saved { generation } if session.is_idle() => Some(*generation),
                _ => None,
            })
    }

    /// Accept every job the start station's board posts but an escort, as
    /// the Jobs pane's Accept does. An escort's hauler flies its leg on its
    /// own and a hauler lost on the way drops the job, which would change
    /// the records between the save and the reload.
    fn accept_the_board(world: &mut World, id: &str) {
        let start = world.resource::<OpenWorldSession>().start.id.clone();
        let offers: Vec<JobId> = world
            .resource::<JobPostings>()
            .board(&start)
            .map(|slots| {
                slots
                    .iter()
                    .filter(|slot| !matches!(slot.offer.kind, JobType::Escort { .. }))
                    .map(|slot| slot.offer.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            !offers.is_empty(),
            "start roles: '{id}' start station '{start}' posts no job but escorts"
        );
        info!(
            "start roles: {id} accepts {} job(s) off '{start}'",
            offers.len()
        );
        world.resource_mut::<RolesWatch>().accepting = offers.len();
        for offer in offers {
            world.write_message(JobAcceptCommand { id: offer });
        }
    }

    /// Every accept the walk asked for landed.
    fn board_accepted() -> Arc<Predicate> {
        when(|world| {
            let accepting = world.resource::<RolesWatch>().accepting;
            accepting > 0 && world.resource::<Jobs>().held().len() == accepting
        })
    }

    /// Disclosed fixture state: the first held job is stamped done and
    /// completed in place, as a hand-in leaves it, so the save carries a
    /// completed job beside the held ones.
    fn complete_one_job(world: &mut World, id: &str) {
        let mut jobs = world.resource_mut::<Jobs>();
        let first = jobs
            .held()
            .first()
            .map(|job| job.id.clone())
            .unwrap_or_else(|| panic!("start roles: '{id}' holds no job to complete"));
        jobs.mark_done(&first);
        jobs.complete(&first);
    }

    /// Disclosed fixture move: the undocked player one cell along +X, the
    /// crossing a flight would make. The crossing retires the cells it
    /// leaves behind and writes the crossing save.
    fn cross_one_cell(world: &mut World, id: &str) {
        let player = player(world);
        let edge = world
            .resource::<WorldConfig<NovaLayeredWorld>>()
            .sector_edge;
        let generation = saved_generation(world)
            .unwrap_or_else(|| panic!("start roles: '{id}' crosses before its save is on disk"));
        let at = world
            .get::<Position>(player)
            .expect("start roles: the player stands")
            .0;
        let home = SectorCoord::containing(Meters3::from_engine(at), edge);
        let next = SectorCoord::new(home.x + 1, home.y, home.z);
        let clock = world.resource::<WorldClock>().play_seconds;
        assert!(clock > 0.0, "start roles: '{id}' crosses at play time zero");
        world
            .entity_mut(player)
            .insert(Position(at + Vec3::X * edge.to_engine()));
        let mut watch = world.resource_mut::<RolesWatch>();
        watch.generation_before = generation;
        watch.next = Some(next);
        watch.clock_at_crossing = clock;
        info!("start roles: {id} crosses from {home} into {next} at {clock:.1} s of play");
    }

    /// The player stands in the next cell and the crossing save is on disk.
    fn crossing_saved() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<RolesWatch>();
            let Some(next) = watch.next else {
                return false;
            };
            world
                .get_resource::<CurrentSector>()
                .is_some_and(|current| current.0 == next)
                && saved_generation(world)
                    .is_some_and(|generation| generation > watch.generation_before)
        })
    }

    /// Read the records the crossing save holds off the live resources: the
    /// save took them this way, and nothing changes them before the Load.
    fn record_the_save(world: &mut World, id: &str) {
        let start = world.resource::<OpenWorldSession>().start.id.clone();
        let radius = world
            .resource::<WorldConfig<NovaLayeredWorld>>()
            .active_radius;
        let chart = world.resource::<Chart>();
        assert!(
            chart.station(&start).is_some(),
            "start roles: '{id}' chart does not hold the start station it docked at"
        );
        let stations: Vec<GeneratedBodyId> = chart
            .stations()
            .iter()
            .map(|station| station.id.clone())
            .collect();
        let jobs = world.resource::<Jobs>().clone();
        assert!(
            !jobs.held().is_empty() || !jobs.completed().is_empty(),
            "start roles: '{id}' saves with no job held or completed"
        );
        let postings = world.resource::<JobPostings>().clone();
        assert!(
            !postings.is_empty(),
            "start roles: '{id}' saves with no board posted"
        );
        let next = world
            .resource::<RolesWatch>()
            .next
            .expect("start roles: the crossing recorded its cell");
        let home = SectorCoord::new(next.x - 1, next.y, next.z);
        let retired: BTreeSet<SectorCoord> = desired_sectors(home, radius)
            .difference(&desired_sectors(next, radius))
            .copied()
            .collect();
        assert!(
            !retired.is_empty(),
            "start roles: '{id}' crossing retired no cell"
        );
        info!(
            "start roles: {id} saved {} station(s), {} held and {} completed job(s), {} retired \
             cell(s)",
            stations.len(),
            jobs.held().len(),
            jobs.completed().len(),
            retired.len()
        );
        world.resource_mut::<RolesWatch>().saved = Some(SavedRecords {
            start,
            stations,
            jobs,
            postings,
            retired,
        });
    }

    /// Load last save brought the records back: the chart's stations, the
    /// jobs, the boards and the retired cells are the crossing save's, and
    /// the play clock is past the crossing's. A `Cleanup` run behind the
    /// restore would have emptied every one of them and restarted the clock.
    fn assert_the_records(world: &mut World, id: &str) {
        let saved = world
            .resource_mut::<RolesWatch>()
            .saved
            .take()
            .expect("start roles: the crossing save was recorded");
        let stations: Vec<GeneratedBodyId> = world
            .resource::<Chart>()
            .stations()
            .iter()
            .map(|station| station.id.clone())
            .collect();
        let lost: Vec<&GeneratedBodyId> = saved
            .stations
            .iter()
            .filter(|station| !stations.contains(station))
            .collect();
        assert!(
            lost.is_empty(),
            "start roles: '{id}' chart after the load lost the stations {lost:?}"
        );
        assert_eq!(
            *world.resource::<Jobs>(),
            saved.jobs,
            "start roles: '{id}' jobs after the load"
        );
        assert_eq!(
            world.resource::<JobPostings>().board(&saved.start),
            saved.postings.board(&saved.start),
            "start roles: '{id}' start station board after the load"
        );
        let clock = world.resource::<WorldClock>().play_seconds;
        let crossed_at = world.resource::<RolesWatch>().clock_at_crossing;
        assert!(
            clock >= crossed_at,
            "start roles: '{id}' play clock {clock:.1} s after the load is behind the crossing's \
             {crossed_at:.1} s"
        );
        let frozen: BTreeSet<SectorCoord> = world
            .resource::<FrozenSectors>()
            .iter()
            .map(|(coord, _)| coord)
            .collect();
        let forgotten: Vec<&SectorCoord> = saved.retired.difference(&frozen).collect();
        assert!(
            forgotten.is_empty(),
            "start roles: '{id}' ledger after the load lost the retired cells {forgotten:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: load last save keeps the saved records",
            serde_json::json!({
                "role": id,
                "stations": stations.len(),
                "held": saved.jobs.held().len(),
                "completed": saved.jobs.completed().len(),
                "retired": saved.retired.len(),
                "clock": clock,
            }),
        );
        info!("start roles: {id} kept its records through the load");
    }

    /// The design section ids of `root`, in design order.
    fn design_ids(world: &World, root: Entity) -> Vec<String> {
        world
            .get::<ShipDesignSections>(root)
            .expect("start roles: a role's ship carries its design sections")
            .0
            .iter()
            .map(|section| section.id.clone())
            .collect()
    }

    /// The live section of `root` that wears design id `id`.
    fn live_section(world: &mut World, root: Entity, id: &str) -> Entity {
        let mut sections =
            world.query_filtered::<(Entity, &ChildOf, &EntityId), With<SectionMarker>>();
        sections
            .iter(world)
            .find(|(_, parent, worn)| parent.parent() == root && worn.0 == id)
            .map(|(entity, ..)| entity)
            .unwrap_or_else(|| panic!("start roles: section '{id}' is not live on the player"))
    }

    /// The player is role `id`'s ship: the hull's design sections, the kit's
    /// credits and hold, and every weapon on its key. Returns the bound
    /// weapon count.
    fn assert_the_ship(world: &mut World, id: &str) -> usize {
        let role = role_of(world, id);
        let player = player(world);
        let hull: Vec<String> = world
            .resource::<GameShipDesigns>()
            .get_design(&role.hull)
            .unwrap_or_else(|| panic!("start roles: '{id}' names a missing hull"))
            .design
            .sections
            .iter()
            .map(|section| section.id.clone())
            .collect();
        assert_eq!(
            design_ids(world, player),
            hull,
            "start roles: '{id}' must spawn the sections of '{}'",
            role.hull
        );

        let credits = world
            .get::<ShipCredits>(player)
            .expect("start roles: the player carries a balance")
            .0;
        assert_eq!(credits, role.kit.credits, "start roles: '{id}' kit credits");
        let held: BTreeMap<String, u32> = world
            .get::<ShipInventory>(player)
            .expect("start roles: the player carries a hold")
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect();
        let kit: BTreeMap<String, u32> = role
            .kit
            .stock
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect();
        assert_eq!(held, kit, "start roles: '{id}' kit stock");

        let trigger = vec![
            InputSource::Mouse(MouseButton::Left),
            InputSource::Gamepad(GamepadButton::RightTrigger2),
        ];
        let weapons: Vec<(String, SectionKind)> = world
            .get::<ShipDesignSections>(player)
            .expect("start roles: the player carries its design sections")
            .0
            .iter()
            .map(|section| (section.id.clone(), section.config.kind.clone()))
            .collect();
        let mut bound = 0;
        for (section_id, kind) in weapons {
            let section = live_section(world, player, &section_id);
            let (found, expected) = match kind {
                SectionKind::Turret(_) => (
                    world
                        .get::<SpaceshipTurretInputBinding>(section)
                        .map(|binding| binding.0.clone()),
                    trigger.clone(),
                ),
                SectionKind::Railgun(_) => (
                    world
                        .get::<SpaceshipRailgunInputBinding>(section)
                        .map(|binding| binding.0.clone()),
                    vec![InputSource::Keyboard(KeyCode::KeyR)],
                ),
                SectionKind::Torpedo(_) => (
                    world
                        .get::<SpaceshipTorpedoInputBinding>(section)
                        .map(|binding| binding.0.clone()),
                    vec![InputSource::Keyboard(KeyCode::KeyF)],
                ),
                SectionKind::Mining(_) => (
                    world
                        .get::<SpaceshipMiningInputBinding>(section)
                        .map(|binding| binding.0.clone()),
                    vec![InputSource::Keyboard(KeyCode::KeyV)],
                ),
                _ => continue,
            };
            assert_eq!(
                found,
                Some(expected),
                "start roles: '{id}' section '{section_id}' binding"
            );
            bound += 1;
        }
        bound
    }

    /// Create started role `id` and its first save landed; the ship is the
    /// role's and passes the post-build checks. Logs the hull price and the
    /// kit value at ask.
    fn assert_the_start(world: &mut World, id: &str) {
        let session = world.resource::<OpenWorldSession>().clone();
        assert_eq!(
            session.role.as_str(),
            id,
            "start roles: Create must start the picked role"
        );
        assert_eq!(session.seed, STATION_SEED, "start roles: Create's seed");
        // The charter follows the role: every role carries a goal, so the
        // Map's CHARTER block reads its title from the first frame.
        let goal_title = role_of(world, id).goal.title;
        let progress = world.resource::<GoalProgress>();
        assert_eq!(
            progress.title, goal_title,
            "start roles: '{id}' charter progress follows the role's goal"
        );
        nova_probe::probe_marker(
            world,
            "outcome: create starts the picked start role",
            serde_json::json!({ "role": id, "charter": goal_title }),
        );

        let bound = assert_the_ship(world, id);
        nova_probe::probe_marker(
            world,
            "outcome: the start role spawns its hull with its kit and its weapons bound",
            serde_json::json!({ "role": id, "bound": bound }),
        );

        let player = player(world);
        let design = world
            .get::<ShipDesignSections>(player)
            .expect("start roles: the player carries its design sections");
        let placed: Vec<RefitPlacedSection<'_>> = design
            .0
            .iter()
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
            })
            .collect();
        let fault = ship_build_fault(&placed);
        assert_eq!(fault, None, "start roles: '{id}' fails a post-build check");
        let items = world.resource::<GameItems>();
        let hull_cr = hull_price_cr(items, design.0.iter().map(|s| &s.config.base.build));
        let role = role_of(world, id);
        let stock_cr: u64 = role
            .kit
            .stock
            .stacks()
            .map(|(item, count)| u64::from(count) * u64::from(items.design(item).ask_cr))
            .sum();
        let kit_cr = u64::from(role.kit.credits) + stock_cr;
        let sections = design.0.len();
        nova_probe::probe_marker(
            world,
            "outcome: the start hull passes the post-build checks",
            serde_json::json!({ "role": id, "sections": sections }),
        );
        info!(
            "start roles: {id} starts {}: {sections} sections, {bound} bound weapons, hull \
             {hull_cr} cr, kit {kit_cr} cr at ask",
            role.hull
        );
    }

    /// The start docked the player at the session's starting station: a
    /// living station on the player's side. Record it and the live station
    /// set.
    fn choose_the_station(world: &mut World, id: &str) {
        let player = player(world);
        let start = world.resource::<OpenWorldSession>().start.id.clone();
        let connection = world
            .get::<DockedShip>(player)
            .expect("start roles: the start docks the player")
            .connection;
        let station = world
            .get::<DockingConnection>(connection)
            .map(|connection| {
                if connection.first_ship == player {
                    connection.second_ship
                } else {
                    connection.first_ship
                }
            })
            .expect("start roles: a docked ship's connection stands");
        let body = world
            .get::<GeneratedBodyId>(station)
            .expect("start roles: the dock partner is a generated body")
            .clone();
        assert_eq!(
            body, start,
            "start roles: {id} starts docked at the session's starting station"
        );
        assert!(
            world.get::<StationMarker>(station).is_some()
                && world.get::<DerelictShipMarker>(station).is_none(),
            "start roles: {id} starts at a living station"
        );
        assert_eq!(
            world.get::<Allegiance>(station),
            Some(&Allegiance::Player),
            "start roles: {id} starts at a station on the player's side"
        );
        let mut station_ids: Vec<GeneratedBodyId> = world
            .query_filtered::<&GeneratedBodyId, With<StationMarker>>()
            .iter(world)
            .cloned()
            .collect();
        station_ids.sort();
        station_ids.dedup();
        assert!(
            !station_ids.is_empty(),
            "start roles: {id} has no live generated stations"
        );
        info!("start roles: {id} starts docked at '{body}'");
        let mut watch = world.resource_mut::<RolesWatch>();
        watch.station = Some(station);
        assert!(
            watch
                .stations_by_role
                .insert(id.to_string(), station_ids)
                .is_none(),
            "start roles: {id} station set was recorded twice"
        );
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("start roles: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("start roles: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The dock took: report it and let go.
    fn report_the_dock(world: &mut World, id: &str) {
        let player = player(world);
        let station = world.resource::<RolesWatch>().station;
        let connection = world
            .get::<DockedShip>(player)
            .map(|docked| docked.connection);
        let body = world.resource::<OpenWorldSession>().start.id.to_string();
        info!("start roles: {id} docked at {station:?} ('{body}') through {connection:?}");
        nova_probe::probe_marker(
            world,
            "outcome: the start hull docks at a generated station",
            serde_json::json!({ "role": id, "station": body }),
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// The player's live cargo intakes, lowest first.
    fn the_intakes(world: &mut World, root: Entity) -> Vec<Entity> {
        let mut intakes = world.query_filtered::<(Entity, &ChildOf), (
            With<CargoIntakeSectionMarker>,
            Without<SectionInactiveMarker>,
        )>();
        let mut found: Vec<Entity> = intakes
            .iter(world)
            .filter(|(_, parent)| parent.parent() == root)
            .map(|(intake, _)| intake)
            .collect();
        found.sort();
        found
    }

    /// The first stack of role `id`'s kit.
    fn kit_item(world: &World, id: &str) -> ItemDesignId {
        role_of(world, id)
            .kit
            .stock
            .stacks()
            .next()
            .map(|(item, _)| item.clone())
            .unwrap_or_else(|| panic!("start roles: '{id}' starts with an empty hold"))
    }

    /// Jettison one unit of the kit's first item through the player's first
    /// intake, as the Inventory pane's Jettison applies its plan.
    fn jettison_one_item(world: &mut World, id: &str) {
        let player = player(world);
        let intake = *the_intakes(world, player)
            .first()
            .unwrap_or_else(|| panic!("start roles: '{id}' has no live cargo intake"));
        let item = kit_item(world, id);
        let docked = world.get::<DockedShip>(player).is_some();
        let own = world
            .get::<ShipInventory>(player)
            .expect("start roles: the player carries a hold")
            .clone();
        let plan = plan_item_jettison(
            world.resource::<GameItems>(),
            docked,
            true,
            None,
            &item,
            Some(1),
            &own,
        )
        .unwrap_or_else(|refusal| panic!("start roles: '{id}' jettison refused: {refusal:?}"));
        world
            .get_mut::<ShipInventory>(player)
            .expect("start roles: the player carries a hold")
            .remove(&item, plan.count);
        world
            .entity_mut(intake)
            .insert(CargoIntakeEjectionQueue(plan.canisters.into()));
        let strays: Vec<Entity> = world
            .query_filtered::<Entity, With<CargoCanister>>()
            .iter(world)
            .collect();
        let mut watch = world.resource_mut::<RolesWatch>();
        watch.intake = Some(intake);
        watch.strays = strays;
        watch.canister = None;
        watch.ejected = 0;
        watch.taken = 0;
    }

    /// The canister the jettison dropped: new since the jettison, holding
    /// one unit of the kit's first item.
    fn find_the_canister(world: &mut World, id: &str) {
        let item = kit_item(world, id);
        let strays = world.resource::<RolesWatch>().strays.clone();
        let mut canisters = world.query::<(Entity, &CargoCanister)>();
        let found: Vec<Entity> = canisters
            .iter(world)
            .filter(|(canister, held)| {
                !strays.contains(canister)
                    && held
                        .stacks()
                        .map(|(i, n)| (i.clone(), n))
                        .eq([(item.clone(), 1)])
            })
            .map(|(canister, _)| canister)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "start roles: '{id}' must drop exactly one canister of one {item}"
        );
        world.resource_mut::<RolesWatch>().canister = found.first().copied();
    }

    /// Disclosed fixture move: the jettisoned canister into the centre of
    /// the watched intake's trigger, square to its face and moving with the
    /// ship, once per frame until the intake takes it.
    fn put_the_canister_in_the_intake(world: &mut World) {
        let player = player(world);
        let watch = world.resource::<RolesWatch>();
        let (Some(intake), Some(canister)) = (watch.intake, watch.canister) else {
            return;
        };
        if world.get_entity(canister).is_err() {
            return;
        }
        let (local_at, local_rot) = local_pose(world, intake, player);
        let position = world
            .get::<Position>(player)
            .expect("a player ship stands")
            .0;
        let rotation = world
            .get::<Rotation>(player)
            .expect("a player ship stands")
            .0;
        let velocity = world
            .get::<LinearVelocity>(player)
            .map_or(Vec3::ZERO, |v| v.0);
        let collider = *world
            .get::<SectionCollider>(intake)
            .expect("start roles: an intake has a collider");
        let gap = world
            .get::<CargoIntakeSectionConfigHelper>(intake)
            .expect("start roles: an intake keeps its config")
            .capture_gap
            .to_engine();
        let intake_rot = rotation * local_rot;
        let (face, normal) =
            cargo_intake_face(position + rotation * local_at, intake_rot, collider);
        world.entity_mut(canister).insert((
            Position(face + normal * (gap * 0.5)),
            Rotation(intake_rot),
            LinearVelocity(velocity),
            AngularVelocity(Vec3::ZERO),
        ));
    }

    /// The watched intake took a canister and the jettisoned one is gone.
    fn taken_back(world: &World) -> bool {
        let watch = world.resource::<RolesWatch>();
        watch.taken >= 1
            && watch
                .canister
                .is_some_and(|canister| world.get_entity(canister).is_err())
    }

    fn report_the_take(world: &mut World, id: &str) {
        let player = player(world);
        let item = kit_item(world, id);
        let held = world
            .get::<ShipInventory>(player)
            .expect("start roles: the player carries a hold")
            .count(&item);
        let kit = role_of(world, id)
            .kit
            .stock
            .stacks()
            .find_map(|(kit_item, count)| (*kit_item == item).then_some(count))
            .unwrap_or_default();
        // The item's count only: the intake may also take a stray canister
        // that drifts into it, and a stray holds the world's items.
        assert_eq!(held, kit, "start roles: '{id}' holds its kit {item} again");
        let watch = world.resource::<RolesWatch>();
        let (ejected, taken, strays) = (watch.ejected, watch.taken, watch.strays.len());
        nova_probe::probe_marker(
            world,
            "outcome: the start hull jettisons and takes an item back in",
            serde_json::json!({ "role": id, "item": item.to_string(), "taken": taken }),
        );
        info!(
            "start roles: {id} jettisoned and took back one {item}: {ejected} dropped, {taken} \
             taken, {strays} stray canisters left alone"
        );
        let mut watch = world.resource_mut::<RolesWatch>();
        watch.intake = None;
        watch.canister = None;
        watch.strays.clear();
    }

    /// Disclosed fixture state: one credit the save never saw.
    fn add_an_unsaved_credit(world: &mut World) {
        let player = player(world);
        world
            .get_mut::<ShipCredits>(player)
            .expect("start roles: the player carries a balance")
            .0 += 1;
        world.resource_mut::<RolesWatch>().before = Some(player);
    }

    /// Load last save brought role `id` back from disk.
    fn assert_the_reload(world: &mut World, id: &str) {
        let role = world.resource::<OpenWorldSession>().role.clone();
        assert_eq!(
            role.as_str(),
            id,
            "start roles: the save must carry the role"
        );
        // The kit's credits, not the extra one: the ship came from disk.
        let bound = assert_the_ship(world, id);
        nova_probe::probe_marker(
            world,
            "outcome: the start role survives a save and a load",
            serde_json::json!({ "role": id, "bound": bound }),
        );
        info!("start roles: {id} came back from its save");
        let mut watch = world.resource_mut::<RolesWatch>();
        watch.before = None;
        watch.started.push(id.to_string());
    }

    /// Press a key until `landed`, then let it go.
    fn tap_key(
        script: AutopilotPlugin<GameStates>,
        label: String,
        key: KeyCode,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("{label}: press"))
            .on_enter(press_key(key))
            .until(landed)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("{label}: let go"))
            .on_enter(release_key(key))
            .add()
    }

    /// One role's start, dock, save round trip and leave.
    fn role_walk(
        script: AutopilotPlugin<GameStates>,
        id: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        let label = |beat: &str| format!("start roles {id}: {beat}");
        let script = script
            .step(label("reach the main menu"))
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                &label("click New Game"),
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step(label("name the world and enter the seed"))
            .on_enter(move |world: &mut World| {
                set_field(world, WORLD_NAME_FIELD, &format!("Role {id}"));
                set_field(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                &label("click Next"),
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                &label("pick the role"),
                &format!("Start Role Button: {id}"),
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                &label("click Create"),
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step(label("the world arms and saves"))
            .until(armed_and_saved())
            .deadline(SESSION_SECS)
            .add()
            .step(label("assert the start"))
            .on_enter(move |world: &mut World| assert_the_start(world, id))
            .add()
            .step(label(
                "the station window streams in and the start docks the player",
            ))
            .until(and(window_streamed(), player_docked()))
            .deadline(SESSION_SECS)
            .add()
            .step(label("assert the docked station is the start"))
            .on_enter(move |world: &mut World| choose_the_station(world, id))
            .add()
            .step(label("accept the board's jobs"))
            .on_enter(move |world: &mut World| accept_the_board(world, id))
            .until(board_accepted())
            .deadline(BEAT_SECS)
            .add()
            .step(label("complete one held job"))
            .on_enter(move |world: &mut World| complete_one_job(world, id))
            .add()
            .step(label("report the dock and undock"))
            .on_enter(move |world: &mut World| report_the_dock(world, id))
            .until(player_free_and_saved())
            .deadline(SESSION_SECS)
            .add()
            .step(label("cross into the next cell"))
            .on_enter(move |world: &mut World| cross_one_cell(world, id))
            .until(crossing_saved())
            .deadline(SESSION_SECS)
            .add()
            .step(label("record the crossing save"))
            .on_enter(move |world: &mut World| record_the_save(world, id))
            .add()
            .step(label("jettison one kit item"))
            .on_enter(move |world: &mut World| jettison_one_item(world, id))
            .until(when(|world| world.resource::<RolesWatch>().ejected == 1))
            .deadline(SESSION_SECS)
            .add()
            .step(label("find the canister"))
            .on_enter(move |world: &mut World| find_the_canister(world, id))
            .add()
            .step(label("put the canister in the intake"))
            .each(|world, _, _| put_the_canister_in_the_intake(world))
            .until(when(taken_back))
            .deadline(SESSION_SECS)
            .add()
            .step(label("report the take"))
            .on_enter(move |world: &mut World| report_the_take(world, id))
            .add()
            .step(label("add an unsaved credit"))
            .on_enter(add_an_unsaved_credit)
            .until(frames(1))
            .add();
        let script = tap_key(
            script,
            label("ESC for Load last save"),
            KeyCode::Escape,
            ui_node_present(RETRY_BUTTON),
        )
        .click_named(
            &label("click Load last save"),
            RETRY_BUTTON,
            pointer_released(),
            BEAT_SECS,
        )
        .step(label("the saved world arms around a new player ship"))
        .until(armed_and_saved())
        .deadline(SESSION_SECS)
        .add()
        .step(label("assert the reload"))
        .on_enter(move |world: &mut World| assert_the_reload(world, id))
        .add()
        .step(label("the reloaded window streams in"))
        .until(window_streamed())
        .deadline(SESSION_SECS)
        .add()
        .step(label("assert the records survived the load"))
        .on_enter(move |world: &mut World| assert_the_records(world, id))
        .add();
        tap_key(
            script,
            label("ESC for the menu"),
            KeyCode::Escape,
            ui_node_present(BACK_TO_MENU_BUTTON),
        )
        .click_named(
            &label("click Back to Main Menu"),
            BACK_TO_MENU_BUTTON,
            ui_node_present(NEW_GAME_BUTTON),
            SESSION_SECS,
        )
    }

    /// The walk: the catalogue check, then every role in turn.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("start roles: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .step("start roles: the catalogue holds the walked roles")
            .on_enter(|world: &mut World| {
                let loaded: Vec<String> = world
                    .resource::<GameStartRoles>()
                    .0
                    .iter()
                    .map(|role| role.id.as_str().to_string())
                    .collect();
                assert_eq!(loaded, ROLES, "start roles: the base role catalogue");
            })
            .add();
        let script = ROLES.into_iter().fold(script, role_walk);
        script
            .step("start roles: report")
            .on_enter(|world: &mut World| {
                let (started, stations_by_role) = {
                    let watch = world.resource::<RolesWatch>();
                    (watch.started.clone(), watch.stations_by_role.clone())
                };
                assert_eq!(started, ROLES, "start roles: every role started");
                nova_probe::probe_marker(
                    world,
                    "outcome: every start role started",
                    serde_json::json!({ "roles": started }),
                );
                let first = stations_by_role
                    .get(ROLES[0])
                    .expect("start roles: the first role's stations were recorded");
                assert_eq!(
                    stations_by_role.len(),
                    ROLES.len(),
                    "start roles: every role recorded its station set"
                );
                for role in ROLES {
                    let stations = stations_by_role
                        .get(role)
                        .unwrap_or_else(|| panic!("start roles: {role} has no station set"));
                    assert!(
                        !stations.is_empty(),
                        "start roles: {role} streamed no generated stations"
                    );
                    assert_eq!(
                        stations, first,
                        "start roles: {role} streams the first role's stations on seed {STATION_SEED}"
                    );
                }
                let station_ids: BTreeMap<String, Vec<String>> = stations_by_role
                    .iter()
                    .map(|(role, stations)| {
                        (
                            role.clone(),
                            stations.iter().map(ToString::to_string).collect(),
                        )
                    })
                    .collect();
                nova_probe::probe_marker(
                    world,
                    "outcome: every start role streams the same stations from one seed",
                    serde_json::json!({ "seed": STATION_SEED, "stations_by_role": station_ids }),
                );
                info!("start roles: PASS every role streams the same stations {first:?}");
                info!("start roles: PASS every start role started");
            })
            .add()
    }
}
