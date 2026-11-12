//! system_station_refine: ore and a fee are refined into an alloy plate at a
//! station that offers Refine, from the Inventory pane.
//!
//! A base waystation stands in a void; it offers Refine. The player's hull is
//! a controller over a hull block with a port on its nose and a drive on its
//! tail. THE HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: three iron
//! ore, one carbon ore and 100 cr, one iron ore more than one batch takes.
//! The player docks, opens the Inventory pane, clicks the iron ore row and
//! switches its form to Refine. The form states the whole batch. Confirm
//! takes two iron ore and the carbon ore, adds one alloy plate and moves the
//! 30 cr fee to the station. The iron ore row then opens Refine again, and
//! the form refuses it by the input the hold is short of.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: ore and a fee are refined into an alloy plate` | the form read the batch and the balance after it; the hold lost two iron ore and one carbon ore and gained one alloy plate; 30 cr moved from the player to the station; `OnCreditsChanged` fired once per party with cause `Refine` |
//! | 1 | `outcome: a refine short of an input is refused on the form` | the form keeps the batch line, names the carbon ore under it and disables Confirm; the hold and both balances are as the refine left them |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_refine --features debug
//! # look for: `station refine probe: PASS refined one alloy plate`,
//! #           `station refine probe: PASS a short refine is refused`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the Refine form
//! before Confirm as `station-refine-form.png`.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_station_refine")]
#[command(version = "1.0.0")]
#[command(about = "Two iron ore, one carbon ore and a 30 cr fee are refined into an alloy plate at a living station from the Inventory pane, and a short batch is refused on the form. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The waystation's park, down -Z from the player.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The station object id.
const STATION_ID: &str = "waystation";
/// The scenario id the void loads under.
const SCENARIO_ID: &str = "station_refine_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// The station is turned a quarter turn about +Y, so its collar, which stands
/// off its -X flank, faces +Z: back toward the player.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Where the player's hull parks relative to the station, engine units: the
/// turned collar's face stands 2.5 cells along +Z from the station origin and
/// the player's port face 1.5 ahead of its hull origin, [`BERTH_GAP`] apart.
const BERTH_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.0 + BERTH_GAP);

/// Item ids of the base refine recipe: two iron ore and one carbon ore make
/// one alloy plate.
const IRON: &str = "IronOre";
const CARBON: &str = "CarbonOre";
const ALLOY: &str = "AlloyPlate";

/// The disclosed fixture hold and balance: one batch and one iron ore over.
const HOLD_IRON: u32 = 3;
const HOLD_CARBON: u32 = 1;
const PLAYER_CREDITS: u32 = 100;

/// What one batch takes, as the base recipe authors it.
const BATCH_IRON: u32 = 2;
const BATCH_CARBON: u32 = 1;
const FEE: u32 = 30;

/// The PNG the capture path shoots of the Refine form.
const FORM_SHOT: &str = "station-refine-form.png";

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
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::RefineWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(station_void(&game_assets)));
}

/// The void: the living waystation and the player on its berth, lit by a
/// three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: "WAYSTATION".to_string(),
            position: STATION_POSITION,
            rotation: Quat::from_rotation_y(STATION_TURN),
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(BLOCK_WAYSTATION_STATION_ID.into()),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::from_engine(STATION_POSITION.to_engine() + BERTH_OFFSET),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    ScenarioConfig {
        description: "A waystation that refines, and a player with ore.".to_string(),
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
                ThreePointRig::around("rig", STATION_POSITION, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Refine Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller over a hull block (the hold), a port on
/// its nose and a drive on its tail, with the fixture hold and balance.
fn player_ship() -> SpaceshipConfig {
    let section =
        |id: &str, prototype: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation,
            source: SectionSource::prototype(prototype),
        };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                section(
                    "controller",
                    BASIC_CONTROLLER_SECTION_ID,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                ),
                section(
                    "hold",
                    REINFORCED_HULL_SECTION_ID,
                    Vec3::NEG_Y,
                    Quat::IDENTITY,
                ),
                section("port", DOCKING_PORT_SECTION_ID, Vec3::NEG_Z, Quat::IDENTITY),
                section("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z, Quat::IDENTITY),
            ],
            ..default()
        }),
        inventory: ShipInventoryStock::new([
            (IRON.into(), HOLD_IRON),
            (CARBON.into(), HOLD_CARBON),
        ]),
        credits: PLAYER_CREDITS,
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use bevy::ui::InteractionDisabled;
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds a load, a pane or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;
    /// How close a teleported hull must settle to its berth, and how slow.
    const BERTH_TOLERANCE: f32 = 0.05;
    const BERTH_REST: f32 = 0.01;
    /// Frames the Inventory pane gets to build its rows and its form.
    const SETTLE_FRAMES: u32 = 10;

    const IRON_ROW: &str = "InventoryRowOwnIronOre";
    const REFINE_CHIP: &str = "InventoryDraftRefine";
    const DRAFT_SUMMARY: &str = "InventoryDraftSummary";
    const CONFIRM: &str = "InventoryDraftConfirm";

    /// The batch line the form states for one batch of the base recipe.
    const BATCH_LINE: &str = "2 Iron ore + 1 Carbon ore + 30 cr -> 1 Alloy plate";

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct RefineWatch {
        /// Every balance event, with its payload.
        credit_events: Vec<serde_json::Value>,
        /// The station's balance before the refine.
        station_before: Option<u32>,
    }

    pub(super) struct RefineWatchPlugin;

    impl Plugin for RefineWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<RefineWatch>();
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<RefineWatch>| {
                if event.name() != "oncreditschanged" {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station refine probe: oncreditschanged {data}");
                watch.credit_events.push(data);
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
        player_root(world).expect("station refine probe: the player is up")
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station refine probe: the station is up")
    }

    fn held(world: &World, root: Entity, item: &str) -> u32 {
        world
            .get::<ShipInventory>(root)
            .expect("station refine probe: every ship carries a hold")
            .count(&item.into())
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station refine probe: every ship carries a balance")
            .0
    }

    /// The player's iron ore, carbon ore and alloy plates.
    fn hold(world: &World) -> (u32, u32, u32) {
        let player = player(world);
        (
            held(world, player, IRON),
            held(world, player, CARBON),
            held(world, player, ALLOY),
        )
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

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station refine: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station refine: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Park the player's hull on the berth, at rest.
    fn berth_player(world: &mut World) {
        let berth = world
            .get::<Position>(station(world))
            .expect("station refine probe: the station has a physics position")
            .0
            + BERTH_OFFSET;
        let player = player(world);
        world.entity_mut(player).insert((
            Position(berth),
            Rotation(Quat::IDENTITY),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
        ));
    }

    fn player_berthed() -> Arc<Predicate> {
        when(|world| {
            let (Some(player), Some(station)) = (player_root(world), station_root(world)) else {
                return false;
            };
            let (Some(position), Some(velocity), Some(station_at)) = (
                world.get::<Position>(player),
                world.get::<LinearVelocity>(player),
                world.get::<Position>(station),
            ) else {
                return false;
            };
            position.0.distance(station_at.0 + BERTH_OFFSET) < BERTH_TOLERANCE
                && velocity.0.length() < BERTH_REST
        })
    }

    fn request_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
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

    /// The station offers Refine; record its balance.
    fn record_the_station(world: &mut World) {
        let station = station(world);
        let services = world
            .get::<StationServices>(station)
            .expect("station refine probe: a station root carries its services");
        assert!(
            services.0.contains(&StationServiceType::Refine),
            "station refine probe: the base waystation must offer Refine: {services:?}"
        );
        assert_eq!(hold(world), (HOLD_IRON, HOLD_CARBON, 0));
        assert_eq!(credits(world, player(world)), PLAYER_CREDITS);
        let before = credits(world, station);
        world.resource_mut::<RefineWatch>().station_before = Some(before);
    }

    fn node_text(world: &mut World, name: &str) -> String {
        let mut texts = world.query::<(&Name, &Text)>();
        texts
            .iter(world)
            .find(|(found, _)| found.as_str() == name)
            .map(|(_, text)| text.0.clone())
            .unwrap_or_else(|| panic!("station refine probe: no text named '{name}'"))
    }

    fn confirm_disabled(world: &mut World) -> bool {
        let mut buttons = world.query::<(&Name, Has<InteractionDisabled>)>();
        buttons
            .iter(world)
            .find(|(found, _)| found.as_str() == CONFIRM)
            .map(|(_, disabled)| disabled)
            .expect("station refine probe: the form has a Confirm")
    }

    /// The form states the whole batch and the balance after it.
    fn assert_the_form(world: &mut World) {
        let summary = node_text(world, DRAFT_SUMMARY);
        assert_eq!(
            summary,
            format!("{BATCH_LINE}\nBalance {} cr", PLAYER_CREDITS - FEE),
            "station refine probe: the form states the batch"
        );
        assert!(
            !confirm_disabled(world),
            "station refine probe: Confirm is enabled on a batch the hold covers"
        );
    }

    fn refine_reported() -> Arc<Predicate> {
        when(|world| world.resource::<RefineWatch>().credit_events.len() >= 2)
    }

    /// Claim 0: the inputs left, the plate arrived, the fee moved, and both
    /// parties reported it.
    fn assert_the_refine(world: &mut World) {
        let station = station(world);
        assert_eq!(
            hold(world),
            (HOLD_IRON - BATCH_IRON, HOLD_CARBON - BATCH_CARBON, 1),
            "station refine probe: one batch left the hold and one plate entered it"
        );
        assert_eq!(credits(world, player(world)), PLAYER_CREDITS - FEE);
        let station_before = world
            .resource::<RefineWatch>()
            .station_before
            .expect("station refine probe: the station was recorded");
        assert_eq!(
            credits(world, station),
            station_before + FEE,
            "station refine probe: the fee goes to the station"
        );
        let events = world.resource::<RefineWatch>().credit_events.clone();
        // The tracker drains each party's reports in entity order, so the
        // two events are compared sorted.
        let mut deltas: Vec<(String, i64)> = events
            .iter()
            .map(|data| {
                assert_eq!(
                    data["cause"], "Refine",
                    "station refine probe: every balance move is the refine's: {data}"
                );
                (
                    data["id"].as_str().unwrap_or_default().to_string(),
                    data["delta_cr"].as_i64().unwrap_or_default(),
                )
            })
            .collect();
        deltas.sort();
        assert_eq!(
            deltas,
            [
                (PLAYER_ID.to_string(), -i64::from(FEE)),
                (STATION_ID.to_string(), i64::from(FEE)),
            ],
            "station refine probe: one OnCreditsChanged per party"
        );
        info!(
            "station refine probe: PASS refined one alloy plate: hold {:?}, player {} cr, station {} cr",
            hold(world),
            credits(world, player(world)),
            credits(world, station)
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: ore and a fee are refined into an alloy plate",
            serde_json::json!({ "t": elapsed, "fee_cr": FEE }),
        );
    }

    /// Claim 1: with no carbon ore left the form keeps the batch line, names
    /// the carbon ore and disables Confirm; nothing moved.
    fn assert_the_refusal(world: &mut World) {
        let summary = node_text(world, DRAFT_SUMMARY);
        assert!(
            summary.starts_with(&format!("{BATCH_LINE}\nRefused: only 0 Carbon ore in "))
                && summary.ends_with(", needs 1"),
            "station refine probe: the form must name the short input: {summary:?}"
        );
        assert!(
            confirm_disabled(world),
            "station refine probe: Confirm is disabled on a refusal"
        );
        assert_eq!(
            hold(world),
            (HOLD_IRON - BATCH_IRON, HOLD_CARBON - BATCH_CARBON, 1)
        );
        assert_eq!(credits(world, player(world)), PLAYER_CREDITS - FEE);
        info!("station refine probe: PASS a short refine is refused: {summary:?}");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a refine short of an input is refused on the form",
            serde_json::json!({ "t": elapsed, "summary": summary }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station refine: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
                    && player_root(world)
                        .is_some_and(|player| world.get::<Position>(player).is_some())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station refine: settle on the berth")
            .on_enter(berth_player)
            .until(player_berthed())
            .deadline(SESSION_SECS)
            .add()
            .step("station refine: dock")
            .each(|world, _, _| request_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station refine: record the station")
            .on_enter(record_the_station)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "station refine: open the Inventory pane",
            "InterfaceTabInventory",
            the_interface_shows(InterfacePaneType::Inventory),
            SESSION_SECS,
        )
        .step("station refine: settle the pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "station refine: pick the iron ore row",
            IRON_ROW,
            ui_node_present(REFINE_CHIP),
            SESSION_SECS,
        )
        .click_named(
            "station refine: switch the draft to Refine",
            REFINE_CHIP,
            frames(SETTLE_FRAMES),
            SESSION_SECS,
        )
        .step("station refine: assert the form")
        .on_enter(assert_the_form)
        .until(frames(1))
        .add()
        .step("station refine: shoot the form")
        .on_enter(|world: &mut World| shoot(world, FORM_SHOT))
        .until(shot_written(FORM_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_named(
            "station refine: confirm the refine",
            CONFIRM,
            when(|world| held(world, player(world), ALLOY) == 1),
            SESSION_SECS,
        )
        // Refine stays the picked chip, so the row opens it again.
        .click_named(
            "station refine: pick the iron ore row again",
            IRON_ROW,
            frames(SETTLE_FRAMES),
            SESSION_SECS,
        )
        .step("station refine: assert the refusal")
        .on_enter(assert_the_refusal)
        .until(frames(1))
        .add();
        // The final assert reads both parties' events, so it waits on them
        // with the clock running: the tracker drains the pane's reports on
        // its own schedule.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station refine: the refine reports")
            .until(refine_reported())
            .deadline(SESSION_SECS)
            .add()
            .step("station refine: assert the refine")
            .on_enter(assert_the_refine)
            .add()
    }
}
