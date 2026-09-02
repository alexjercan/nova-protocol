//! The NOVA OS range and the keyboard path the ship-computer walks type on.
//!
//! Every keystroke goes through the real input path - a `ButtonInput<KeyCode>`
//! edge or a `KeyboardInput` message - never a direct call into the terminal, so
//! a computer that stopped listening fails the run instead of quietly producing
//! the previous shot again.
//!
//! Included by each ship-computer producer with
//! `#[path = "shared/computer.rs"] mod computer;`. It lives one level down on
//! purpose - `catalog_matches_disk`
//! (`crates/nova_probe_cli/tests/catalog_drift.rs`) treats every `.rs` DIRECTLY
//! under a category dir as a cataloged example, so a sibling `computer.rs` would
//! fail the catalog check.

// Each producer includes the whole kit and types what its shot needs; the keys
// it never presses are not dead code, they are another walk's beat.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use std::collections::BTreeMap;

#[cfg(feature = "debug")]
use bevy::input::{
    keyboard::{Key, KeyboardInput},
    ButtonState,
};
use bevy::prelude::*;
use nova_protocol::{
    nova_os_ui::{
        nova_os::prelude::{NovaOsTerminal, TerminalMode},
        terminal::nova_os_openness,
    },
    prelude::*,
};

/// A single named player ship at the origin - enough for the NOVA OS computer to
/// spawn (it keys off the player ship root) and for `ship` to have real sections.
///
/// It carries a turret and a torpedo bay on its flanks as well as the spine, so
/// the schematic app has something to be a schematic OF and both shots show the
/// weapon cockpit codes the page talks about, not three blocks in a line.
pub fn nova_os_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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
            input_mapping: BTreeMap::new(),
            speed_cap: None,
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
                // The 1x1x2 tube seats on the half cell; its aft flank socket
                // at z 1.0 mates the hull's -X face.
                at(
                    "player_torpedo",
                    "torpedo_section",
                    Vec3::new(-1.0, 0.0, 0.5),
                ),
            ],
            ..default()
        }),
        ..default()
    };

    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        // The scene lights itself: the engine spawns no light, so a
        // scenario that authors none renders black.
        actions: [
            vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "player_ship".to_string(),
                        name: "Ceres Queen".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(player),
                },
            )],
            ThreePointRig::around("range", Meters3::ZERO, 1.0).actions(),
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
pub fn press_tab(world: &mut World) {
    if let Some(mut keys) = world.get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.press(KeyCode::Tab);
    }
}

/// Send one printable character to the terminal through the real keyboard path.
#[cfg(feature = "debug")]
pub fn type_char(world: &mut World, ch: &str) {
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
pub fn type_word(world: &mut World, word: &str) {
    for ch in word.chars() {
        type_char(world, &ch.to_string());
    }
}

/// Press Escape via the real `ButtonInput<KeyCode>` edge - in an app this returns
/// to the prompt (the context-keyed Escape owner), so the script can move from one
/// app to the next.
#[cfg(feature = "debug")]
pub fn press_escape(world: &mut World) {
    if let Some(mut keys) = world.get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.press(KeyCode::Escape);
    }
}

/// Press Enter to submit the current command line.
#[cfg(feature = "debug")]
pub fn press_enter(world: &mut World) {
    world.write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// Advance once the CRT's raster has finished blooming open.
///
/// Not a dwell: the raster blooms on over real time and the tube shows a
/// squeezed window onto the image until it settles, so a shot taken mid-slide
/// is a picture of the transition rather than of the computer.
#[cfg(feature = "debug")]
pub fn raster_open() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        nova_os_openness(world).is_some_and(|open| open >= 1.0 - f32::EPSILON)
    })
}

/// Advance once the shell says `id` owns the screen - the terminal model's own
/// answer, not a node count a half-built app surface would satisfy.
#[cfg(feature = "debug")]
pub fn app_owns_the_screen(
    id: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<NovaOsTerminal>(move |terminal| {
        terminal.active_mode() == TerminalMode::App { id }
    })
}

/// Advance once the terminal's command line holds exactly `text`.
///
/// [`type_word`] writes every character in ONE frame, so a frame count after it
/// was never a typing rate - it was a guess at how long the shell takes to
/// answer. This is the shell's own record of what it took.
#[cfg(feature = "debug")]
pub fn command_line_reads(
    text: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<NovaOsTerminal>(move |terminal| terminal.prompt() == text)
}

/// What the scrollback read when the last command was submitted, so
/// [`the_shell_answered`] can tell this command's output from the last one's.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct ShellBaseline(u64);

/// Advance once the shell has PRINTED something new and is back at the prompt -
/// the honest end of "run a command", where a frame count only said that some
/// frames had gone by.
///
/// The revision, not the row count: a command whose output scrolls the oldest
/// rows off the top would leave the count unchanged.
#[cfg(feature = "debug")]
pub fn the_shell_answered() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let before = world
            .get_resource::<ShellBaseline>()
            .map_or(0, |mark| mark.0);
        world
            .get_resource::<NovaOsTerminal>()
            .is_some_and(|terminal| {
                terminal.scrollback_revision() > before
                    && terminal.active_mode() == TerminalMode::Prompt
            })
    })
}

/// Type a command and submit it, marking the scrollback first so
/// [`the_shell_answered`] answers for THIS command.
#[cfg(feature = "debug")]
pub fn run_command(world: &mut World, command: &str) {
    let before = world
        .get_resource::<NovaOsTerminal>()
        .map_or(0, |terminal| terminal.scrollback_revision());
    world.insert_resource(ShellBaseline(before));
    type_word(world, command);
    press_enter(world);
}
