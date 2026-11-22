//! The role goal's session resources: the saved facts, [`GoalRecord`], and
//! what the Map and the Jobs pane show this frame, [`GoalProgress`]; the
//! hand-in rule ([`plan_goal_hand_in`]) and the two events a step raises.
//!
//! A start role's goal is content (`RoleGoal` in `nova_scenario`): a finite
//! chain of steps, each a predicate over facts the world save carries (the
//! jobs the player handed in, the stations the player owns and the trading
//! hulls the player took out, the one fact the record itself counts) and a
//! reward. The open world crate derives the progress every frame and raises
//! [`GoalStepDone`] when the current step turns met; the Jobs pane hands the
//! step in from anywhere, pays its reward and stamps the record. The step
//! after the last paid one is the current step; with every step paid the
//! goal is met. The Map's chart view, the Jobs pane and the save header read
//! both resources. They live here, below the world crates, for the same
//! reason the chart and the jobs do: the interface cannot depend on the
//! world crates. A world save carries the record beside the jobs, a Load
//! puts it back on the arming frame, and both clear with the frozen ledger.

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::goal::prelude::*`.
pub mod prelude {
    pub use super::{
        plan_goal_hand_in, GoalHandInRefusalType, GoalLine, GoalProgress, GoalRecord, GoalStep,
        GoalStepDone, GoalStepHandedIn,
    };
}

/// The saved facts of the role goal: the play-time stamp of each paid step,
/// and the trading hulls the player took out.
///
/// The Jobs pane pushes one stamp per step hand-in, with the world clock's
/// reading, and nothing takes one back while the world stands: a hand-in
/// cannot be undone, and a station sold later does not unmake a paid step.
/// A save whose stamps outnumber the role's steps, run backwards, or stand
/// ahead of its clock is refused. The record is saved and shown only: the
/// world does not react to a met goal.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct GoalRecord {
    /// Play seconds on the world clock when each step was handed in, in
    /// step order: its length is the number of steps paid.
    pub paid_at: Vec<f64>,
    /// Trading hulls the player neutralized, of any civilization on any
    /// stance: the count an outlaw step reads. Counted for every role, so
    /// the record means the same whichever goal reads it.
    pub hulls_taken: u32,
}

impl GoalRecord {
    /// The index of the current step: the first one not paid.
    pub fn current(&self) -> usize {
        self.paid_at.len()
    }

    /// The play time the goal of `steps` steps was met at: the last step's
    /// stamp once every step is paid; `None` while one stands, and for a
    /// goal of no step.
    pub fn met_at(&self, steps: usize) -> Option<f64> {
        (steps > 0 && self.paid_at.len() >= steps)
            .then(|| self.paid_at.last().copied())
            .flatten()
    }

    /// Forget the stamps and the count: the world on hand is gone.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether a saved record is one a session could have written for a
    /// goal of `steps` steps: at most `steps` stamps, each finite, at least
    /// zero, at least the one before it and at most `clock_seconds`, the
    /// saved clock.
    ///
    /// # Errors
    ///
    /// The count or the stamp that is not, named.
    pub fn validate(&self, steps: usize, clock_seconds: f64) -> Result<(), String> {
        if self.paid_at.len() > steps {
            return Err(format!(
                "the goal record pays {} steps; the role's goal has {steps}",
                self.paid_at.len()
            ));
        }
        let mut before = 0.0;
        for (index, at) in self.paid_at.iter().copied().enumerate() {
            if !(at.is_finite() && at >= before && at <= clock_seconds) {
                return Err(format!(
                    "goal step {} is stamped paid at {at} s of play, outside {before}..={clock_seconds}",
                    index + 1
                ));
            }
            before = at;
        }
        Ok(())
    }
}

/// What the Map and the Jobs pane show for the role goal this frame: its
/// heading and each step's `have / need` line with its reward. Derived from
/// saved facts every frame, never saved. Empty off the open world.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GoalProgress {
    /// The goal's heading, as the role authored it.
    pub title: String,
    /// Each step in chain order, with what the save shows.
    pub steps: Vec<GoalStep>,
}

impl GoalProgress {
    /// Whether there is no goal to show.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// One step of a goal as the save shows it: its line and its reward.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalStep {
    /// What the step counts.
    pub line: GoalLine,
    /// What its hand-in pays.
    pub reward_cr: u32,
}

/// One count of a goal: what the save shows against what the goal asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalLine {
    /// What is counted, as the Map prints it.
    pub label: String,
    /// What the save shows.
    pub have: u64,
    /// What the goal asks for.
    pub need: u64,
}

impl GoalLine {
    /// A line labelled `label` reading `have / need`.
    pub fn new(label: &str, have: u64, need: u64) -> Self {
        Self {
            label: label.to_string(),
            have,
            need,
        }
    }

    /// Whether `have` reaches `need`.
    pub fn is_met(&self) -> bool {
        self.have >= self.need
    }
}

/// Why a goal step's hand-in is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalHandInRefusalType {
    /// The session has no goal, or the step is past its chain.
    NoGoal,
    /// The step is paid already, or an earlier step is not.
    NotCurrent,
    /// The step's line is short of its need.
    NotMet,
}

/// The reward the hand-in of `step` pays, if `progress` and `record` allow
/// it: the step is in the chain, it is the current one and its line is met.
///
/// # Errors
///
/// The [`GoalHandInRefusalType`] that blocks it.
pub fn plan_goal_hand_in(
    progress: &GoalProgress,
    record: &GoalRecord,
    step: usize,
) -> Result<u32, GoalHandInRefusalType> {
    let Some(goal_step) = progress.steps.get(step) else {
        return Err(GoalHandInRefusalType::NoGoal);
    };
    if step != record.current() {
        return Err(GoalHandInRefusalType::NotCurrent);
    }
    if !goal_step.line.is_met() {
        return Err(GoalHandInRefusalType::NotMet);
    }
    Ok(goal_step.reward_cr)
}

/// Raised by the open world the frame the current goal step reads met: its
/// line turned met, or it became current already met. Never on a Load, a
/// New Game or the first frame a goal is read.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoalStepDone {
    /// The step's index in the chain.
    pub step: usize,
    /// What its hand-in pays.
    pub reward_cr: u32,
}

/// Raised by the Jobs pane once a goal step's hand-in is paid and stamped.
/// Only the pane raises it: a Load restores paid steps without it.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoalStepHandedIn {
    /// The step's index in the chain.
    pub step: usize,
    /// What its hand-in paid.
    pub reward_cr: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(paid_at: &[f64]) -> GoalRecord {
        GoalRecord {
            paid_at: paid_at.to_vec(),
            hulls_taken: 0,
        }
    }

    /// A two-step kill list: three bounties for 3,000 cr, then one station
    /// for 10,000 cr.
    fn kill_list(bounties: u64, stations: u64) -> GoalProgress {
        GoalProgress {
            title: "A kill list".to_string(),
            steps: vec![
                GoalStep {
                    line: GoalLine::new("Bounties handed in", bounties, 3),
                    reward_cr: 3_000,
                },
                GoalStep {
                    line: GoalLine::new("Stations owned", stations, 1),
                    reward_cr: 10_000,
                },
            ],
        }
    }

    #[test]
    fn more_paid_steps_than_the_role_has_refuse_the_save() {
        assert!(record(&[]).validate(2, 0.0).is_ok());
        assert!(record(&[5.0, 9.0]).validate(2, 9.0).is_ok());
        let error = record(&[5.0, 6.0, 7.0])
            .validate(2, 9.0)
            .expect_err("three stamps on a two-step goal");
        assert!(error.contains("pays 3 steps"), "{error}");
    }

    /// A stamp ahead of the clock, below zero, not finite or earlier than
    /// the step before it is refused; stamps in order within the clock pass.
    #[test]
    fn an_out_of_order_or_ahead_of_clock_stamp_refuses_the_save() {
        assert!(record(&[5.0, 5.0]).validate(2, 5.0).is_ok());
        for at in [5.5, -1.0, f64::NAN, f64::INFINITY] {
            assert!(record(&[at]).validate(2, 5.0).is_err(), "{at}");
        }
        let error = record(&[4.0, 3.0])
            .validate(2, 5.0)
            .expect_err("step 2 paid before step 1");
        assert!(error.contains("goal step 2"), "{error}");
    }

    /// The goal is met at the last step's stamp once every step is paid,
    /// and a goal of no step is never met.
    #[test]
    fn the_goal_is_met_at_the_last_paid_steps_stamp() {
        assert_eq!(record(&[5.0]).met_at(2), None);
        assert_eq!(record(&[5.0, 9.0]).met_at(2), Some(9.0));
        assert_eq!(record(&[]).met_at(0), None);
    }

    #[test]
    fn a_met_current_step_pays_its_reward() {
        assert_eq!(
            plan_goal_hand_in(&kill_list(3, 0), &record(&[]), 0),
            Ok(3_000)
        );
        assert_eq!(
            plan_goal_hand_in(&kill_list(3, 1), &record(&[5.0]), 1),
            Ok(10_000)
        );
    }

    /// A step short of its need, a step paid already, a step after the
    /// current one and a step past the chain are each refused.
    #[test]
    fn a_step_that_is_not_met_or_not_current_is_refused() {
        assert_eq!(
            plan_goal_hand_in(&kill_list(2, 0), &record(&[]), 0),
            Err(GoalHandInRefusalType::NotMet)
        );
        assert_eq!(
            plan_goal_hand_in(&kill_list(3, 1), &record(&[5.0]), 0),
            Err(GoalHandInRefusalType::NotCurrent)
        );
        assert_eq!(
            plan_goal_hand_in(&kill_list(3, 1), &record(&[]), 1),
            Err(GoalHandInRefusalType::NotCurrent)
        );
        assert_eq!(
            plan_goal_hand_in(&kill_list(3, 1), &record(&[5.0, 6.0]), 2),
            Err(GoalHandInRefusalType::NoGoal)
        );
        assert_eq!(
            plan_goal_hand_in(&GoalProgress::default(), &record(&[]), 0),
            Err(GoalHandInRefusalType::NoGoal)
        );
    }
}
