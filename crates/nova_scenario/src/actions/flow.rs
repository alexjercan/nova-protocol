//! Scenario-flow actions: how a scenario ends and what comes next.

use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

/// Which way a scenario ended. The variant picks the overlay's banner and
/// styling (gold VICTORY / red DEFEAT); everything else about the ending
/// stays the author's composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScenarioOutcomeKind {
    /// The player won: the overlay shows the gold VICTORY banner.
    Victory,
    /// The player lost: the overlay shows the red DEFEAT banner.
    Defeat,
}

/// Declare the scenario's outcome: show the win/lose overlay. Presentation
/// only - what happens NEXT stays composed from the existing vocabulary: pair
/// with `NextScenario(linger: true)` so [Enter] continues (Victory) or
/// retries (Defeat); with nothing queued, [Enter] returns to the main menu.
/// In strict RON the optional message is written with its variant:
/// `Outcome((outcome: Defeat, message: Some("...")))`, never a bare string.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutcomeActionConfig {
    /// Victory or Defeat.
    pub outcome: ScenarioOutcomeKind,
    /// Optional flavor line under the banner.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub message: Option<String>,
    /// Timed overlay: after this many REAL seconds (the overlay pauses virtual
    /// time) the banner advances the queued LINGERING chain exactly as if
    /// Continue were pressed. Strict RON: `auto_advance_secs: Some(6.0)`.
    /// Absent = wait for the player; meaningless without a queued lingering
    /// NextScenario.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub auto_advance_secs: Option<f64>,
}

impl OutcomeActionConfig {
    /// Construct with a message.
    pub fn new(outcome: ScenarioOutcomeKind, message: &str) -> Self {
        Self {
            outcome,
            message: Some(message.to_string()),
            auto_advance_secs: None,
        }
    }
}

/// The currently-declared scenario outcome, `None` while a scenario is in
/// play. Written by [`OutcomeActionConfig`], cleared by scenario teardown
/// (both the load and unload paths), read by the outcome overlay in
/// `nova_menu` and by the Enter handler's return-to-menu fallback.
#[derive(Resource, Debug, Default, Clone, PartialEq, Reflect)]
pub struct CurrentOutcome(pub Option<OutcomeActionConfig>);

impl EventAction<NovaEventWorld> for OutcomeActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let outcome = self.clone();
        debug!("Outcome: declaring {:?}", outcome.outcome);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // get_resource_mut, not resource_mut: headless rigs that
                // exercise scenario scripts without the loader plugin have no
                // outcome resource, and the action must not panic there.
                let Some(mut current) = world.get_resource_mut::<CurrentOutcome>() else {
                    warn!("Outcome: no CurrentOutcome resource (scenario loader not loaded)");
                    return;
                };
                current.0 = Some(outcome);
            });
        });
    }
}

/// Runtime cap on the delayed cut (panic-proofing Timer construction);
/// content_lint warns above [`NEXT_SCENARIO_DELAY_WARN_SECS`] already.
pub const NEXT_SCENARIO_DELAY_MAX_SECS: f32 = 300.0;
/// The authored range content_lint considers sane for a delayed cut.
pub const NEXT_SCENARIO_DELAY_WARN_SECS: f32 = 60.0;
/// Runtime cap on the timed banner (same panic-proofing).
pub const OUTCOME_AUTO_ADVANCE_MAX_SECS: f64 = 300.0;

/// Action that queues a switch to another scenario.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NextScenarioActionConfig {
    /// The id of the scenario to switch to.
    #[reflect(@Names::Scenario)]
    pub scenario_id: String,
    /// When true, defer the switch until something releases it (the
    /// scenario-advance input or the outcome overlay's Continue/Retry).
    pub linger: bool,
    /// Delayed non-lingering switch: with `linger: false`, hold the cut for
    /// this many seconds while the world keeps playing - the middle gear
    /// between the hard cut and the modal overlay. Strict RON: `delay:
    /// Some(4.0)`. Ticks on virtual (pause-frozen) time; non-positive or absent
    /// = instant. Meaningless with `linger: true` (the overlay's Continue is
    /// the release; see `OutcomeActionConfig::auto_advance_secs` for a timed
    /// overlay).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub delay: Option<f32>,
}

impl EventAction<NovaEventWorld> for NextScenarioActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!(
            "NextScenario: queuing scenario '{}' (linger: {}, delay: {:?})",
            self.scenario_id, self.linger, self.delay
        );
        world.next_scenario = Some(self.clone());
        // Arm the delayed cut only for the non-lingering shape; a fresh queue
        // always resets the clock (last request wins wholesale). Finite-check
        // and cap before Timer::from_seconds - an authored 1e30 parses fine and
        // would PANIC Duration::from_secs_f32; content_lint warns outside (0,
        // 60].
        world.next_scenario_delay = match self.delay {
            Some(delay) if !self.linger && delay > 0.0 && delay.is_finite() => Some(
                Timer::from_seconds(delay.min(NEXT_SCENARIO_DELAY_MAX_SECS), TimerMode::Once),
            ),
            _ => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::GameObjectives;

    use super::*;

    /// The transition-pacing fields parse in their documented strict-RON shapes
    /// and default when omitted.
    #[cfg(feature = "serde")]
    #[test]
    fn transition_pacing_ron_parses_and_defaults() {
        let delayed = r#"NextScenario((scenario_id: "x", linger: false, delay: Some(4.0)))"#;
        let parsed: EventActionConfig = ron::from_str(delayed).expect("delay syntax parses");
        let EventActionConfig::NextScenario(next) = &parsed else {
            panic!("NextScenario variant");
        };
        assert_eq!(next.delay, Some(4.0));

        let plain = r#"NextScenario((scenario_id: "x", linger: true))"#;
        let parsed: EventActionConfig = ron::from_str(plain).expect("omitted delay parses");
        let EventActionConfig::NextScenario(next) = &parsed else {
            panic!("NextScenario variant");
        };
        assert_eq!(next.delay, None);

        let timed = r#"Outcome((outcome: Victory, auto_advance_secs: Some(6.0)))"#;
        let parsed: EventActionConfig = ron::from_str(timed).expect("auto_advance parses");
        let EventActionConfig::Outcome(outcome) = &parsed else {
            panic!("Outcome variant");
        };
        assert_eq!(outcome.auto_advance_secs, Some(6.0));
        assert_eq!(outcome.message, None);
    }

    /// The Outcome action's EFFECT through the production drain: the action
    /// queues a command on the event world, and the state sync applies it to
    /// the `CurrentOutcome` resource - fire -> drain -> assert on the world,
    /// not just the config struct.
    #[test]
    fn outcome_action_sets_current_outcome_through_the_drain() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<CurrentOutcome>();
        // state_to_world_system mirrors objectives unconditionally.
        app.init_resource::<GameObjectives>();

        let action = EventActionConfig::Outcome(OutcomeActionConfig::new(
            ScenarioOutcomeKind::Victory,
            "The belt is quiet again.",
        ));
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            action.action(&mut world, &GameEventInfo { data: None });
        }
        // Nothing lands before the drain: the action only queues.
        assert_eq!(app.world().resource::<CurrentOutcome>().0, None);

        NovaEventWorld::state_to_world_system(app.world_mut());

        let current = app.world().resource::<CurrentOutcome>();
        assert_eq!(
            current.0.as_ref().map(|outcome| outcome.outcome),
            Some(ScenarioOutcomeKind::Victory),
            "the drained command writes the declared outcome"
        );
        assert_eq!(
            current
                .0
                .as_ref()
                .and_then(|outcome| outcome.message.as_deref()),
            Some("The belt is quiet again."),
        );
    }

    /// Graceful degradation: a headless rig without the loader's
    /// `CurrentOutcome` resource must not panic when a script declares an
    /// outcome - the command drops.
    #[test]
    fn outcome_action_without_the_resource_does_not_panic_or_conjure_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        // Deliberately NO CurrentOutcome resource.

        let action = EventActionConfig::Outcome(OutcomeActionConfig {
            outcome: ScenarioOutcomeKind::Defeat,
            message: None,
            auto_advance_secs: None,
        });
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            action.action(&mut world, &GameEventInfo { data: None });
        }
        NovaEventWorld::state_to_world_system(app.world_mut());
        assert!(
            app.world().get_resource::<CurrentOutcome>().is_none(),
            "the action must not conjure the resource it warns about"
        );
    }
}
