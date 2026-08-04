//! screenshot_nova_os: capture the Tab NOVA OS ship-computer for the HTML
//! fidelity pass (task 20260726-180807). It boots a one-ship range, opens the
//! computer with Tab, drives a couple of commands through the real keyboard
//! path, and captures the screen so contrast, the input box, inline completion
//! and the CRT treatment can be compared against
//! `web/design/nova_os_terminal_poc.html`.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, open the computer,
//!   run the command script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_REEL=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! The beats are autopilot STEPS, so the driver owns completion: it reports done
//! when the last step ends and the run prints `autopilot: cycle complete, no
//! panic`. A step that never resolves inside its deadline is an error exit
//! NAMING that step, so a stalled walk fails loudly instead of exiting 0 with
//! the beats unplayed (task 20260729-222131).
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_nova_os --features debug
//! # look for: `autopilot: cycle complete, no panic`
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
        let capturing = std::env::var_os(nova_protocol::nova_debug::harness::REEL_ENV).is_some();
        // The per-beat settle counts are LOAD-BEARING for the capture path: a
        // beat must be still before its shot, and `save_to_disk` must land
        // before the next beat navigates away. Carried over from the stage
        // machine this replaced, not re-derived.
        let settle = if capturing { 40 } else { 6 };
        let after_capture = if capturing { 20 } else { 2 };
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Wait for the ship to EXIST: the computer keys off the player
                // ship root, so a beat fired before it spawned would open
                // nothing.
                .step("load the nova_os range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("open the computer")
                .on_enter(press_tab)
                .until(frames(settle))
                .add()
                .step("capture the welcome screen")
                .on_enter(move |world| shoot(world, capturing, "nova-os-welcome.png"))
                .until(frames(after_capture))
                .add()
                // Run `help` then `ship view` so command-output formatting is
                // on screen (bare `ship` now LAUNCHES the app; `ship view` is
                // the CLI status print).
                .step("run the help command")
                .on_enter(|world| run_command(world, "help"))
                .until(frames(6))
                .add()
                .step("run the ship view command")
                .on_enter(|world| run_command(world, "ship view"))
                .until(frames(6))
                .add()
                // Leave a valid prefix in the input to show the inline
                // completion ghost.
                .step("leave an inline-completion prefix")
                .on_enter(|world| type_word(world, "lo"))
                .until(frames(settle))
                .add()
                .step("capture the active screen")
                .on_enter(move |world| shoot(world, capturing, "nova-os-active.png"))
                .until(frames(after_capture))
                .add()
                // Flush the leftover `lo` prefix, then type `map`.
                .step("type the map command")
                .on_enter(|world| {
                    press_enter(world);
                    type_word(world, "map");
                })
                .until(frames(6))
                .add()
                .step("launch the map app")
                .on_enter(press_enter)
                .until(frames(settle))
                .add()
                .step("capture the map app")
                .on_enter(move |world| shoot(world, capturing, "nova-os-map.png"))
                .until(frames(after_capture))
                .add()
                // Leave the map app back to the prompt, then type `ship`.
                .step("type the ship command")
                .on_enter(|world| {
                    press_escape(world);
                    type_word(world, "ship");
                })
                .until(frames(6))
                .add()
                // Launch the ship schematic app and let its RTT scene
                // build/settle. This exercises the real render path (a
                // wgsl/render panic would fail the run).
                .step("launch the ship app")
                .on_enter(press_enter)
                .until(frames(settle))
                .add()
                // The last step's hold is what gives `save_to_disk` room to
                // land: `capture_window` spawns a bare `Screenshot`, so nothing
                // registers it as a completion collector and the driver reports
                // done the moment this step ends.
                .step("capture the ship app")
                .on_enter(move |world| shoot(world, capturing, "nova-os-ship.png"))
                .until(frames(after_capture))
                .add(),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
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

/// Type a command and submit it.
#[cfg(feature = "debug")]
fn run_command(world: &mut World, command: &str) {
    type_word(world, command);
    press_enter(world);
}

/// Request one shot of the primary window. Captures only when `NOVA_REEL` is
/// set, so the plain autopilot smoke run drives the same beats without writing
/// files.
#[cfg(feature = "debug")]
fn shoot(world: &mut World, capturing: bool, path: &str) {
    if capturing {
        capture_window(world, path);
        info!("nova os capture: {path}");
    }
}
