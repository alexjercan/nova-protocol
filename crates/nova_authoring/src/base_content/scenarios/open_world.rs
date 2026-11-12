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
//! session. A fresh world posts one objective, [`TAKE_A_JOB_OBJECTIVE`], so
//! the HUD names the first step and the interface key; the player's first
//! accepted job completes it. It is a cue, not a gate: nothing waits on it.
//! A Load restores it through the saved scenario state, so only `OnStart`
//! posts it. Its chip is held until then, and the hold is not saved, so both
//! openings set it; on a Load after the accept there is nothing left to hold.
//! Station control speaks three times, once each per world: when the first
//! job's work is done it sends the player back to the poster, on the first
//! hand-in it pays [`FIRST_DELIVERY_BONUS_CR`] over the reward, and when the
//! world takes a held job it calls the contract void. A procurement has no
//! work in the world, so the hand-in line does not wait on the first one.
//! The jobs the player takes and the start role's goal are the open
//! world's aims, and neither runs on scenario objectives. The one outcome is
//! the player's death, a Defeat with nothing queued: the menu offers Load last
//! save in a saved world and Main Menu everywhere.

use nova_events::prelude::Meters3;
use nova_scenario::prelude::*;
use nova_world_base::prelude::OPEN_WORLD_SCENARIO_ID;

use crate::{
    base_content::assets::BaseContentAssets,
    scenario_helpers::prelude::{comms, entity},
};

/// The player's ship, by the id events and tools address it with.
const PLAYER_ID: &str = "player";

/// The first-step cue a fresh world posts, by the id its completion names.
const TAKE_A_JOB_OBJECTIVE: &str = "take_a_job";

/// Who speaks the job lines: the station side of the comms channel.
const STATION_CONTROL: &str = "Station Control";

/// What the first hand-in of a world pays over the job's reward, in credits.
const FIRST_DELIVERY_BONUS_CR: i32 = 50;

/// A filter that passes a job the world took: `event.cause == "Lost"`.
fn job_lost() -> EventFilterConfig {
    let text = |factor| VariableExpressionNode::new_term(VariableTermNode::new_factor(factor));
    EventFilterConfig::Expression(ExpressionFilterConfig(VariableConditionNode::new_equals(
        text(VariableFactorNode::new_event_field("cause")),
        text(VariableFactorNode::new_literal(VariableLiteral::String(
            "Lost".to_string(),
        ))),
    )))
}

/// The open-world bootstrap scenario.
pub(crate) fn open_world(assets: &BaseContentAssets) -> ScenarioConfig {
    let mut start = vec![EventActionConfig::SpawnStartRole(StartRoleSpawnConfig {
        id: PLAYER_ID.to_string(),
    })];
    // Directional: the rig lights the whole world, wherever the start is.
    start.extend(ThreePointRig::around(OPEN_WORLD_SCENARIO_ID, Meters3::ZERO, 25.0).actions());
    let hold = EventActionConfig::ObjectiveChipHold(ObjectiveChipHoldActionConfig {
        id: TAKE_A_JOB_OBJECTIVE.to_string(),
    });
    // The objective first: a hold takes effect only on an active objective.
    let mut fresh_start = start.clone();
    fresh_start.push(EventActionConfig::Objective(ObjectiveActionConfig::new(
        TAKE_A_JOB_OBJECTIVE,
        "Take a job",
    )));
    fresh_start.push(hold.clone());
    start.push(hold);

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
                position: None,
                name: EventConfig::OnStart,
                once: true,
                filters: vec![],
                actions: fresh_start,
            },
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnResume,
                once: true,
                filters: vec![],
                actions: start,
            },
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnDestroyed,
                once: true,
                filters: vec![entity(PLAYER_ID)],
                actions: vec![EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Your ship was destroyed.",
                ))],
            },
            // From here on the list only grows at its end: a save names its
            // spent `once` handlers by index, so a handler added before any
            // of these would move them.
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnJobAccepted,
                once: true,
                filters: vec![],
                actions: vec![EventActionConfig::ObjectiveComplete(
                    ObjectiveCompleteActionConfig {
                        id: TAKE_A_JOB_OBJECTIVE.to_string(),
                    },
                )],
            },
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnJobDone,
                once: true,
                filters: vec![],
                actions: vec![comms(
                    STATION_CONTROL,
                    "Work confirmed. Return to the station that posted the job and hand it in.",
                )],
            },
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnJobHandedIn,
                once: true,
                filters: vec![],
                actions: vec![
                    EventActionConfig::GrantCredits(GrantCreditsActionConfig {
                        ship: PLAYER_ID.to_string(),
                        amount: FIRST_DELIVERY_BONUS_CR,
                    }),
                    comms(
                        STATION_CONTROL,
                        format!(
                            "First delivery logged. A {FIRST_DELIVERY_BONUS_CR} cr bonus is on \
                             your account. The board has more work."
                        ),
                    ),
                ],
            },
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnJobDropped,
                once: true,
                filters: vec![job_lost()],
                actions: vec![comms(
                    STATION_CONTROL,
                    "Contract void. The target of the job is lost. The board has other work.",
                )],
            },
        ],
        ..ScenarioConfig::new(OPEN_WORLD_SCENARIO_ID, "Open World", assets.cubemap.clone())
    }
}
