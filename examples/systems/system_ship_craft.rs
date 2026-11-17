//! system_ship_craft: a Ship recipe crafts in flight, and a Station recipe
//! refuses undocked.
//!
//! The player's hull floats alone in a void: a controller over a hull block
//! (the hold), a port on its nose and a drive on its tail. THE HOLD AND THE
//! BALANCE ARE DISCLOSED FIXTURE STATE: two iron ore, one stone ore and
//! 100 cr. The walk starts one batch of the base `base/pdc_round` recipe
//! through the Craft view's command, lets the flight clock run it, then asks
//! for the base `base/rail_slug` Station recipe and finally forces a Station
//! job onto the undocked ship.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: a ship recipe crafts in flight` | the start inserted a one-batch job and took nothing; when the batch ended the hold had lost one iron ore and one stone ore and gained 100 PDC rounds, the balance did not move and the job ended `Crafted { batches: 1 }` |
//! | 1 | `outcome: a station recipe refuses undocked` | the start was refused and inserted no job; a Station job put on the ship ended at once `Stopped { NotAtCraftStation }` with nothing taken |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ship_craft --features debug
//! # look for: `ship craft probe: PASS a ship recipe crafted in flight`,
//! #           `ship craft probe: PASS a station recipe refused undocked`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ship_craft")]
#[command(version = "1.0.0")]
#[command(about = "One batch of PDC rounds crafts in flight on the game clock, and a Station recipe is refused undocked. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario id the void loads under.
const SCENARIO_ID: &str = "ship_craft_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// The base Ship recipe and the base Station recipe the walk runs.
const SHIP_RECIPE: &str = "base/pdc_round";
const STATION_RECIPE: &str = "base/rail_slug";

/// Item ids of the base PDC round recipe: one iron ore and one stone ore
/// make 100 rounds.
const IRON: &str = "IronOre";
const STONE: &str = "StoneOre";
const ROUNDS: &str = "PdcRound";

/// The disclosed fixture hold and balance.
const HOLD_IRON: u32 = 2;
const HOLD_STONE: u32 = 1;
const PLAYER_CREDITS: u32 = 100;

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
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::CraftWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A lone player ship with ore aboard.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(player)],
                ThreePointRig::around("rig", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Ship Craft Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
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
        inventory: ShipInventoryStock::new([(IRON.into(), HOLD_IRON), (STONE.into(), HOLD_STONE)]),
        credits: PLAYER_CREDITS,
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate},
        nova_interface::craft::prelude::*,
    };

    use super::*;

    /// Seconds a load or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;

    /// Every craft job end the run saw.
    #[derive(Resource, Default, Debug)]
    struct CraftWatch(Vec<CraftJobEnded>);

    pub(super) struct CraftWatchPlugin;

    impl Plugin for CraftWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<CraftWatch>();
            app.add_systems(
                Update,
                (|mut ends: MessageReader<CraftJobEnded>, mut watch: ResMut<CraftWatch>| {
                    for end in ends.read() {
                        info!("ship craft probe: {end:?}");
                        watch.0.push(end.clone());
                    }
                })
                .after(CraftJobSystems),
            );
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player_root(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("ship craft probe: the player is up")
    }

    /// The player's iron ore, stone ore and PDC rounds.
    fn hold(world: &World) -> (u32, u32, u32) {
        let hold = world
            .get::<ShipInventory>(player(world))
            .expect("ship craft probe: every ship carries a hold");
        (
            hold.count(&IRON.into()),
            hold.count(&STONE.into()),
            hold.count(&ROUNDS.into()),
        )
    }

    fn credits(world: &World) -> u32 {
        world
            .get::<ShipCredits>(player(world))
            .expect("ship craft probe: every ship carries a balance")
            .0
    }

    fn job(world: &World) -> Option<ShipCraftJob> {
        world.get::<ShipCraftJob>(player(world)).cloned()
    }

    fn ends(world: &World) -> usize {
        world.resource::<CraftWatch>().0.len()
    }

    fn start(recipe: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            world.write_message(CraftStartCommand {
                recipe: RecipeDesignId::from(recipe),
                batches: 1,
            });
        }
    }

    fn assert_the_fixture(world: &mut World) {
        assert_eq!(hold(world), (HOLD_IRON, HOLD_STONE, 0));
        assert_eq!(credits(world), PLAYER_CREDITS);
        assert!(
            world.get::<DockedShip>(player(world)).is_none(),
            "ship craft probe: the player is undocked"
        );
        assert!(job(world).is_none());
    }

    /// The start inserted one batch and took nothing.
    fn assert_the_start(world: &mut World) {
        // The flight clock runs, so the job may already hold elapsed time.
        let job = job(world).expect("ship craft probe: a job runs");
        assert_eq!(
            (job.recipe, job.queued, job.done),
            (RecipeDesignId::from(SHIP_RECIPE), 1, 0),
            "ship craft probe: one batch of PDC rounds runs"
        );
        assert_eq!(
            hold(world),
            (HOLD_IRON, HOLD_STONE, 0),
            "ship craft probe: a start takes nothing"
        );
    }

    /// Claim 0: the batch took its inputs, added its rounds and ended
    /// crafted; the balance did not move.
    fn assert_the_craft(world: &mut World) {
        assert_eq!(
            hold(world),
            (HOLD_IRON - 1, HOLD_STONE - 1, 100),
            "ship craft probe: one batch spent one iron and one stone ore for 100 rounds"
        );
        assert_eq!(credits(world), PLAYER_CREDITS, "ship craft probe: no fee");
        assert!(job(world).is_none(), "ship craft probe: the job ended");
        let end = world.resource::<CraftWatch>().0[0].clone();
        assert_eq!(
            (end.recipe.clone(), end.end.clone()),
            (
                RecipeDesignId::from(SHIP_RECIPE),
                CraftJobEndType::Crafted { batches: 1 }
            ),
            "ship craft probe: the job ended crafted"
        );
        info!(
            "ship craft probe: PASS a ship recipe crafted in flight: hold {:?}",
            hold(world)
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a ship recipe crafts in flight",
            serde_json::json!({ "t": elapsed, "rounds": 100 }),
        );
    }

    /// The Station recipe's start was refused undocked: no job.
    fn assert_the_refused_start(world: &mut World) {
        assert!(
            job(world).is_none(),
            "ship craft probe: no Station job starts undocked"
        );
    }

    fn force_station_job(world: &mut World) {
        let player = player(world);
        world
            .entity_mut(player)
            .insert(ShipCraftJob::new(STATION_RECIPE.into(), 1));
    }

    /// Claim 1: a Station job on an undocked ship stops at once and takes
    /// nothing.
    fn assert_the_stop(world: &mut World) {
        assert!(
            job(world).is_none(),
            "ship craft probe: the Station job ended"
        );
        let end = world.resource::<CraftWatch>().0[1].clone();
        assert_eq!(
            end.end,
            CraftJobEndType::Stopped {
                batches: 0,
                reason: CraftRefusalType::NotAtCraftStation,
            },
            "ship craft probe: the Station job stops undocked"
        );
        assert_eq!(hold(world), (HOLD_IRON - 1, HOLD_STONE - 1, 100));
        assert_eq!(credits(world), PLAYER_CREDITS);
        info!("ship craft probe: PASS a station recipe refused undocked");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a station recipe refuses undocked",
            serde_json::json!({ "t": elapsed }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("ship craft: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                player_root(world)
                    .is_some_and(|player| world.get::<ShipInventory>(player).is_some())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("ship craft: check the fixture")
            .on_enter(assert_the_fixture)
            .until(frames(1))
            .add()
            .step("ship craft: start one batch of PDC rounds")
            .on_enter(start(SHIP_RECIPE))
            .until(when(|world| job(world).is_some()))
            .deadline(SESSION_SECS)
            .add()
            .step("ship craft: check the start")
            .on_enter(assert_the_start)
            .until(frames(1))
            .add()
            .step("ship craft: run the batch on the flight clock")
            .until(when(|world| ends(world) >= 1))
            .deadline(SESSION_SECS)
            .add()
            .step("ship craft: check the craft")
            .on_enter(assert_the_craft)
            .until(frames(1))
            .add()
            .step("ship craft: ask for a Station recipe undocked")
            .on_enter(start(STATION_RECIPE))
            // The start command is read the frame after it is written; a
            // refused start leaves no world value to wait on.
            .until(frames(3))
            .add()
            .step("ship craft: check the refused start")
            .on_enter(assert_the_refused_start)
            .until(frames(1))
            .add()
            .step("ship craft: put a Station job on the undocked ship")
            .on_enter(force_station_job)
            .until(when(|world| ends(world) >= 2))
            .deadline(SESSION_SECS)
            .add()
            .step("ship craft: check the stop")
            .on_enter(assert_the_stop)
            .add()
    }
}
