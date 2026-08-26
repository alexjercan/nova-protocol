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

#[cfg(feature = "debug")]
use bevy::input::{
    keyboard::{Key, KeyboardInput},
    ButtonState,
};
use bevy::{platform::collections::HashMap, prelude::*};
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

/// Type a command and submit it.
#[cfg(feature = "debug")]
pub fn run_command(world: &mut World, command: &str) {
    type_word(world, command);
    press_enter(world);
}
