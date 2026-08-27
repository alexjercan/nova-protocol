//! Generic constructors for Rust-authored scenario configuration.
//!
//! Use this catalog for small RON-facing expression, filter, action, and watch
//! nodes. Keep story policy, scenario ids, pacing, and object design in the
//! owning scenario module.

use nova_scenario::prelude::*;

/// Glob-import surface for generic scenario authoring constructors.
pub mod prelude {
    pub use super::{
        attach_objective_marker, clear_hint_emphasis, complete_objective, despawn_object,
        detach_objective_marker, entity, entity_pair, increment_variable, number, number_equals,
        number_greater_than, number_less_than, post_objective, scenario_elapsed_watch, sequence,
        set_number, set_variable, show_hint_emphasis, spawn_object, start_timer, step,
        story_message, timer, until_step, variable,
    };
}

/// Build a numeric literal expression.
pub fn number(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

/// Build a named variable expression.
pub fn variable(name: impl Into<String>) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(VariableFactorNode::new_name(
        name,
    )))
}

/// Build an action that stores an expression in a mutable variable.
pub fn set_variable(
    key: impl Into<String>,
    expression: VariableExpressionNode,
) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: key.into(),
        expression,
    })
}

/// Build an action that stores a numeric literal in a mutable variable.
pub fn set_number(key: impl Into<String>, value: f64) -> EventActionConfig {
    set_variable(key, number(value))
}

/// Build the standard `key = key + 1` counter action.
pub fn increment_variable(key: impl Into<String>) -> EventActionConfig {
    let key = key.into();
    set_variable(
        key.clone(),
        VariableExpressionNode::new_add(
            VariableTermNode::new_factor(VariableFactorNode::new_name(key)),
            number(1.0),
        ),
    )
}

/// Filter for a numeric variable equal to `value`.
pub fn number_equals(name: impl Into<String>, value: f64) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(VariableConditionNode::new_equals(
        variable(name),
        number(value),
    )))
}

/// Filter for a numeric variable less than `value`.
pub fn number_less_than(name: impl Into<String>, value: f64) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(
        VariableConditionNode::new_less_than(variable(name), number(value)),
    ))
}

/// Filter for a numeric variable greater than `value`.
pub fn number_greater_than(name: impl Into<String>, value: f64) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(
        VariableConditionNode::new_greater_than(variable(name), number(value)),
    ))
}

/// Build an exact-id entity filter.
pub fn entity(id: impl Into<String>) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.into()),
        ..Default::default()
    })
}

/// Build an exact-id entity-pair filter.
pub fn entity_pair(id: impl Into<String>, other_id: impl Into<String>) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.into()),
        other_id: Some(other_id.into()),
        ..Default::default()
    })
}

/// Build a scenario-elapsed watch under an authored variable name.
pub fn scenario_elapsed_watch(variable: impl Into<String>) -> WatchConfig {
    WatchConfig {
        variable: variable.into(),
        query: QueryConfig::Scenario(ScenarioQuery {
            property: ScenarioProperty::Elapsed,
        }),
    }
}

/// Build a scenario-object spawn action.
pub fn spawn_object(object: ScenarioObjectConfig) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(object)
}

/// Build a scenario-object despawn action.
pub fn despawn_object(id: impl Into<String>) -> EventActionConfig {
    let id = id.into();
    EventActionConfig::DespawnScenarioObject(DespawnScenarioObjectActionConfig::new(&id))
}

/// Build an objective-post action.
pub fn post_objective(id: impl Into<String>, message: impl Into<String>) -> EventActionConfig {
    let id = id.into();
    let message = message.into();
    EventActionConfig::Objective(ObjectiveActionConfig::new(&id, &message))
}

/// Build an objective-completion action.
pub fn complete_objective(id: impl Into<String>) -> EventActionConfig {
    EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig { id: id.into() })
}

/// Build an objective-marker attachment action.
pub fn attach_objective_marker(
    target_id: impl Into<String>,
    label: impl Into<String>,
) -> EventActionConfig {
    let target_id = target_id.into();
    let label = label.into();
    EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
        &target_id, &label,
    ))
}

/// Build an objective-marker detachment action.
pub fn detach_objective_marker(target_id: impl Into<String>) -> EventActionConfig {
    let target_id = target_id.into();
    EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig::new(&target_id))
}

/// Build a HUD hint-emphasis action.
pub fn show_hint_emphasis(verb: impl Into<String>) -> EventActionConfig {
    let verb = verb.into();
    EventActionConfig::HintEmphasisSet(HintEmphasisSetActionConfig::new(&verb))
}

/// Build a HUD hint-emphasis clear action.
pub fn clear_hint_emphasis(verb: impl Into<String>) -> EventActionConfig {
    let verb = verb.into();
    EventActionConfig::HintEmphasisClear(HintEmphasisClearActionConfig::new(&verb))
}

/// Build a speaker-attributed story message with default dwell and no icon.
pub fn story_message(speaker: impl Into<String>, text: impl Into<String>) -> EventActionConfig {
    EventActionConfig::StoryMessage(StoryMessageActionConfig {
        speaker: speaker.into(),
        text: text.into(),
        dwell: None,
        icon: None,
    })
}

/// Start (or restart) a keyed scenario timer. Pairs with [`timer`] on an
/// `OnTimerEnd` handler: the readable way to space beats a few seconds apart
/// without threading a clock mark through an expression tree.
pub fn start_timer(key: impl Into<String>, seconds: f64) -> EventActionConfig {
    EventActionConfig::TimerStart(TimerStartActionConfig {
        key: key.into(),
        seconds: number(seconds),
    })
}

/// Match an `OnTimerEnd` event by its timer key.
pub fn timer(key: impl Into<String>) -> EventFilterConfig {
    EventFilterConfig::Timer(TimerFilterConfig { key: key.into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_build_the_expected_ron_facing_variants() {
        assert!(matches!(
            number_equals("act", 1.0),
            EventFilterConfig::Expression(_)
        ));
        assert!(matches!(entity("ship"), EventFilterConfig::Entity(_)));
        assert!(matches!(
            post_objective("go", "Go"),
            EventActionConfig::Objective(_)
        ));
        assert!(matches!(
            scenario_elapsed_watch("elapsed").query,
            QueryConfig::Scenario(_)
        ));
    }
}

/// Build a keyed sequence action: an ordered beat chain the engine walks.
///
/// The engine holds the cursor, so a chain of beats costs ONE handler and no
/// scenario variable. Prefer this to sibling handlers strung together by a
/// counter or a stamped deadline.
pub fn sequence(key: impl Into<String>, steps: Vec<SequenceStepConfig>) -> EventActionConfig {
    EventActionConfig::Sequence(SequenceActionConfig {
        key: key.into(),
        steps,
    })
}

/// Build a sequence step that runs `after` seconds of scenario time from when
/// the step became current.
pub fn step(after: f64, actions: Vec<EventActionConfig>) -> SequenceStepConfig {
    SequenceStepConfig {
        after: Some(after),
        actions,
        ..Default::default()
    }
}

/// Build a sequence step that waits for BOTH `after` seconds of scenario time
/// and `event` (qualified by `filters`), gives up loudly `deadline` seconds
/// after the step became current, then runs `actions`.
///
/// The two waits run together, not one behind the other: a gate that opens
/// early still owes the delay, and a delay that elapses first still owes the
/// gate. Pass `after: 0.0` for a step that waits only on the gate.
pub fn until_step(
    after: f64,
    event: EventConfig,
    filters: Vec<EventFilterConfig>,
    deadline: f64,
    actions: Vec<EventActionConfig>,
) -> SequenceStepConfig {
    SequenceStepConfig {
        after: Some(after),
        until: Some(SequenceGateConfig {
            name: event,
            filters,
        }),
        deadline: Some(deadline),
        actions,
    }
}
