//! The New Game harness: the scenario New Game loads onto the generated world.
//!
//! The scenario spawns the player's line warship and the lights. It declares
//! `world: Generated((seed: None))`, so it takes the seed the player typed and
//! the Scenarios picker renders no row for it. `nova_world_base` sees the
//! world block and the one player ship and streams the seeded sectors in
//! around it. There is no objective: the generated wrecks are the loot a
//! docked player may Take. The one outcome is the player's death, a Defeat
//! with nothing queued: the menu offers Load last save in a saved world and
//! Main Menu everywhere.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::{ItemRoleType, ShipInventoryStock};
use nova_input::prelude::InputSource;
use nova_scenario::prelude::*;
use nova_ship::prelude::ShipCapabilities;
use nova_world_base::prelude::OPEN_WORLD_SCENARIO_ID;

use crate::{
    base_content::{
        assets::BaseContentAssets,
        ships::{self, LINE_WARSHIP_PDC_IDS, LINE_WARSHIP_SHIP_ID},
    },
    scenario_helpers::prelude::entity,
};

/// The player's ship, by the id events and tools address it with.
const PLAYER_ID: &str = "player";

/// The warship's railgun and torpedo bays.
const RAILGUN_ID: &str = "railgun_spinal";
const TORPEDO_IDS: [&str; 2] = ["torpedo_port", "torpedo_starboard"];

/// What the player's ship is called.
const PLAYER_NAME: &str = "Line Warship";

/// The open-world bootstrap scenario.
pub(crate) fn open_world(assets: &BaseContentAssets) -> ScenarioConfig {
    let mut start = vec![EventActionConfig::SpawnScenarioObject(player())];
    start.extend(ThreePointRig::around(OPEN_WORLD_SCENARIO_ID, Meters3::ZERO, 25.0).actions());

    ScenarioConfig {
        description: "An open world generated from your seed: sparse asteroid clusters, \
                      planetoids and generated ships, streamed in around your line warship as you fly."
            .to_string(),
        world: ScenarioWorldType::Generated(GeneratedWorldConfig {
            seed: None,
            bounds: None,
        }),
        events: vec![
            // One spawn list for both openings: a fresh start and a resumed
            // load put the same ship and the same lights at the origin, and
            // the save's ship state lands on the ship once it stands.
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnStart,
                once: true,
                filters: vec![],
                actions: start.clone(),
            },
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnResume,
                once: true,
                filters: vec![],
                actions: start,
            },
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnDestroyed,
                once: true,
                filters: vec![entity(PLAYER_ID)],
                actions: vec![EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Your ship was destroyed.",
                ))],
            },
        ],
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
            design: ships::design(LINE_WARSHIP_SHIP_ID),
            // At rest at the bootstrap origin, before any sector streams in:
            // an intentional stationary start, not a well interaction.
            initial_velocity: MetersPerSecond3::ZERO,
            // Plates for repair and a finite reserve for every idle reload:
            // point defense, railgun and torpedo bays. 3,520 kg in all.
            inventory: ShipInventoryStock::new([
                (ItemRoleType::HullPlate.into(), 12),
                (ItemRoleType::PdcRound.into(), 6000),
                (ItemRoleType::RailSlug.into(), 20),
                (ItemRoleType::Torpedo.into(), 12),
            ]),
            lootable: false,
            credits: 2_000,
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
    let mut bindings: BTreeMap<SectionId, Vec<InputSource>> = LINE_WARSHIP_PDC_IDS
        .iter()
        .map(|id| (id.to_string(), pdc.clone()))
        .collect();
    bindings.insert(
        RAILGUN_ID.to_string(),
        vec![InputSource::Keyboard(KeyCode::KeyR)],
    );
    bindings.extend(
        TORPEDO_IDS
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
        for pdc in LINE_WARSHIP_PDC_IDS {
            assert_eq!(
                bindings[pdc],
                [
                    InputSource::Mouse(MouseButton::Left),
                    InputSource::Gamepad(GamepadButton::RightTrigger2),
                ],
                "'{pdc}'"
            );
        }
        assert_eq!(bindings[RAILGUN_ID], [InputSource::Keyboard(KeyCode::KeyR)]);
        for bay in TORPEDO_IDS {
            assert_eq!(
                bindings[bay],
                [InputSource::Keyboard(KeyCode::KeyF)],
                "'{bay}'"
            );
        }
    }
}
