//! Typed query sampling, keyed timers, and the per-frame scenario pulse.

use avian3d::prelude::LinearVelocity;
use bevy::{platform::collections::HashMap, prelude::*};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use super::scenario_is_live;
use crate::prelude::*;

/// Accumulate the scenario clock. Registered CHAINED AHEAD of
/// [`fire_on_update`] under the same live+unpaused gate, so the pulse that
/// evaluates time-gated handlers always sees this frame's clock; pausing
/// (ESC menu or the outcome frame) freezes the clock by construction.
pub(super) fn tick_scenario_clock(time: Res<Time>, mut world: ResMut<NovaEventWorld>) {
    world.advance_scenario_elapsed(time.delta_secs_f64());
}

/// Run condition: something in the loaded scenario can READ an entity query.
///
/// [`sample_scenario_queries`] scales with the WORLD and not with the
/// scenario - it walks every `EntityId` carrying a velocity and allocates two
/// `String`s per match, every frame, and severing turns each hull section into
/// another free body. No shipped scenario reads an entity query at all, so
/// ungated it is dead work in every one of them.
fn scenario_reads_an_entity_query(world: Res<NovaEventWorld>) -> bool {
    world.reads_entity_queries()
}

/// Sample every exact-id entity speed once for a coherent query snapshot.
/// Entities without velocity are outside the `Speed` query domain. In
/// particular, ship section children carry reusable section-local `EntityId`s;
/// scanning them would falsely report duplicates across unrelated ships.
fn sample_scenario_queries(
    entities: Query<(&EntityId, &LinearVelocity)>,
    mut world: ResMut<NovaEventWorld>,
) {
    let mut speeds = HashMap::new();
    for (id, velocity) in &entities {
        // Engine boundary: avian reports world units per second, and the
        // expression language a scenario compares this against is SI.
        let value = Some(f64::from(
            MetersPerSecond::from_engine(velocity.length()).get(),
        ));
        match speeds.entry(id.0.clone()) {
            bevy::platform::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
            bevy::platform::collections::hash_map::Entry::Occupied(mut entry) => {
                if entry.get().is_some() {
                    error!(
                        "entity speed query id '{}' matched more than one entity; value is unavailable",
                        id.0
                    );
                }
                entry.insert(None);
            }
        }
    }
    world.sample_entity_speeds(speeds);
}

/// Read the current pause-frozen scenario clock.
pub(super) fn scenario_elapsed(world: &NovaEventWorld) -> f64 {
    world.scenario_elapsed()
}

/// Fire one event for every timer that crossed its deadline. Expired keys are
/// removed before dispatch, so an `OnTimerEnd` handler can restart the same key.
pub(crate) fn tick_scenario_timers(mut commands: Commands, mut world: ResMut<NovaEventWorld>) {
    let now = scenario_elapsed(&world);
    for key in world.drain_ended_timers(now) {
        commands.fire::<OnTimerEndEvent>(OnTimerEndEventInfo { key });
    }
}

/// The ONE registration of the clock, timers, and pulse, shared by the plugin
/// and test rigs. Timer-end events queue before the frame's OnUpdate pulse.
///
/// Gated on [`scenario_has_settled`] as well as live-and-unpaused: while a
/// scenario's queued spawns are still landing the world is not yet LIVE, so
/// its clock must not advance, its timers must not expire and the pulse must
/// not offer handlers a half-built world to read.
///
/// The `Sequence` driver sits inside the same chain, after the query sample and
/// before the timers: a step's actions therefore read the same snapshot a
/// handler does, and a step that starts a timer still has that timer checked
/// this frame.
///
/// Only the pulse is conditional. The clock, the query sample, the sequence
/// driver and the timers run whenever the scenario is live, because each of
/// them is what PRODUCES a reason to wake.
pub(super) fn register_clock_and_pulse(app: &mut App) {
    app.add_systems(
        Update,
        (
            tick_scenario_clock,
            sample_scenario_queries.run_if(scenario_reads_an_entity_query),
            advance_scenario_sequences,
            tick_scenario_timers,
            fire_on_update.run_if(scenario_pulse_is_due),
        )
            .chain()
            .run_if(
                scenario_is_live
                    .and_then(in_state(PauseStates::Unpaused))
                    .and_then(scenario_has_settled),
            ),
    );
}

/// Run condition on the pulse: is there anything for an `OnUpdate` handler to
/// react to?
///
/// The scenario SLEEPS otherwise, the way a settled rigidbody does. Waking is
/// not a poll - a write to a variable some filter reads, or the clock crossing
/// a threshold one compares against. See `loader::wake` for what is provable
/// and what falls back to every frame.
fn scenario_pulse_is_due(world: Res<NovaEventWorld>) -> bool {
    world.is_wake_due()
}

/// The per-frame pulse behind `EventConfig::OnUpdate` handlers. Scenarios
/// use it for value-gated milestones (e.g. shakedown's crate tally), which
/// must not depend on handler execution order within another event.
fn fire_on_update(mut commands: Commands, mut world: ResMut<NovaEventWorld>) {
    world.consume_wake();
    commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::fixtures::*;

    /// A `Sequence` end to end through the LIVE chain: the loader's gate
    /// handler, the driver in the pulse, the real clock.
    ///
    /// The chain is what makes this worth a rig - the unit tests drive the
    /// cursor by hand, and neither the one-frame gate latency nor the gate
    /// handler's registration exists at that level.
    #[test]
    fn a_sequence_walks_its_steps_on_the_live_pulse() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::GameEventsPlugin;
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);

        let mark = |ordinal: f64| {
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "at".to_string(),
                expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(ordinal)),
                )),
            })
        };

        // Beat one lands half a second in; beat two waits for the timer beat
        // one starts. Both spellings in one chain, the way content uses them.
        let config = scenario_with(
            "live",
            vec![event_with(vec![EventActionConfig::Sequence(
                SequenceActionConfig {
                    key: "opening".to_string(),
                    steps: vec![
                        SequenceStepConfig {
                            after: Some(0.5),
                            actions: vec![
                                mark(1.0),
                                EventActionConfig::TimerStart(TimerStartActionConfig {
                                    key: "ping".to_string(),
                                    seconds: VariableExpressionNode::new_term(
                                        VariableTermNode::new_factor(
                                            VariableFactorNode::new_literal(
                                                VariableLiteral::Number(0.5),
                                            ),
                                        ),
                                    ),
                                }),
                            ],
                            ..default()
                        },
                        SequenceStepConfig {
                            until: Some(SequenceGateConfig {
                                name: EventConfig::OnTimerEnd,
                                filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                                    key: "ping".to_string(),
                                })],
                            }),
                            deadline: Some(30.0),
                            actions: vec![mark(2.0)],
                            ..default()
                        },
                    ],
                },
            )])],
        );
        crate::test_support::register_scenario_handlers(&mut app, &config);
        app.insert_resource(CurrentScenario(Some(config)));

        let at = |app: &App| match app.world().resource::<NovaEventWorld>().get_variable("at") {
            Some(VariableLiteral::Number(n)) => Some(*n),
            _ => None,
        };

        app.world_mut()
            .commands()
            .fire::<OnStartEvent>(OnStartEventInfo);

        // ~0.3s of scenario time: beat one is not due.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(at(&app), None, "the first step holds until its delay");

        // Past 0.5s beat one lands and starts the timer it gates beat two on.
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(at(&app), Some(1.0), "the first step lands on the clock");

        // The timer ends ~0.5s later; the gate handler opens the step and the
        // next pulse delivers it.
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(at(&app), Some(2.0), "the gated step waits for its event");
    }

    /// A scenario with nothing to react to does not pulse at all, and one
    /// write wakes it EXACTLY once.
    ///
    /// Counted on the fired event itself rather than on a side effect: the
    /// question is whether `OnUpdate` was queued, and a handler's actions would
    /// answer a different one.
    #[test]
    fn the_pulse_sleeps_until_a_variable_it_reads_is_written() {
        let mut app = pulse_app(vec![on_update_filtered(vec![equals("armed", 1.0)])]);

        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            pulses(&app),
            0,
            "nothing has been written; the scenario sleeps"
        );

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("armed".to_string(), VariableLiteral::Number(1.0));

        app.update();
        assert_eq!(pulses(&app), 1, "the write wakes the pulse");

        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            pulses(&app),
            1,
            "the filter still passes, but nothing changed - it sleeps again"
        );
    }

    /// A literal clock threshold wakes the pulse ONCE, as it is crossed. A
    /// level test rather than a crossing would pulse every frame after it,
    /// which is the bug this pins.
    #[test]
    fn a_scheduled_time_wakes_the_pulse_once_as_it_is_crossed() {
        let mut app = pulse_app(vec![on_update_filtered(vec![
            EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_greater_than(
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_name("scenario_elapsed"),
                    )),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(0.5)),
                    )),
                ),
            )),
        ])]);

        for _ in 0..3 {
            app.update();
        }
        assert_eq!(pulses(&app), 0, "before the threshold");

        for _ in 0..10 {
            app.update();
        }
        assert_eq!(pulses(&app), 1, "crossed once, not held down");
    }

    /// A scenario the analyser cannot prove keeps every frame it has today.
    #[test]
    fn an_unprovable_scenario_still_pulses_every_frame() {
        let mut app = pulse_app(vec![on_update_filtered(vec![])]);

        for _ in 0..5 {
            app.update();
        }
        assert!(
            pulses(&app) >= 4,
            "an unfiltered OnUpdate is the fail-safe case, got {}",
            pulses(&app)
        );
    }

    #[derive(Resource, Default)]
    struct Pulses(usize);

    fn pulses(app: &App) -> usize {
        app.world().resource::<Pulses>().0
    }

    fn equals(key: &str, value: f64) -> EventFilterConfig {
        EventFilterConfig::Expression(ExpressionFilterConfig(VariableConditionNode::new_equals(
            VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_name(key),
            )),
            VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Number(value)),
            )),
        )))
    }

    fn on_update_filtered(filters: Vec<EventFilterConfig>) -> ScenarioEventConfig {
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            filters,
            ..event_with(vec![])
        }
    }

    /// The live chain plus a counter on the fired `OnUpdate` event.
    fn pulse_app(events: Vec<ScenarioEventConfig>) -> App {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventKind, GameEvent, GameEventsPlugin, OnUpdateEvent};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.init_resource::<Pulses>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.add_observer(|event: On<GameEvent>, mut pulses: ResMut<Pulses>| {
            if event.event().name() == OnUpdateEvent::name() {
                pulses.0 += 1;
            }
        });
        register_clock_and_pulse(&mut app);

        let config = ScenarioConfig {
            watches: vec![WatchConfig {
                variable: "scenario_elapsed".to_string(),
                query: QueryConfig::Scenario(ScenarioQuery {
                    property: ScenarioProperty::Elapsed,
                }),
            }],
            ..scenario_with("pulse", events)
        };
        crate::test_support::register_scenario_handlers(&mut app, &config);
        app.insert_resource(CurrentScenario(Some(config)));
        app
    }

    /// The OnUpdate pulse: fires every frame while a scenario is live and stays
    /// silent otherwise. Proven through a real OnUpdate handler mutating a
    /// variable - a handler that could not fire without the pulse.
    #[test]
    fn on_update_pulses_only_while_a_scenario_is_live() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.add_systems(Update, fire_on_update.run_if(scenario_is_live));

        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "pulsed".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        // No scenario: silent.
        app.update();
        app.update();
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("pulsed")
                .is_none(),
            "no pulse without a live scenario"
        );

        // Scenario live: the handler fires within a frame or two.
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("pulsed"),
            Some(&VariableLiteral::Boolean(true)),
            "a live scenario pulses OnUpdate handlers"
        );
    }

    /// The OnUpdate pulse is Unpaused-gated: a handler whose predicate is
    /// already true must NOT re-run its action every frame while the game is
    /// Paused (the ESC menu / outcome frame), and must resume firing on
    /// unpause. Proven with a filterless OnUpdate handler that INCREMENTS a
    /// counter - a value that would keep climbing under pause if the pulse
    /// leaked through. Uses the exact production run condition
    /// (`scenario_is_live.and_then(in_state(Unpaused))`).
    #[test]
    fn on_update_pulse_freezes_while_paused_and_resumes_on_unpause() {
        use bevy::state::app::StatesPlugin;
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.add_systems(
            Update,
            fire_on_update.run_if(scenario_is_live.and_then(in_state(PauseStates::Unpaused))),
        );

        // Seed the counter so the increment expression has a number to read
        // (an undefined variable would error the action out).
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("count".to_string(), VariableLiteral::Number(0.0));

        // A filterless OnUpdate handler that does `count = count + 1` - its
        // predicate is trivially always true, so every pulse re-runs it.
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "count".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("count")),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);

        let count = |app: &App| match app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable("count")
        {
            Some(VariableLiteral::Number(n)) => *n,
            other => panic!("count must be a number, got {other:?}"),
        };

        // Live scenario, Unpaused (default): the counter climbs each frame.
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.update();
        app.update();
        let while_unpaused = count(&app);
        assert!(
            while_unpaused > 0.0,
            "an Unpaused live scenario must pulse OnUpdate handlers ({while_unpaused})"
        );

        // Pause: the pulse stops, so the already-true handler stops re-firing
        // and the counter is frozen no matter how many frames pass.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update(); // applies the transition
        let at_pause = count(&app);
        app.update();
        app.update();
        app.update();
        assert_eq!(
            count(&app),
            at_pause,
            "a Paused game must freeze the OnUpdate pulse: an already-true \
             handler must not re-run its action while paused"
        );

        // Unpause: delivery-guarded, not dropped - the pulse resumes and the
        // counter climbs again.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update(); // applies the transition
        app.update();
        app.update();
        assert!(
            count(&app) > at_pause,
            "unpausing must resume the OnUpdate pulse ({} -> {})",
            at_pause,
            count(&app)
        );
    }

    /// The scenario SCRIPT is held until the spawn queue drains. A slow spawn
    /// batch queued before the first frame keeps the world settling for several
    /// frames; across those the clock must not advance and the `OnUpdate` pulse
    /// must not hand a handler a half-built world. Both resume on the first
    /// frame after the last command lands.
    ///
    /// Fail-first: without the gate the pulse fires on frame one, while four
    /// fifths of the scenario's objects do not exist yet.
    #[test]
    fn the_script_is_held_until_the_spawn_queue_drains() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventHandler, EventWorld, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);

        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "pulsed".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));

        // The OnStart burst: five objects, each costing about the whole
        // per-frame drain budget, queued before any frame runs.
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            for _ in 0..5 {
                world.push_command(|commands| {
                    commands.queue(|_: &mut World| {
                        std::thread::sleep(Duration::from_millis(2));
                    });
                });
            }
        }

        app.update();
        assert!(
            app.world().resource::<NovaEventWorld>().is_settling(),
            "delivery guard: the batch is still landing after one frame"
        );
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("pulsed")
                .is_none(),
            "the OnUpdate pulse must not fire against a half-built world"
        );
        assert_eq!(
            app.world().resource::<NovaEventWorld>().scenario_elapsed(),
            0.0,
            "the scenario clock must not run while the world is being built"
        );

        let mut frames = 1;
        while app.world().resource::<NovaEventWorld>().is_settling() {
            app.update();
            frames += 1;
            assert!(frames < 100, "the drain must terminate");
        }
        assert!(frames >= 2, "the batch spanned {frames} frame(s)");

        // Live at last: the pulse and the clock resume.
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("pulsed"),
            Some(&VariableLiteral::Boolean(true)),
            "the script runs once the world is fully spawned"
        );
        assert!(
            app.world().resource::<NovaEventWorld>().scenario_elapsed() > 0.0,
            "the scenario clock resumes with the script"
        );
    }

    /// A keyed timer freezes under pause, then emits one filtered end event
    /// after unpause. This uses the production clock/timer/dispatch chain.
    #[test]
    fn timer_end_dispatches_once_after_live_unpaused_time() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventAction, EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);

        let mut matching = EventHandler::<NovaEventWorld>::from(EventConfig::OnTimerEnd);
        matching.add_filter(EventFilterConfig::Timer(TimerFilterConfig {
            key: "briefing".to_string(),
        }));
        matching.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "ended".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(matching);

        let mut wrong_key = EventHandler::<NovaEventWorld>::from(EventConfig::OnTimerEnd);
        wrong_key.add_filter(EventFilterConfig::Timer(TimerFilterConfig {
            key: "other".to_string(),
        }));
        wrong_key.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "wrong_key_fired".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(wrong_key);

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        TimerStartActionConfig {
            key: "briefing".to_string(),
            seconds: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Number(0.3)),
            )),
        }
        .action(
            &mut app.world_mut().resource_mut::<NovaEventWorld>(),
            &default(),
        );

        app.update();
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        for _ in 0..5 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("ended")
                .is_none(),
            "paused frames do not consume timer time"
        );

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        for _ in 0..5 {
            app.update();
        }
        let world = app.world().resource::<NovaEventWorld>();
        assert_eq!(
            world.get_variable("ended"),
            Some(&VariableLiteral::Boolean(true))
        );
        assert!(world.get_variable("wrong_key_fired").is_none());
        assert!(!world.timer_is_running("briefing"));
    }

    /// The scenario clock: accumulates live unpaused seconds into a typed
    /// query and gates a real time-filtered OnUpdate handler - held before
    /// the threshold, fired after. Driven through the production tick + pulse
    /// pair on a manual 0.1s clock (steps under Time<Virtual>'s 0.25s max_delta
    /// clamp - the manual-time-rig lesson).
    #[test]
    fn scenario_clock_gates_time_filtered_handlers() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);

        // A one-shot beat the way an author writes it: elapsed > 0.5s AND
        // the flag unfired, then the action raises the flag.
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_filter(EventFilterConfig::Expression(ExpressionFilterConfig(
            VariableConditionNode::new_greater_than(
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_query(QueryConfig::Scenario(ScenarioQuery {
                        property: ScenarioProperty::Elapsed,
                    })),
                )),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(0.5)),
                )),
            ),
        )));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "beat_fired".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));

        // ~0.3s of scenario time: the gate must hold (fails closed while
        // the clock is below the threshold).
        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("beat_fired")
                .is_none(),
            "the time gate holds before the threshold"
        );

        // Past 0.5s the beat fires - the delivery guard proving the clock
        // is what advanced (with the tick removed this stays None forever).
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("beat_fired"),
            Some(&VariableLiteral::Boolean(true)),
            "the beat fires once the scenario clock passes the threshold"
        );
    }

    /// The clock freezes under pause exactly like the pulse it feeds (same
    /// chained registration, same run condition), and resumes on unpause.
    #[test]
    fn scenario_clock_freezes_while_paused() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::GameEventsPlugin;
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);
        let elapsed = |app: &App| app.world().resource::<NovaEventWorld>().scenario_elapsed();

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        for _ in 0..3 {
            app.update();
        }
        let before_pause = elapsed(&app);
        assert!(
            before_pause > 0.0,
            "a live unpaused scenario ticks the clock"
        );

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update(); // applies the transition
        let at_pause = elapsed(&app);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            elapsed(&app),
            at_pause,
            "a paused game must freeze the scenario clock"
        );

        // Unpause: delivery-guarded - the clock climbs again.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update();
        app.update();
        assert!(
            elapsed(&app) > at_pause,
            "unpausing must resume the scenario clock"
        );
    }

    /// The clock resets with the event world on teardown or retry, so the next
    /// scenario never inherits stale seconds.
    #[test]
    fn scenario_clock_resets_with_the_event_world() {
        let mut world = NovaEventWorld::default();
        world.advance_scenario_elapsed(42.0);
        world.clear();
        assert_eq!(world.scenario_elapsed(), 0.0, "teardown resets the clock");
    }

    /// An exact-id speed watch tracks live avian velocity, ignores other ids,
    /// becomes unavailable with no match, and freezes under pause. The test
    /// uses the production query sampling and update gate.
    #[test]
    fn entity_speed_watch_tracks_live_velocity_and_fails_closed() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::GameEventsPlugin;
        use nova_gameplay::prelude::{GameObjectives, PlayerSpaceshipMarker, SpaceshipRootMarker};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);
        let speed = |app: &App| match app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable("player_speed")
        {
            Some(VariableLiteral::Number(n)) => *n,
            _ => 0.0,
        };
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .set_watches(
                vec![WatchConfig {
                    variable: "player_speed".to_string(),
                    query: QueryConfig::Entity(EntityQuery {
                        filter: EntityQueryFilter {
                            id: "player_spaceship".to_string(),
                        },
                        property: EntityProperty::Speed,
                    }),
                }],
                true,
            );

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));

        // An AI ship (root marker, NO player marker) burning fast the whole
        // test: player_speed must never read its velocity (the player-scope pin).
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("scavenger".to_string()),
            LinearVelocity(Vec3::new(30.0, 0.0, 40.0)), // |v| = 50 u/s, 500 m/s
        ));
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId::new("player_spaceship".to_string()),
                LinearVelocity(Vec3::new(3.0, 0.0, 4.0)), // |v| = 5 u/s, 50 m/s
            ))
            .id();
        // Section ids are local to a ship and commonly repeat. They have no
        // LinearVelocity and must not enter strict entity-speed matching.
        app.world_mut().spawn(EntityId::new("engine_port"));
        app.world_mut().spawn(EntityId::new("engine_port"));

        app.update();
        assert_eq!(
            speed(&app),
            50.0,
            "player_speed reads the player ship's |LinearVelocity| in m/s, not the AI's 500"
        );

        // Zero velocity reads 0.0 (the readout tracks live).
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().0 = Vec3::ZERO;
        app.update();
        assert_eq!(speed(&app), 0.0, "a stationary player reads zero speed");

        // A new velocity is reflected the next tick.
        // |v| = 10 u/s, which the readout prints as 100 m/s.
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().0 = Vec3::new(6.0, 0.0, 8.0);
        app.update();
        assert_eq!(speed(&app), 100.0, "player_speed follows the live velocity");

        // Pause freezes the readout: it holds its last value even as the
        // velocity changes underneath (the shared pause gate, same as the clock).
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update(); // applies the transition
        let at_pause = speed(&app);
        assert_eq!(at_pause, 100.0, "the readout latches its pre-pause value");
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().0 = Vec3::new(100.0, 0.0, 0.0);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            speed(&app),
            at_pause,
            "a paused game freezes player_speed even as the ship's velocity changes"
        );

        // Unpause: the readout tracks the live velocity again.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update();
        app.update();
        assert_eq!(
            speed(&app),
            1_000.0,
            "unpausing resumes tracking the live velocity"
        );

        // No player ship at all (teardown / between retries): fail-closed 0.0.
        app.world_mut().entity_mut(player).despawn();
        app.update();
        assert_eq!(
            speed(&app),
            0.0,
            "with no player ship, player_speed fails closed to 0.0 (not the AI's 500)"
        );
    }
}
