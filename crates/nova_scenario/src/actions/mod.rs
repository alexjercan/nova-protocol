//! The action config vocabulary: `EventActionConfig` and its dispatch into the
//! per-concern action modules.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_events::prelude::*;

use crate::{
    filters::EventFilterConfig, names::Names, variables::VariableExpressionNode,
    world::NovaEventWorld,
};
mod flow;
mod mission;
mod sequence;
mod ship;
mod spawn;
mod timer;
mod view;

pub use flow::*;
pub use mission::*;
pub use sequence::*;
pub use ship::*;
pub use spawn::*;
pub use timer::*;
pub use view::*;

/// Glob-import surface: `use nova_scenario::actions::prelude::*` brings the
/// action config vocabulary and scenario-object types into scope.
pub mod prelude {
    pub use super::{
        advance_scenario_sequences, apply_infinite_ammo, apply_pending_skybox_swaps,
        base_scenario_object, live_ship_sections, refill_section, sequence_gate_handlers,
        AILeashConfig, ActionEffect, BaseScenarioObjectConfig, ClearShipOrderActionConfig,
        CurrentOutcome, DebugMessageActionConfig, DespawnScenarioObjectActionConfig,
        EventActionConfig, ForceAlignActionConfig, ForceRailgunFireActionConfig,
        ForceTorpedoFireActionConfig, HintEmphasisClearActionConfig, HintEmphasisSetActionConfig,
        HudReadoutActionConfig, HudReadoutFormatConfig, MoveShipToActionConfig,
        NextScenarioActionConfig, ObjectiveActionConfig, ObjectiveCompleteActionConfig,
        ObjectiveMarkerAttachActionConfig, ObjectiveMarkerDetachActionConfig,
        OrbitShipActionConfig, OutcomeActionConfig, PatrolShipActionConfig, PendingSkyboxSwap,
        RefillAmmoActionConfig, ScatterObjectsConfig, ScatterRegion, ScenarioAreaConfig,
        ScenarioObjectConfig, ScenarioObjectKind, ScenarioOutcomeKind, ScreenshotActionConfig,
        SequenceActionConfig, SequenceGateConfig, SequenceStepConfig, SetAIEngageRangeActionConfig,
        SetAILeashActionConfig, SetAIPointDefenseRangeActionConfig, SetAllegianceActionConfig,
        SetCameraActionConfig, SetControllerVerbActionConfig, SetInfiniteAmmoActionConfig,
        SetSkyboxActionConfig, SetSpeedCapActionConfig, StopShipActionConfig,
        StoryMessageActionConfig, TimerCancelActionConfig, TimerStartActionConfig,
        VariableSetActionConfig, CAPTURE_DIR_ENV, MAX_SCATTER_COUNT, NEXT_SCENARIO_DELAY_MAX_SECS,
        NEXT_SCENARIO_DELAY_WARN_SECS, OUTCOME_AUTO_ADVANCE_MAX_SECS,
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
    /// Start or restart a keyed scenario timer.
    TimerStart(TimerStartActionConfig),
    /// Cancel a keyed scenario timer.
    TimerCancel(TimerCancelActionConfig),
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
    /// Switch unlimited ammunition on or off for a scoped ship's weapons by id.
    SetInfiniteAmmo(SetInfiniteAmmoActionConfig),
    /// Refill a scoped ship's finite magazines by id, or one section of it.
    RefillAmmo(RefillAmmoActionConfig),
    /// Enable or disable one flight verb on a scoped ship's controller by id.
    SetControllerVerb(SetControllerVerbActionConfig),
    /// Overwrite a scoped ship's `Allegiance` at runtime (neutral-until-provoked).
    SetAllegiance(SetAllegianceActionConfig),
    /// Fly an ordered ship to an authored mark with the real autopilot.
    MoveShipTo(MoveShipToActionConfig),
    /// Turn an ordered ship's hull onto an authored bearing and hold it.
    ForceAlign(ForceAlignActionConfig),
    /// Bring an ordered ship to rest with the real STOP maneuver.
    StopShip(StopShipActionConfig),
    /// Fly an ordered ship one loop of an authored route.
    PatrolShip(PatrolShipActionConfig),
    /// Put an ordered ship into a station-keeping orbit and hold it.
    OrbitShip(OrbitShipActionConfig),
    /// Release an ordered ship's helm and let it drift.
    ClearShipOrder(ClearShipOrderActionConfig),
    /// Set or clear one AI ship's territorial leash.
    SetAILeash(SetAILeashActionConfig),
    /// Set or clear one AI ship's hostile-detection range.
    SetAIEngageRange(SetAIEngageRangeActionConfig),
    /// Set or clear one AI ship's point-defense range.
    SetAIPointDefenseRange(SetAIPointDefenseRangeActionConfig),
    /// Fire one named railgun section of a scripted ship.
    ForceRailgunFire(ForceRailgunFireActionConfig),
    /// Launch one named torpedo bay of a scripted ship at one named target.
    ForceTorpedoFire(ForceTorpedoFireActionConfig),
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
    /// Start a keyed ordered beat chain whose cursor the engine holds.
    Sequence(SequenceActionConfig),
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
            EventActionConfig::TimerStart(config) => {
                config.action(world, info);
            }
            EventActionConfig::TimerCancel(config) => {
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
            EventActionConfig::SetInfiniteAmmo(config) => {
                config.action(world, info);
            }
            EventActionConfig::RefillAmmo(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetControllerVerb(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetAllegiance(config) => {
                config.action(world, info);
            }
            EventActionConfig::MoveShipTo(config) => {
                config.action(world, info);
            }
            EventActionConfig::ForceAlign(config) => {
                config.action(world, info);
            }
            EventActionConfig::StopShip(config) => {
                config.action(world, info);
            }
            EventActionConfig::PatrolShip(config) => {
                config.action(world, info);
            }
            EventActionConfig::OrbitShip(config) => {
                config.action(world, info);
            }
            EventActionConfig::ClearShipOrder(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetAILeash(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetAIEngageRange(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetAIPointDefenseRange(config) => {
                config.action(world, info);
            }
            EventActionConfig::ForceRailgunFire(config) => {
                config.action(world, info);
            }
            EventActionConfig::ForceTorpedoFire(config) => {
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
            EventActionConfig::Sequence(config) => {
                config.action(world, info);
            }
        }
    }
}

impl EventActionConfig {
    /// Visit this action and every action NESTED inside it, self first, in
    /// authored order.
    ///
    /// `Sequence` is the only nesting arm today, and everything that reads an
    /// event's action list goes through here - inline queries, the object
    /// count, both lint passes. A second nesting arm therefore cannot be
    /// honoured by one walker and quietly missed by the others.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a EventActionConfig)) {
        visit(self);
        if let EventActionConfig::Sequence(config) = self {
            for step in &config.steps {
                for action in &step.actions {
                    action.walk(visit);
                }
            }
        }
    }

    /// Visit every filter nested inside this action - the `until` gates of a
    /// `Sequence` and of any sequence started from one of its steps.
    ///
    /// A gate carries the same filter vocabulary a handler does, so whatever
    /// reads a handler's filters must read these too or an inline query inside
    /// a gate is never sampled.
    pub fn walk_filters<'a>(&'a self, visit: &mut impl FnMut(&'a EventFilterConfig)) {
        self.walk(&mut |action| {
            if let EventActionConfig::Sequence(config) = action {
                for step in &config.steps {
                    for filter in step.until.iter().flat_map(|gate| &gate.filters) {
                        visit(filter);
                    }
                }
            }
        });
    }
}

/// Action that evaluates an expression and stores the result in a scenario
/// variable.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariableSetActionConfig {
    /// The scenario variable to write.
    #[reflect(@Names::Variable)]
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
#[derive(Clone, Debug, Reflect)]
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

/// What an action does to the world it runs in.
///
/// The one question that decides whether a scenario is a creative map: does
/// this action change the simulated world in a way playing could not have
/// produced? Every scenario alive uses objectives, variables and outcomes -
/// classing those as reaching-in would make every scenario a creative map and
/// the badge would mean nothing.
///
/// This is about the CONTENT, never about the player. A mod built around
/// unlimited ammunition is that mod working exactly as authored: it carries a
/// badge, its runs do not reach the stats, and nobody did anything wrong. The
/// player's own mark is `RunCheats`, set by the ORIGIN of a command - reaching
/// in through the shell or the wire - rather than by any action's class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionEffect {
    /// Scenario state and presentation: objectives, variables, timers, story,
    /// camera, outcome. Nothing the ship can feel.
    Bookkeeping,
    /// Reaches into the simulation: spawns, despawns, allegiance, speed caps,
    /// magazines, forced launches, sensor areas.
    Injection,
}

impl EventActionConfig {
    /// This action's own injection name, ignoring anything it schedules.
    ///
    /// The match is exhaustive on purpose: a new action does not compile until
    /// somebody has answered the question for it. It is the only place the
    /// classification is stated, so [`Self::effect`] and
    /// [`Self::collect_injections`] cannot disagree with it.
    fn own_injection(&self) -> Option<&'static str> {
        match self {
            EventActionConfig::DebugMessage(_)
            | EventActionConfig::VariableSet(_)
            | EventActionConfig::TimerStart(_)
            | EventActionConfig::TimerCancel(_)
            | EventActionConfig::Objective(_)
            | EventActionConfig::ObjectiveComplete(_)
            | EventActionConfig::ObjectiveMarkerAttach(_)
            | EventActionConfig::ObjectiveMarkerDetach(_)
            | EventActionConfig::HintEmphasisSet(_)
            | EventActionConfig::HintEmphasisClear(_)
            | EventActionConfig::NextScenario(_)
            | EventActionConfig::SetCamera(_)
            | EventActionConfig::Screenshot(_)
            | EventActionConfig::SetSkybox(_)
            | EventActionConfig::Outcome(_)
            | EventActionConfig::StoryMessage(_)
            | EventActionConfig::HudReadout(_) => None,

            EventActionConfig::SpawnScenarioObject(_) => Some("SpawnScenarioObject"),
            EventActionConfig::ScatterObjects(_) => Some("ScatterObjects"),
            EventActionConfig::DespawnScenarioObject(_) => Some("DespawnScenarioObject"),
            EventActionConfig::SetSpeedCap(_) => Some("SetSpeedCap"),
            EventActionConfig::SetInfiniteAmmo(_) => Some("SetInfiniteAmmo"),
            EventActionConfig::RefillAmmo(_) => Some("RefillAmmo"),
            EventActionConfig::SetControllerVerb(_) => Some("SetControllerVerb"),
            EventActionConfig::SetAllegiance(_) => Some("SetAllegiance"),
            EventActionConfig::MoveShipTo(_) => Some("MoveShipTo"),
            EventActionConfig::ForceAlign(_) => Some("ForceAlign"),
            EventActionConfig::StopShip(_) => Some("StopShip"),
            EventActionConfig::PatrolShip(_) => Some("PatrolShip"),
            EventActionConfig::OrbitShip(_) => Some("OrbitShip"),
            EventActionConfig::ClearShipOrder(_) => Some("ClearShipOrder"),
            EventActionConfig::SetAILeash(_) => Some("SetAILeash"),
            EventActionConfig::SetAIEngageRange(_) => Some("SetAIEngageRange"),
            EventActionConfig::SetAIPointDefenseRange(_) => Some("SetAIPointDefenseRange"),
            EventActionConfig::ForceRailgunFire(_) => Some("ForceRailgunFire"),
            EventActionConfig::ForceTorpedoFire(_) => Some("ForceTorpedoFire"),
            EventActionConfig::CreateScenarioArea(_) => Some("CreateScenarioArea"),

            // A sequence is only a schedule. What it is worth is whatever its
            // steps do, so it has no class of its own.
            EventActionConfig::Sequence(_) => None,
        }
    }

    /// Add every injection action this one runs to `into`, walking into a
    /// sequence's steps so a spawn buried three beats deep still counts.
    pub fn collect_injections(&self, into: &mut BTreeSet<&'static str>) {
        if let Some(name) = self.own_injection() {
            into.insert(name);
        }
        if let EventActionConfig::Sequence(config) = self {
            for action in config.steps.iter().flat_map(|step| step.actions.iter()) {
                action.collect_injections(into);
            }
        }
    }

    /// How this action is classed for the creative-map lint.
    pub fn effect(&self) -> ActionEffect {
        let mut names = BTreeSet::new();
        self.collect_injections(&mut names);
        if names.is_empty() {
            ActionEffect::Bookkeeping
        } else {
            ActionEffect::Injection
        }
    }
}

#[cfg(test)]
mod effect_tests {
    use super::*;

    #[test]
    fn scenario_bookkeeping_is_not_an_injection() {
        assert_eq!(
            EventActionConfig::Objective(ObjectiveActionConfig {
                id: "obj".to_string(),
                message: "do it".to_string(),
            })
            .effect(),
            ActionEffect::Bookkeeping
        );
    }

    #[test]
    fn the_new_ammo_actions_reach_into_the_world() {
        assert_eq!(
            EventActionConfig::SetInfiniteAmmo(SetInfiniteAmmoActionConfig {
                id: "player".to_string(),
                enabled: true,
            })
            .effect(),
            ActionEffect::Injection
        );
        assert_eq!(
            EventActionConfig::RefillAmmo(RefillAmmoActionConfig {
                id: "player".to_string(),
                section: None,
            })
            .effect(),
            ActionEffect::Injection
        );
    }

    /// A sequence is the one action whose class is not its own: burying a
    /// spawn inside a beat chain must not launder it into bookkeeping.
    #[test]
    fn a_sequence_carries_the_strongest_class_of_its_steps() {
        let step = |actions: Vec<EventActionConfig>| SequenceStepConfig {
            after: None,
            until: None,
            deadline: None,
            actions,
        };
        let quiet = EventActionConfig::Sequence(SequenceActionConfig {
            key: "quiet".to_string(),
            steps: vec![step(vec![EventActionConfig::StoryMessage(
                StoryMessageActionConfig {
                    speaker: "OKONO".to_string(),
                    text: "Strip it clean.".to_string(),
                    dwell: None,
                    icon: None,
                },
            )])],
        });
        assert_eq!(quiet.effect(), ActionEffect::Bookkeeping);

        let EventActionConfig::Sequence(mut loud) = quiet else {
            unreachable!("built as a sequence")
        };
        loud.steps.push(step(vec![EventActionConfig::SetSpeedCap(
            SetSpeedCapActionConfig {
                id: "player".to_string(),
                cap: Some(MetersPerSecond(50.0)),
            },
        )]));
        assert_eq!(
            EventActionConfig::Sequence(loud).effect(),
            ActionEffect::Injection
        );
    }
}
