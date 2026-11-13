//! news_0180_events_blocks: the v0.18.0 news editor still - the Events canvas
//! of the mod editor on a mod whose scenario is a generated world with three
//! handlers: `OnStart` spawns the start role as `player`, an `OnJobHandedIn`
//! hat stands over a `GrantCredits` block, and an `OnDocked` hat stands over
//! a `PostJob` block whose station is `$id`, the hull docked with.
//!
//! Boots the shipped app on a profile folder of its own with that mod
//! installed in it. THE MOD IS WRITTEN BY HAND, in [`CONTENT`]: nobody drags
//! a block in this walk. The walk opens the mod from the Mod Hub with the
//! harness pointer, takes the Events tab, checks that the editor draws the
//! six blocks with none at fault, and shoots the canvas.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the walk with its asserts,
//!   writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write `events-blocks-jobs.png`
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0180-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0180_events_blocks --features debug
//! # look for: `events blocks: PASS the three handlers draw with no fault`
//! ```

use std::sync::Arc;

use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    prelude::*,
};

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

#[derive(Parser)]
#[command(name = "news_0180_events_blocks")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.18.0 editor still: two job handlers on the Events canvas",
    long_about = None
)]
struct Cli;

/// The mod the walk opens. The hub lists only ids that start with `editor_`.
const MOD_ID: &str = "editor_events_blocks";

/// The mod's one content file. The world is generated, because only a
/// generated station posts jobs, and `OnStart` spawns the `player` that
/// `GrantCredits` pays: lint refuses an id that the scenario does not spawn.
const CONTENT: &str = r#"[
    Scenario((
        id: "events_blocks",
        name: "Events Blocks Demo",
        description: "A bonus on a hand-in, and a job posted by the station docked with.",
        cubemap: "base/textures/cubemap.png",
        thumbnail: None,
        world: Generated((
            seed: None,
            bounds: None,
        )),
        events: [
            (
                name: OnStart,
                once: true,
                actions: [
                    SpawnStartRole((
                        id: "player",
                    )),
                ],
            ),
            (
                name: OnJobHandedIn,
                once: false,
                actions: [
                    GrantCredits((
                        ship: "player",
                        amount: 50,
                    )),
                ],
            ),
            (
                name: OnDocked,
                once: true,
                actions: [
                    PostJob((
                        station: "$id",
                        key: "ore_run",
                        item: "IronOre",
                        count: 4,
                        reward_cr: 250,
                    )),
                ],
            ),
        ],
    )),
]
"#;

/// The hub row of the mod, and the blocks the three handlers draw: three
/// hats and one action under each.
const OPEN: &str = "Hub Row editor_events_blocks";
const HANDLER_BLOCKS: usize = 6;

/// The still this producer writes.
const SHOT: &str = "events-blocks-jobs.png";

/// Real seconds the app gets to boot to the hub, and a beat to land, on a
/// software rasterizer.
const BOOT_SECS: f32 = 300.0;
const BEAT_SECS: f32 = 60.0;

/// Install the mod in this run's profile.
fn install_the_demo_mod() {
    let bundle = format!("{MOD_ID}.bundle.ron");
    let file = format!("{MOD_ID}.content.ron");
    let manifest = format!(
        "(\n    content: [\n        \"{file}\",\n    ],\n    meta: (\n        name: \"Events Blocks Demo\",\n        \
         description: \"Three handlers.\",\n        author: \"\",\n        version: \"1.0.0\",\n        \
         dependencies: [],\n        icon: None,\n        screenshots: [],\n    ),\n)\n"
    );
    nova_assets::mod_cache::install_local(
        MOD_ID,
        "1.0.0",
        &bundle,
        &[
            (bundle.clone(), manifest.into_bytes()),
            (file, CONTENT.as_bytes().to_vec()),
        ],
    )
    .expect("the demo mod installs in the run's profile");
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));
    install_the_demo_mod();

    let mut app = editor_app(true, None);
    app.add_systems(OnEnter(GameStates::MainMenu), open_the_sandbox);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_systems(Startup, size_the_window);
    app.add_plugins(script());
    app.run()
}

/// Fix the window at the fleet's capture size.
fn size_the_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(1920.0, 1080.0);
        window.resizable = false;
    }
}

/// What the Sandbox button does, once.
fn open_the_sandbox(
    mut opened: Local<bool>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if !*opened {
        *opened = true;
        *mode = GameMode::Sandbox;
        state.set(GameStates::Playing);
    }
}

fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
}

/// Panic unless the editor draws the three handlers with no block at fault.
fn assert_the_handlers(world: &mut World) {
    let shell = &world.resource::<EditorProbe>().shell;
    assert_eq!(shell.block_faults, 0, "events blocks: {shell:?}");
    assert_eq!(shell.blocks, HANDLER_BLOCKS, "events blocks: {shell:?}");
    // TODO: the editor lints the open document as a `Void` world, because the
    // scenario it lowers for the lint does not carry the file's `world`. The
    // Problems tab then lists three errors for this valid file. Assert
    // `shell.errors == 0` here when the lint reads the file's world.
    info!(
        "events blocks: PASS the three handlers draw with no fault; the Problems tab lists {} \
         error(s)",
        shell.errors
    );
}

fn script() -> AutopilotPlugin<GameStates> {
    let script = AutopilotPlugin::<GameStates>::new()
        .step("events blocks: the hub lists the mod")
        .until(ui_node_present(OPEN))
        .deadline(BOOT_SECS)
        .add();
    let script = open_world_stage::open_hub_row(
        script,
        "events blocks: open the mod",
        OPEN,
        ui_node_present("View Tab Events"),
        BOOT_SECS,
    );
    script
        .click_named(
            "events blocks: take the Events tab",
            "View Tab Events",
            editor(|probe| probe.shell.rail_tab == "Events" && probe.shell.blocks > 0),
            BEAT_SECS,
        )
        .step("events blocks: assert the three handlers")
        .on_enter(assert_the_handlers)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("events blocks: shoot the canvas")
        .on_enter(|world: &mut World| shoot(world, SHOT))
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
