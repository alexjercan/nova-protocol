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
/// that introduces it, so the exhaustive rule is: nothing that runs in one
/// frame posts both a StoryMessage and an Objective. This is the regression
/// pin for the whole "objectives-appear-during-conversations" complaint across
/// the mainline.
#[test]
fn no_mainline_handler_posts_an_objective_alongside_a_conversation() {
    for (name, config) in mainline_scenarios() {
        for (idx, event) in config.events.iter().enumerate() {
            for group in event.action_groups() {
                let has_story = group
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::StoryMessage(_)));
                let has_objective = group
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::Objective(_)));
                assert!(
                    !(has_story && has_objective),
                    "{name}: handler #{idx} ({:?}) posts an objective in the same \
                         frame as a comms line - give the objective a beat of its \
                         own (pacing::beat_later)",
                    event.name,
                );
            }
        }
    }
}

/// Corollary of the pacing pass: the objectives panel stays EMPTY through
/// each scenario's opening - no objective is posted in the OnStart FRAME. The
/// first objective posts only after the opening dispatch/conversation, on a
/// beat of the chain OnStart starts.
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
        // ...and there IS a deferred objective post - a beat of a chain, not
        // the frame that started it - so the deferral is actually wired and the
        // scenario is not simply objective-less.
        let deferred_posts = config
            .events
            .iter()
            // `skip(1)` drops the handler's OWN frame; what is left is the
            // chain beats it queued.
            .flat_map(|event| event.action_groups().into_iter().skip(1))
            .filter(|group| {
                group
                    .iter()
                    .any(|a| matches!(a, EventActionConfig::Objective(_)))
            })
            .count();
        assert!(
            deferred_posts > 0,
            "{name}: no deferred objective post - the opening objective was \
                 dropped, not deferred"
        );
    }
}

/// Behaviour (b) of the pacing pass, pinned structurally: the FIRST
/// objective of each scenario (the one that follows the opening dispatch or
/// conversation) is deferred past frame one - it posts on a BEAT of the
/// opening chain, which is deferred either because the beat owes a delay or a
/// gate, or because beats ahead of it do. This is scoped to the OPENING
/// objective posts, not mid-beat progress updates (the crate tally re-posts on
/// a gameplay counter, which is a different, legitimately state-gated animal).
/// The end-to-end timing itself is proven in shakedown's walk tests.
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
        for id in ids {
            let mut posts = 0_usize;
            let mut deferred = 0_usize;
            for action in config.events.iter().flat_map(|event| event.actions.iter()) {
                action.walk(&mut |action| {
                    let EventActionConfig::Sequence(chain) = action else {
                        return;
                    };
                    for (index, step) in chain.steps.iter().enumerate() {
                        if !step
                            .actions
                            .iter()
                            .any(|a| matches!(a, EventActionConfig::Objective(o) if o.id == *id))
                        {
                            continue;
                        }
                        posts += 1;
                        // A beat is past frame one when it owes a wait of its
                        // own, or when beats ahead of it owed one.
                        if index > 0
                            || step.until.is_some()
                            || step.after.is_some_and(|after| after > 0.0)
                        {
                            deferred += 1;
                        }
                    }
                });
            }
            assert!(
                posts > 0,
                "{name}: the opening objective '{id}' is not posted by any beat \
                     chain - a same-frame post would appear during the \
                     dispatch/conversation"
            );
            assert_eq!(
                posts, deferred,
                "{name}: the opening objective '{id}' posts on a beat that owes \
                     no wait - it would appear on frame one, during the \
                     dispatch/conversation"
            );
        }
    }
}

/// Regression pin: no OnStart handler may READ the engine clock
/// `scenario_elapsed` in its own frame. It is undefined at OnStart (the first
/// `tick_scenario_clock` has not run) and the content evaluator errors on an
/// undefined read, so a stamp written there silently fails and whatever waits
/// on it never fires. An opening beat is spelled as a `Sequence` step, whose
/// delay the ENGINE holds - it never reads the clock at all.
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
