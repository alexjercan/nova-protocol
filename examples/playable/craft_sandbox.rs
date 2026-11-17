//! craft_sandbox: start docked at a station that offers Craft, with ore and
//! credits aboard, and run recipes from the Craft pane.
//!
//! The shipped waystation (`block_waystation`) stands in a void; Craft is
//! one of its services, so its Station recipes run here as well as the Ship
//! recipes. The player's hull is the tender from `shared/docking_pair.rs`.
//! THE HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: 12 iron ore, 6
//! stone ore, 6 carbon ore and 300 cr, and no alloy plate, so the torpedo
//! tile is greyed by its input.
//!
//! What to do:
//!
//! 1. press Tab and open the Craft tab;
//! 2. click a tile: the card shows what the hold has of each input, the fee
//!    of a Station recipe and the time a batch runs;
//! 3. set the batches with `-`, `+` or Max and click Craft;
//! 4. close Tab: the job runs only while you fly, and the chip under the RCS
//!    chip shows it. Each batch takes its inputs and fee and adds its output.
//!
//! Press D to undock: the Station tiles grey with `Dock at a Craft station`,
//! and a running Station job stops. The diagnostics layer (F3) is on at
//! start: a panel line with the hold and the balance, and the running job.
//!
//! ```text
//! nix develop --command cargo run --example craft_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk checks the staging, opens the Craft pane, checks the tiles, starts
//! two batches of PDC rounds, closes the interface until the first batch
//! lands, and reopens the pane on the running job. `NOVA_CAPTURE=1` shoots
//! the idle tiles and the running job.

#[expect(dead_code, reason = "this example flies the tender and moors no spar")]
#[path = "shared/docking_pair.rs"]
mod docking_pair;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The waystation's park, meters.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The station's scenario id and name.
const STATION_ID: &str = "waystation";
const STATION_NAME: &str = "WAYSTATION";
/// The player's ship name.
const PLAYER_NAME: &str = "Tender";
/// The fixture hold.
const HOLD: [(&str, u32); 3] = [("IronOre", 12), ("StoneOre", 6), ("CarbonOre", 6)];
/// The fixture balance.
const CREDITS: u32 = 300;

/// The intro line.
const INTRO: &str = "Docked with ore: Tab, Craft, click a tile, set the batches, Craft, close Tab.";

fn main() -> bevy::app::AppExit {
    // NovaMenuPlugin explicitly: the interface's clock hold lives in
    // nova_menu, and `with_game_plugins` turns the menu plugin off. The hold
    // keeps a craft job still while Tab is open, as in the game.
    let mut app = AppBuilder::new()
        .with_game_plugins((craft_plugin, NovaMenuPlugin))
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        PostUpdate,
        draw_the_craft
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn craft_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(Update, stage_the_dock.run_if(in_state(GameStates::Playing)));
}

fn load_the_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    let player = station_void::player(
        PLAYER_NAME,
        station_void::tender_berth(STATION_POSITION),
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(docking_pair::tender()),
            inventory: station_void::stock(&HOLD),
            credits: CREDITS,
            ..default()
        },
    );
    commands.trigger(LoadScenario(station_void::void(
        "craft_sandbox",
        "Craft Sandbox",
        vec![
            station_void::waystation(STATION_ID, STATION_NAME, STATION_POSITION),
            player,
        ],
        &[(STATION_POSITION, 1.0)],
        &game_assets,
    )));
}

/// Whether the first dock was staged. The staging stops there, so the player
/// can undock and fly.
#[derive(Resource, Default)]
struct Staging {
    docked: bool,
}

/// Put the player on the waystation's berth and ask to dock, each frame
/// until the dock takes.
fn stage_the_dock(world: &mut World) {
    if world.resource::<Staging>().docked {
        return;
    }
    if station_void::berth_and_dock_at(world, STATION_ID) {
        world.resource_mut::<Staging>().docked = true;
        info!("craft_sandbox: docked at {STATION_NAME}; TAB opens the interface");
    }
}

/// The panel: the hold, the balance and the running job.
fn draw_the_craft(
    mut panel: ResMut<DiagnosticsPanel>,
    items: Res<GameItems>,
    recipes: Res<GameRecipes>,
    pause: Option<Res<State<PauseStates>>>,
    q_player: Query<
        (
            &ShipInventory,
            &ShipCredits,
            Option<&ShipCraftJob>,
            Has<DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
) {
    let Ok((hold, credits, job, docked)) = q_player.single() else {
        return;
    };
    let held: Vec<String> = hold
        .stacks()
        .map(|(item, count)| format!("{count} {}", items.design(item).name))
        .collect();
    panel.line(format!("Hold: {}; {} cr", held.join(", "), credits.0));
    match job {
        Some(job) => {
            let recipe = recipes.design(&job.recipe);
            panel.line(format!(
                "Job: {} batch {}/{}, {:.0}%, {:.1} s left",
                job.recipe,
                job.done + 1,
                job.queued,
                job.progress(recipe) * 100.0,
                job.remaining_secs(recipe)
            ));
        }
        None => panel.line("Job: none".to_string()),
    }
    panel.detail(format!(
        "Hold load: {} / {}",
        kg_text(u64::from(hold.used_g(&items))),
        kg_text(u64::from(hold.capacity_g()))
    ));
    if station_void::interface_up(pause.as_deref()) {
        panel.hint("Craft", "click a tile, set the batches, Craft");
    } else if docked {
        panel.hint("TAB", "Craft");
        panel.hint("D", "undock");
    } else {
        panel.hint("TAB", "Craft: Station tiles grey undocked");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::{craft::prelude::*, pane::InterfacePaneType},
    };

    use super::*;

    /// Seconds the walk gives the void to load and the dock to take.
    const LOAD_SECS: f32 = 120.0;
    /// Seconds any later beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Real seconds the first 8 s batch may take under a software renderer.
    const BATCH_SECS: f32 = 240.0;
    /// Frames the Craft pane gets to build its tiles and card.
    const SETTLE_FRAMES: u32 = 10;

    const PDC_RECIPE: &str = "base/pdc_round";
    const TORPEDO_RECIPE: &str = "base/torpedo";

    /// The frames the capture path writes.
    const IDLE_SHOT: &str = "craft-sandbox-idle.png";
    const RUNNING_SHOT: &str = "craft-sandbox-running.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn tile(recipe: &str) -> String {
        craft_tile_name(&RecipeDesignId::from(recipe))
    }

    fn player(world: &mut World) -> Entity {
        station_void::player_root(world).expect("craft_sandbox: a player")
    }

    fn held(world: &mut World, id: &str) -> u32 {
        let player = player(world);
        world
            .get::<ShipInventory>(player)
            .expect("craft_sandbox: the player carries a hold")
            .count(&ItemDesignId::from(id))
    }

    fn job(world: &World) -> Option<ShipCraftJob> {
        world
            .try_query_filtered::<&ShipCraftJob, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next().cloned())
    }

    /// Every text under the UI node named `name`, depth first.
    fn texts_under(world: &mut World, name: &str) -> Vec<String> {
        let root = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, each)| each.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("craft_sandbox: a node named {name}"));
        let mut texts = Vec::new();
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            if let Some(text) = world.get::<Text>(entity) {
                texts.push(text.0.clone());
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter());
            }
        }
        texts
    }

    /// The staging: docked at the waystation, which offers Craft, with the
    /// fixture hold and balance and no job.
    fn assert_the_staging(world: &mut World) {
        let station = station_void::station_root(world, STATION_ID)
            .expect("craft_sandbox: the waystation is up");
        assert!(
            station_void::docked_at(world, STATION_ID),
            "craft_sandbox: the player starts docked at the waystation"
        );
        let services = world
            .get::<StationServices>(station)
            .expect("craft_sandbox: a station root carries its services");
        assert!(
            services.0.contains(&StationServiceType::Craft),
            "craft_sandbox: the waystation must offer Craft: {services:?}"
        );
        for (id, count) in HOLD {
            assert_eq!(held(world, id), count, "craft_sandbox: {id} aboard");
        }
        let player = player(world);
        assert_eq!(
            world.get::<ShipCredits>(player).map(|credits| credits.0),
            Some(CREDITS)
        );
        assert!(job(world).is_none(), "craft_sandbox: no job at the start");
        station_void::assert_at_rest(world, player, station);
        info!("craft_sandbox: PASS staged docked with the fixture hold");
    }

    /// The idle tiles: PDC rounds live, the torpedo greyed by its alloy
    /// plates, and every base recipe shown.
    fn assert_the_tiles(world: &mut World) {
        let rounds = texts_under(world, &tile(PDC_RECIPE));
        assert!(
            rounds.contains(&"x100  SHIP".to_string()) && rounds.len() == 2,
            "craft_sandbox: the PDC round tile is live: {rounds:?}"
        );
        let torpedo = texts_under(world, &tile(TORPEDO_RECIPE));
        assert!(
            torpedo.contains(&"Short of alloy plate: 0 of 2".to_string()),
            "craft_sandbox: the torpedo tile is greyed by its input: {torpedo:?}"
        );
        let shown = world
            .query::<&Name>()
            .iter(world)
            .filter(|name| name.as_str().starts_with("CraftTile-"))
            .count();
        assert_eq!(shown, 6, "craft_sandbox: the six base recipes show");
        info!("craft_sandbox: PASS the tiles: {rounds:?}, {torpedo:?}");
    }

    /// The pane started two batches of PDC rounds and took nothing yet.
    fn assert_the_job(world: &mut World) {
        let job = job(world).expect("craft_sandbox: a job runs");
        assert_eq!(
            (job.recipe, job.queued, job.done),
            (RecipeDesignId::from(PDC_RECIPE), 2, 0),
            "craft_sandbox: two batches of PDC rounds run"
        );
        assert_eq!(
            held(world, "IronOre"),
            12,
            "craft_sandbox: a start takes nothing"
        );
        info!("craft_sandbox: PASS the pane started 2 batches of PDC rounds");
    }

    /// One batch landed: its inputs left the hold and its rounds entered it,
    /// and the pane shows the second batch running.
    fn assert_the_first_batch(world: &mut World) {
        assert_eq!(
            held(world, "IronOre"),
            11,
            "craft_sandbox: one iron ore spent"
        );
        assert_eq!(
            held(world, "StoneOre"),
            5,
            "craft_sandbox: one stone ore spent"
        );
        assert_eq!(
            held(world, "PdcRound"),
            100,
            "craft_sandbox: 100 rounds made"
        );
        let row = texts_under(world, CRAFT_JOB_ROW);
        assert!(
            row.contains(&"PDC round x100  batch 2/2".to_string()),
            "craft_sandbox: the job row shows the second batch: {row:?}"
        );
        info!("craft_sandbox: PASS the first batch landed; the row reads {row:?}");
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("craft sandbox: load the void and dock")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.docked)
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("craft sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("craft sandbox: open the Craft pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Craft);
            })
            .until(ui_node_present(tile(TORPEDO_RECIPE)))
            .deadline(BEAT_SECS)
            .add()
            .step("craft sandbox: settle the pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("craft sandbox: check the tiles")
            .on_enter(assert_the_tiles)
            .until(frames(1))
            .add()
            .click_named(
                "craft sandbox: pick the PDC round tile",
                &tile(PDC_RECIPE),
                when(|world| {
                    world.resource::<CraftRuntime>().selected()
                        == Some(&RecipeDesignId::from(PDC_RECIPE))
                }),
                BEAT_SECS,
            )
            .step("craft sandbox: settle the card")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("craft sandbox: shoot the idle tiles")
            .on_enter(|world: &mut World| shoot(world, IDLE_SHOT))
            .until(shot_written(IDLE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .click_named(
                "craft sandbox: one batch more",
                CRAFT_MORE,
                when(|world| world.resource::<CraftRuntime>().batches() == 2),
                BEAT_SECS,
            )
            .step("craft sandbox: settle the card at two batches")
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                "craft sandbox: craft",
                CRAFT_BUTTON,
                when(|world| job(world).is_some()),
                BEAT_SECS,
            )
            .step("craft sandbox: check the job")
            .on_enter(assert_the_job)
            .until(frames(1))
            .add()
            .step("craft sandbox: close the interface until the first batch lands")
            .on_enter(|world: &mut World| {
                world
                    .resource_mut::<NextState<PauseStates>>()
                    .set(PauseStates::Unpaused);
            })
            .until(when(|world| job(world).is_some_and(|job| job.done >= 1)))
            .deadline(BATCH_SECS)
            .add()
            .step("craft sandbox: reopen the Craft pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Craft);
            })
            .until(ui_node_present(tile(TORPEDO_RECIPE)))
            .deadline(BEAT_SECS)
            .add()
            .step("craft sandbox: settle the running job")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("craft sandbox: check the first batch")
            .on_enter(assert_the_first_batch)
            .until(frames(1))
            .add()
            .step("craft sandbox: shoot the running job")
            .on_enter(|world: &mut World| shoot(world, RUNNING_SHOT))
            .until(shot_written(RUNNING_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
