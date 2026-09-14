//! The action config vocabulary: `EventActionConfig` and its dispatch into the
//! per-concern action modules.
//!
//! The vocabulary itself is the TABLE below, expanded by
//! [`registry::scenario_actions`]. One row declares an action's enum arm, its
//! dispatch, its menu label and id stem, its creative-map class and whether an
//! inspector can draw its payload. Adding an action is that row, the payload
//! struct in a module beside this one, a stock value in the editor, a lint rule
//! and a line of documentation.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::SpaceshipRootMarker;

use crate::{
    filters::EventFilterConfig, loader::prelude::ScenarioScopedMarker, names::Names,
    variables::VariableExpressionNode, world::NovaEventWorld,
};
mod audio;
mod cinematic;
mod flow;
mod mission;
pub(crate) mod registry;
mod sequence;
mod ship;
mod spawn;
mod timer;
mod view;

pub use audio::*;
pub use cinematic::*;
pub use flow::*;
pub use mission::*;
pub use sequence::*;
pub use ship::*;
pub use spawn::*;
pub use timer::*;
pub use view::*;

/// Glob-import surface: `use nova_scenario::actions::prelude::*` brings the
/// action config vocabulary and scenario-object types into scope.
///
/// A glob over the module, not a written list: the action vocabulary is
/// generated from one table and a hand-kept list is one more place for a new
/// action to be missing from.
pub mod prelude {
    pub use super::*;
}

/// The query filter that decides which live entities an authored scenario id
/// may address: scenario-SCOPED entities, and nothing else.
///
/// Load-bearing, not decoration. Spaceship SECTIONS also carry [`EntityId`] -
/// their per-ship section ids like "controller" - and an unscoped match on such
/// an id would rip that section out of every ship in the scene, or answer a
/// speed query with whichever unrelated hull the iterator reached first.
///
/// Every authored-id lookup in this crate goes through this ONE filter: the
/// three `World` resolutions below, and the sampler's system `Query`. A tenth
/// site that wants a different class of entity narrows this filter rather than
/// restating it, so the section rule cannot be forgotten in a copy.
pub type ScenarioAddressable = With<ScenarioScopedMarker>;

/// Every live entity an authored scenario id addresses.
///
/// Plural because an authored id is not unique - a scatter field and a
/// re-spawn can both put one id on several objects - and the actions that ACT
/// on an id (despawn, objective-marker attach and detach) are meant to reach
/// all of them. Callers report the empty case themselves: a miss is a typo for
/// one action and a legitimate script shape for another.
pub fn scoped_entities(world: &mut World, id: &str) -> Vec<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), ScenarioAddressable>();
    query
        .iter(world)
        .filter(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
        .collect()
}

/// The first live entity an authored scenario id addresses.
///
/// The singular variant, for the actions that READ one entity - a camera
/// anchor and its look-at target, where a second match would mean framing two
/// shots at once.
pub fn scoped_entity(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), ScenarioAddressable>();
    query
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// The live spaceship ROOT an authored scenario id addresses.
///
/// Narrower than [`scoped_entity`], and deliberately: the ship actions retune a
/// HULL - a speed cap, an allegiance, a helm order, a magazine - so a scoped
/// beacon or salvage crate that happens to share the id is not an answer to the
/// question they ask. Their warnings say "no scoped SHIP with id" for the same
/// reason.
pub fn scoped_ship(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world
        .query_filtered::<(Entity, &EntityId), (ScenarioAddressable, With<SpaceshipRootMarker>)>();
    query
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

registry::scenario_actions! {
    /// Add a HUD objective by id.
    Objective(ObjectiveActionConfig) {
        label: "Objective",
        stem: "objective",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Complete a HUD objective by id.
    ObjectiveComplete(ObjectiveCompleteActionConfig) {
        label: "Objective Complete",
        stem: "complete",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Attach the gold objective marker chip to the scoped object by id.
    ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig) {
        label: "Marker Attach",
        stem: "marker",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Remove the objective marker chip from the scoped object by id.
    ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig) {
        label: "Marker Detach",
        stem: "unmarker",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Speaker-attributed story text on a named channel, rendered by the HUD
    /// comms panel.
    NarrativeCue(NarrativeCueActionConfig) {
        label: "Narrative Cue",
        stem: "cue",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Show (or clear) a named HUD readout bound to a scenario variable - the
    /// display half of the scenario-variable vocabulary.
    HudReadout(HudReadoutActionConfig) {
        label: "HUD Readout",
        stem: "readout",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Pulse one keybind-dock chip gold.
    HintEmphasisSet(HintEmphasisSetActionConfig) {
        label: "Hint Emphasis",
        stem: "hint",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Clear a keybind-dock chip's gold emphasis.
    HintEmphasisClear(HintEmphasisClearActionConfig) {
        label: "Hint Clear",
        stem: "unhint",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Spawn a scenario object.
    SpawnScenarioObject(ScenarioObjectConfig) {
        label: "Spawn Object",
        stem: "spawn",
        effect: Injection,
        inspect: Reflect,
    },
    /// Spawn many scenario objects across a region (id-prefixed).
    ScatterObjects(ScatterObjectsConfig) {
        label: "Scatter Objects",
        stem: "scatter",
        effect: Injection,
        inspect: Reflect,
    },
    /// Despawn the scoped object whose id matches.
    DespawnScenarioObject(DespawnScenarioObjectActionConfig) {
        label: "Despawn Object",
        stem: "despawn",
        effect: Injection,
        inspect: Reflect,
    },
    /// Spawn a spherical sensor zone that drives `OnEnter`/`OnExit`.
    CreateScenarioArea(ScenarioAreaConfig) {
        label: "Create Area",
        stem: "area",
        effect: Injection,
        inspect: Reflect,
    },
    /// Swap the scenario's skybox cubemap mid-scenario (modding hook).
    SetSkybox(SetSkyboxActionConfig) {
        label: "Set Skybox",
        stem: "sky",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Pose the scenario camera for a scripted shot (photo mode).
    SetCamera(SetCameraActionConfig) {
        label: "Set Camera",
        stem: "camera",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Anchor the scenario camera to a live object and hold a pose relative to
    /// it (a cinematic that keeps the player's ship in the shot).
    SetCameraAnchor(SetCameraAnchorActionConfig) {
        label: "Anchor Camera",
        stem: "anchor",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Hand the scenario camera back to the player's chase rig.
    ReleaseCamera(ReleaseCameraActionConfig) {
        label: "Release Camera",
        stem: "release",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Block human gameplay input and clear held ship intent.
    SuspendPlayerControl(SuspendPlayerControlActionConfig) {
        label: "Suspend Player Control",
        stem: "suspend",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Restore human gameplay input after an explicit suspension.
    ResumePlayerControl(ResumePlayerControlActionConfig) {
        label: "Resume Player Control",
        stem: "resume",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Play an authored sound cue.
    PlaySound(PlaySoundActionConfig) {
        label: "Play Sound",
        stem: "sound",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Capture the primary window to a PNG (photo mode).
    Screenshot(ScreenshotActionConfig) {
        label: "Screenshot",
        stem: "shot",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Install or remove the manual flight speed cap on a scoped ship by id.
    SetSpeedCap(SetSpeedCapActionConfig) {
        label: "Set Speed Cap",
        stem: "cap",
        effect: Injection,
        inspect: Reflect,
    },
    /// Enable or disable one flight verb on a scoped ship's controller by id.
    SetControllerVerb(SetControllerVerbActionConfig) {
        label: "Set Flight Verb",
        stem: "verb",
        effect: Injection,
        inspect: Reflect,
    },
    /// Overwrite a scoped ship's `Allegiance` at runtime (neutral-until-provoked).
    SetAllegiance(SetAllegianceActionConfig) {
        label: "Set Allegiance",
        stem: "allegiance",
        effect: Injection,
        inspect: Reflect,
    },
    /// Fly an ordered ship to an authored mark with the real autopilot.
    MoveShipTo(MoveShipToActionConfig) {
        label: "Move Ship To",
        stem: "move",
        effect: Injection,
        inspect: Reflect,
    },
    /// Turn an ordered ship's hull onto an authored bearing and hold it.
    ForceAlign(ForceAlignActionConfig) {
        label: "Force Align",
        stem: "align",
        effect: Injection,
        inspect: Reflect,
    },
    /// Bring an ordered ship to rest with the real STOP maneuver.
    StopShip(StopShipActionConfig) {
        label: "Stop Ship",
        stem: "stop",
        effect: Injection,
        inspect: Reflect,
    },
    /// Fly an ordered ship one loop of an authored route.
    PatrolShip(PatrolShipActionConfig) {
        label: "Patrol Ship",
        stem: "patrol",
        effect: Injection,
        inspect: Reflect,
    },
    /// Put an ordered ship into a station-keeping orbit and hold it.
    OrbitShip(OrbitShipActionConfig) {
        label: "Orbit Ship",
        stem: "orbit",
        effect: Injection,
        inspect: Reflect,
    },
    /// Release an ordered ship's helm and let it drift.
    ClearShipOrder(ClearShipOrderActionConfig) {
        label: "Clear Ship Order",
        stem: "unorder",
        effect: Injection,
        inspect: Reflect,
    },
    /// Set or clear one AI ship's territorial leash.
    SetAILeash(SetAILeashActionConfig) {
        label: "Set AI Leash",
        stem: "leash",
        effect: Injection,
        inspect: Reflect,
    },
    /// Set or clear one AI ship's hostile-detection range.
    SetAIEngageRange(SetAIEngageRangeActionConfig) {
        label: "Set AI Engage Range",
        stem: "engage",
        effect: Injection,
        inspect: Reflect,
    },
    /// Set or clear one AI ship's point-defense range.
    SetAIPointDefenseRange(SetAIPointDefenseRangeActionConfig) {
        label: "Set AI PD Range",
        stem: "pd",
        effect: Injection,
        inspect: Reflect,
    },
    /// Fire one named railgun section of a scripted ship.
    ForceRailgunFire(ForceRailgunFireActionConfig) {
        label: "Railgun Fire",
        stem: "railgun",
        effect: Injection,
        inspect: Reflect,
    },
    /// Launch one named torpedo bay of a scripted ship at one named target.
    ForceTorpedoFire(ForceTorpedoFireActionConfig) {
        label: "Torpedo Fire",
        stem: "torpedo",
        effect: Injection,
        inspect: Reflect,
    },
    /// Switch unlimited ammunition on or off for a scoped ship's weapons by id.
    SetInfiniteAmmo(SetInfiniteAmmoActionConfig) {
        label: "Set Infinite Ammo",
        stem: "unlimited",
        effect: Injection,
        inspect: Reflect,
    },
    /// Refill a scoped ship's finite magazines by id, or one section of it.
    RefillAmmo(RefillAmmoActionConfig) {
        label: "Refill Ammo",
        stem: "refill",
        effect: Injection,
        inspect: Reflect,
    },
    /// Declare how the scenario ended - won or lost - and raise the outcome
    /// overlay that says so.
    Outcome(OutcomeActionConfig) {
        label: "Outcome",
        stem: "outcome",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Queue a switch to another scenario by id.
    NextScenario(NextScenarioActionConfig) {
        label: "Next Scenario",
        stem: "next",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Start a keyed ordered beat chain whose cursor the engine holds.
    Sequence(SequenceActionConfig) {
        label: "Sequence",
        stem: "sequence",
        effect: Nested,
        inspect: Opaque,
    },
    /// Start or restart a keyed scenario timer.
    TimerStart(TimerStartActionConfig) {
        label: "Timer Start",
        stem: "timer",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Cancel a keyed scenario timer.
    TimerCancel(TimerCancelActionConfig) {
        label: "Timer Cancel",
        stem: "untimer",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Play an ordered beat chain as a skip-safe scene.
    Cinematic(CinematicActionConfig) {
        label: "Cinematic",
        stem: "scene",
        effect: Nested,
        inspect: Opaque,
    },
    /// Post the shot's title card: where, when, and one line about it.
    CinematicTitle(CinematicTitleActionConfig) {
        label: "Cinematic Title",
        stem: "title",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// End a running cinematic from the scenario.
    CancelCinematic(CancelCinematicActionConfig) {
        label: "Cancel Cinematic",
        stem: "unscene",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Evaluate an expression into a scenario variable.
    VariableSet(VariableSetActionConfig) {
        label: "Variable Set",
        stem: "set",
        effect: Bookkeeping,
        inspect: Reflect,
    },
    /// Log a message.
    DebugMessage(DebugMessageActionConfig) {
        label: "Debug Message",
        stem: "debug",
        effect: Bookkeeping,
        inspect: Reflect,
    },
}

impl EventActionConfig {
    /// The keyed beat chain this action runs, if it runs one: the ONE place a
    /// nesting arm is declared.
    ///
    /// Everything that reads an event's action list - inline queries, the
    /// object count, the wake profile, both lint passes, the gate handlers the
    /// loader spawns - goes through this or through the walkers built on it.
    /// A nesting arm therefore cannot be honoured by one reader and quietly
    /// missed by the others.
    pub fn step_chain(&self) -> Option<(&str, &[SequenceStepConfig])> {
        match self {
            EventActionConfig::Sequence(config) => Some((&config.key, &config.steps)),
            EventActionConfig::Cinematic(config) => Some((&config.key, &config.steps)),
            _ => None,
        }
    }

    /// The same chain, for writing: the mutable half of [`step_chain`].
    ///
    /// [`step_chain`]: EventActionConfig::step_chain
    pub fn step_chain_mut(&mut self) -> Option<&mut Vec<SequenceStepConfig>> {
        match self {
            EventActionConfig::Sequence(config) => Some(&mut config.steps),
            EventActionConfig::Cinematic(config) => Some(&mut config.steps),
            _ => None,
        }
    }

    /// Visit this action and every action NESTED inside it, self first, in
    /// authored order, with each one open to edit.
    ///
    /// The authoring pass that stamps a portrait onto every line goes through
    /// here, so a line inside a scene is reached on the same terms as one at
    /// the top of a handler.
    pub fn walk_mut(&mut self, visit: &mut impl FnMut(&mut EventActionConfig)) {
        visit(self);
        let Some(steps) = self.step_chain_mut() else {
            return;
        };
        for step in steps {
            for action in &mut step.actions {
                action.walk_mut(visit);
            }
        }
    }

    /// Visit this action and every action NESTED inside it, self first, in
    /// authored order.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a EventActionConfig)) {
        visit(self);
        let Some((_, steps)) = self.step_chain() else {
            return;
        };
        for step in steps {
            for action in &step.actions {
                action.walk(visit);
            }
        }
    }

    /// Visit every filter nested inside this action - the `until` gates of its
    /// own beats and of any chain started from one of them.
    ///
    /// A gate carries the same filter vocabulary a handler does, so whatever
    /// reads a handler's filters must read these too or an inline query inside
    /// a gate is never sampled.
    pub fn walk_filters<'a>(&'a self, visit: &mut impl FnMut(&'a EventFilterConfig)) {
        self.walk(&mut |action| {
            let Some((_, steps)) = action.step_chain() else {
                return;
            };
            for step in steps {
                for filter in step.until.iter().flat_map(|gate| &gate.filters) {
                    visit(filter);
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
    /// Add every injection action this one runs to `into`, walking into a
    /// sequence's steps so a spawn buried three beats deep still counts.
    pub fn collect_injections(&self, into: &mut BTreeSet<&'static str>) {
        if let Some(name) = self.tag().injection() {
            into.insert(name);
        }
        if let Some((_, steps)) = self.step_chain() {
            for action in steps.iter().flat_map(|step| step.actions.iter()) {
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
    use nova_gameplay::prelude::CHANNEL_COMMS;

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
            steps: vec![step(vec![EventActionConfig::NarrativeCue(
                NarrativeCueActionConfig {
                    channel: CHANNEL_COMMS.to_string(),
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

#[cfg(test)]
mod table_tests {
    use std::collections::BTreeSet;

    use super::*;

    /// The table is what a menu lists and what a minted id is named after, so
    /// two rows sharing either would put two different actions behind one row
    /// and one name.
    #[test]
    fn every_action_has_its_own_name_label_and_stem() {
        for field in [ActionTag::name, ActionTag::label, ActionTag::stem] {
            let distinct: BTreeSet<&str> = ActionTag::ALL.iter().map(|tag| field(*tag)).collect();
            assert_eq!(
                distinct.len(),
                ActionTag::COUNT,
                "two rows share a name, a label or a stem"
            );
            assert!(
                distinct.iter().all(|value| !value.trim().is_empty()),
                "a row left one of its three names empty"
            );
        }
    }

    /// The name is the RON tag, so the lookup a hand-written mod is read
    /// through has to answer for every row and refuse everything else.
    #[test]
    fn an_action_is_found_by_the_tag_it_is_authored_as() {
        for tag in ActionTag::ALL {
            assert_eq!(ActionTag::named(tag.name()), Some(tag));
        }
        assert_eq!(
            ActionTag::named("StoryMesssage"),
            None,
            "a typo finds nothing"
        );
        assert_eq!(ActionTag::named(""), None);
    }

    /// The RON tag an action SERIALIZES as and the name the table reports are
    /// one string. A row whose name drifted from its variant would send a lint
    /// or an editor looking for an action the file never spells that way.
    #[cfg(feature = "serde")]
    #[test]
    fn the_table_name_is_the_tag_the_action_serializes_as() {
        let samples = [
            EventActionConfig::DebugMessage(DebugMessageActionConfig {
                message: String::new(),
            }),
            EventActionConfig::Objective(ObjectiveActionConfig::new("id", "text")),
            EventActionConfig::Sequence(SequenceActionConfig {
                key: "run".to_string(),
                steps: Vec::new(),
            }),
            EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig {
                id: "player".to_string(),
                cap: None,
            }),
            EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig),
        ];
        for action in samples {
            let ron = ron::to_string(&action).expect("the action serializes");
            let tag = action.tag();
            assert!(
                ron.starts_with(tag.name()),
                "{ron} does not open with the table's name {}",
                tag.name()
            );
        }
    }

    /// An action that reaches into the simulation reports itself under its own
    /// name, so the creative-map badge names something a reader can find in the
    /// file. A bookkeeping row reports nothing at all.
    #[test]
    fn an_injection_reports_its_own_name_and_bookkeeping_reports_none() {
        for tag in ActionTag::ALL {
            if let Some(name) = tag.injection() {
                assert_eq!(name, tag.name());
            }
        }
        assert_eq!(ActionTag::Objective.injection(), None);
        assert_eq!(
            ActionTag::Sequence.injection(),
            None,
            "a schedule is worth its steps"
        );
        assert_eq!(
            ActionTag::SpawnScenarioObject.injection(),
            Some("SpawnScenarioObject")
        );
    }

    /// Only `Sequence` is drawn by the editor itself; everything else has to
    /// hand an inspector its fields, or its panel comes up blank.
    #[test]
    fn every_action_but_the_beat_chain_offers_a_reflected_payload() {
        let mut sequence = EventActionConfig::Sequence(SequenceActionConfig {
            key: "run".to_string(),
            steps: Vec::new(),
        });
        assert!(sequence.payload().is_none());
        assert!(sequence.payload_mut().is_none());

        let mut leaf = EventActionConfig::DebugMessage(DebugMessageActionConfig {
            message: String::new(),
        });
        assert!(leaf.payload().is_some());
        assert!(leaf.payload_mut().is_some());
    }
}

#[cfg(test)]
mod scope_tests {
    use nova_gameplay::prelude::SectionMarker;

    use super::*;

    /// A spaceship SECTION carrying an `EntityId` that spells an authored
    /// scenario id is addressed by NOTHING: not the plural resolution the
    /// despawn and the objective marker use, not the singular one the camera
    /// uses, not the ship one the helm and ammunition actions use, and not the
    /// filter the per-frame speed sampler runs.
    ///
    /// Section ids are local to a hull and reused across unrelated ships, so an
    /// unscoped match would rip that section out of every ship in the scene.
    /// Asserted through all four spellings of the rule in one test: a tenth
    /// call site inherits this pin instead of answering the question a tenth
    /// way.
    #[test]
    fn a_hull_section_never_answers_an_authored_scenario_id() {
        let mut world = World::new();
        let ship = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("cutter".to_string()),
            ))
            .id();
        // The collision: a section of that very hull whose per-ship id spells
        // the authored one. Sections are spawned under their ship and never as
        // scenario objects, so it carries no ScenarioScopedMarker.
        world.spawn((
            ChildOf(ship),
            SectionMarker,
            EntityId::new("cutter".to_string()),
        ));

        assert_eq!(
            scoped_entities(&mut world, "cutter"),
            vec![ship],
            "a despawn or a marker attach reaches the authored object only"
        );
        assert_eq!(
            scoped_entity(&mut world, "cutter"),
            Some(ship),
            "a camera anchor frames the authored object, not a section of it"
        );
        assert_eq!(
            scoped_ship(&mut world, "cutter"),
            Some(ship),
            "a helm or ammunition action retunes the hull, not a section"
        );

        let mut sampled = world.query_filtered::<&EntityId, ScenarioAddressable>();
        assert_eq!(
            sampled.iter(&world).filter(|id| id.0 == "cutter").count(),
            1,
            "the speed sampler sees one entity under an authored id, so it \
             reports a value instead of a duplicate-id error"
        );
    }

    /// `scoped_ship` is the deliberately NARROW variant: the ship actions
    /// retune a hull, so a scoped object of another kind that happens to share
    /// the id is not an answer to the question they ask, and their "no scoped
    /// SHIP with id" warning is the honest report.
    #[test]
    fn a_scoped_object_that_is_not_a_ship_answers_no_ship_lookup() {
        let mut world = World::new();
        let beacon = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        assert_eq!(scoped_entity(&mut world, "beacon_1"), Some(beacon));
        assert_eq!(scoped_ship(&mut world, "beacon_1"), None);
    }
}

#[cfg(test)]
mod walk_tests {
    use super::*;
    use crate::events::EventConfig;

    fn note(message: &str) -> EventActionConfig {
        EventActionConfig::DebugMessage(DebugMessageActionConfig {
            message: message.to_string(),
        })
    }

    fn gate(id: &str) -> SequenceGateConfig {
        SequenceGateConfig {
            name: EventConfig::OnEnter,
            filters: vec![EventFilterConfig::Entity(
                crate::prelude::EntityFilterConfig {
                    id: Some(id.to_string()),
                    ..Default::default()
                },
            )],
        }
    }

    /// A sequence inside a sequence: the beat chain the strike is authored as.
    fn nested() -> EventActionConfig {
        EventActionConfig::Sequence(SequenceActionConfig {
            key: "outer".to_string(),
            steps: vec![SequenceStepConfig {
                until: Some(gate("outer_mark")),
                deadline: Some(60.0),
                actions: vec![
                    note("outer beat"),
                    EventActionConfig::Sequence(SequenceActionConfig {
                        key: "inner".to_string(),
                        steps: vec![SequenceStepConfig {
                            until: Some(gate("inner_mark")),
                            deadline: Some(60.0),
                            actions: vec![note("inner beat")],
                            ..Default::default()
                        }],
                    }),
                ],
                ..Default::default()
            }],
        })
    }

    /// Everything that reads an action list goes through `walk`, so a beat
    /// buried in a nested sequence has to arrive at the visitor - self first,
    /// then in authored order.
    #[test]
    fn walk_reaches_a_beat_inside_a_nested_sequence() {
        let mut seen = Vec::new();
        nested().walk(&mut |action| seen.push(action.tag()));
        assert_eq!(
            seen,
            vec![
                ActionTag::Sequence,
                ActionTag::DebugMessage,
                ActionTag::Sequence,
                ActionTag::DebugMessage,
            ]
        );
    }

    /// The gate of a nested sequence carries the same filter vocabulary a
    /// handler does. Missing it would leave an inline query inside that gate
    /// unsampled at runtime, which expressions then fail closed on.
    #[test]
    fn walk_filters_reaches_the_gate_of_a_nested_sequence() {
        let mut ids = Vec::new();
        nested().walk_filters(&mut |filter| {
            if let EventFilterConfig::Entity(entity) = filter {
                ids.push(entity.id.clone().unwrap_or_default());
            }
        });
        assert_eq!(
            ids,
            vec!["outer_mark".to_string(), "inner_mark".to_string()]
        );
    }

    /// A leaf action has no children, and the walkers must still visit IT.
    #[test]
    fn walking_a_leaf_visits_itself_and_no_filters() {
        let mut seen = Vec::new();
        note("alone").walk(&mut |action| seen.push(action.tag()));
        assert_eq!(seen, vec![ActionTag::DebugMessage]);

        let mut filters = 0;
        note("alone").walk_filters(&mut |_| filters += 1);
        assert_eq!(filters, 0);
    }

    /// The buried action every reader below has to find: an injection, so the
    /// creative-map class is checked by the same fixture as the walkers.
    fn buried() -> EventActionConfig {
        EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig {
            id: "player".to_string(),
            cap: None,
        })
    }

    /// One step chain, wrapped by each action that nests one.
    ///
    /// `step_chain` is the single accessor the four readers go through, so
    /// this list is the whole test surface a new nesting arm has to be added
    /// to - and until it is, that arm is silently a leaf.
    fn every_chain_shape() -> Vec<EventActionConfig> {
        let steps = || {
            vec![SequenceStepConfig {
                until: Some(gate("buried_mark")),
                deadline: Some(60.0),
                actions: vec![buried()],
                ..Default::default()
            }]
        };
        vec![
            EventActionConfig::Sequence(SequenceActionConfig {
                key: "run".to_string(),
                steps: steps(),
            }),
            EventActionConfig::Cinematic(CinematicActionConfig {
                key: "scene".to_string(),
                skippable: true,
                steps: steps(),
            }),
        ]
    }

    /// Every nesting action is nested for EVERY reader. A second arm honoured
    /// by `walk` but missed by `walk_filters` is the exact bug the shared
    /// accessor exists to make impossible: an inline query inside that gate is
    /// never sampled, and the expression over it then fails closed with a
    /// debug log nobody reads.
    #[test]
    fn every_nesting_action_is_read_by_every_reader() {
        for mut outer in every_chain_shape() {
            let tag = outer.tag();
            assert!(
                outer.step_chain().is_some(),
                "{tag:?} nests a chain but does not report one"
            );
            assert!(
                outer.step_chain_mut().is_some(),
                "{tag:?} nests a chain no authoring pass can edit"
            );

            let mut written = Vec::new();
            outer.walk_mut(&mut |action| written.push(action.tag()));
            assert_eq!(
                written,
                vec![tag, ActionTag::SetSpeedCap],
                "{tag:?} does not offer its own beats for editing"
            );

            let mut seen = Vec::new();
            outer.walk(&mut |action| seen.push(action.tag()));
            assert_eq!(
                seen,
                vec![tag, ActionTag::SetSpeedCap],
                "{tag:?} does not walk its own beats"
            );

            let mut ids = Vec::new();
            outer.walk_filters(&mut |filter| {
                if let EventFilterConfig::Entity(entity) = filter {
                    ids.push(entity.id.clone().unwrap_or_default());
                }
            });
            assert_eq!(
                ids,
                vec!["buried_mark".to_string()],
                "{tag:?} does not walk the gates of its own beats"
            );

            let mut names = BTreeSet::new();
            outer.collect_injections(&mut names);
            assert!(
                names.contains("SetSpeedCap"),
                "{tag:?} launders a buried injection into bookkeeping"
            );
            assert_eq!(outer.effect(), ActionEffect::Injection);
        }
    }

    /// A scene inside a sequence, which is how the strike is authored: the
    /// shift plays out as a chain, and one of its beats is the cinematic.
    #[test]
    fn walk_reaches_a_beat_inside_a_cinematic_inside_a_sequence() {
        let action = EventActionConfig::Sequence(SequenceActionConfig {
            key: "shift".to_string(),
            steps: vec![SequenceStepConfig {
                actions: vec![EventActionConfig::Cinematic(CinematicActionConfig {
                    key: "strike".to_string(),
                    skippable: true,
                    steps: vec![SequenceStepConfig {
                        until: Some(gate("meridian")),
                        deadline: Some(30.0),
                        actions: vec![note("the light arrives first")],
                        ..Default::default()
                    }],
                })],
                ..Default::default()
            }],
        });

        let mut seen = Vec::new();
        action.walk(&mut |action| seen.push(action.tag()));
        assert_eq!(
            seen,
            vec![
                ActionTag::Sequence,
                ActionTag::Cinematic,
                ActionTag::DebugMessage,
            ]
        );

        let mut ids = Vec::new();
        action.walk_filters(&mut |filter| {
            if let EventFilterConfig::Entity(entity) = filter {
                ids.push(entity.id.clone().unwrap_or_default());
            }
        });
        assert_eq!(ids, vec!["meridian".to_string()]);
    }
}
