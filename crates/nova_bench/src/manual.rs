//! What the agent is told: the pilot manual (the system prompt), the
//! opening message and the one nudge. The manual is `manual.md` beside this
//! file, so a change to the wording is a doc edit.

use serde_json::Value;

/// The system prompt.
pub const MANUAL: &str = include_str!("manual.md");

/// The goal when `--goal` is not given.
pub const DEFAULT_GOAL: &str = "Complete the scenario's objectives and reach the Victory outcome. \
Take as little damage as you can and do not waste ammunition.";

/// What pi is told when it settles with the run still open.
pub const NUDGE: &str = "The run is not over: the referee has not declared an outcome and you \
have budget left. Keep playing with `act`, or call `finish` if you have reached the goal or \
cannot make progress.";

/// The first user message: the scenario, the goal, the first view.
pub fn opening(scenario: &str, goal: &str, first: &Value) -> String {
    format!(
        "Scenario: {scenario}\nGoal: {goal}\n\nYour first observation:\n```json\n{first:#}\n```\n\n\
Begin. Use only the observe, act and finish tools. Call finish when the goal is reached \
or when you cannot make progress."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manual_documents_every_tool_and_the_opening_names_the_goal() {
        for word in [
            "observe",
            "act",
            "finish",
            "bearing_deg",
            "flight.main_drive",
            "targeting.radar_hold",
        ] {
            assert!(MANUAL.contains(word), "manual lacks {word}");
        }
        let text = opening("tutorial", "Win.", &serde_json::json!({ "tick": 1 }));
        assert!(text.contains("Scenario: tutorial"));
        assert!(text.contains("Goal: Win."));
        assert!(text.contains("\"tick\": 1"));
    }
}
