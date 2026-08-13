//! Cross-chapter structural pacing tests.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::*;

/// The mainline campaign, built the way the loader builds it (texture
/// refs do not affect the event script, so defaults suffice).
fn mainline_scenarios() -> Vec<(&'static str, ScenarioConfig)> {
    let tex = || AssetRef::<Image>::default();
    vec![
        ("shakedown_run", shakedown::shakedown_run(tex(), tex())),
        ("broadside", broadside::broadside(tex(), tex())),
        (
            "broadside_gunship",
            broadside::broadside_gunship(tex(), tex()),
        ),
        ("lifeline", lifeline::lifeline(tex(), tex())),
        ("final_tally", final_tally::final_tally(tex(), tex())),
    ]
}

fn has_action(event: &ScenarioEventConfig, pred: impl Fn(&EventActionConfig) -> bool) -> bool {
    event.actions.iter().any(pred)
}

/// Owner pacing pass: an objective must never appear in the same frame as a
/// conversation line. Every objective posts a beat AFTER the story line
/// that introduces it (via the shared `pacing` deadline), so the exhaustive
/// rule is: no single handler posts both a StoryMessage and an Objective.
/// This is the regression pin for the whole
/// "objectives-appear-during-conversations" complaint across the mainline.
#[test]
fn no_mainline_handler_posts_an_objective_alongside_a_conversation() {
    for (name, config) in mainline_scenarios() {
        for (idx, event) in config.events.iter().enumerate() {
            let has_story = has_action(event, |a| matches!(a, EventActionConfig::StoryMessage(_)));
            let has_objective = has_action(event, |a| matches!(a, EventActionConfig::Objective(_)));
            assert!(
                !(has_story && has_objective),
                "{name}: handler #{idx} ({:?}) posts an objective in the same \
                     frame as a comms line - defer the objective a beat past the \
                     conversation (pacing::gated_once)",
                event.name,
            );
        }
    }
}

/// Corollary of the pacing pass: the objectives panel stays EMPTY through
/// each scenario's opening - no objective is posted at OnStart. The first
/// objective posts only after the opening dispatch/conversation, on a
/// clock-gated OnUpdate handler.
#[test]
fn no_mainline_scenario_posts_an_objective_at_onstart() {
    for (name, config) in mainline_scenarios() {
        for event in config
            .events
            .iter()
            .filter(|e| matches!(e.name, EventConfig::OnStart))
        {
            assert!(
                !has_action(event, |a| matches!(a, EventActionConfig::Objective(_))),
                "{name}: OnStart posts an objective - the opening panel must \
                     stay empty until the dispatch line hands off"
            );
        }
        // ...and there IS a deferred objective post (an OnUpdate handler
        // that posts an objective), so the deferral is actually wired and
        // the scenario is not simply objective-less.
        assert!(
            config
                .events
                .iter()
                .any(|e| matches!(e.name, EventConfig::OnUpdate)
                    && has_action(e, |a| matches!(a, EventActionConfig::Objective(_)))),
            "{name}: no deferred objective post - the opening objective was \
                 dropped, not deferred"
        );
    }
}

/// Behaviour (b) of the pacing pass, pinned structurally: the FIRST
/// objective of each scenario (the one that follows the opening dispatch or
/// conversation) is deferred past frame one - either by a
/// `scenario_elapsed > deadline` clock gate (the shared `pacing::clock_past`,
/// used by every story-scenario opening and objective swap) or by the
/// opening-conversation step latch (`open_step`, shakedown's clock-paced
/// five-line hand-off). This is scoped to the OPENING objective posts, not
/// mid-beat progress updates (the crate tally re-posts on a gameplay
/// counter, which is a different, legitimately state-gated animal). The
/// end-to-end timing itself is proven in shakedown's walk tests.
#[test]
fn opening_objectives_are_deferred_past_frame_one() {
    // The opening objective ids: the first goal each scenario posts after
    // its dispatch/conversation. Kept explicit so the test pins the exact
    // beat the owner playtest flagged, not whatever else posts objectives.
    let opening_objectives: &[(&str, &[&str])] = &[
        ("shakedown_run", &["b1_burn"]),
        ("broadside", &["contact"]),
        ("broadside_gunship", &["screen", "break"]),
        ("lifeline", &["screen_convoy"]),
        ("final_tally", &["survey"]),
    ];
    for (name, config) in mainline_scenarios() {
        let ids = opening_objectives
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, ids)| *ids)
            .unwrap_or(&[]);
        for event in config.events.iter() {
            if !matches!(event.name, EventConfig::OnUpdate) {
                continue;
            }
            let posts_opening = event.actions.iter().any(
                |a| matches!(a, EventActionConfig::Objective(o) if ids.contains(&o.id.as_str())),
            );
            if !posts_opening {
                continue;
            }
            let deferred = event.filters.iter().any(|f| {
                let rendered = format!("{f:?}");
                (rendered.contains("GreaterThan") && rendered.contains("scenario_elapsed"))
                    || rendered.contains("open_step")
            });
            assert!(
                deferred,
                "{name}: the opening objective posts with no clock gate and \
                     no conversation latch - it would appear on frame one, during \
                     the dispatch/conversation"
            );
        }
    }
}

/// Regression pin: no OnStart handler may READ the engine clock
/// `scenario_elapsed`. It is undefined at OnStart (the first
/// `tick_scenario_clock` has not run) and the content evaluator errors on
/// an undefined read, so a `mark_clock`/gate stamp at OnStart silently
/// fails and the opening objective never posts. OnStart gates must use an
/// ABSOLUTE deadline (`pacing::open_gate`), not a
/// `scenario_elapsed`-relative one.
#[test]
fn no_onstart_handler_reads_the_scenario_clock() {
    for (name, config) in mainline_scenarios() {
        for event in config
            .events
            .iter()
            .filter(|e| matches!(e.name, EventConfig::OnStart))
        {
            for action in &event.actions {
                if let EventActionConfig::VariableSet(set) = action {
                    let rendered = format!("{:?}", set.expression);
                    assert!(
                        !rendered.contains("scenario_elapsed"),
                        "{name}: OnStart sets '{}' from scenario_elapsed, which \
                             is undefined at OnStart - use pacing::open_gate for an \
                             absolute deadline",
                        set.key,
                    );
                }
            }
        }
    }
}
