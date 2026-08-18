//! screenshot_nova_os: capture the Tab NOVA OS ship-computer for the site's two
//! NOVA OS figures. It boots a one-ship range, opens the computer with Tab,
//! drives commands through the real keyboard path, and shoots the terminal with
//! output and an inline-completion ghost on it, then the ship app filling the
//! tube.
//!
//! Two shots, four apps' worth of walking. The walk still opens the welcome
//! screen and launches the map app before it gets to the ship app, and those
//! beats are kept: they are what exercises the map app and the RTT/wgsl
//! schematic path, so a render panic in either fails this run. What they no
//! longer do is WRITE a file. Their captures existed for the HTML fidelity pass
//! (task 20260726-180807, closed) against `web/design/nova_os_terminal_poc.html`
//! and no page has ever referenced them.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, open the computer,
//!   run the command script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
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
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
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
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
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
                .until(frames(SETTLE_FRAMES))
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
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("capture the terminal")
                .on_enter(move |world| shoot(world, "news-090-nova-os-terminal.png"))
                .until(shot_written("news-090-nova-os-terminal.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                .until(frames(SETTLE_FRAMES))
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
                .until(frames(SETTLE_FRAMES))
                .add()
                // The last step holds until the PNG is on disk, so the driver
                // cannot report done out from under a pending write.
                .step("capture the ship app")
                .on_enter(move |world| shoot(world, "news-090-nova-os-apps.png"))
                .until(shot_written("news-090-nova-os-apps.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(nova_os_range(&game_assets, &sections)));
}

/// A single named player ship at the origin - enough for the NOVA OS computer to
/// spawn (it keys off the player ship root) and for `ship` to have real sections.
///
/// It carries a turret and a torpedo bay on its flanks as well as the spine, so
/// the schematic app has something to be a schematic OF and both shots show the
/// weapon cockpit codes the page talks about, not three blocks in a line.
fn nova_os_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
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
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    "player_controller",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 0.0),
                ),
                at(
                    "player_hull",
                    "reinforced_hull_section",
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                at(
                    "player_thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                ),
                SpaceshipSectionConfig {
                    id: "player_turret".to_string(),
                    // Seated on the hull's +X face. The shared PDC bolts down by
                    // its base plate alone, so it sits a quarter-cell in from
                    // that face rather than a whole cell out, and is rolled to
                    // stand out of it.
                    position: Vec3::new(0.75, 0.0, 1.0),
                    rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                    source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
                    modifications: vec![],
                },
                at(
                    "player_torpedo",
                    "torpedo_section",
                    Vec3::new(-1.0, 0.0, 1.0),
                ),
            ],
            ..default()
        }),
        ..default()
    };

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        // The scene lights itself: the engine spawns no light, so a
        // scenario that authors none renders black.
        actions: [
            vec![EventActionConfig::SpawnScenarioObject(
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
            ThreePointRig::around("range", Vec3::ZERO, 1.0).actions(),
        ]
        .concat(),
    }];

    ScenarioConfig {
        description: "A range for the NOVA OS computer screenshots.".to_string(),
        events,
        ..ScenarioConfig::new(
            "nova_os_range".to_string(),
            "NOVA OS Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
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
