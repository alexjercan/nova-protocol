//! What the agent is told: the pilot manual (the system prompt), the flight
//! manual's pages, the opening message and the one nudge. Every text is a
//! `.md` file beside this one, so a change to the wording is a doc edit.
//!
//! The split is by WHO NEEDS THE KNOWLEDGE. The manual holds how to drive the
//! game blind - the tools, the act-and-tick loop, the gesture verbs, the
//! view's field names, and the control-response constants that stand in for a
//! screen. The pages hold what the game IS, which a player with a screen
//! learns too. The test for one line: would a player ever need this? Turret
//! range, yes - a page. 27 pixels of `camera_rotate` per degree, never - the
//! manual, because an agent commits to an input for 60 to 300 ticks and has
//! to know what it will do before it does it.

use serde_json::Value;

/// The system prompt.
pub const MANUAL: &str = include_str!("manual.md");

/// The tools the referee answers, in the order the manual introduces them.
///
/// One source of truth: the pi spawn's `--tools` allowlist is built from this,
/// and a test holds the TypeScript relay to it. Nothing links the relay to the
/// referee at compile time - it is resolved by path at run time - so a tool
/// registered on one side and forgotten on the other would simply be absent,
/// with no error anywhere.
pub const TOOLS: &[&str] = &["observe", "act", "page", "finish"];

/// The tools as the `--tools` allowlist spells them.
pub fn tool_list() -> String {
    TOOLS.join(",")
}

/// The flight manual, by page name. Read one with the `page` tool.
pub const PAGES: &[(&str, &str)] = &[
    ("targeting", include_str!("pages/targeting.md")),
    ("weapons", include_str!("pages/weapons.md")),
    ("travel", include_str!("pages/travel.md")),
    ("orbit", include_str!("pages/orbit.md")),
    ("fighting", include_str!("pages/fighting.md")),
];

/// One page by name, or `None` for a name that is not a page.
pub fn page(name: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(page, _)| *page == name)
        .map(|(_, text)| *text)
}

/// The page names, as the error that lists them prints them.
pub fn page_names() -> String {
    PAGES
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

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
Begin. Use only the {tools} tools. Call finish when the goal is reached or when you cannot \
make progress.",
        tools = TOOLS.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manual_documents_every_tool_and_the_opening_names_the_goal() {
        for word in TOOLS {
            assert!(MANUAL.contains(word), "manual lacks {word}");
        }
        for word in ["bearing_deg", "flight.main_drive", "targeting.radar_hold"] {
            assert!(MANUAL.contains(word), "manual lacks {word}");
        }
        let text = opening("tutorial", "Win.", &serde_json::json!({ "tick": 1 }));
        assert!(text.contains("Scenario: tutorial"));
        assert!(text.contains("Goal: Win."));
        assert!(text.contains("\"tick\": 1"));
    }

    /// A page the manual names but does not ship would be a dead end the
    /// agent finds only at run time.
    #[test]
    fn every_page_the_manual_names_is_a_page_that_exists() {
        for (name, text) in PAGES {
            assert!(MANUAL.contains(name), "the manual never names `{name}`");
            assert!(!text.trim().is_empty(), "`{name}` is empty");
            assert_eq!(page(name), Some(*text));
        }
        assert_eq!(page("warp-drive"), None);
        assert!(page_names().contains("targeting"));
    }

    /// The cap `parse_gesture` enforces, stated in the manual the agent is
    /// handed. Without it a driver meets the bound only as a refusal, having
    /// spent a turn to find it.
    #[test]
    fn the_manual_states_the_aim_tick_cap_the_parser_enforces() {
        let cap = crate::prelude::MAX_AIM_TICKS;
        assert!(
            MANUAL.contains(&format!("1 to {cap}")),
            "the manual does not state the {cap}-tick cap on `aim`"
        );
    }

    /// The relay is resolved by path at run time, so nothing else would catch
    /// a tool or a page that exists on one side of the socket and not the
    /// other. This is that check.
    #[test]
    fn the_pi_relay_registers_exactly_the_tools_the_referee_answers() {
        const RELAY: &str = include_str!("../../../tools/nova_bench/pi/index.ts");
        for tool in TOOLS {
            assert!(
                RELAY.contains(&format!("name: \"{tool}\"")),
                "the relay registers no `{tool}` tool"
            );
        }
        let registered = RELAY.matches("pi.registerTool(").count();
        assert_eq!(
            registered,
            TOOLS.len(),
            "the relay registers {registered} tools and the referee answers {}",
            TOOLS.len()
        );
        // The relay must not enumerate the pages: the referee's own error
        // lists them, and a hardcoded list here would hide a new page.
        for (name, _) in PAGES {
            assert!(
                !RELAY.contains(name),
                "the relay names the `{name}` page; let `page_names()` list them"
            );
        }
    }

    /// The echo rule, in the words the agent reads: no `refused` list to
    /// consult, and the effect fields named instead.
    #[test]
    fn the_discipline_sends_the_agent_to_the_world_not_to_a_verdict() {
        assert!(MANUAL.contains("An input has no verdict"));
        assert!(!MANUAL.contains("`refused`"));
        assert!(MANUAL.contains("me.autopilot.engaged"));
        assert!(MANUAL.contains("me.radar.dwell_fill"));
    }
}
