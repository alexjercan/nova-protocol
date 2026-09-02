//! The run's cheat mark: whether the player deliberately armed the command
//! shell's cheats, and whether this attempt therefore carries the mark.
//!
//! Two different subjects share the word "cheat" and must not share a flag:
//!
//! - THIS one is about the PLAYER. Running `cheats enable` from the command
//!   shell or the process channel marks the current attempt, one way, and no
//!   command puts the mark back.
//! - Scenario CONTENT that injects world state is a different question,
//!   answered by the content linter's creative-map classification. Authored
//!   content never accuses the player of anything.
//!
//! A fresh scenario is a fresh run, so loading one clears both the arming and
//! the mark.

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::cheats::prelude::*`.
pub mod prelude {
    pub use super::RunCheats;
}

/// Whether cheats are armed for this run, and whether the run is marked.
///
/// The two are not the same flag on purpose. Arming can only ever be true after
/// it was set, and the mark outlives nothing but the run itself - so a player
/// who arms cheats, runs none, and plays on still carries the mark, which is
/// what makes the mark honest.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct RunCheats {
    armed: bool,
    marked: bool,
}

impl RunCheats {
    /// Whether a cheat command may run right now.
    pub fn is_armed(self) -> bool {
        self.armed
    }

    /// Whether this run carries the cheat mark.
    pub fn is_marked(self) -> bool {
        self.marked
    }

    /// Arm cheats and mark the run. Irreversible within the run: there is no
    /// disarm, because a run that was ever armed was never clean.
    ///
    /// Returns whether this call is what armed it, so the shell can tell "armed
    /// now" from "already armed".
    pub fn arm(&mut self) -> bool {
        let newly = !self.armed;
        self.armed = true;
        self.marked = true;
        newly
    }

    /// A fresh scenario is a fresh attempt: clear the arming and the mark.
    pub fn begin_new_run(&mut self) {
        *self = Self::default();
    }

    /// The two-word state the shell's banner and `cheats status` print.
    pub fn banner(self) -> &'static str {
        if self.armed {
            "enabled / run marked"
        } else {
            "disabled / run clean"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arming is one way inside a run, and only a fresh run clears it - a
    /// player cannot arm a cheat, use it, and hand back a clean attempt.
    #[test]
    fn arming_marks_the_run_irreversibly_until_a_fresh_one() {
        let mut cheats = RunCheats::default();
        assert!(!cheats.is_armed() && !cheats.is_marked());
        assert_eq!(cheats.banner(), "disabled / run clean");

        assert!(cheats.arm(), "the first arm is the one that arms");
        assert!(!cheats.arm(), "arming again changes nothing");
        assert!(cheats.is_armed() && cheats.is_marked());
        assert_eq!(cheats.banner(), "enabled / run marked");

        cheats.begin_new_run();
        assert!(!cheats.is_armed() && !cheats.is_marked());
    }
}
