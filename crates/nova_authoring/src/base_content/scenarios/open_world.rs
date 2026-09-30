//! The open world's bootstrap: what New Game loads before the world streams.
//!
//! The scenario spawns the player's line warship, a lootable derelict tender
//! moored ahead of its port collar, and the lights. It declares
//! [`ScenarioRole::OpenWorld`], so the Scenarios picker renders no row for it.
//! `nova_world_base` sees the role and the one player ship and streams the
//! seeded sectors in around it. There is no objective and no outcome: the
//! world is the scenario.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::{Allegiance, ItemType};
use nova_input::prelude::InputSource;
use nova_scenario::prelude::*;
use nova_ship::prelude::ShipCapabilities;
use nova_world_base::prelude::{
    BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID, BLOCK_LINE_WARSHIP_SHIP_ID, OPEN_WORLD_SCENARIO_ID,
};

use crate::base_content::{
    assets::BaseContentAssets,
    ships::{
        self, BLOCK_LINE_WARSHIP_PDC_IDS, BLOCK_LINE_WARSHIP_RAILGUN_ID,
        BLOCK_LINE_WARSHIP_TORPEDO_IDS,
    },
};

/// The player's ship, by the id events and tools address it with.
const PLAYER_ID: &str = "player";

/// What the player's ship is called.
const PLAYER_NAME: &str = "Line Warship";

/// The derelict's id.
const DERELICT_ID: &str = "derelict_tender";

/// What the derelict is called.
const DERELICT_NAME: &str = "Derelict Tender";

/// Where the derelict stands. The warship's port collar is cell (-2, 0, 0)
/// facing -X, its face at (-25, 0, 0) m; the tender's port collar is cell
/// (-2, 0, -3) facing -X in its own frame, turned by [`DERELICT_ROTATION`] to
/// face +X at (-30, 0, -140) m. The player flies 140 m ahead and stops with the
/// two faces square, 5 m apart, inside the 10 m docking gap.
///
/// The warship turns about its centre of mass, 8.5 m aft of its origin. No
/// part of its hull reaches past 114 m from that centre, and no part of the
/// tender comes within 138 m of it, so turning in place at spawn cannot strike
/// the tender. Across world seeds 0 to 39,999, 20,260,922 and 20,260,923, the
/// origin cell places no generated body within reach of the tender that does
/// not already reach the warship. Each measured starboard pose clear of the
/// turn let a generated body reach the tender alone for some seed.
const DERELICT_POSITION: Meters3 = Meters3::new(-55.0, 0.0, -170.0);

/// The derelict turned 180 degrees about Y, so its port collar faces the
/// warship's port collar line.
const DERELICT_ROTATION: Quat = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);

/// The open-world bootstrap scenario.
pub(crate) fn open_world(assets: &BaseContentAssets) -> ScenarioConfig {
    let mut start = vec![
        EventActionConfig::SpawnScenarioObject(player()),
        EventActionConfig::SpawnScenarioObject(derelict()),
    ];
    start.extend(ThreePointRig::around(OPEN_WORLD_SCENARIO_ID, Meters3::ZERO, 25.0).actions());

    ScenarioConfig {
        description: "An open world generated from your seed: sparse asteroid clusters, \
                      planetoids and derelicts, streamed in around your line warship as you fly."
            .to_string(),
        role: ScenarioRole::OpenWorld,
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions: start,
        }],
        ..ScenarioConfig::new(OPEN_WORLD_SCENARIO_ID, "Open World", assets.cubemap.clone())
    }
}

/// The player's line warship at the origin, with every helm verb.
fn player() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: PLAYER_NAME.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: weapon_bindings(),
            }),
            capabilities: ShipCapabilities::default(),
            design: ships::design(BLOCK_LINE_WARSHIP_SHIP_ID),
            inventory: [(ItemType::HullPlate, 12)].into_iter().collect(),
            lootable: false,
        }),
    }
}

/// A damaged frame tender with nobody aboard, lootable, carrying the hull
/// plates a docked player can Take.
fn derelict() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: DERELICT_ID.to_string(),
            name: DERELICT_NAME.to_string(),
            position: DERELICT_POSITION,
            rotation: DERELICT_ROTATION,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            capabilities: ShipCapabilities::default(),
            design: ships::design(BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID),
            inventory: [(ItemType::HullPlate, 8)].into_iter().collect(),
            lootable: true,
        }),
    }
}

/// The warship's weapon bindings, on the keys every other player gun uses:
/// point defense on the primary trigger, the railgun on `R`, and the torpedo
/// bays on `F`.
fn weapon_bindings() -> BTreeMap<SectionId, Vec<InputSource>> {
    let pdc = vec![
        InputSource::Mouse(MouseButton::Left),
        InputSource::Gamepad(GamepadButton::RightTrigger2),
    ];
    let mut bindings: BTreeMap<SectionId, Vec<InputSource>> = BLOCK_LINE_WARSHIP_PDC_IDS
        .iter()
        .map(|id| (id.to_string(), pdc.clone()))
        .collect();
    bindings.insert(
        BLOCK_LINE_WARSHIP_RAILGUN_ID.to_string(),
        vec![InputSource::Keyboard(KeyCode::KeyR)],
    );
    bindings.extend(
        BLOCK_LINE_WARSHIP_TORPEDO_IDS
            .iter()
            .map(|id| (id.to_string(), vec![InputSource::Keyboard(KeyCode::KeyF)])),
    );
    bindings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every warship weapon is bound, each group on its own convention: point
    /// defense on the primary trigger, the railgun on `R`, torpedoes on `F`.
    #[test]
    fn the_open_world_warship_binds_pdc_to_the_trigger_railgun_to_r_and_torpedoes_to_f() {
        let bindings = weapon_bindings();
        assert_eq!(bindings.len(), 6 + 1 + 2);
        for pdc in BLOCK_LINE_WARSHIP_PDC_IDS {
            assert_eq!(
                bindings[pdc],
                [
                    InputSource::Mouse(MouseButton::Left),
                    InputSource::Gamepad(GamepadButton::RightTrigger2),
                ],
                "'{pdc}'"
            );
        }
        assert_eq!(
            bindings[BLOCK_LINE_WARSHIP_RAILGUN_ID],
            [InputSource::Keyboard(KeyCode::KeyR)]
        );
        for bay in BLOCK_LINE_WARSHIP_TORPEDO_IDS {
            assert_eq!(
                bindings[bay],
                [InputSource::Keyboard(KeyCode::KeyF)],
                "'{bay}'"
            );
        }
    }
}
