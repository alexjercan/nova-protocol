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
    VariableLiteral, VariableTermNode,
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

/// What Add > `add` would hang the new node from, with `marked` marked.
fn parent_for(
    world: &mut World,
    scenario: Entity,
    marked: Option<Entity>,
    add: ScriptAdd,
) -> Option<Entity> {
    world
        .run_system_once(move |script: ScriptNodes| add_parent(add, marked, &script, scenario))
        .expect("the question is asked")
}

/// Press Add > `add` with `marked` marked.
fn added(world: &mut World, scenario: Entity, marked: Option<Entity>, add: ScriptAdd) -> Entity {
    world
        .run_system_once(
            move |mut commands: Commands,
                  script: ScriptNodes,
                  mut ordinals: Query<&mut NextChildOrdinal>| {
                let parent = add_parent(add, marked, &script, scenario).expect("the row is live");
                spawn_script_node(&mut commands, &mut ordinals, parent, add)
            },
        )
        .expect("the add runs")
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

/// A sequence is the one action with children, and the children are ordered:
/// steps read back out of order are a different scenario.
#[test]
fn a_sequences_steps_keep_the_order_they_were_written_in() {
    let script = vec![ScenarioEventConfig {
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

/// A filter switched to an expression arrives with a condition already in it:
/// an operator with nothing under it is a filter the save would drop.
#[test]
fn a_filter_switched_to_an_expression_arrives_with_a_condition() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
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
        name: EventConfig::OnStart,
        once: false,
        filters: vec![named("player")],
        actions: vec![message("hello")],
    }]
}

/// Add lands the node under what is marked, and a fresh filter arrives
/// matching nothing: an id nobody typed would gate the handler on a choice
/// nobody made.
#[test]
fn a_filter_lands_under_the_marked_handler() {
    let mut world = World::new();
    let scenario = seeded(&mut world, one_handler());
    let handler = handlers(&mut world, scenario)[0];

    added(&mut world, scenario, Some(handler), ScriptAdd::Filter);

    let script = lowered(&mut world, scenario);
    assert_eq!(script[0].filters.len(), 2, "the handler kept its own");
    let EventFilterConfig::Entity(fresh) = &script[0].filters[1] else {
        panic!("a fresh filter matches entities");
    };
    assert_eq!(fresh.id, None, "and matches none of them yet");
}

/// A marked node that cannot take the new one hands the question UP: three
/// actions in a row is three presses, not three presses and two reselections.
#[test]
fn a_marked_action_adds_its_sibling() {
    let mut world = World::new();
    let scenario = seeded(&mut world, one_handler());
    let handler = handlers(&mut world, scenario)[0];
    let (_, actions) = under(&mut world, handler);

    assert_eq!(
        parent_for(&mut world, scenario, Some(actions[0]), ScriptAdd::Action),
        Some(handler)
    );
}

/// A combinator holds exactly what it combines. A third operand under an `And`
/// would be dropped by the lowering, so the add goes to the handler instead.
#[test]
fn a_full_combinator_takes_no_more_operands() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            once: false,
            filters: vec![EventFilterConfig::Conditional(
                ConditionalFilterConfig::and(named("a"), named("b")),
            )],
            actions: vec![],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let (filters, _) = under(&mut world, handler);
    let both = filters[0];
    let (operands, _) = under(&mut world, both);

    assert_eq!(
        parent_for(&mut world, scenario, Some(both), ScriptAdd::Filter),
        Some(handler),
        "a full And sends the filter to its own handler"
    );
    assert_eq!(
        parent_for(&mut world, scenario, Some(operands[0]), ScriptAdd::Filter),
        Some(handler),
        "and so does the leaf inside it, which is the same climb"
    );
}

/// A beat waits for one thing. The row that would give it a second is greyed,
/// which is the same `None` the verb reads.
#[test]
fn a_beat_that_already_waits_takes_no_second_gate() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![EventActionConfig::Sequence(SequenceActionConfig {
                key: "briefing".to_string(),
                steps: vec![SequenceStepConfig {
                    until: Some(SequenceGateConfig {
                        name: EventConfig::OnDestroyed,
                        filters: vec![],
                    }),
                    ..default()
                }],
            })],
        }],
    );
    let handler = handlers(&mut world, scenario)[0];
    let (_, actions) = under(&mut world, handler);
    let sequence = actions[0];
    let step = world
        .run_system_once(move |script: ScriptNodes| script.steps_of(sequence))
        .expect("the walk runs")[0];

    assert_eq!(
        parent_for(&mut world, scenario, Some(step), ScriptAdd::Gate),
        None
    );
    assert_eq!(
        parent_for(&mut world, scenario, Some(step), ScriptAdd::Step),
        Some(sequence),
        "a marked beat still adds the beat after it"
    );
}

/// Switching a combinator keeps the operands the new kind holds and drops the
/// rest: an operand a filter cannot hold is a node with no row.
#[test]
fn switching_a_combinator_keeps_the_operands_it_can_hold() {
    let mut world = World::new();
    let scenario = seeded(
        &mut world,
        vec![ScenarioEventConfig {
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

/// DESTROY X, REACH Y, SURVIVE T - the three objectives of the task, written in
/// the vocabulary the editor already offers and nothing else.
///
/// One handler posts them and starts the clock; one completes on the hulk
/// breaking, one on the marker area being entered, one on the timer ending -
/// and that last one declares the win.
fn objective_set() -> Vec<ScenarioEventConfig> {
    vec![
        ScenarioEventConfig {
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

/// The whole claim of the folded-in objectives task: destroy X, reach Y,
/// survive T is authorable in the vocabulary the editor already holds, survives
/// the tree, and completes its player path.
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
