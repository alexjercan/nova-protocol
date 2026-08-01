//! The reserved engine variables and the per-frame clock/pulse pair.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use super::scenario_is_live;
use crate::prelude::*;

/// The reserved scenario-clock variable: seconds of LIVE, UNPAUSED scenario
/// time, maintained by `tick_scenario_clock` and readable from any expression
/// filter as `Term(Factor(Name("scenario_elapsed")))`. Authors GATE on it, they
/// never write it - a `VariableSet` on this key is a content_lint ERROR,
/// because the engine overwrites it every tick. It clears with the rest of the
/// event world at teardown, so it is the CURRENT scenario's clock (a retry
/// restarts it), and an early read before the first tick fails closed via the
/// undefined-variable rule.
///
/// One-shots compose with the standard act/flag gate: `elapsed > N` plus an act
/// filter, then advance the act. Repeating waves compose with a rearm write:
/// gate on `elapsed > next_at`, then `VariableSet(next_at, Add(next_at,
/// interval))`.
pub const SCENARIO_ELAPSED_VAR: &str = "scenario_elapsed";

/// The reserved player-speed variable: the PLAYER ship's live speed in
/// units/second (the magnitude of its avian [`LinearVelocity`]), maintained by
/// `track_player_speed` and readable from any expression filter as
/// `Term(Factor(Name("player_speed")))`. Like [`SCENARIO_ELAPSED_VAR`] it is
/// ENGINE-written every live-unpaused tick, so authors GATE on it and never
/// write it (a `VariableSet` on this key is a content_lint ERROR). It reads
/// `0.0` when no player ship exists (fail-closed, same as an early clock read)
/// and freezes under pause / clears at teardown by riding the same gate + event
/// world as the clock. Content uses it for speed-gated beats - e.g. a stealth
/// run where burning too hot wakes a picket.
pub const PLAYER_SPEED_VAR: &str = "player_speed";

/// True for the engine-owned reserved variables the loader writes every tick
/// ([`SCENARIO_ELAPSED_VAR`], [`PLAYER_SPEED_VAR`]). content_lint reads this to
/// exempt them from the undefined-variable rule (they need no `VariableSet` to
/// be readable) and to REJECT an authored `VariableSet` onto them (the engine
/// overwrites it every frame). One list so the two rules cannot drift apart.
pub fn is_reserved_engine_var(name: &str) -> bool {
    name == SCENARIO_ELAPSED_VAR || name == PLAYER_SPEED_VAR
}

/// Accumulate the scenario clock. Registered CHAINED AHEAD of
/// [`fire_on_update`] under the same live+unpaused gate, so the pulse that
/// evaluates time-gated handlers always sees this frame's clock; pausing
/// (ESC menu or the outcome frame) freezes the clock by construction.
pub(super) fn tick_scenario_clock(time: Res<Time>, mut world: ResMut<NovaEventWorld>) {
    let elapsed = scenario_elapsed(&world);
    world.insert_variable(
        SCENARIO_ELAPSED_VAR.to_string(),
        VariableLiteral::Number(elapsed + time.delta_secs_f64()),
    );
}

/// Publish the PLAYER ship's live speed into [`PLAYER_SPEED_VAR`] every
/// live-unpaused tick, so speed-gated expression filters read this frame's
/// value. Player-scoped (`With<PlayerSpaceshipMarker>`), like
/// [`track_player_locks`], so an AI ship's velocity never drives content. No
/// player ship (pre-spawn, between retries, teardown) publishes `0.0` - the
/// same fail-closed default a filter sees for the clock before its first tick.
/// Registered CHAINED AHEAD of [`fire_on_update`] alongside the clock so the
/// pulse's speed gates see the current value; the shared pause gate freezes it.
fn track_player_speed(
    q_player: Query<&LinearVelocity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut world: ResMut<NovaEventWorld>,
) {
    let speed = q_player
        .iter()
        .next()
        .map_or(0.0, |velocity| velocity.length() as f64);
    world.insert_variable(PLAYER_SPEED_VAR.to_string(), VariableLiteral::Number(speed));
}

/// Read the current scenario clock (seconds of live-unpaused time) off the
/// event world, with the same `None -> 0.0` fallback as [`tick_scenario_clock`]
/// so a read before the first tick (or after teardown's `world.clear`) sees a
/// fresh clock. The clock-derived trackers below ([`track_orbit_holds`],
/// [`track_player_locks`]) measure their 5s windows against this instead of
/// accumulating their own `Time` delta, so pausing and teardown/retry freeze
/// and reset every window in one place.
pub(super) fn scenario_elapsed(world: &NovaEventWorld) -> f64 {
    match world.get_variable(SCENARIO_ELAPSED_VAR) {
        Some(VariableLiteral::Number(n)) => *n,
        _ => 0.0,
    }
}

/// The ONE registration of the clock + pulse pair, shared by the plugin and the
/// test rigs so the load-bearing chain + gate cannot drift between them: tick
/// first, pulse second, both gated live + Unpaused.
pub(super) fn register_clock_and_pulse(app: &mut App) {
    app.add_systems(
        Update,
        (tick_scenario_clock, track_player_speed, fire_on_update)
            .chain()
            .run_if(scenario_is_live.and_then(in_state(PauseStates::Unpaused))),
    );
}

/// The per-frame pulse behind `EventConfig::OnUpdate` handlers. Scenarios
/// use it for value-gated milestones (e.g. shakedown's crate tally), which
/// must not depend on handler execution order within another event.
fn fire_on_update(mut commands: Commands) {
    commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::fixtures::*;

    /// The OnUpdate pulse: fires every frame while a scenario is live and stays
    /// silent otherwise. Proven through a real OnUpdate handler mutating a
    /// variable - a handler that could not fire without the pulse.
    #[test]
    fn on_update_pulses_only_while_a_scenario_is_live() {
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

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
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

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

    /// The scenario clock: accumulates live unpaused seconds into the reserved
    /// variable and gates a real time-filtered OnUpdate handler - held before
    /// the threshold, fired after. Driven through the production tick + pulse
    /// pair on a manual 0.1s clock (steps under Time<Virtual>'s 0.25s max_delta
    /// clamp - the manual-time-rig lesson).
    #[test]
    fn scenario_clock_gates_time_filtered_handlers() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

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
                    VariableFactorNode::new_name(SCENARIO_ELAPSED_VAR),
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
        use bevy_common_systems::prelude::{GameEventsPlugin, GameObjectives};

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
        let elapsed = |app: &App| match app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable(SCENARIO_ELAPSED_VAR)
        {
            Some(VariableLiteral::Number(n)) => *n,
            _ => 0.0,
        };

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

    /// The clock dies with the event world (teardown/retry): after clear()
    /// the variable is gone, so a time gate on the next scenario fails
    /// closed until the fresh clock ticks - never inherits stale seconds.
    #[test]
    fn scenario_clock_resets_with_the_event_world() {
        let mut world = NovaEventWorld::default();
        world.insert_variable(
            SCENARIO_ELAPSED_VAR.to_string(),
            VariableLiteral::Number(42.0),
        );
        world.clear();
        assert!(
            world.get_variable(SCENARIO_ELAPSED_VAR).is_none(),
            "teardown clears the clock with the rest of the event world"
        );
    }

    /// The reserved `player_speed` variable tracks the PLAYER ship's live
    /// speed off its avian `LinearVelocity`, is PLAYER-scoped (an AI ship's
    /// velocity never leaks in), reads 0.0 with no player (fail-closed), and
    /// freezes under pause - all through the REAL `register_clock_and_pulse`
    /// registration, so the gate + chain match production exactly. The
    /// magnitude asserts double as the delivery guard: with `track_player_speed`
    /// unregistered the variable stays absent and every `speed(&app)` read is
    /// 0.0, so each non-zero expectation below fails.
    #[test]
    fn player_speed_var_tracks_live_velocity_and_fails_closed() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{GameEventsPlugin, GameObjectives};
        use nova_gameplay::prelude::{PlayerSpaceshipMarker, SpaceshipRootMarker};

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
            .get_variable(PLAYER_SPEED_VAR)
        {
            Some(VariableLiteral::Number(n)) => *n,
            _ => 0.0,
        };

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));

        // An AI ship (root marker, NO player marker) burning fast the whole
        // test: player_speed must never read its velocity (the player-scope pin).
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("scavenger".to_string()),
            LinearVelocity(Vec3::new(30.0, 0.0, 40.0)), // |v| = 50
        ));
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId::new("player_spaceship".to_string()),
                LinearVelocity(Vec3::new(3.0, 0.0, 4.0)), // |v| = 5
            ))
            .id();

        app.update();
        assert_eq!(
            speed(&app),
            5.0,
            "player_speed reads the player ship's |LinearVelocity|, not the AI's 50"
        );

        // Zero velocity reads 0.0 (the readout tracks live).
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().0 = Vec3::ZERO;
        app.update();
        assert_eq!(speed(&app), 0.0, "a stationary player reads zero speed");

        // A new velocity is reflected the next tick.
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().0 = Vec3::new(6.0, 0.0, 8.0); // |v| = 10
        app.update();
        assert_eq!(speed(&app), 10.0, "player_speed follows the live velocity");

        // Pause freezes the readout: it holds its last value even as the
        // velocity changes underneath (the shared pause gate, same as the clock).
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update(); // applies the transition
        let at_pause = speed(&app);
        assert_eq!(at_pause, 10.0, "the readout latches its pre-pause value");
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
            100.0,
            "unpausing resumes tracking the live velocity"
        );

        // No player ship at all (teardown / between retries): fail-closed 0.0.
        app.world_mut().entity_mut(player).despawn();
        app.update();
        assert_eq!(
            speed(&app),
            0.0,
            "with no player ship, player_speed fails closed to 0.0 (not the AI's 50)"
        );
    }
}
