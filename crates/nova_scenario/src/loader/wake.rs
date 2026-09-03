//! What a loaded scenario needs to be WOKEN for.
//!
//! `fire_on_update` used to queue an `OnUpdate` event every frame, and the
//! dispatcher then walked the whole bucket re-evaluating filters that could not
//! have changed their answer. This module derives, once at load, the two facts
//! that make the pulse skippable: which variables an `OnUpdate` filter reads,
//! and which clock thresholds one compares against.
//!
//! Nothing is added to the authored language. The filters already declare all
//! of it, and an authored wake list would be a second source of truth that can
//! disagree with them - name two variables, read three, and the handler
//! silently never fires on the third.
//!
//! The default is [`WakeProfile::EveryFrame`]. Anything the analyser cannot
//! prove keeps the old behaviour, so a case it does not understand is SLOW,
//! never wrong.

use bevy::platform::collections::HashSet;

use crate::prelude::*;

/// What the `OnUpdate` pulse must be fired for, for one loaded scenario.
#[derive(Debug, Default, PartialEq)]
pub enum WakeProfile {
    /// Fire every frame: the fail-safe answer.
    #[default]
    EveryFrame,
    /// Fire only on a write to one of `vars`, or when the scenario clock
    /// crosses one of `times`.
    OnChange {
        /// Content variable names some `OnUpdate` filter reads.
        vars: HashSet<String>,
        /// Absolute scenario-clock thresholds some `OnUpdate` filter compares
        /// against.
        times: Vec<f64>,
    },
}

/// Install the shape a loaded scenario declares: its watches, whether anything
/// reads an entity query, and what the pulse must wake for.
///
/// The ONE place a config becomes live world shape, for the same reason
/// `ScenarioEventConfig::build_handler` is the one place a config becomes a
/// handler: nine headless rigs re-implement the loader, and a fact derived in
/// only one of them is a scenario the game does not run.
pub(crate) fn configure_scenario_shape(world: &mut NovaEventWorld, scenario: &ScenarioConfig) {
    world.set_watches(scenario.watches.clone(), scenario.reads_an_entity_query());
    world.set_wake(profile(scenario));
}

/// Everything that can be dispatched on `OnUpdate`, scanned for what it reads.
///
/// A `Sequence` step's `until` gate is a REAL handler the loader spawns
/// (`sequence_gate_handlers`), so a gate waiting on `OnUpdate` needs the pulse
/// exactly as an authored handler does. Missing it would stall a chain
/// forever - the fifth walker in the family stage 2 started.
pub(crate) fn profile(scenario: &ScenarioConfig) -> WakeProfile {
    let clock = clock_watches(scenario);
    let sampled = sampled_watches(scenario);
    let mut vars = HashSet::new();
    let mut times = Vec::new();

    for event in &scenario.events {
        let on_update = matches!(event.name, EventConfig::OnUpdate);
        if on_update && !collect_handler(&event.filters, &clock, &sampled, &mut vars, &mut times) {
            return WakeProfile::EveryFrame;
        }
        let mut proven = true;
        for action in &event.actions {
            action.walk(&mut |action| match action {
                EventActionConfig::Sequence(config) => {
                    for gate in config.steps.iter().filter_map(|step| step.until.as_ref()) {
                        if matches!(gate.name, EventConfig::OnUpdate)
                            && !collect_handler(
                                &gate.filters,
                                &clock,
                                &sampled,
                                &mut vars,
                                &mut times,
                            )
                        {
                            proven = false;
                        }
                    }
                }
                // What an OnUpdate handler WRITES joins what it reads. A
                // handler mid-run that mutates state must keep getting frames
                // until its own filters stop passing, and a write it makes is
                // the only thing that could stop them. Without this a counter
                // advanced from an OnUpdate handler freezes the moment nothing
                // else in the scenario writes.
                EventActionConfig::VariableSet(config) if on_update => {
                    vars.insert(config.key.clone());
                }
                _ => {}
            });
        }
        if !proven {
            return WakeProfile::EveryFrame;
        }
    }

    WakeProfile::OnChange { vars, times }
}

/// Watch names fed by the scenario clock. Read against a literal these
/// SCHEDULE; read any other way they cannot be, because the value moves every
/// frame by construction.
fn clock_watches(scenario: &ScenarioConfig) -> HashSet<&str> {
    scenario
        .watches
        .iter()
        .filter(|watch| {
            matches!(&watch.query, QueryConfig::Scenario(query)
                if query.property == ScenarioProperty::Elapsed)
        })
        .map(|watch| watch.variable.as_str())
        .collect()
}

/// Watch names fed by a per-frame sample that is NOT the clock - `player_speed`
/// and anything like it. A filter reading one is a continuous question and
/// keeps its per-frame pulse.
fn sampled_watches(scenario: &ScenarioConfig) -> HashSet<&str> {
    scenario
        .watches
        .iter()
        .filter(|watch| matches!(&watch.query, QueryConfig::Entity(_)))
        .map(|watch| watch.variable.as_str())
        .collect()
}

/// One handler's (or gate's) filter list. `false` means the analyser could not
/// prove what it reads, and the whole scenario falls back to every frame.
fn collect_handler(
    filters: &[EventFilterConfig],
    clock: &HashSet<&str>,
    sampled: &HashSet<&str>,
    vars: &mut HashSet<String>,
    times: &mut Vec<f64>,
) -> bool {
    // An unfiltered OnUpdate asks its question every frame by definition.
    if filters.is_empty() {
        return false;
    }
    filters
        .iter()
        .all(|filter| collect_filter(filter, clock, sampled, vars, times))
}

/// One filter. A `Not` inverts the ANSWER, not the moments it can change, so a
/// threshold or a variable read under one is collected exactly as it is above
/// one.
fn collect_filter(
    filter: &EventFilterConfig,
    clock: &HashSet<&str>,
    sampled: &HashSet<&str>,
    vars: &mut HashSet<String>,
    times: &mut Vec<f64>,
) -> bool {
    match filter {
        EventFilterConfig::Expression(config) => {
            collect_condition(&config.0, clock, sampled, vars, times)
        }
        EventFilterConfig::Conditional(ConditionalFilterConfig::Not(inner)) => {
            collect_filter(inner, clock, sampled, vars, times)
        }
        EventFilterConfig::Conditional(
            ConditionalFilterConfig::Or(left, right) | ConditionalFilterConfig::And(left, right),
        ) => {
            let left = collect_filter(left, clock, sampled, vars, times);
            let right = collect_filter(right, clock, sampled, vars, times);
            left && right
        }
        // Entity, Timer and ShipOrder filters read the fired event, not the
        // world. None of them says anything about when the answer could change.
        EventFilterConfig::Entity(_)
        | EventFilterConfig::Timer(_)
        | EventFilterConfig::ShipOrder(_) => false,
    }
}

fn collect_condition(
    node: &VariableConditionNode,
    clock: &HashSet<&str>,
    sampled: &HashSet<&str>,
    vars: &mut HashSet<String>,
    times: &mut Vec<f64>,
) -> bool {
    let (left, right) = match node {
        // Equality against a float clock never reliably passes, so it is not
        // worth a scheduled wake and falls through to the general walk - which
        // bails, because the clock is unschedulable there.
        VariableConditionNode::Equal(left, right) => (left, right),
        VariableConditionNode::LessThan(left, right)
        | VariableConditionNode::GreaterThan(left, right) => {
            // `scenario_elapsed > 95.0`, either way round, is a wake at 95.0.
            if reads_clock(left, clock) {
                if let Some(at) = literal_number(right) {
                    times.push(at);
                    return true;
                }
            }
            if reads_clock(right, clock) {
                if let Some(at) = literal_number(left) {
                    times.push(at);
                    return true;
                }
            }
            (left, right)
        }
    };
    collect_expression(left, clock, sampled, vars)
        && collect_expression(right, clock, sampled, vars)
}

/// A bare reference to a clock watch or to the scenario-elapsed query itself.
/// Deliberately shallow: `scenario_elapsed * 2` is arithmetic, not a threshold.
fn reads_clock(node: &VariableExpressionNode, clock: &HashSet<&str>) -> bool {
    match bare_factor(node) {
        Some(VariableFactorNode::Name(name)) => clock.contains(name.as_str()),
        Some(VariableFactorNode::Query(QueryConfig::Scenario(query))) => {
            query.property == ScenarioProperty::Elapsed
        }
        _ => false,
    }
}

fn literal_number(node: &VariableExpressionNode) -> Option<f64> {
    match bare_factor(node) {
        Some(VariableFactorNode::Literal(VariableLiteral::Number(value))) => Some(*value),
        _ => None,
    }
}

fn bare_factor(node: &VariableExpressionNode) -> Option<&VariableFactorNode> {
    let VariableExpressionNode::Term(VariableTermNode::Factor(factor)) = node else {
        return None;
    };
    match factor {
        VariableFactorNode::Parens(inner) => bare_factor(inner),
        factor => Some(factor),
    }
}

fn collect_expression(
    node: &VariableExpressionNode,
    clock: &HashSet<&str>,
    sampled: &HashSet<&str>,
    vars: &mut HashSet<String>,
) -> bool {
    match node {
        VariableExpressionNode::Add(left, right)
        | VariableExpressionNode::Subtract(left, right) => {
            collect_term(left, clock, sampled, vars)
                && collect_expression(right, clock, sampled, vars)
        }
        VariableExpressionNode::Term(term) => collect_term(term, clock, sampled, vars),
    }
}

fn collect_term(
    node: &VariableTermNode,
    clock: &HashSet<&str>,
    sampled: &HashSet<&str>,
    vars: &mut HashSet<String>,
) -> bool {
    match node {
        VariableTermNode::Multiply(left, right) | VariableTermNode::Divide(left, right) => {
            collect_factor(left, clock, sampled, vars) && collect_term(right, clock, sampled, vars)
        }
        VariableTermNode::Factor(factor) => collect_factor(factor, clock, sampled, vars),
    }
}

fn collect_factor(
    node: &VariableFactorNode,
    clock: &HashSet<&str>,
    sampled: &HashSet<&str>,
    vars: &mut HashSet<String>,
) -> bool {
    match node {
        VariableFactorNode::Parens(inner) => collect_expression(inner, clock, sampled, vars),
        VariableFactorNode::Literal(_) => true,
        // An inline query is answered from a snapshot resampled every frame.
        VariableFactorNode::Query(_) => false,
        VariableFactorNode::Name(name) => {
            // A clock read the threshold shape did not recognise cannot be
            // scheduled - `scenario_elapsed > overspeed_deadline` is a keyed
            // timer written by hand, and it keeps its per-frame pulse until it
            // is written as one.
            if clock.contains(name.as_str()) || sampled.contains(name.as_str()) {
                return false;
            }
            vars.insert(name.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::default;

    use super::*;
    use crate::loader::fixtures::*;

    fn number(value: f64) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(value)),
        ))
    }

    fn name(key: &str) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_name(key),
        ))
    }

    fn equals(key: &str, value: f64) -> EventFilterConfig {
        EventFilterConfig::Expression(ExpressionFilterConfig(VariableConditionNode::new_equals(
            name(key),
            number(value),
        )))
    }

    fn on_update(filters: Vec<EventFilterConfig>) -> ScenarioEventConfig {
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            filters,
            ..event_with(vec![])
        }
    }

    fn clock_watch() -> WatchConfig {
        WatchConfig {
            variable: "scenario_elapsed".to_string(),
            query: QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
        }
    }

    fn watched(events: Vec<ScenarioEventConfig>, watches: Vec<WatchConfig>) -> ScenarioConfig {
        ScenarioConfig {
            watches,
            ..scenario_with("wake", events)
        }
    }

    fn vars_of(profile: &WakeProfile) -> Vec<String> {
        let WakeProfile::OnChange { vars, .. } = profile else {
            panic!("expected a provable profile, got {profile:?}");
        };
        let mut vars: Vec<String> = vars.iter().cloned().collect();
        vars.sort();
        vars
    }

    fn times_of(profile: &WakeProfile) -> Vec<f64> {
        let WakeProfile::OnChange { times, .. } = profile else {
            panic!("expected a provable profile, got {profile:?}");
        };
        times.clone()
    }

    /// The ordinary case: a milestone gated on content variables wakes on a
    /// write to one of them and on nothing else.
    #[test]
    fn a_value_gated_milestone_wakes_on_its_own_variables() {
        let profile = profile(&watched(
            vec![on_update(vec![equals("beat", 3.0), equals("crates", 3.0)])],
            vec![clock_watch()],
        ));
        assert_eq!(vars_of(&profile), vec!["beat", "crates"]);
        assert!(times_of(&profile).is_empty(), "nothing reads the clock");
    }

    /// A literal clock threshold is a SCHEDULED wake, not a variable: the clock
    /// moves every frame, so treating it as one would wake every frame.
    #[test]
    fn a_literal_clock_threshold_becomes_a_scheduled_time() {
        let profile = profile(&watched(
            vec![on_update(vec![
                equals("act", 1.0),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_greater_than(name("scenario_elapsed"), number(95.0)),
                )),
            ])],
            vec![clock_watch()],
        ));
        assert_eq!(
            vars_of(&profile),
            vec!["act"],
            "the clock is not a variable"
        );
        assert_eq!(times_of(&profile), vec![95.0]);
    }

    /// The clock compared against anything but a literal cannot be scheduled -
    /// it is a keyed timer written by hand - so the scenario keeps its frames.
    #[test]
    fn a_clock_compared_against_a_variable_is_not_provable() {
        let profile = profile(&watched(
            vec![on_update(vec![EventFilterConfig::Expression(
                ExpressionFilterConfig(VariableConditionNode::new_greater_than(
                    name("scenario_elapsed"),
                    name("deadline"),
                )),
            )])],
            vec![clock_watch()],
        ));
        assert_eq!(profile, WakeProfile::EveryFrame);
    }

    /// A per-frame entity sample is a continuous question and keeps polling.
    #[test]
    fn a_sampled_watch_keeps_its_per_frame_pulse() {
        let profile = profile(&watched(
            vec![on_update(vec![EventFilterConfig::Expression(
                ExpressionFilterConfig(VariableConditionNode::new_greater_than(
                    name("player_speed"),
                    number(8.0),
                )),
            )])],
            vec![
                clock_watch(),
                WatchConfig {
                    variable: "player_speed".to_string(),
                    query: QueryConfig::Entity(EntityQuery {
                        filter: EntityQueryFilter {
                            id: "player_spaceship".to_string(),
                        },
                        property: EntityProperty::Speed,
                    }),
                },
            ],
        ));
        assert_eq!(profile, WakeProfile::EveryFrame);
    }

    /// `Or` and `Not` combine expressions the analyser already understands, so
    /// a handler behind one is still provable. A `Not` inverts the ANSWER, not
    /// the moments it can change, so its threshold is still a scheduled wake -
    /// lifeline's two branch milestones are exactly this shape.
    #[test]
    fn a_conditional_over_expressions_is_still_provable() {
        let profile = profile(&watched(
            vec![on_update(vec![
                EventFilterConfig::Conditional(ConditionalFilterConfig::or(
                    equals("queen_down", 1.0),
                    equals("meridian_down", 1.0),
                )),
                EventFilterConfig::Conditional(ConditionalFilterConfig::not(
                    EventFilterConfig::Expression(ExpressionFilterConfig(
                        VariableConditionNode::new_greater_than(
                            name("scenario_elapsed"),
                            number(240.0),
                        ),
                    )),
                )),
            ])],
            vec![clock_watch()],
        ));
        assert_eq!(vars_of(&profile), vec!["meridian_down", "queen_down"]);
        assert_eq!(times_of(&profile), vec![240.0]);
    }

    /// A `Conditional` proves only what it wraps: an entity filter hidden one
    /// level down still takes the whole scenario back to every frame.
    #[test]
    fn a_conditional_hiding_an_entity_filter_falls_back() {
        let profile = profile(&watched(
            vec![on_update(vec![EventFilterConfig::Conditional(
                ConditionalFilterConfig::or(
                    equals("act", 1.0),
                    EventFilterConfig::Entity(EntityFilterConfig {
                        id: Some("picket_a".to_string()),
                        ..default()
                    }),
                ),
            )])],
            vec![clock_watch()],
        ));
        assert_eq!(profile, WakeProfile::EveryFrame);
    }

    /// The fail-safe cases, each on its own: an unfiltered pulse, a filter that
    /// reads the fired event rather than the world, and an inline query.
    #[test]
    fn anything_unprovable_falls_back_to_every_frame() {
        let unfiltered = profile(&watched(vec![on_update(vec![])], vec![clock_watch()]));
        assert_eq!(unfiltered, WakeProfile::EveryFrame, "no filters");

        let entity_filter = profile(&watched(
            vec![on_update(vec![EventFilterConfig::Entity(
                EntityFilterConfig {
                    id: Some("picket_a".to_string()),
                    ..default()
                },
            )])],
            vec![clock_watch()],
        ));
        assert_eq!(entity_filter, WakeProfile::EveryFrame, "an entity filter");

        let inline_query = profile(&watched(
            vec![on_update(vec![EventFilterConfig::Expression(
                ExpressionFilterConfig(VariableConditionNode::new_greater_than(
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_query(QueryConfig::Entity(EntityQuery {
                            filter: EntityQueryFilter {
                                id: "player_spaceship".to_string(),
                            },
                            property: EntityProperty::Speed,
                        })),
                    )),
                    number(8.0),
                )),
            )])],
            vec![clock_watch()],
        ));
        assert_eq!(inline_query, WakeProfile::EveryFrame, "an inline query");
    }

    /// A `Sequence` step's `until` gate is a real handler on the pulse. Missing
    /// it would stall the chain forever - `final_tally` gates its cast-off on
    /// exactly this shape.
    #[test]
    fn a_sequence_gate_waiting_on_the_pulse_is_a_reason_to_wake() {
        let profile = profile(&watched(
            vec![event_with(vec![EventActionConfig::Sequence(
                SequenceActionConfig {
                    key: "cast_off".to_string(),
                    steps: vec![SequenceStepConfig {
                        until: Some(SequenceGateConfig {
                            name: EventConfig::OnUpdate,
                            filters: vec![equals("surveyed", 1.0)],
                        }),
                        deadline: Some(600.0),
                        ..default()
                    }],
                },
            )])],
            vec![clock_watch()],
        ));
        assert_eq!(vars_of(&profile), vec!["surveyed"]);
    }

    /// What an OnUpdate handler writes joins what it reads, so a handler
    /// mid-run keeps getting frames until its own filters stop passing.
    #[test]
    fn what_an_on_update_handler_writes_is_also_a_reason_to_wake() {
        let profile = profile(&watched(
            vec![ScenarioEventConfig {
                label: None,
                name: EventConfig::OnUpdate,
                filters: vec![equals("act", 1.0)],
                actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "ticks".to_string(),
                    expression: number(1.0),
                })],
                ..event_with(vec![])
            }],
            vec![clock_watch()],
        ));
        assert_eq!(vars_of(&profile), vec!["act", "ticks"]);
    }

    /// A write from a handler on another event does NOT join the set - it wakes
    /// the pulse only if an OnUpdate filter actually reads it.
    #[test]
    fn a_write_from_another_event_is_not_a_reason_to_wake() {
        let profile = profile(&watched(
            vec![
                on_update(vec![equals("act", 1.0)]),
                ScenarioEventConfig {
                    label: None,
                    name: EventConfig::OnDefeated,
                    actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                        key: "picket_a_down".to_string(),
                        expression: number(1.0),
                    })],
                    ..event_with(vec![])
                },
            ],
            vec![clock_watch()],
        ));
        assert_eq!(vars_of(&profile), vec!["act"]);
    }
}
