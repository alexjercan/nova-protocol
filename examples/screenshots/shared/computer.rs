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
use nova_input::prelude::InputSource;
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::nova_os::prelude::{NovaOsTerminal, TerminalMode};
use nova_protocol::prelude::*;

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
    };

    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            // The turret carries the trigger a player ship's turret carries -
            // the same pair `shared/hollow.rs` writes for every gun it places.
            // A section with NO binding cannot be rebound: the SHIP app reads
            // the binding component to decide whether its rebind button is
            // live, so a range with an empty mapping is a range where the
            // rebind lesson has nothing to photograph.
            input_mapping: BTreeMap::from([(
                "player_turret".to_string(),
                vec![
                    InputSource::Mouse(MouseButton::Left),
                    InputSource::Gamepad(GamepadButton::RightTrigger2),
                ],
            )]),
            speed_cap: None,
        }),
        design: ShipDesignSource::Inline(ShipDesign {
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

/// Where the plot range parks its three contacts, in the map's own screen
/// axes rather than the world's.
///
/// The map opens at a fixed heading (`MAP_THETA_DEFAULT`, 45.8 degrees off
/// world +Z, tilted 35.5 degrees down), and its viewport is two and a half
/// times wider than it is tall. So the plot has ROOM to the sides and none to
/// speak of top and bottom: the default framing fits the furthest contact
/// exactly at the vertical edge. These three sit on the screen's horizontal,
/// where that margin is generous - one hostile close to starboard, the tender
/// out to port, the second hostile furthest out and still well inside the
/// frame it sets.
const PLOT_SCREEN_RIGHT: Vec3 = Vec3::new(0.697, 0.0, -0.717);

/// The contacts the plot range carries, as (id, name, allegiance, screen
/// offset, range in metres). The RAIDER IS FIRST, and the order matters: the
/// map's contact list is the own ship followed by the ships in spawn order, so
/// a walk that cycles twice lands on this one.
const PLOT_CONTACTS: [(&str, &str, Allegiance, f32, f32); 3] = [
    ("plot_raider", "Raider", Allegiance::Enemy, 1.0, 340.0),
    ("plot_tender", "Ore Tender", Allegiance::Player, -1.0, 420.0),
    ("plot_lance", "Lance", Allegiance::Enemy, 1.0, 620.0),
];

/// How far each contact rides off the plot's plane, so three hulls at one
/// height do not read as a drawn line. Indexed alongside [`PLOT_CONTACTS`].
const PLOT_CONTACT_HEIGHTS: [f32; 3] = [30.0, -60.0, 90.0];

/// [`nova_os_range`] with traffic around it: the same one ship, plus a hostile
/// close in, a friendly tender to port and a second hostile further out.
///
/// The map app plots what is in the scenario, and the rock hollow the flight
/// lessons are shot in has forty-eight asteroids in it - every one of them a
/// contact, each drawn at its own projected size, which buries the two ships
/// the lesson is about under a field of white discs. A plot is READ, so the
/// range it is read on carries the traffic and nothing else.
pub fn nova_os_plot_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let contact = |index: usize| {
        let (id, name, allegiance, side, range) = PLOT_CONTACTS[index];
        let offset = PLOT_SCREEN_RIGHT * side * range;
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: Meters3::new(offset.x, PLOT_CONTACT_HEIGHTS[index], offset.z),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                allegiance: Some(allegiance),
                design: ShipDesignSource::Inline(ShipDesign {
                    sections: vec![
                        SpaceshipSectionConfig {
                            id: format!("{id}_controller"),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Inline(section("basic_controller_section")),
                        },
                        SpaceshipSectionConfig {
                            id: format!("{id}_hull"),
                            position: Vec3::new(0.0, 0.0, 1.0),
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Inline(section("reinforced_hull_section")),
                        },
                        SpaceshipSectionConfig {
                            id: format!("{id}_thruster"),
                            position: Vec3::new(0.0, 0.0, 2.0),
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Inline(section("basic_thruster_section")),
                        },
                    ],
                    ..default()
                }),
                ..default()
            }),
        })
    };

    let mut config = nova_os_range(game_assets, sections);
    let event = config
        .events
        .first_mut()
        .expect("the NOVA OS range authors one OnStart event");
    event.actions.extend((0..PLOT_CONTACTS.len()).map(contact));
    config.id = "nova_os_plot_range".to_string();
    config.name = "NOVA OS Plot Range".to_string();
    config.description = "A range with traffic on it, for the NOVA OS map.".to_string();
    config
}

/// Press Tab to toggle the computer via the real `ButtonInput<KeyCode>` edge.
#[cfg(feature = "debug")]
pub fn press_tab(world: &mut World) {
    if let Some(mut keys) = world.get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.press(KeyCode::Tab);
    }
}

/// Send one printable character to the terminal through the real keyboard path.
///
/// A PRESS AND A RELEASE, because this goes through the real path and the real
/// path has a latch at the end of it: bevy's own keyboard system folds these
/// messages into `ButtonInput<KeyCode>`, so a press with no release leaves the
/// carrier key held down for the rest of the walk. The carrier is `KeyA` for
/// every character (the terminal reads `logical_key`, not the code), and `A` is
/// `novaos_pan_left` - so typing `map` used to open the map app with the pan
/// key stuck, and the plot slid off its own contacts while the still was taken.
#[cfg(feature = "debug")]
pub fn type_char(world: &mut World, ch: &str) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        world.write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character(ch.into()),
            state,
            text: Some(ch.into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
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

/// Advance once the CRT has finished sliding OFF the screen.
///
/// The complement of `nova_os_raster_open`, and it has to be the openness
/// rather than the pause state: the close is animated
/// (`NovaOsCloseTransition`), so the state is back to what it was several
/// frames before the raster has stopped covering what is underneath it.
#[cfg(feature = "debug")]
pub fn the_shell_is_closed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        nova_protocol::nova_os_ui::prelude::nova_os_openness(world)
            .is_none_or(|open| open <= f32::EPSILON)
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
