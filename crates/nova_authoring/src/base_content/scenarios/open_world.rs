//! The New Game harness: the scenario New Game loads onto the generated world.
//!
//! The scenario spawns the player's ship as the start role picked on the New
//! Game screen, and the lights. It declares `world: Generated((seed: None))`,
//! so it takes the seed and the role the player picked and the Scenarios
//! picker renders no row for it. `nova_world_base` sees the world block and
//! the one player ship and streams the seeded sectors in around it.
//!
//! The player starts docked at the session's starting station, the allied
//! station nearest the seed's origin, which `SpawnStartRole` reads from the
//! session. The harness posts no objective: the jobs the player takes and
//! the start role's goal are the open world's aims, and neither runs on
//! scenario objectives. The one outcome is the player's death, a Defeat with
//! nothing queued: the menu offers Load last save in a saved world and Main
//! Menu everywhere.

use nova_events::prelude::Meters3;
use nova_scenario::prelude::*;
use nova_world_base::prelude::OPEN_WORLD_SCENARIO_ID;

use crate::{base_content::assets::BaseContentAssets, scenario_helpers::prelude::entity};

/// The player's ship, by the id events and tools address it with.
const PLAYER_ID: &str = "player";

/// The open-world bootstrap scenario.
pub(crate) fn open_world(assets: &BaseContentAssets) -> ScenarioConfig {
    let mut start = vec![EventActionConfig::SpawnStartRole(StartRoleSpawnConfig {
        id: PLAYER_ID.to_string(),
    })];
    // Directional: the rig lights the whole world, wherever the start is.
    start.extend(ThreePointRig::around(OPEN_WORLD_SCENARIO_ID, Meters3::ZERO, 25.0).actions());

    ScenarioConfig {
        description: "An open world generated from your seed: sparse asteroid clusters, \
                      planetoids and generated ships, streamed in around your ship as you fly."
            .to_string(),
        world: ScenarioWorldType::Generated(GeneratedWorldConfig {
            seed: None,
            bounds: None,
        }),
        events: vec![
            // One spawn list for both openings: a fresh start and a resumed
            // load put the same ship and the same lights in the world, and
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
