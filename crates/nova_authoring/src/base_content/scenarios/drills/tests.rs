//! Structural pins for the practice ranges.
//!
//! These enforce what makes a drill a drill - it ends, both ways; it hands
//! over only the verbs its lessons teach; its board carries one live line at a
//! time; every chip it lights goes out again - rather than a transcript of the
//! lines. A rewrite of what Range Control says should not touch an assertion
//! below.

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::base_content::assets::BaseContentAssets;

fn ranges() -> Vec<ScenarioConfig> {
    catalog(&BaseContentAssets::from_paths())
}

/// Every action in the scenario, flattened through every chain: a drill's
/// ending lives one sequence step down, so a shallow scan would miss it.
fn all_actions(config: &ScenarioConfig) -> Vec<EventActionConfig> {
    let mut actions = Vec::new();
    for action in config.events.iter().flat_map(|event| event.actions.iter()) {
        action.walk(&mut |action| actions.push(action.clone()));
    }
    actions
}

fn outcomes(config: &ScenarioConfig, kind: ScenarioOutcomeKind) -> usize {
    all_actions(config)
        .iter()
        .filter(|action| matches!(action, EventActionConfig::Outcome(outcome) if outcome.outcome == kind))
        .count()
}

/// The trainer's helm as the drill spawns it.
fn spawned_helm(config: &ScenarioConfig) -> ShipCapabilities {
    all_actions(config)
        .into_iter()
        .find_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) if object.base.id == ID_TRAINER => {
                match object.kind {
                    ScenarioObjectKind::Spaceship(ship) => Some(ship.capabilities),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("every drill spawns the trainer")
}

/// The verbs a helm withholds, by the name the handbook teaches them under.
fn withheld(capabilities: ShipCapabilities) -> BTreeSet<&'static str> {
    [
        ("STOP", capabilities.stop_enabled),
        ("RCS", capabilities.rcs_enabled),
        ("LOCK", capabilities.lock_enabled),
        ("GOTO", capabilities.goto_enabled),
        ("ORBIT", capabilities.orbit_enabled),
    ]
    .into_iter()
    .filter(|(_, enabled)| !enabled)
    .map(|(name, _)| name)
    .collect()
}

/// A drill ENDS. Flying the last beat raises the VICTORY banner, and losing
/// the trainer raises DEFEAT - twice over, because a hull can be shot to
/// pieces or merely left unable to fight, and a drill the player cannot fly
/// out of is over either way.
#[test]
fn every_drill_ends_both_ways() {
    for config in ranges() {
        assert_eq!(
            outcomes(&config, ScenarioOutcomeKind::Victory),
            1,
            "{} must end in exactly one VICTORY",
            config.id
        );
        assert_eq!(
            outcomes(&config, ScenarioOutcomeKind::Defeat),
            2,
            "{} must lose on both a destroyed and a neutralized trainer",
            config.id
        );
    }
}

/// The VICTORY queues nothing behind it: a drill is not a chapter, so the
/// overlay offers Main Menu alone and the player lands back at the handbook.
/// The DEFEAT queues the drill itself, which is what makes the overlay's
/// button a Retry rather than a trip through the menu.
#[test]
fn a_won_drill_queues_nothing_and_a_lost_one_queues_itself() {
    for config in ranges() {
        let queued: Vec<String> = all_actions(&config)
            .into_iter()
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => Some(next.scenario_id),
                _ => None,
            })
            .collect();
        assert_eq!(
            queued,
            vec![config.id.clone(), config.id.clone()],
            "{} must queue only itself, once per way of losing it",
            config.id
        );
    }
}

/// The helm is the lesson's. Each range hands over the verbs its lessons teach
/// and withholds the rest, so the verb that would fly the drill FOR the player
/// is not there to press.
#[test]
fn a_drill_withholds_every_verb_its_lessons_do_not_need() {
    let expected: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::from([
        // Aim and momentum: the computer is locked out entirely, and there is
        // nothing on the range to lock.
        (
            DRILL_MOMENTUM_ID,
            BTreeSet::from(["STOP", "LOCK", "GOTO", "ORBIT"]),
        ),
        // STOP and the thrusters: both are handed over, and the autopilot that
        // would fly the leg instead is not.
        (DRILL_STOP_ID, BTreeSet::from(["LOCK", "GOTO", "ORBIT"])),
        // The flight computer's own range: everything it needs, minus the
        // manual brake that would let a player fly around the lesson.
        (DRILL_AUTOPILOT_ID, BTreeSet::from(["STOP"])),
        // The firing line: the lock is the lesson, and the autopilot would fly
        // the approach the player is meant to fly.
        (DRILL_GUNNERY_ID, BTreeSet::from(["GOTO", "ORBIT"])),
    ]);

    for config in ranges() {
        let held_back = withheld(spawned_helm(&config));
        assert_eq!(
            held_back,
            expected[config.id.as_str()],
            "{} hands over the wrong helm",
            config.id
        );
    }
}

/// One live line on the board. Every objective a drill posts is posted once
/// and completed once, so no beat leaves its line standing under the next one
/// - which the event world would take as a duplicate and warn about.
#[test]
fn every_objective_a_drill_posts_is_posted_once_and_completed_once() {
    for config in ranges() {
        let mut posted: Vec<String> = Vec::new();
        let mut completed: Vec<String> = Vec::new();
        for action in all_actions(&config) {
            match action {
                EventActionConfig::Objective(objective) => posted.push(objective.id),
                EventActionConfig::ObjectiveComplete(objective) => completed.push(objective.id),
                _ => {}
            }
        }
        let unique: BTreeSet<&String> = posted.iter().collect();
        assert_eq!(
            unique.len(),
            posted.len(),
            "{} posts an objective id twice: {posted:?}",
            config.id
        );
        posted.sort();
        completed.sort();
        assert_eq!(
            posted, completed,
            "{} must complete every objective it posts",
            config.id
        );
    }
}

/// Every chip a drill lights goes out again when its beat ends, so a range
/// never leaves a verb pulsing at a player who has already used it.
#[test]
fn every_chip_a_drill_lights_is_cleared_again() {
    for config in ranges() {
        let mut lit: Vec<String> = Vec::new();
        let mut cleared: Vec<String> = Vec::new();
        for action in all_actions(&config) {
            match action {
                EventActionConfig::HintEmphasisSet(hint) => lit.push(hint.verb),
                EventActionConfig::HintEmphasisClear(hint) => cleared.push(hint.verb),
                _ => {}
            }
        }
        lit.sort();
        cleared.sort();
        assert_eq!(lit, cleared, "{} lights a chip it never clears", config.id);
    }
}

/// The role is the whole enforcement of "a practice range is purpose-built":
/// it keeps a drill out of the Scenarios picker and out of every campaign, and
/// it is what a lesson's `practice` field is checked against.
#[test]
fn no_drill_is_a_chapter() {
    for config in ranges() {
        assert!(
            config.role.is_lesson(),
            "{} must declare role: Lesson",
            config.id
        );
    }
}
