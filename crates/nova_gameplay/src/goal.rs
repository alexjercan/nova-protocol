//! The role goal's two session resources: the one saved fact, [`GoalRecord`],
//! and what the Map shows this frame, [`GoalProgress`].
//!
//! A start role's goal is content (`RoleGoal` in `nova_scenario`): a
//! predicate over facts the world save already carries: the jobs the player
//! handed in, the player's hull, owned stations and fleet, and the
//! civilization stances. The open world crate derives the
//! progress every frame and stamps the record once the predicate holds; the
//! Map's chart view reads both. They live here, below the world crates, for
//! the same reason the chart and the jobs do: the interface cannot depend on
//! the world crates. A world save carries the record beside the jobs, a Load
//! puts it back on the arming frame, and both clear with the frozen ledger.

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::goal::prelude::*`.
pub mod prelude {
    pub use super::{GoalLine, GoalProgress, GoalRecord};
}

/// The one saved fact of the role goal: standing, or met at a play-time
/// stamp.
///
/// Stamped once by the open world crate when the goal's progress is met,
/// with the world clock's reading, and never cleared back while the world
/// stands: a hand-in cannot be undone, and a hull sold later does not unmake
/// a flagship. A save that stamps it ahead of its clock is refused. The
/// record is saved and shown only: the world does not react to a met goal.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct GoalRecord {
    /// Play seconds on the world clock when the goal was met; `None` while
    /// it stands.
    pub met_at: Option<f64>,
}

impl GoalRecord {
    /// Whether the goal was met.
    pub fn is_met(&self) -> bool {
        self.met_at.is_some()
    }

    /// Forget the stamp: the world on hand is gone.
    pub fn clear(&mut self) {
        self.met_at = None;
    }

    /// Whether a saved record is one a session could have written: no stamp,
    /// or a stamp that is finite, at least zero and at most `clock_seconds`,
    /// the saved clock.
    ///
    /// # Errors
    ///
    /// The stamp that is not, named.
    pub fn validate(&self, clock_seconds: f64) -> Result<(), String> {
        match self.met_at {
            Some(at) if !(at.is_finite() && at >= 0.0 && at <= clock_seconds) => Err(format!(
                "the goal is stamped met at {at} s of play, outside 0..={clock_seconds}"
            )),
            _ => Ok(()),
        }
    }
}

/// What the Map shows for the role goal this frame: its heading and each
/// `have / need` line. Derived from saved facts every frame, never saved.
/// Empty for a role with no goal and off the open world.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GoalProgress {
    /// The goal's heading, as the role authored it.
    pub title: String,
    /// Each count the goal asks for, with what the save shows.
    pub lines: Vec<GoalLine>,
}

impl GoalProgress {
    /// Whether there is no goal to show.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Whether every line's `have` reaches its `need`, and there is at least
    /// one line: an empty progress is no goal, not a met one.
    pub fn is_met(&self) -> bool {
        !self.lines.is_empty() && self.lines.iter().all(|line| line.have >= line.need)
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stamp ahead of the clock, below zero or not finite is refused; a
    /// standing record and a stamp at or under the clock pass.
    #[test]
    fn a_record_validates_only_a_stamp_within_the_saved_clock() {
        assert!(GoalRecord::default().validate(0.0).is_ok());
        assert!(GoalRecord { met_at: Some(5.0) }.validate(5.0).is_ok());
        for at in [5.5, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                GoalRecord { met_at: Some(at) }.validate(5.0).is_err(),
                "{at}"
            );
        }
    }

    /// An empty progress is no goal; one short line keeps it standing; every
    /// line reached is met.
    #[test]
    fn a_progress_is_met_only_with_every_line_reached() {
        assert!(!GoalProgress::default().is_met());
        let mut progress = GoalProgress {
            title: "A kill list".to_string(),
            lines: vec![GoalLine::new("Hulls", 6, 6), GoalLine::new("Rings", 1, 2)],
        };
        assert!(!progress.is_met());
        progress.lines[1].have = 2;
        assert!(progress.is_met());
    }
}
