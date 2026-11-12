//! What the script tree keeps, and what it gives back.

use bevy::ecs::system::RunSystemOnce;
use nova_events::prelude::{
    CommandsGameEventExt, EventKind, GameEventsPlugin, OnDestroyedEvent, OnDestroyedEventInfo,
    OnEnterEvent, OnEnterEventInfo, OnStartEvent, OnStartEventInfo, OnTimerEndEvent,
    OnTimerEndEventInfo, SPACESHIP_TYPE_NAME,
};
use nova_gameplay::prelude::GameObjectives;
use nova_scenario::prelude::{
    CurrentOutcome, ExpressionFilterConfig, NovaEventWorld, ObjectiveActionConfig,
    ObjectiveCompleteActionConfig, OutcomeActionConfig, ScenarioOutcomeKind, TimerFilterConfig,
    TimerStartActionConfig, VariableConditionNode, VariableExpressionNode, VariableFactorNode,
    VariableLiteral, VariableSetActionConfig, VariableTermNode,
};

use super::*;
use crate::scenario::default_script;

/// Lift `events` under a fresh scenario, then lower the tree back out.
fn round_trip(events: Vec<ScenarioEventConfig>) -> Vec<ScenarioEventConfig> {
    let mut world = World::new();
    let scenario = seeded(&mut world, events);
    lowered(&mut world, scenario)
}

/// A scenario node whose script is `events`, taken apart into nodes.
fn seeded(world: &mut World, events: Vec<ScenarioEventConfig>) -> Entity {
    let scenario = world.spawn_empty().id();
    let held = events;
    world
        .run_system_once(move |mut commands: Commands| {
            lift(&mut commands, scenario, held.clone());
        })
        .expect("the lift runs");
    scenario
}

/// The script of `scenario`, as the save would write it.
fn lowered(world: &mut World, scenario: Entity) -> Vec<ScenarioEventConfig> {
    world
        .run_system_once(move |script: ScriptNodes| script.lower(scenario))
        .expect("the lowering runs")
}

/// The handlers of `scenario`, in authored order.
fn handlers(world: &mut World, scenario: Entity) -> Vec<Entity> {
    world
        .run_system_once(move |script: ScriptNodes| script.events_of(scenario))
        .expect("the walk runs")
}

/// The children of `node` that are filters, then the ones that are actions.
fn under(world: &mut World, node: Entity) -> (Vec<Entity>, Vec<Entity>) {
    world
        .run_system_once(move |script: ScriptNodes| {
            (script.filters_of(node), script.actions_of(node))
        })
        .expect("the walk runs")
}

/// The configs carry no `PartialEq`, and their `Debug` is the whole tree.
fn spelled(events: &[ScenarioEventConfig]) -> String {
    format!("{events:#?}")
}

fn message(text: &str) -> EventActionConfig {
    EventActionConfig::DebugMessage(DebugMessageActionConfig {
        message: text.to_string(),
    })
}

fn named(id: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.to_string()),
        ..default()
    })
}

/// The seeded script is the widest handler list the editor writes by itself,
/// so it is the one the round trip has to survive.
#[test]
fn the_seeded_script_survives_a_lift_and_a_lowering() {
    let script = default_script();
    assert_eq!(spelled(&round_trip(script.clone())), spelled(&script));
}

/// The canvas is a view over the tree, so opening a shipped scenario in it and
/// saving must write what the builders wrote.
#[test]
fn every_shipped_scenario_lifts_and_lowers_unchanged() {
    for scenario in nova_authoring::generation::build_scenarios() {
        assert_eq!(
            spelled(&round_trip(scenario.events.clone())),
            spelled(&scenario.events),
            "{}",
            scenario.id,
        );
    }
}

/// A sequence is the one action with children, and the children are ordered:
/// steps read back out of order are a different scenario.
#[test]
fn a_sequences_steps_keep_the_order_they_were_written_in() {
    let script = vec![ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnStart,
        once: true,
        filters: vec![],
        actions: vec![EventActionConfig::Sequence(SequenceActionConfig {
            key: "briefing".to_string(),
            steps: vec![
                SequenceStepConfig {
                    actions: vec![message("first")],
                    ..default()
                },
                SequenceStepConfig {
                    after: Some(2.0),
                    actions: vec![message("second"), message("and a half")],
                    ..default()
                },
                SequenceStepConfig {
                    after: None,
                    until: Some(SequenceGateConfig {
                        name: EventConfig::OnDestroyed,
                        filters: vec![named("turret_1")],
                    }),
                    deadline: Some(60.0),
                    actions: vec![message("third")],
                },
            ],
        })],
    }];

    assert_eq!(spelled(&round_trip(script.clone())), spelled(&script));
}

/// The combinators nest, so they are nodes with operand children rather than
/// one row holding a boxed filter.
#[test]
fn a_nested_condition_round_trips_through_operand_nodes() {
    let script = vec![ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnEnter,
        once: false,
        filters: vec![EventFilterConfig::Conditional(
            ConditionalFilterConfig::And(
                Box::new(EventFilterConfig::Conditional(
                    ConditionalFilterConfig::Not(Box::new(named("player"))),
                )),
                Box::new(EventFilterConfig::Conditional(ConditionalFilterConfig::Or(
                    Box::new(EventFilterConfig::Timer(TimerFilterConfig {
                        key: "patrol".to_string(),
                    })),
                    Box::new(named("raider_1")),
                ))),
            ),
        )],
        actions: vec![message("caught")],
    }];

    assert_eq!(spelled(&round_trip(script.clone())), spelled(&script));
}

/// The operators of a condition are nodes too, and the brackets the grammar
/// needs come back where the tree's shape says they belong.
#[test]
fn an_expression_filter_round_trips_through_operator_nodes() {
    let two = || VariableTermNode::new_factor(VariableFactorNode::new_literal(literal(2.0)));
    let sum = VariableExpressionNode::new_add(
        two(),
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(literal(3.0)),
        )),
    );
    let script = vec![ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
            VariableConditionNode::new_greater_than(
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_name("beat"),
                )),
                VariableExpressionNode::new_term(VariableTermNode::new_multiply(
                    VariableFactorNode::new_parens(sum),
                    VariableTermNode::new_factor(VariableFactorNode::new_literal(literal(4.0))),
                )),
            ),
        ))],
        actions: vec![message("late")],
    }];

    assert_eq!(spelled(&round_trip(script.clone())), spelled(&script));
}

/// A handler that runs only when `text` holds.
fn guarded(text: &str) -> Vec<ScenarioEventConfig> {
    vec![ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnUpdate,
        once: false,
        filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
            text.parse().expect("the condition parses"),
        ))],
        actions: vec![message("held")],
    }]
}

/// Every query the grammar has opens as a block picked from lists, and the
/// save writes the query the file held.
#[test]
fn every_query_opens_as_a_picked_block_and_saves_as_written() {
    for text in [
        "scenario.elapsed > 1",
        r#"scenario.sector == "sector_0_0_0""#,
        r#"entity("courier").speed < 5"#,
        r#"entity("player").credits > 100"#,
        r#"entity("player").items("IronOre") > 2"#,
    ] {
        let script = guarded(text);
        let (mut world, scenario) = block_world(script.clone());
        let handler = handlers(&mut world, scenario)[0];
        let filter = under(&mut world, handler).0[0];
        let root = operands(&mut world, filter)[0];
        let read = operands(&mut world, root)[0];

        assert!(
            matches!(
                world.get::<ExpressionNode>(read).map(|node| &node.kind),
                Some(ExprKind::Query(_))
            ),
            "{text}"
        );
        assert_eq!(
            spelled(&lowered(&mut world, scenario)),
            spelled(&script),
            "{text}"
        );
    }
}

/// The query block offers every read. A read of a ship takes its id from the
/// scenario's objects and its item from the catalog, and a read of the
/// scenario takes neither.
#[test]
fn a_query_block_picks_its_read_its_object_and_its_item() {
    use crate::inspect::{choose_field, query_rows, write_field, CatalogType, PathStep, RowValue};

    let (mut world, scenario) = block_world(guarded("scenario.elapsed > 2"));
    let handler = handlers(&mut world, scenario)[0];
    let filter = under(&mut world, handler).0[0];
    let root = operands(&mut world, filter)[0];
    let read = operands(&mut world, root)[0];
    let rows = |world: &World| {
        let Some(ExprKind::Query(operand)) =
            world.get::<ExpressionNode>(read).map(|node| &node.kind)
        else {
            panic!("the left side is a query");
        };
        query_rows(read, operand)
    };
    let field = |name: &str| vec![PathStep::Field(name.to_string())];

    let stock = rows(&world);
    assert_eq!(stock.len(), 1, "the scenario has no object: {stock:?}");
    let RowValue::Choice { options, .. } = &stock[0].value else {
        panic!("the read is a choice, not {:?}", stock[0].value);
    };
    assert_eq!(
        options,
        &["Elapsed", "Sector", "Speed", "Credits", "Items"],
        "every query the grammar has"
    );

    let mut node = world
        .get_mut::<ExpressionNode>(read)
        .expect("the query stands");
    let config = expr_config_mut(&mut node.kind).expect("a query has a config");
    choose_field(config, &field("read"), "Items").expect("the read switches");
    write_field(config, &field("id"), false, "player").expect("the id takes");
    write_field(config, &field("item"), false, "IronOre").expect("the item takes");

    let picked = rows(&world);
    assert_eq!(picked.len(), 3, "{picked:?}");
    assert_eq!(picked[1].names, Some(Names::Object));
    assert_eq!(picked[2].catalog, Some(CatalogType::Item));
    let text = world
        .run_system_once(move |script: ScriptNodes| condition_text(&script, filter))
        .expect("the walk runs");
    assert_eq!(
        text.as_deref(),
        Some(r#"entity("player").items("IronOre") > 2"#)
    );
}

/// A filter switched to an expression arrives with a condition already in it:
/// an operator with nothing under it is a filter the save would drop.
#[test]
fn a_filter_switched_to_an_expression_arrives_with_a_condition() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![named("player")],
            actions: vec![],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let filter = under(&mut world, handler).0[0];

    retype_script_node(&mut world, filter, "Expression");

    let script = lowered(&mut world, scenario);
    let EventFilterConfig::Expression(ExpressionFilterConfig(condition)) = &script[0].filters[0]
    else {
        panic!("the filter is an expression");
    };
    assert_eq!(condition.to_string(), "0 == 0");
}

/// Switching an operator to a value drops the sides it no longer has, and the
/// condition it stands in still lowers.
#[test]
fn an_operator_switched_to_a_value_drops_its_operands() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_equals(
                    VariableExpressionNode::new_add(
                        VariableTermNode::new_factor(VariableFactorNode::new_name("beat")),
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_literal(literal(1.0)),
                        )),
                    ),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(literal(4.0)),
                    )),
                ),
            ))],
            actions: vec![],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let filter = under(&mut world, handler).0[0];
    let root = operands(&mut world, filter)[0];
    let sum = operands(&mut world, root)[0];

    retype_script_node(&mut world, sum, "value");

    assert!(
        operands(&mut world, sum).is_empty(),
        "a value holds no operands"
    );
    let script = lowered(&mut world, scenario);
    let EventFilterConfig::Expression(ExpressionFilterConfig(condition)) = &script[0].filters[0]
    else {
        panic!("the filter is still an expression");
    };
    assert_eq!(condition.to_string(), "0 == 4");
}

/// A `VariableSet` writes an EXPRESSION, so its value is operand nodes like a
/// condition's - and the brackets the grammar needs come back where the tree's
/// shape says they belong.
#[test]
fn a_variable_set_round_trips_through_operator_nodes() {
    let script = vec![ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnUpdate,
        once: false,
        filters: vec![],
        actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "score".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_multiply(
                    VariableFactorNode::new_name("kills"),
                    VariableTermNode::new_factor(VariableFactorNode::new_literal(literal(10.0))),
                ),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_name("bonus"),
                )),
            ),
        })],
    }];

    assert_eq!(spelled(&round_trip(script.clone())), spelled(&script));
}

/// An action switched to a variable set arrives WRITING something, and one
/// switched away takes the value with it: an assignment with no value is an
/// action the next save would drop, and a value under a debug message is a
/// node with no row and no way back to one.
#[test]
fn an_action_switched_to_a_variable_set_arrives_writing_a_value() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![message("noted")],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let action = under(&mut world, handler).1[0];

    retype_script_node(&mut world, action, "Variable Set");

    let script = lowered(&mut world, scenario);
    let EventActionConfig::VariableSet(config) = &script[0].actions[0] else {
        panic!("the action is a variable set");
    };
    assert_eq!(config.expression.to_string(), "0");

    retype_script_node(&mut world, action, "Debug Message");

    assert!(
        operands(&mut world, action).is_empty(),
        "the value went with the kind that held it"
    );
}

/// The tree row of a variable set READS AS ITS ASSIGNMENT, and falls back to
/// the kind while there is no variable to assign to.
#[test]
fn a_variable_set_reads_as_the_assignment_it_makes() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnUpdate,
            once: false,
            filters: vec![],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: String::new(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name("beat")),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(literal(1.0)),
                    )),
                ),
            })],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let action = under(&mut world, handler).1[0];

    assert_eq!(
        assignment(&mut world, action),
        None,
        "an unnamed key has nothing to assign to"
    );

    let Some(ActionKind::VariableSet(head)) = world
        .get_mut::<ActionNode>(action)
        .map(|node| &mut node.into_inner().kind)
    else {
        panic!("the action is a variable set");
    };
    head.key = "beat".to_string();

    assert_eq!(
        assignment(&mut world, action).as_deref(),
        Some("beat = beat + 1")
    );
}

/// The condition under `node`, as the grammar spells it.
fn condition_text(script: &ScriptNodes, node: Entity) -> Option<String> {
    let root = script.operands_of(node).into_iter().next()?;
    Some(script.lower_condition(root)?.to_string())
}

/// What a `VariableSet` writes, as the grammar spells it. An unnamed key has
/// nothing to assign to.
fn assignment_text(script: &ScriptNodes, node: Entity) -> Option<String> {
    let ActionKind::VariableSet(head) = &script.action(node)?.kind else {
        return None;
    };
    let key = head.key.trim();
    if key.is_empty() {
        return None;
    }
    let root = script.operands_of(node).into_iter().next()?;
    Some(format!("{key} = {}", script.lower_expression(root)?))
}

/// What a variable set writes.
fn assignment(world: &mut World, node: Entity) -> Option<String> {
    world
        .run_system_once(move |script: ScriptNodes| assignment_text(&script, node))
        .expect("the walk runs")
}

/// A number, as the grammar's literal.
fn literal(value: f64) -> VariableLiteral {
    VariableLiteral::Number(value)
}

/// The operand nodes under `node`, in authored order.
fn operands(world: &mut World, node: Entity) -> Vec<Entity> {
    world
        .run_system_once(move |script: ScriptNodes| script.operands_of(node))
        .expect("the walk runs")
}

/// A handler that places something and then points at it names both sides,
/// and the reflect attribute is what tells them apart.
#[test]
fn a_handler_names_what_it_places_and_what_it_expects() {
    let mut object = stock_object();
    object.base.id = "beacon_1".to_string();
    let event = ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![named("player")],
        actions: vec![
            EventActionConfig::SpawnScenarioObject(object),
            EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
                "beacon_1", "Reach",
            )),
        ],
    };

    let ids = named_ids(&event);

    assert_eq!(ids.declared, vec!["beacon_1".to_string()]);
    assert_eq!(
        ids.referenced,
        vec!["player".to_string(), "beacon_1".to_string()],
        "the filter and the marker both expect an id to exist"
    );
    assert!(ids.prefixes.is_empty(), "nothing was scattered");
}

/// A scatter does not name its objects: it names the stem they all start
/// with, and a reference is satisfied by the stem.
#[test]
fn a_scatter_declares_a_prefix_rather_than_an_id() {
    let ActionKind::Leaf(EventActionConfig::ScatterObjects(mut scatter)) =
        ActionChoice::ScatterObjects.stock()
    else {
        unreachable!("the catalog builds a scatter")
    };
    scatter.id_prefix = "belt".to_string();
    let event = ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: vec![EventActionConfig::ScatterObjects(scatter)],
    };

    let ids = named_ids(&event);

    assert_eq!(ids.prefixes, vec!["belt".to_string()]);
    assert!(ids.declared.is_empty(), "no single object was placed");
}

/// One handler with one filter and one action, which is enough tree to ask
/// every "where would this land" question of.
fn one_handler() -> Vec<ScenarioEventConfig> {
    vec![ScenarioEventConfig {
        label: None,
        position: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![named("player")],
        actions: vec![message("hello")],
    }]
}

/// Add lands the node under what is marked, and a fresh filter arrives
/// matching nothing: an id nobody typed would gate the handler on a choice
/// nobody made.
/// Switching a combinator keeps the operands the new kind holds and drops the
/// rest: an operand a filter cannot hold is a node with no row.
#[test]
fn switching_a_combinator_keeps_the_operands_it_can_hold() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![EventFilterConfig::Conditional(
                ConditionalFilterConfig::and(named("first"), named("second")),
            )],
            actions: vec![],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let both = under(&mut world, handler).0[0];

    retype_script_node(&mut world, both, "Not");

    let script = lowered(&mut world, scenario);
    let EventFilterConfig::Conditional(ConditionalFilterConfig::Not(inner)) = &script[0].filters[0]
    else {
        panic!("the And became a Not");
    };
    let EventFilterConfig::Entity(kept) = inner.as_ref() else {
        panic!("the first operand stayed");
    };
    assert_eq!(kept.id.as_deref(), Some("first"));
}

/// The steps WERE the sequence. An action that is no longer one has nowhere to
/// hang them, so they go with it.
#[test]
fn leaving_a_sequence_drops_its_beats() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![EventActionConfig::Sequence(SequenceActionConfig {
                key: "briefing".to_string(),
                steps: vec![SequenceStepConfig {
                    actions: vec![message("one")],
                    ..default()
                }],
            })],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let sequence = under(&mut world, handler).1[0];

    retype_script_node(&mut world, sequence, "Debug Message");

    let script = lowered(&mut world, scenario);
    assert!(
        matches!(script[0].actions[0], EventActionConfig::DebugMessage(_)),
        "the action is what it was switched to"
    );
    assert!(
        world
            .run_system_once(move |script: ScriptNodes| script.steps_of(sequence))
            .expect("the walk runs")
            .is_empty(),
        "and holds no beats it cannot lower"
    );
}

/// A switched filter keeps its ORDINAL and takes the new kind's stem: it is
/// still the first filter of its handler, under a name that says what it is.
#[test]
fn a_switched_filter_keeps_its_place_in_the_handler() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![
                named("player"),
                EventFilterConfig::Timer(TimerFilterConfig {
                    key: "patrol".to_string(),
                }),
            ],
            actions: vec![],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let first = under(&mut world, handler).0[0];

    retype_script_node(&mut world, first, "Expression");

    assert_eq!(
        world.get::<NodeId>(first).expect("the node kept its id").0,
        "expression_1"
    );
    let script = lowered(&mut world, scenario);
    assert!(
        matches!(script[0].filters[0], EventFilterConfig::Expression(_)),
        "the switched filter is still the first"
    );
    assert!(matches!(script[0].filters[1], EventFilterConfig::Timer(_)));
}

// --- the objective set, authored and then played -----------------------------

/// Seconds, as the expression grammar holds a plain number.
fn seconds(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

fn entity_named(id: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.to_string()),
        ..default()
    })
}

/// DESTROY X, REACH Y, SURVIVE T - the three worked objectives, written in
/// the vocabulary the editor already offers and nothing else.
///
/// One handler posts them and starts the clock; one completes on the hulk
/// breaking, one on the marker area being entered, one on the timer ending -
/// and that last one declares the win.
fn objective_set() -> Vec<ScenarioEventConfig> {
    vec![
        ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions: vec![
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "destroy",
                    "Destroy the hulk",
                )),
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "reach",
                    "Reach the marker",
                )),
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "survive",
                    "Hold out for 60 seconds",
                )),
                EventActionConfig::TimerStart(TimerStartActionConfig {
                    key: "hold".to_string(),
                    seconds: seconds(60.0),
                }),
            ],
        },
        ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![entity_named("hulk_1")],
            actions: vec![EventActionConfig::ObjectiveComplete(
                ObjectiveCompleteActionConfig {
                    id: "destroy".to_string(),
                },
            )],
        },
        ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnEnter,
            once: true,
            filters: vec![entity_named("marker_area")],
            actions: vec![EventActionConfig::ObjectiveComplete(
                ObjectiveCompleteActionConfig {
                    id: "reach".to_string(),
                },
            )],
        },
        ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "hold".to_string(),
            })],
            actions: vec![
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: "survive".to_string(),
                }),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "The range is yours",
                )),
            ],
        },
    ]
}

/// An engine that runs authored handlers and nothing else: the events flush,
/// the objectives list, and the outcome the overlay reads.
fn played(script: Vec<ScenarioEventConfig>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<CurrentOutcome>();
    for event in &script {
        app.world_mut().spawn(event.build_handler());
    }
    app
}

/// Fire one game event and let the flush land.
fn fire<E: EventKind>(app: &mut App, info: E::Info)
where
    E::Info: Clone + Send + Sync + 'static,
{
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<E>(info.clone());
        })
        .expect("the event fires");
    app.update();
    app.update();
}

/// Which objectives are still standing, in HUD order.
fn standing(app: &App) -> Vec<String> {
    app.world()
        .resource::<GameObjectives>()
        .objectives
        .iter()
        .map(|objective| objective.id.clone())
        .collect()
}

/// The whole claim: destroy X, reach Y, survive T is authorable in the
/// vocabulary the editor already holds, survives the tree, and completes its
/// player path.
///
/// The timer END is fired here rather than waited for - the clock that fires it
/// is `nova_scenario`'s and has its own tests - so what this drives is the
/// script: the handler the editor wrote, filtered on the key the editor typed.
#[test]
fn the_objective_set_is_authored_as_nodes_and_plays_through() {
    let authored = objective_set();
    let through_the_tree = round_trip(authored.clone());
    assert_eq!(
        format!("{through_the_tree:?}"),
        format!("{authored:?}"),
        "the set is held as nodes with nothing lost"
    );

    let mut app = played(through_the_tree);
    fire::<OnStartEvent>(&mut app, OnStartEventInfo);
    assert_eq!(
        standing(&app),
        vec!["destroy", "reach", "survive"],
        "the start posts all three"
    );

    fire::<OnDestroyedEvent>(
        &mut app,
        OnDestroyedEventInfo {
            id: "hulk_1".to_string(),
            type_name: SPACESHIP_TYPE_NAME.to_string(),
        },
    );
    assert_eq!(standing(&app), vec!["reach", "survive"], "destroy X");

    fire::<OnEnterEvent>(
        &mut app,
        OnEnterEventInfo {
            id: "marker_area".to_string(),
            other_id: "player_spaceship".to_string(),
            other_type_name: SPACESHIP_TYPE_NAME.to_string(),
        },
    );
    assert_eq!(standing(&app), vec!["survive"], "reach Y");

    fire::<OnTimerEndEvent>(
        &mut app,
        OnTimerEndEventInfo {
            key: "hold".to_string(),
        },
    );
    assert!(standing(&app).is_empty(), "survive T");
    let outcome = app.world().resource::<CurrentOutcome>().0.clone();
    assert_eq!(
        outcome.map(|outcome| outcome.outcome),
        Some(ScenarioOutcomeKind::Victory),
        "and the set declares the win itself"
    );
}

/// A handler that fires on the WRONG id changes nothing: the filters the editor
/// wrote are the filters the engine runs.
#[test]
fn an_objective_stands_until_the_beat_it_names_happens() {
    let mut app = played(round_trip(objective_set()));
    fire::<OnStartEvent>(&mut app, OnStartEventInfo);

    fire::<OnDestroyedEvent>(
        &mut app,
        OnDestroyedEventInfo {
            id: "rock_4".to_string(),
            type_name: SPACESHIP_TYPE_NAME.to_string(),
        },
    );

    assert_eq!(
        standing(&app),
        vec!["destroy", "reach", "survive"],
        "nothing the set names was destroyed"
    );
}

/// A handler's NAME survives the round trip. It is the one field of a handler
/// that nothing else can reconstruct: the trigger, the filters and the actions
/// all come back from the tree, and the sentence saying what the handler is
/// for comes back only if it was kept.
#[test]
fn a_named_handler_keeps_its_name() {
    let events = round_trip(vec![ScenarioEventConfig {
        label: Some("picket warden wakes".to_string()),
        position: None,
        name: EventConfig::OnEnter,
        once: true,
        filters: vec![],
        actions: vec![],
    }]);

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].label.as_deref(), Some("picket warden wakes"));
}

/// Every shape of incomplete condition the document can hold, none of which
/// the handler list can spell.
fn half_built(world: &mut World, scenario: Entity) -> Entity {
    let handler = handlers(world, scenario)[0];
    let not = world
        .run_system_once(
            move |mut commands: Commands, mut ordinals: Query<&mut NextChildOrdinal>| {
                let id = crate::node::mint_id(&mut ordinals, handler, FilterChoice::Not.stem());
                spawn_node(
                    &mut commands,
                    handler,
                    "Filter",
                    id,
                    FilterNode {
                        kind: FilterKind::Not,
                    },
                )
            },
        )
        .expect("the spawn runs");
    world.flush();
    not
}

/// A `Not` with nothing in it does not lower: the file cannot spell it. What
/// must not happen is that the builder is never told.
#[test]
fn a_half_built_combinator_raises_a_gap_rather_than_vanishing() {
    let mut world = World::new();
    let scenario = seeded(&mut world, one_handler());
    let before = lowered(&mut world, scenario)[0].filters.len();
    let not = half_built(&mut world, scenario);

    assert_eq!(
        lowered(&mut world, scenario)[0].filters.len(),
        before,
        "the file has no way to spell an empty Not, so the lowering loses it"
    );
    let gaps = world
        .run_system_once(move |script: ScriptNodes| script.gaps(scenario))
        .expect("the walk runs");
    assert_eq!(
        gaps.iter().map(|gap| gap.0).collect::<Vec<_>>(),
        vec![not],
        "the block that would vanish is the block named: {gaps:?}"
    );
}

/// A world holding the document verbs need beside the script.
fn block_world(events: Vec<ScenarioEventConfig>) -> (World, Entity) {
    let mut world = World::new();
    let scenario = seeded(&mut world, events);
    (world, scenario)
}

fn messages(world: &mut World, scenario: Entity, handler: usize) -> Vec<String> {
    lowered(world, scenario)[handler]
        .actions
        .iter()
        .map(|action| match action {
            EventActionConfig::DebugMessage(message) => message.message.clone(),
            other => format!("{:?}", other.tag()),
        })
        .collect()
}

fn three_messages() -> Vec<ScenarioEventConfig> {
    vec![
        ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: Vec::new(),
            actions: vec![message("a"), message("b"), message("c")],
        },
        ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnResume,
            once: false,
            filters: Vec::new(),
            actions: vec![message("x")],
        },
    ]
}

fn ids_of(world: &World, nodes: &[Entity]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| world.get::<NodeId>(*node).expect("an id").0.clone())
        .collect()
}

/// The last action moving to the top reads in the new order, and no two
/// siblings share an id.
#[test]
fn a_reorder_keeps_ids_unique_and_lowers_in_the_new_order() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let actions = under(&mut world, handler).1;

    move_block(
        &mut world,
        actions[2],
        BlockSlotType::Action {
            parent: handler,
            index: 0,
        },
    )
    .expect("an action fits a stack");

    assert_eq!(messages(&mut world, scenario, 0), ["c", "a", "b"]);
    let after = under(&mut world, handler).1;
    let mut ids = ids_of(&world, &after);
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "every sibling keeps an id of its own");
}

/// A stack is carried by its top: what stands below goes with it, and the
/// receiving parent's counter never hands out an ordinal twice.
#[test]
fn dragging_a_block_carries_the_blocks_below_it() {
    let (mut world, scenario) = block_world(three_messages());
    let all = handlers(&mut world, scenario);
    let (from, to) = (all[0], all[1]);
    let actions = under(&mut world, from).1;
    let counter = world.get::<NextChildOrdinal>(to).expect("a counter").0;

    move_block(
        &mut world,
        actions[1],
        BlockSlotType::Action {
            parent: to,
            index: 1,
        },
    )
    .expect("an action fits a stack");

    assert_eq!(messages(&mut world, scenario, 0), ["a"]);
    assert_eq!(messages(&mut world, scenario, 1), ["x", "b", "c"]);
    assert_eq!(
        world.get::<NextChildOrdinal>(to).expect("a counter").0,
        counter + 2,
        "each arrival took a fresh ordinal"
    );
}

/// Every shape-and-slot mismatch is refused, and refused means nothing moved.
#[test]
fn an_illegal_drop_changes_nothing() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let actions = under(&mut world, handler).1;
    let before = spelled(&lowered(&mut world, scenario));

    for slot in [
        BlockSlotType::Canvas(Vec2::ZERO),
        BlockSlotType::Filter {
            parent: handler,
            index: 0,
        },
        BlockSlotType::Value {
            parent: handler,
            index: 0,
        },
        BlockSlotType::Gate { step: handler },
    ] {
        let refused = move_block(&mut world, actions[0], slot);
        assert!(refused.is_err(), "{slot:?} takes no action");
    }

    assert_eq!(spelled(&lowered(&mut world, scenario)), before);
}

/// A stack dropped into its own inside would leave no owner to lower it from.
#[test]
fn a_block_cannot_go_inside_itself() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let sequence = place_new(
        &mut world,
        BlockItemType::Action(ActionChoice::Sequence),
        BlockSlotType::Action {
            parent: handler,
            index: 3,
        },
    )
    .expect("a sequence is an action");
    let step = world
        .run_system_once(move |script: ScriptNodes| script.steps_of(sequence)[0])
        .expect("the walk runs");

    let refused = move_block(
        &mut world,
        sequence,
        BlockSlotType::Action {
            parent: step,
            index: 0,
        },
    );

    assert!(refused.is_err());
}

/// A gate with no deadline is a lint error, so the gate brings one.
#[test]
fn a_new_gate_arrives_with_a_deadline() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let sequence = place_new(
        &mut world,
        BlockItemType::Action(ActionChoice::Sequence),
        BlockSlotType::Action {
            parent: handler,
            index: 0,
        },
    )
    .expect("a sequence is an action");
    let step = world
        .run_system_once(move |script: ScriptNodes| script.steps_of(sequence)[0])
        .expect("the walk runs");

    place_new(
        &mut world,
        BlockItemType::Gate,
        BlockSlotType::Gate { step },
    )
    .expect("an empty step takes a gate");

    assert_eq!(
        world.get::<StepNode>(step).expect("a step").deadline,
        Some(NEW_GATE_DEADLINE)
    );
    assert!(
        place_new(
            &mut world,
            BlockItemType::Gate,
            BlockSlotType::Gate { step }
        )
        .is_err(),
        "a step that waits takes no second gate"
    );
}

/// A reporter lands on a literal, which is the socket's own field; it does not
/// land on another reporter; and pulling it out leaves a `0`, so a condition
/// never loses a side.
#[test]
fn a_value_socket_is_never_empty() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let filter = place_new(
        &mut world,
        BlockItemType::Expr(ExprChoice::LessThan),
        BlockSlotType::Filter {
            parent: handler,
            index: 0,
        },
    )
    .expect("a comparison wraps itself in a filter");
    let root = operands(&mut world, filter)[0];

    let elapsed = place_new(
        &mut world,
        BlockItemType::Expr(ExprChoice::Query),
        BlockSlotType::Value {
            parent: root,
            index: 1,
        },
    )
    .expect("a reporter replaces a literal");
    let text = |world: &mut World| {
        world
            .run_system_once(move |script: ScriptNodes| condition_text(&script, filter))
            .expect("the walk runs")
            .expect("the condition lowers")
    };
    assert_eq!(text(&mut world), "0 < scenario.elapsed");
    assert!(
        place_new(
            &mut world,
            BlockItemType::Expr(ExprChoice::Query),
            BlockSlotType::Value {
                parent: root,
                index: 1,
            },
        )
        .is_err(),
        "a reporter is not replaced by a drop"
    );

    move_block(
        &mut world,
        elapsed,
        BlockSlotType::Value {
            parent: root,
            index: 0,
        },
    )
    .expect("the other socket holds a literal");
    assert_eq!(text(&mut world), "scenario.elapsed < 0");
}

/// A comparison pulled across handlers keeps its operands, and a filter moved
/// out of a combinator leaves the combinator incomplete and said to be.
#[test]
fn a_filter_pulled_from_a_combinator_leaves_a_gap_on_it() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let both = place_new(
        &mut world,
        BlockItemType::Filter(FilterChoice::And),
        BlockSlotType::Filter {
            parent: handler,
            index: 0,
        },
    )
    .expect("a combinator is a filter");
    let inner = place_new(
        &mut world,
        BlockItemType::Filter(FilterChoice::Timer),
        BlockSlotType::Operand {
            parent: both,
            index: 0,
        },
    )
    .expect("the socket is empty");
    let third = BlockSlotType::Operand {
        parent: both,
        index: 0,
    };
    assert!(
        place_new(
            &mut world,
            BlockItemType::Filter(FilterChoice::Timer),
            third
        )
        .is_err(),
        "a full socket refuses"
    );

    let gaps = world
        .run_system_once(move |script: ScriptNodes| script.gaps(scenario))
        .expect("the walk runs");
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].0, both);
    assert!(world.get::<FilterNode>(inner).is_some());
}

/// The stored position survives the trip through the file's shape, and a
/// handler with no position stays that way.
#[test]
fn a_handler_position_round_trips_through_the_lowering() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];

    move_block(
        &mut world,
        handler,
        BlockSlotType::Canvas(Vec2::new(40.0, 60.0)),
    )
    .expect("a handler stands on the canvas");

    let events = lowered(&mut world, scenario);
    assert_eq!(events[0].position, Some(Vec2::new(40.0, 60.0)));
    assert_eq!(events[1].position, None);
    assert_eq!(
        round_trip(events.clone())[0].position,
        Some(Vec2::new(40.0, 60.0))
    );
}

/// The palette is the tables: nothing they declare is missing from it.
#[test]
fn the_palette_covers_every_kind_in_the_tables() {
    let items = BlockItemType::all();
    for trigger in EventConfig::ALL {
        assert!(items.contains(&BlockItemType::Hat(trigger)), "{trigger:?}");
    }
    for action in ActionChoice::ALL {
        assert!(items.contains(&BlockItemType::Action(action)), "{action:?}");
    }
    for filter in FilterChoice::ALL {
        assert!(items.contains(&BlockItemType::Filter(filter)), "{filter:?}");
    }
    for expr in ExprChoice::ALL {
        assert!(items.contains(&BlockItemType::Expr(expr)), "{expr:?}");
    }
    assert!(
        items.iter().all(|item| !item.hint().is_empty()),
        "every block says what it is for"
    );
}

/// Deleting the top of a stack takes the blocks below, as dragging it would.
#[test]
fn deleting_a_block_takes_the_blocks_below_it() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let actions = under(&mut world, handler).1;

    delete_block(&mut world, actions[1]);

    assert_eq!(messages(&mut world, scenario, 0), ["a"]);
}

/// The operator of a variable set, retyped from its stock value to `0 - 0`.
fn retyped_subtraction(world: &mut World, handler: Entity) -> Entity {
    let writes = place_new(
        world,
        BlockItemType::Action(ActionChoice::VariableSet),
        BlockSlotType::Action {
            parent: handler,
            index: 0,
        },
    )
    .expect("an action joins a stack");
    let root = operands(world, writes)[0];
    retype_expression(world, root, ExprChoice::Subtract);
    root
}

/// The operands a retype spawns hold ordinals 1 and 2, so the counter has to
/// stand past them or the next reporter takes an ordinal already in use.
#[test]
fn a_reporter_dropped_on_a_retyped_operator_keeps_its_side() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let root = retyped_subtraction(&mut world, handler);

    place_new(
        &mut world,
        BlockItemType::Expr(ExprChoice::Query),
        BlockSlotType::Value {
            parent: root,
            index: 1,
        },
    )
    .expect("a reporter replaces a literal");

    let spelled = world
        .run_system_once(move |script: ScriptNodes| script.lower_expression(root))
        .expect("the walk runs")
        .expect("the operator lowers")
        .to_string();
    assert_eq!(spelled, "0 - scenario.elapsed");
    let held = operands(&mut world, root);
    let ids = ids_of(&world, &held);
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "{ids:?}");
}

/// A combinator holds what its kind can lower. A third condition has no socket:
/// it would be held, drawn nowhere and dropped on save.
#[test]
fn a_full_combinator_refuses_a_third_condition() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let both = place_new(
        &mut world,
        BlockItemType::Filter(FilterChoice::And),
        BlockSlotType::Filter {
            parent: handler,
            index: 0,
        },
    )
    .expect("a combinator is a filter");
    for index in 0..2 {
        place_new(
            &mut world,
            BlockItemType::Filter(FilterChoice::Timer),
            BlockSlotType::Operand {
                parent: both,
                index,
            },
        )
        .expect("the socket is empty");
    }
    let last = under(&mut world, both).0[1];
    let slot = world
        .run_system_once(move |script: ScriptNodes| {
            slot_after(&script, last, BlockShapeType::Boolean)
        })
        .expect("the walk runs")
        .expect("beside a filter there is a slot to ask about");

    assert!(place_new(&mut world, BlockItemType::Filter(FilterChoice::Timer), slot).is_err());
    assert_eq!(under(&mut world, both).0.len(), 2);
}

/// Three conditions on a two-socket combinator cannot be drawn or written, so a
/// document holding them is told so rather than losing one.
#[test]
fn too_many_conditions_on_a_combinator_raise_a_gap() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let both = place_new(
        &mut world,
        BlockItemType::Filter(FilterChoice::And),
        BlockSlotType::Filter {
            parent: handler,
            index: 0,
        },
    )
    .expect("a combinator is a filter");
    for ordinal in 1..=3 {
        world
            .run_system_once(move |mut commands: Commands| {
                spawn_node(
                    &mut commands,
                    both,
                    "Filter",
                    NodeId(format!("timer_{ordinal}")),
                    FilterNode {
                        kind: FilterChoice::Timer.stock(),
                    },
                );
            })
            .expect("the spawn runs");
    }

    let gaps = world
        .run_system_once(move |script: ScriptNodes| script.gaps(scenario))
        .expect("the walk runs");

    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert_eq!(gaps[0].0, both);
}

/// Both ways across one operator: the block leaves its socket for a stock `0`
/// and takes the socket it was dropped on.
#[test]
fn a_reporter_crosses_its_own_operator_both_ways() {
    let (mut world, scenario) = block_world(three_messages());
    let handler = handlers(&mut world, scenario)[0];
    let filter = place_new(
        &mut world,
        BlockItemType::Expr(ExprChoice::LessThan),
        BlockSlotType::Filter {
            parent: handler,
            index: 0,
        },
    )
    .expect("a comparison wraps itself in a filter");
    let root = operands(&mut world, filter)[0];
    let elapsed = place_new(
        &mut world,
        BlockItemType::Expr(ExprChoice::Query),
        BlockSlotType::Value {
            parent: root,
            index: 0,
        },
    )
    .expect("a reporter replaces a literal");
    let text = |world: &mut World| {
        world
            .run_system_once(move |script: ScriptNodes| condition_text(&script, filter))
            .expect("the walk runs")
            .expect("the condition lowers")
    };
    assert_eq!(text(&mut world), "scenario.elapsed < 0");

    let right = BlockSlotType::Value {
        parent: root,
        index: 1,
    };
    move_block(&mut world, elapsed, right).expect("the other socket holds a literal");
    assert_eq!(text(&mut world), "0 < scenario.elapsed");

    let left = BlockSlotType::Value {
        parent: root,
        index: 0,
    };
    move_block(&mut world, elapsed, left).expect("the other socket holds a literal");
    assert_eq!(text(&mut world), "scenario.elapsed < 0");
    assert_eq!(operands(&mut world, root).len(), 2);
}
