//! The action config vocabulary: `EventActionConfig` and its dispatch into the
//! per-concern action modules.

use bevy::prelude::*;
use nova_events::prelude::*;

use crate::{variables::VariableExpressionNode, world::NovaEventWorld};
mod flow;
mod mission;
mod ship;
mod spawn;
mod view;

pub use flow::*;
pub use mission::*;
pub use ship::*;
pub use spawn::*;
pub use view::*;

/// Glob-import surface: `use nova_scenario::actions::prelude::*` brings the
/// action config vocabulary and scenario-object types into scope.
pub mod prelude {
    pub use super::{
        apply_pending_skybox_swaps, base_scenario_object, BaseScenarioObjectConfig, CurrentOutcome,
        DebugMessageActionConfig, DespawnScenarioObjectActionConfig, EventActionConfig,
        HintEmphasisClearActionConfig, HintEmphasisSetActionConfig, HudReadoutActionConfig,
        HudReadoutFormat, NextScenarioActionConfig, ObjectiveActionConfig,
        ObjectiveCompleteActionConfig, ObjectiveMarkerAttachActionConfig,
        ObjectiveMarkerDetachActionConfig, OutcomeActionConfig, PendingSkyboxSwap,
        ScatterObjectsConfig, ScatterRegion, ScenarioAreaConfig, ScenarioObjectConfig,
        ScenarioObjectKind, ScenarioOutcomeKind, ScreenshotActionConfig, SetAllegianceActionConfig,
        SetCameraActionConfig, SetControllerVerbActionConfig, SetSkyboxActionConfig,
        SetSpeedCapActionConfig, StoryMessageActionConfig, VariableSetActionConfig,
        NEXT_SCENARIO_DELAY_MAX_SECS, NEXT_SCENARIO_DELAY_WARN_SECS, OUTCOME_AUTO_ADVANCE_MAX_SECS,
    };
}

/// What a handler does when it fires: one entry in the RON `actions` list,
/// run in order after every filter passes.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventActionConfig {
    /// Log a message.
    DebugMessage(DebugMessageActionConfig),
    /// Evaluate an expression into a scenario variable.
    VariableSet(VariableSetActionConfig),
    /// Add a HUD objective by id.
    Objective(ObjectiveActionConfig),
    /// Complete a HUD objective by id.
    ObjectiveComplete(ObjectiveCompleteActionConfig),
    /// Attach the gold objective marker chip to the scoped object by id.
    ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig),
    /// Remove the objective marker chip from the scoped object by id.
    ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig),
    /// Pulse one keybind-dock chip gold.
    HintEmphasisSet(HintEmphasisSetActionConfig),
    /// Clear a keybind-dock chip's gold emphasis.
    HintEmphasisClear(HintEmphasisClearActionConfig),
    /// Spawn a scenario object.
    SpawnScenarioObject(ScenarioObjectConfig),
    /// Spawn many scenario objects across a region (id-prefixed).
    ScatterObjects(ScatterObjectsConfig),
    /// Despawn the scoped object whose id matches.
    DespawnScenarioObject(DespawnScenarioObjectActionConfig),
    /// Install or remove the manual flight speed cap on a scoped ship by id.
    SetSpeedCap(SetSpeedCapActionConfig),
    /// Enable or disable one flight verb on a scoped ship's controller by id.
    SetControllerVerb(SetControllerVerbActionConfig),
    /// Overwrite a scoped ship's `Allegiance` at runtime (neutral-until-provoked).
    SetAllegiance(SetAllegianceActionConfig),
    /// Spawn a spherical sensor zone that drives `OnEnter`/`OnExit`.
    CreateScenarioArea(ScenarioAreaConfig),
    /// Queue a switch to another scenario by id.
    NextScenario(NextScenarioActionConfig),
    /// Pose the scenario camera for a scripted shot (photo mode).
    SetCamera(SetCameraActionConfig),
    /// Capture the primary window to a PNG (photo mode).
    Screenshot(ScreenshotActionConfig),
    /// Swap the scenario's skybox cubemap mid-scenario (modding hook).
    SetSkybox(SetSkyboxActionConfig),
    /// Declare the scenario's win/lose outcome (drives the outcome overlay).
    Outcome(OutcomeActionConfig),
    /// Speaker-attributed story text, rendered by the HUD comms panel.
    StoryMessage(StoryMessageActionConfig),
    /// Show (or clear) a named HUD readout bound to a scenario variable - the
    /// display half of the scenario-variable vocabulary.
    HudReadout(HudReadoutActionConfig),
}

impl EventAction<NovaEventWorld> for EventActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        match self {
            EventActionConfig::DebugMessage(config) => {
                config.action(world, info);
            }
            EventActionConfig::VariableSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::Objective(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveComplete(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveMarkerAttach(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveMarkerDetach(config) => {
                config.action(world, info);
            }
            EventActionConfig::HintEmphasisSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::HintEmphasisClear(config) => {
                config.action(world, info);
            }
            EventActionConfig::SpawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::ScatterObjects(config) => {
                config.action(world, info);
            }
            EventActionConfig::DespawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetSpeedCap(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetControllerVerb(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetAllegiance(config) => {
                config.action(world, info);
            }
            EventActionConfig::CreateScenarioArea(config) => {
                config.action(world, info);
            }
            EventActionConfig::NextScenario(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetCamera(config) => {
                config.action(world, info);
            }
            EventActionConfig::Screenshot(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetSkybox(config) => {
                config.action(world, info);
            }
            EventActionConfig::Outcome(config) => {
                config.action(world, info);
            }
            EventActionConfig::StoryMessage(config) => {
                config.action(world, info);
            }
            EventActionConfig::HudReadout(config) => {
                config.action(world, info);
            }
        }
    }
}

/// Action that evaluates an expression and stores the result in a scenario
/// variable.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariableSetActionConfig {
    /// The scenario variable to write.
    pub key: String,
    /// The expression evaluated (against the current variables) into that key.
    pub expression: VariableExpressionNode,
}

impl EventAction<NovaEventWorld> for VariableSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        match self.expression.evaluate(world) {
            Ok(literal) => {
                world.insert_variable(self.key.clone(), literal);
            }
            Err(e) => {
                error!(
                    "VariableSetActionConfig: failed to evaluate expression for key '{}': {:?}",
                    self.key, e
                );
            }
        }
    }
}

/// Action that logs a message; an authoring/debugging aid with no game effect.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DebugMessageActionConfig {
    /// The text to log.
    pub message: String,
}

impl EventAction<NovaEventWorld> for DebugMessageActionConfig {
    fn action(&self, _: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!("Event Action Message: {}", self.message);
    }
}
