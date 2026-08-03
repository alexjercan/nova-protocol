//! screenshot_nova_os: capture the Tab NOVA OS ship-computer for the HTML
//! fidelity pass (task 20260726-180807). It boots a one-ship range, opens the
//! computer with Tab, drives a couple of commands through the real keyboard
//! path, and captures the screen so contrast, the input box, inline completion
//! and the CRT treatment can be compared against
//! `examples/ui/nova_os_terminal_poc.html`.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, open the computer,
//!   run the command script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_REEL=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! The script SELF-ENDS, so the 24s hold is a runway and the final stage prints
//! `probe: script complete, exiting` - the sentinel the smoke suite accepts in
//! place of the autopilot's `autopilot: cycle complete, no panic`. A completion
//! guard turns the silent-exit hole into a failure: an `AppExit` while the
//! script is unfinished panics naming the stage it stalled in, instead of
//! exiting 0 with the beats unplayed (task 20260729-222131).
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_nova_os --features debug
//! # look for: `probe: script complete, exiting`
//! ```
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_AUTOPILOT=1 NOVA_REEL=1 \
//!   cargo run --example screenshot_nova_os --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::input::{
    keyboard::{Key, KeyboardInput},
    ButtonState,
};
use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_nova_os")]
#[command(version = "1.0.0")]
#[command(about = "Capture the Tab NOVA OS ship-computer for HTML fidelity work", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<NovaOsScript>();
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Script-owned completion: this example SELF-ENDS (the script
                // walks a finite beat list and stops), so the 24s hold is a
                // RUNWAY, not the finish line - the final stage reports done,
                // and a runway expiry with the script pending is an error exit
                // naming it, never a silent pass.
                .self_completing()
                .hold(GameStates::Loading, 24.0)
                .input(nova_os_capture_script),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Completion guard: an AppExit with the script unfinished is a
            // stalled walk, not a pass.
            app.add_systems(Last, guard_script_completion);
        }
    }

    app.run()
}

/// Force the window to 1920x1080 (the 16:9 the web figures use) at startup.
#[cfg(feature = "debug")]
fn force_resolution(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(1920.0, 1080.0);
        window.resizable = false;
    }
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(nova_os_range(&game_assets, &sections)));
}

/// A single named player ship at the origin - enough for the NOVA OS computer to
/// spawn (it keys off the player ship root) and for `ship` to have real sections.
fn nova_os_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, z: f32| SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::new(0.0, 0.0, z),
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        sections: vec![
            at("player_controller", "basic_controller_section", 0.0),
            at("player_hull", "reinforced_hull_section", 1.0),
            at("player_thruster", "basic_thruster_section", 2.0),
        ],
    };

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: vec![EventActionConfig::SpawnScenarioObject(
            ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "player_ship".to_string(),
                    name: "Ceres Queen".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(player),
            },
        )],
    }];

    ScenarioConfig {
        id: "nova_os_range".to_string(),
        name: "NOVA OS Range".to_string(),
        description: "A range for the NOVA OS computer screenshots.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events,
        ..Default::default()
    }
}

/// Progress of the scripted capture run. Each stage fires once and then waits.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct NovaOsScript {
    stage: u32,
    wait: u32,
    /// Set by the final stage. Until then, any `AppExit` is a stalled walk.
    done: bool,
}

/// The self-ending contract's other half: the run may only end once the script
/// has reported done. A premature `AppExit` panics naming the stage it stalled
/// in, instead of exiting 0 with the beats unplayed. It runs in `Last`,
/// unordered against the harness's own watcher, so it catches an exit written
/// before that schedule - the class it is here for (the script's own
/// `PreUpdate` write) - but not one written later within `Last` itself.
#[cfg(feature = "debug")]
fn guard_script_completion(mut exits: MessageReader<AppExit>, script: Option<Res<NovaOsScript>>) {
    let Some(script) = script else { return };
    if exits.read().next().is_some() && !script.done {
        panic!(
            "screenshot_nova_os: run ended with the script stalled in stage {}",
            script.stage
        );
    }
}

/// Press Tab to toggle the computer via the real `ButtonInput<KeyCode>` edge.
#[cfg(feature = "debug")]
fn press_tab(world: &mut World) {
    if let Some(mut keys) = world.get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.press(KeyCode::Tab);
    }
}

/// Send one printable character to the terminal through the real keyboard path.
#[cfg(feature = "debug")]
fn type_char(world: &mut World, ch: &str) {
    world.write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character(ch.into()),
        state: ButtonState::Pressed,
        text: Some(ch.into()),
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// Type a whole word (one event per character).
#[cfg(feature = "debug")]
fn type_word(world: &mut World, word: &str) {
    for ch in word.chars() {
        type_char(world, &ch.to_string());
    }
}

/// Press Escape via the real `ButtonInput<KeyCode>` edge - in an app this returns
/// to the prompt (the context-keyed Escape owner), so the script can move from one
/// app to the next.
#[cfg(feature = "debug")]
fn press_escape(world: &mut World) {
    if let Some(mut keys) = world.get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.press(KeyCode::Escape);
    }
}

/// Press Enter to submit the current command line.
#[cfg(feature = "debug")]
fn press_enter(world: &mut World) {
    world.write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// Drive: open the computer, run `help`/`ship`, leave a prefix in the input to
/// show inline completion, and capture. Captures only when `NOVA_REEL` is set.
#[cfg(feature = "debug")]
fn nova_os_capture_script(world: &mut World, _elapsed: f32) {
    let capturing = std::env::var_os("NOVA_REEL").is_some();
    let settle = if capturing { 40 } else { 6 };

    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }

    let mut state = world.remove_resource::<NovaOsScript>().unwrap();
    if state.done {
        // Reported done already; the harness watcher owns the exit from here.
        world.insert_resource(state);
        return;
    }
    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }

    match state.stage {
        // Open the computer and let the slide + first layout settle.
        0 => {
            press_tab(world);
            state.wait = settle;
        }
        // Capture the freshly opened welcome screen (empty prompt).
        1 => {
            if capturing {
                capture_window(world, "nova-os-welcome.png");
                info!("nova os capture: nova-os-welcome.png");
            }
            state.wait = if capturing { 20 } else { 2 };
        }
        // Run `help` then `ship view` so command-output formatting is on screen
        // (bare `ship` now LAUNCHES the app; `ship view` is the CLI status print).
        2 => {
            type_word(world, "help");
            press_enter(world);
            state.wait = 6;
        }
        3 => {
            type_word(world, "ship view");
            press_enter(world);
            state.wait = 6;
        }
        // Leave a valid prefix in the input to show the inline completion ghost.
        4 => {
            type_word(world, "lo");
            state.wait = settle;
        }
        // Capture the active screen (command output + inline completion).
        5 => {
            if capturing {
                capture_window(world, "nova-os-active.png");
                info!("nova os capture: nova-os-active.png");
            }
            state.wait = if capturing { 20 } else { 2 };
        }
        // Flush the leftover `lo` prefix, then type `map`.
        6 => {
            press_enter(world);
            type_word(world, "map");
            state.wait = 6;
        }
        // Launch the `map` app (schematic 3D minimap) and let its scene settle.
        7 => {
            press_enter(world);
            state.wait = settle;
        }
        // Capture the map app (schematic scene + projected contact blips).
        8 => {
            if capturing {
                capture_window(world, "nova-os-map.png");
                info!("nova os capture: nova-os-map.png");
            }
            state.wait = if capturing { 20 } else { 2 };
        }
        // Leave the map app back to the prompt, then launch the `ship` app.
        9 => {
            press_escape(world);
            type_word(world, "ship");
            state.wait = 6;
        }
        // Launch the ship schematic app and let its RTT scene build/settle. This
        // exercises the real render path (a wgsl/render panic would fail the run).
        10 => {
            press_enter(world);
            state.wait = settle;
        }
        // Capture the ship app (green-phosphor section blocks + code blips).
        11 => {
            if capturing {
                capture_window(world, "nova-os-ship.png");
                info!("nova os capture: nova-os-ship.png");
            }
            state.wait = if capturing { 20 } else { 2 };
        }
        _ => {
            // The suite's completion sentinel for SELF-ENDING examples
            // (tests/examples_smoke.rs accepts it in place of the autopilot's
            // "cycle complete"): the autopilot's own line only prints when its
            // lifetime expires, and idling out the remaining runway buys
            // nothing. Report done rather than writing `AppExit` here: the
            // autopilot is a REGISTERED completion collector, and the watcher
            // owns the exit once every collector reports. The captures are not
            // collectors (`capture_window` spawns a bare `Screenshot`, so
            // nothing registers for them) - stage 11's 20-frame settle is what
            // gives `save_to_disk` room to land, and it stays load-bearing.
            info!("probe: script complete, exiting");
            state.done = true;
            world
                .resource_mut::<nova_protocol::nova_debug::harness::HarnessCompletion>()
                .done(nova_protocol::nova_debug::harness::AUTOPILOT);
        }
    }

    state.stage += 1;
    world.insert_resource(state);
}
