//! system_station_swap_intake: a generated station whose civilization knows
//! the wide intake offers it as a swap for the base cargo intake, and the
//! swap leaves the hold as it was.
//!
//! A generated yard stands in a void, Neutral and living, at advancement
//! 0.35: past the 0.3 the wide intake opens at. A civilization knows a part
//! above advancement 0 on a 50 % draw, so the range takes the first station
//! seed whose build list names the wide intake. The player's hull is a
//! controller over a hull block (the hold), a port on its nose, a drive on
//! its tail and a base cargo intake on the controller's top. THE HOLD IS
//! DISCLOSED FIXTURE STATE: iron ore, carbon ore and a medkit, none of them a
//! part of the wide intake's recipe, so the station sells every recipe part
//! at ask and the hold has nothing to pay with. The hold is the
//! `ShipInventory` on the ship root, sized by the hull sections alone, so an
//! intake swap has no claim on it. The player berths at the station by the
//! open world's own berth rule, docks, selects the intake in the Ship pane,
//! opens the Catalogue, narrows it to Structure, clicks the wide intake's
//! tile and Swap on its card.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: a generated station offers the wide intake as a swap` | the station's build list names the wide intake; its catalogue entry is placeable and offers `Swap <code>` at the price the payment rule plans |
//! | 1 | `outcome: an intake swap keeps the hold` | the hold's stacks and capacity are as they were; the mounted section is the wide intake at full health under its old code; the player's credits fell by the quoted price, which reached the station; `OnSwapped` fired once |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_swap_intake --features debug
//! # look for: `station swap intake probe: PASS the catalogue offers the wide intake`,
//! #           `station swap intake probe: PASS the swap kept the hold`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use generated_station::generated_station_design;
use nova_protocol::prelude::*;

#[path = "../shared/generated_station.rs"]
mod generated_station;

#[derive(Parser)]
#[command(name = "system_station_swap_intake")]
#[command(version = "1.0.0")]
#[command(about = "A cargo intake is swapped for the wide intake at a generated station that builds it, and the hold comes out as it went in. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The station object id.
const STATION_ID: &str = "yard";
/// The scenario id the void loads under.
const SCENARIO_ID: &str = "station_swap_intake_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// The civilization's advancement: past the 0.3 the wide intake opens at,
/// and the yard `system_station_orbit` stands, with a clearance under 170 m.
const STATION_ADVANCEMENT: f32 = 0.35;
/// How many station seeds the range tries for a civilization that knows the
/// wide intake. Each knows it on an even draw.
const STATION_SEEDS: u32 = 32;

/// Where the player spawns, clear of the yard's clearance; the walk berths
/// it.
const PLAYER_POSITION: Meters3 = Meters3::new(0.0, 0.0, 400.0);

/// The intake's design section id, and the two parts. `nova_ship` exports no
/// id for either: fixture ids stay local.
const INTAKE: &str = "intake";
const CARGO_INTAKE_SECTION_ID: &str = "cargo_intake_section";
const WIDE_INTAKE_SECTION_ID: &str = "wide_intake_section";

/// The player's balance, over any ask for the wide intake's parts.
const PLAYER_CREDITS: u32 = 5_000;

/// The cargo: nothing the wide intake's recipe names.
const CARGO: [(&str, u32); 3] = [("IronOre", 6), ("CarbonOre", 3), ("Medkit", 1)];

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // The fleet's 1920x1080, the size the catalogue gallery is walked at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::SwapWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

/// Stand the first yard whose civilization knows the wide intake.
///
/// # Panics
///
/// When no seed under [`STATION_SEEDS`] builds it: the range has no station
/// to swap at.
fn setup_void(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    loaded: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
) {
    let (seed, yard) = (0..STATION_SEEDS)
        .map(|seed| {
            let (_, yard) = generated_station_design(
                &loaded,
                &items,
                StationArchetypeType::Yard,
                STATION_ADVANCEMENT,
                seed,
            );
            (seed, yard)
        })
        .find(|(_, yard)| yard.builds.iter().any(|id| id == WIDE_INTAKE_SECTION_ID))
        .unwrap_or_else(|| {
            panic!(
                "station swap intake: no yard of seeds 0..{STATION_SEEDS} at advancement \
                 {STATION_ADVANCEMENT} builds '{WIDE_INTAKE_SECTION_ID}'"
            )
        });
    info!(
        "station swap intake probe: the yard of seed {seed} builds {} parts, the wide intake \
         among them",
        yard.builds.len()
    );
    commands.trigger(LoadScenario(station_void(&game_assets, yard)));
}

/// The void: the generated `yard` and the player clear of it, lit by a
/// three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets, yard: StationDesign) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: "YARD".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Inline(yard),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: PLAYER_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    ScenarioConfig {
        description: "A yard that builds the wide intake, and a player with a full hold."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(station),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("rig", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Swap Intake Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller over a hull block (the hold), a port on
/// its nose, a drive on its tail and a cargo intake on the controller's top.
/// The intake is three cells across and two high, so its centre stands a
/// cell and a half up and its lower middle socket meets the controller's
/// top face. Its door faces forward, over the port.
fn player_ship() -> SpaceshipConfig {
    let section = |id: &str, prototype: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                section("controller", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
                section("hold", REINFORCED_HULL_SECTION_ID, Vec3::NEG_Y),
                section("port", DOCKING_PORT_SECTION_ID, Vec3::NEG_Z),
                section("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
                section(INTAKE, CARGO_INTAKE_SECTION_ID, Vec3::new(0.0, 1.5, 0.0)),
            ],
            ..default()
        }),
        inventory: ShipInventoryStock::new(CARGO.map(|(item, count)| (item.into(), count))),
        credits: PLAYER_CREDITS,
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::{
            pane::InterfacePaneType,
            ship::{SectionCode, ShipRuntime},
        },
    };

    use super::*;

    /// Seconds a load, a pane or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;
    /// Frames the Ship pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct SwapWatch {
        /// Every swap event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The intake's entity and code before the swap.
        intake_before: Option<(Entity, String)>,
        /// The hold's stacks and capacity before the swap.
        hold_before: Option<(BTreeMap<String, u32>, u32)>,
        /// The station's balance before the swap.
        station_cr_before: Option<u32>,
        /// The price the payment rule plans, which the catalogue quotes.
        quoted_cr: Option<u32>,
    }

    pub(super) struct SwapWatchPlugin;

    impl Plugin for SwapWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<SwapWatch>();
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<SwapWatch>| {
                let name = event.name();
                if name != "onswapped" {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station swap intake probe: {name} {data}");
                watch.events.push((name.to_string(), data));
            });
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn station_root(world: &World) -> Option<Entity> {
        let mut stations =
            world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
        stations
            .iter(world)
            .find(|(_, id)| id.0 == STATION_ID)
            .map(|(entity, _)| entity)
    }

    fn player_root(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("station swap intake probe: the player is up")
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station swap intake probe: the station is up")
    }

    /// The live intake of the player's hull, or `None` while it has none.
    fn intake_of(world: &World) -> Option<Entity> {
        let player = player_root(world)?;
        world.get::<Children>(player)?.iter().find(|child| {
            world
                .get::<EntityId>(*child)
                .is_some_and(|id| id.0 == INTAKE)
        })
    }

    /// The hold of `root`: its stacks by item id, and its capacity.
    fn hold(world: &World, root: Entity) -> (BTreeMap<String, u32>, u32) {
        let hold = world
            .get::<ShipInventory>(root)
            .expect("station swap intake probe: every ship carries a hold");
        (
            hold.stacks()
                .map(|(item, count)| (item.to_string(), count))
                .collect(),
            hold.capacity_g(),
        )
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station swap intake probe: every ship carries a balance")
            .0
    }

    /// The prototype id the player's design resolves the intake to.
    fn resolved_intake(world: &World) -> String {
        world
            .get::<ShipDesignSections>(player(world))
            .and_then(|design| design.get(INTAKE))
            .expect("station swap intake probe: the design has the intake")
            .config
            .base
            .id
            .clone()
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

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station swap intake: press {action}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station swap intake: let {action} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// The design sections and the skin flag of `root`, as the berth rule
    /// reads them.
    fn design_of(world: &World, root: Entity) -> (&[DesignSection], bool) {
        let (Some(design), Some(&ShipSkin(skin))) = (
            world.get::<ShipDesignSections>(root),
            world.get::<ShipSkin>(root),
        ) else {
            panic!("station swap intake probe: {root:?} has no design sections or skin");
        };
        (&design.0, skin)
    }

    /// Put the player on the berth the open world's start takes at a
    /// station ([`outward_berth`]), at rest with its rig on the berth's
    /// heading, and ask for a dock. A no-op once the player is docked.
    fn berth_and_dock(world: &mut World) {
        let (player, station) = (player(world), station(world));
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let ((station_sections, station_skin), (ship_sections, ship_skin)) =
            (design_of(world, station), design_of(world, player));
        let (_, _, at, turn) = outward_berth(
            station_sections,
            station_skin,
            ship_sections,
            ship_skin,
            &[],
            &[],
        )
        .expect("station swap intake probe: the player stands clear at a port of the yard");
        let position = world
            .get::<Position>(station)
            .expect("station swap intake probe: the station stands")
            .0;
        let rotation = world
            .get::<Rotation>(station)
            .expect("station swap intake probe: the station stands")
            .0;
        let (at, turn) = (position + rotation * at, (rotation * turn).normalize());
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
        ));
        // Left on the spawn heading, the rig turns the hull off the berth.
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

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn intake_coded() -> Arc<Predicate> {
        when(|world| {
            intake_of(world).is_some_and(|intake| world.get::<SectionCode>(intake).is_some())
        })
    }

    /// Record the intake, the hold, the station's balance and the price the
    /// payment rule plans for the wide intake's recipe.
    fn record_the_dock(world: &mut World) {
        let (player, station) = (player(world), station(world));
        let intake = intake_of(world).expect("station swap intake probe: the intake is live");
        let code = world
            .get::<SectionCode>(intake)
            .expect("station swap intake probe: the pane coded the intake")
            .0
            .clone();
        assert_eq!(resolved_intake(world), CARGO_INTAKE_SECTION_ID);
        let hold_before = hold(world, player);
        let cargo: BTreeMap<String, u32> = CARGO
            .iter()
            .map(|&(item, count)| (item.to_string(), count))
            .collect();
        assert_eq!(
            hold_before.0, cargo,
            "station swap intake probe: the fixture hold carries its cargo"
        );
        let recipe = world
            .resource::<GameSections>()
            .get_section(WIDE_INTAKE_SECTION_ID)
            .expect("station swap intake probe: the catalog ships the wide intake")
            .base
            .build
            .clone();
        for part in recipe.parts.keys() {
            assert!(
                !cargo.contains_key(part.as_str()),
                "station swap intake probe: the cargo must not pay the recipe, it holds {part}"
            );
        }
        let payment = plan_recipe_payment(
            world.resource::<GameItems>(),
            &recipe,
            world
                .get::<ShipInventory>(station)
                .expect("station swap intake probe: the station carries a hold"),
            credits(world, station),
            world
                .get::<ShipInventory>(player)
                .expect("station swap intake probe: the player carries a hold"),
            credits(world, player),
        )
        .unwrap_or_else(|refusal| {
            panic!("station swap intake probe: the wide intake cannot be paid: {refusal:?}")
        });
        assert!(
            payment.from_hold.is_empty(),
            "station swap intake probe: the hold pays no part: {:?}",
            payment.from_hold
        );
        let station_cr = credits(world, station);
        let mut watch = world.resource_mut::<SwapWatch>();
        watch.intake_before = Some((intake, code));
        watch.hold_before = Some(hold_before);
        watch.station_cr_before = Some(station_cr);
        watch.quoted_cr = Some(payment.price_cr);
    }

    fn intake_selected(world: &World) -> bool {
        let selected = world.resource::<ShipRuntime>().selected();
        selected.is_some() && selected == intake_of(world)
    }

    /// Step the selection with the viewer key, one press and one release per
    /// two frames, until the intake is selected.
    fn step_to_the_intake(world: &mut World, _elapsed: f32, frame: u32) {
        if intake_selected(world) {
            release_action("viewer_next")(world);
            return;
        }
        if frame.is_multiple_of(2) {
            press_action("viewer_next")(world);
        } else {
            release_action("viewer_next")(world);
        }
    }

    /// The wide intake's tile in the catalogue gallery, named by its
    /// catalogue name.
    fn wide_intake_tile(world: &World) -> String {
        let sections = world.resource::<GameSections>();
        let wide = sections
            .get_section(WIDE_INTAKE_SECTION_ID)
            .expect("station swap intake: the catalogue lists the wide intake");
        format!("Gallery Tile {}", wide.base.name)
    }

    /// Claim 0: the station builds the wide intake and its catalogue entry
    /// offers the swap at the planned price.
    fn assert_the_offer(world: &mut World) {
        let station = station(world);
        let builds = world
            .get::<StationBuilds>(station)
            .expect("station swap intake probe: a refit station carries its build list")
            .0
            .clone();
        assert!(
            builds.iter().any(|id| id == WIDE_INTAKE_SECTION_ID),
            "station swap intake probe: the station builds the wide intake: {builds:?}"
        );
        let watch = world.resource::<SwapWatch>();
        let code = watch
            .intake_before
            .clone()
            .expect("station swap intake probe: the intake was recorded")
            .1;
        let quoted = watch
            .quoted_cr
            .expect("station swap intake probe: the price was recorded");
        let entry = world
            .query::<&GalleryListing>()
            .iter(world)
            .next()
            .expect("station swap intake probe: the open catalogue has a listing")
            .entry(WIDE_INTAKE_SECTION_ID)
            .expect("station swap intake probe: the catalogue lists the wide intake")
            .clone();
        assert!(
            entry.placeable,
            "station swap intake probe: the wide intake is on offer: {entry:?}"
        );
        let offer = format!("Swap {code}, {}", cr_text(quoted));
        assert_eq!(
            entry.alternate.as_deref(),
            Some(offer.as_str()),
            "station swap intake probe: the card offers the swap at the planned price"
        );
        info!("station swap intake probe: PASS the catalogue offers the wide intake: '{offer}'");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a generated station offers the wide intake as a swap",
            serde_json::json!({ "t": elapsed, "offer": offer, "builds": builds.len() }),
        );
    }

    fn intake_swapped() -> Arc<Predicate> {
        when(|world| {
            let before = world
                .resource::<SwapWatch>()
                .intake_before
                .as_ref()
                .map(|(entity, _)| *entity);
            intake_of(world).is_some_and(|intake| Some(intake) != before)
        })
    }

    /// Claim 1: the hold is as it was, the mounted section is the wide
    /// intake under the old code, and the quoted price moved to the station.
    fn assert_the_swap(world: &mut World) {
        let (player, station) = (player(world), station(world));
        let watch = world.resource::<SwapWatch>();
        let (_, code) = watch
            .intake_before
            .clone()
            .expect("station swap intake probe: the intake was recorded");
        let hold_before = watch
            .hold_before
            .clone()
            .expect("station swap intake probe: the hold was recorded");
        let station_cr = watch
            .station_cr_before
            .expect("station swap intake probe: the station was recorded");
        let quoted = watch
            .quoted_cr
            .expect("station swap intake probe: the price was recorded");
        let swapped: Vec<serde_json::Value> =
            watch.events.iter().map(|(_, data)| data.clone()).collect();

        let hold_after = hold(world, player);
        assert_eq!(
            hold_after, hold_before,
            "station swap intake probe: the swap must leave the hold's stacks and capacity"
        );
        let intake = intake_of(world).expect("station swap intake probe: the intake is live");
        assert_eq!(
            resolved_intake(world),
            WIDE_INTAKE_SECTION_ID,
            "station swap intake probe: the design resolves the intake to the wide intake"
        );
        assert_eq!(
            world.get::<SectionClass>(intake),
            Some(&SectionClass::CargoIntake),
            "station swap intake probe: the mounted section is a cargo intake"
        );
        let health = world
            .get::<Health>(intake)
            .expect("station swap intake probe: the intake has health");
        let wide_health = world
            .resource::<GameSections>()
            .get_section(WIDE_INTAKE_SECTION_ID)
            .expect("station swap intake probe: the catalog ships the wide intake")
            .base
            .health;
        assert!(
            health.max == wide_health && health.current == health.max,
            "station swap intake probe: the mounted intake has the wide intake's pool, whole: \
             {health:?}"
        );
        assert_eq!(
            world.get::<SectionCode>(intake).map(|code| code.0.as_str()),
            Some(code.as_str()),
            "station swap intake probe: the intake keeps its code"
        );
        assert_eq!(
            credits(world, player),
            PLAYER_CREDITS - quoted,
            "station swap intake probe: the player pays the quoted price"
        );
        assert_eq!(
            credits(world, station),
            station_cr + quoted,
            "station swap intake probe: the price reaches the station"
        );
        assert_eq!(swapped.len(), 1, "station swap intake probe: one OnSwapped");
        let event = swapped[0].clone();
        assert_eq!(event["section"], code);
        assert_eq!(event["from"], CARGO_INTAKE_SECTION_ID);
        assert_eq!(event["to"], WIDE_INTAKE_SECTION_ID);
        assert_eq!(event["price_cr"], quoted);

        info!(
            "station swap intake probe: PASS the swap kept the hold: {:?} in {} g, {code} is the \
             wide intake, {quoted} cr paid",
            hold_after.0, hold_after.1
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: an intake swap keeps the hold",
            serde_json::json!({
                "t": elapsed,
                "hold": hold_after.0,
                "capacity_g": hold_after.1,
                "price": quoted,
                "event": event,
            }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station swap intake: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                station_root(world).is_some_and(|station| {
                    world.get::<Position>(station).is_some()
                        && world.get::<ShipDesignSections>(station).is_some()
                }) && player_root(world).is_some_and(|player| {
                    world.get::<Position>(player).is_some()
                        && world.get::<ShipDesignSections>(player).is_some()
                })
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station swap intake: the pane codes the intake")
            .until(intake_coded())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap intake: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap intake: record the dock")
            .on_enter(record_the_dock)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "station swap intake: open the Ship pane",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        .step("station swap intake: settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station swap intake: select the intake")
        .each(step_to_the_intake)
        .until(when(intake_selected))
        .deadline(SESSION_SECS)
        .add()
        .step("station swap intake: let the viewer key up")
        .on_enter(release_action("viewer_next"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "station swap intake: open the catalogue",
            "ShipCatalogue",
            ui_node_present("Gallery Grid"),
            SESSION_SECS,
        )
        .step("station swap intake: narrow the catalogue to Structure")
        .on_enter(click_named(format!(
            "Gallery Category {}",
            GalleryCategory::Structure.label()
        )))
        .until(pointer_pressed())
        .deadline(SESSION_SECS)
        .add()
        .step("station swap intake: let the Structure chip up")
        .on_enter(release_mouse(MouseButton::Left))
        .until(when(|world| {
            world.resource::<GalleryState>().category == GalleryCategory::Structure
                && ui_node_present(wide_intake_tile(world))(world)
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("station swap intake: settle the Structure catalogue")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station swap intake: assert the offer")
        .on_enter(assert_the_offer)
        .until(frames(1))
        .add()
        .step("station swap intake: press the wide intake's tile")
        .on_enter(|world: &mut World| click_named(wide_intake_tile(world))(world))
        .until(pointer_pressed())
        .deadline(SESSION_SECS)
        .add()
        .step("station swap intake: release the wide intake's tile")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present("Gallery Alternate Button"))
        .deadline(SESSION_SECS)
        .add()
        .step("station swap intake: settle the card")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "station swap intake: swap the intake for the wide intake",
            "Gallery Alternate Button",
            intake_swapped(),
            SESSION_SECS,
        );
        // The pane pushed its report under the interface pause; the tracker
        // drains it on the first fixed tick after the clock runs again.
        tap(
            script,
            "interface_toggle",
            when(|world| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
            }),
        )
        .step("station swap intake: the swap reports")
        .until(when(|world| {
            !world.resource::<SwapWatch>().events.is_empty()
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("station swap intake: assert the swap")
        .on_enter(assert_the_swap)
        .add()
    }
}
