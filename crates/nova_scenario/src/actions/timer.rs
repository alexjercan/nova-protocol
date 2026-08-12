//! Scenario timer actions: start or cancel keyed, pause-frozen delays.

use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

/// Start or restart a keyed scenario timer.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimerStartActionConfig {
    /// Scenario-local timer key.
    pub key: String,
    /// Positive finite duration in seconds.
    pub seconds: VariableExpressionNode,
}

impl EventAction<NovaEventWorld> for TimerStartActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        if self.key.trim().is_empty() {
            error!("TimerStartActionConfig: timer key must not be empty");
            return;
        }
        match self.seconds.evaluate(world) {
            Ok(VariableLiteral::Number(seconds)) if seconds.is_finite() && seconds > 0.0 => {
                if !world.start_timer(self.key.clone(), seconds) {
                    error!(
                        "TimerStartActionConfig: timer '{}' duration overflows the scenario clock",
                        self.key
                    );
                }
            }
            Ok(value) => {
                error!(
                    "TimerStartActionConfig: timer '{}' duration must be a positive finite number, got {:?}",
                    self.key, value
                );
            }
            Err(error) => {
                error!(
                    "TimerStartActionConfig: failed to evaluate duration for timer '{}': {:?}",
                    self.key, error
                );
            }
        }
    }
}

/// Cancel a keyed scenario timer if it is running.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimerCancelActionConfig {
    /// Scenario-local timer key.
    pub key: String,
}

impl EventAction<NovaEventWorld> for TimerCancelActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.cancel_timer(&self.key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(value)),
        ))
    }

    #[test]
    fn start_restarts_and_cancel_is_idempotent() {
        let mut world = NovaEventWorld::default();
        world.insert_variable(
            crate::loader::SCENARIO_ELAPSED_VAR.to_string(),
            VariableLiteral::Number(10.0),
        );
        let start = TimerStartActionConfig {
            key: "beat".to_string(),
            seconds: number(2.0),
        };
        start.action(&mut world, &default());
        assert!(world.timer_is_running("beat"));

        world.insert_variable(
            crate::loader::SCENARIO_ELAPSED_VAR.to_string(),
            VariableLiteral::Number(11.0),
        );
        start.action(&mut world, &default());
        assert!(
            world.drain_ended_timers(12.0).is_empty(),
            "restart moved the deadline"
        );
        assert_eq!(world.drain_ended_timers(13.0), ["beat"]);

        TimerCancelActionConfig {
            key: "beat".to_string(),
        }
        .action(&mut world, &default());
        assert!(!world.timer_is_running("beat"));

        start.action(&mut world, &default());
        world.clear();
        assert!(!world.timer_is_running("beat"), "teardown clears timers");
    }

    #[test]
    fn timer_configs_round_trip_through_ron() {
        let action = EventActionConfig::TimerStart(TimerStartActionConfig {
            key: "briefing".to_string(),
            seconds: number(2.5),
        });
        let ron = ron::ser::to_string(&action).expect("serialize timer action");
        let parsed: EventActionConfig = ron::de::from_str(&ron).expect("parse timer action");
        assert!(
            matches!(parsed, EventActionConfig::TimerStart(config) if config.key == "briefing")
        );

        let filter = EventFilterConfig::Timer(TimerFilterConfig {
            key: "briefing".to_string(),
        });
        let ron = ron::ser::to_string(&filter).expect("serialize timer filter");
        let parsed: EventFilterConfig = ron::de::from_str(&ron).expect("parse timer filter");
        assert!(matches!(parsed, EventFilterConfig::Timer(config) if config.key == "briefing"));
    }

    #[test]
    fn invalid_duration_does_not_replace_a_running_timer() {
        let mut world = NovaEventWorld::default();
        assert!(world.start_timer("beat".to_string(), 5.0));
        TimerStartActionConfig {
            key: "beat".to_string(),
            seconds: number(0.0),
        }
        .action(&mut world, &default());
        assert!(world.timer_is_running("beat"));
        assert_eq!(world.drain_ended_timers(5.0), ["beat"]);
    }
}
