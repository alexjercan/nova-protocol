//! Reference and pacing checks over one scenario or campaign config.

use std::collections::HashSet;

use super::{ship::check_object_prototypes, KnownSections, LintIssue};
use crate::prelude::*;

/// Everything a scenario's actions can DECLARE, collected in one pass:
/// spawnable entity ids (spawns + areas), scatter prefixes, set variables,
/// posted objective ids.
#[derive(Default)]
struct Declared {
    spawn_ids: Vec<String>,
    scatter_prefixes: Vec<String>,
    set_vars: HashSet<String>,
    objective_ids: HashSet<String>,
    completed_objectives: HashSet<String>,
}

/// Lint one campaign against the scenario ids the caller knows about
/// (`known_scenarios`, normally base + all installed bundles). A campaign owns
/// an ordered `scenarios` list; each member must resolve to a real scenario, or
/// the picker would render a header row that launches nothing. Findings are
/// keyed (via [`LintIssue::scenario`]) by the CAMPAIGN id, since a campaign is
/// the element the finding is about.
///
/// Checks:
/// - a member id absent from `known_scenarios` is a DANGLING reference (Error) -
///   the same class as a `NextScenario` targeting a missing scenario;
/// - a member id listed more than once in the campaign is a duplicate (Warn) -
///   almost certainly an authoring slip, but the campaign still lists.
pub fn lint_campaign(
    campaign: &CampaignConfig,
    known_scenarios: &HashSet<String>,
) -> Vec<LintIssue> {
    let id = campaign.id.as_str();
    let mut issues = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for member in &campaign.scenarios {
        if !known_scenarios.contains(member) {
            issues.push(LintIssue::error(
                id,
                format!(
                    "campaign '{id}' lists member scenario '{member}', which no bundle provides"
                ),
            ));
        }
        if !seen.insert(member.as_str()) {
            issues.push(LintIssue::warn(
                id,
                format!("campaign '{id}' lists member scenario '{member}' more than once"),
            ));
        }
    }
    issues
}

/// Lint one scenario against the identifier sets the caller knows about:
/// `sections` (the section-prototype catalog visible to this scenario's
/// bundle) and `known_scenarios` (every scenario id a `NextScenario` may
/// target, normally base + all installed bundles).
pub fn lint_scenario(
    scenario: &ScenarioConfig,
    sections: &KnownSections,
    known_scenarios: &HashSet<String>,
) -> Vec<LintIssue> {
    let id = scenario.id.as_str();
    let mut issues = Vec::new();

    // Pass 1: what the scenario declares. Spawn ids are tracked per event
    // so the duplicate check can tell a definite bug from a branch pattern.
    let mut declared = Declared::default();
    let mut spawns_per_event: Vec<Vec<String>> = Vec::new();
    for event in &scenario.events {
        let mut event_spawns = Vec::new();
        for action in &event.actions {
            collect_declared(action, &mut declared);
            match action {
                EventActionConfig::SpawnScenarioObject(config) => {
                    event_spawns.push(config.base.id.clone());
                }
                EventActionConfig::CreateScenarioArea(config) => {
                    event_spawns.push(config.id.clone());
                }
                _ => {}
            }
        }
        spawns_per_event.push(event_spawns);
    }

    // Duplicate spawned ids: within ONE handler's action list two objects
    // definitely answer one id (Error); across handlers the spawns may sit
    // in mutually exclusive branches (e.g. a choice fork spawning the same
    // boss id either way), which is fine IF only one can fire - flag it for
    // eyes, do not fail the gate (Warn).
    for event_spawns in &spawns_per_event {
        let mut seen = HashSet::new();
        for spawn_id in event_spawns {
            if !seen.insert(spawn_id.as_str()) {
                issues.push(LintIssue::error(
                    id,
                    format!("duplicate spawned object id '{spawn_id}' within one handler"),
                ));
            }
        }
    }
    let mut seen_across: HashSet<&str> = HashSet::new();
    let mut warned: HashSet<&str> = HashSet::new();
    for event_spawns in &spawns_per_event {
        for spawn_id in event_spawns.iter().collect::<HashSet<_>>() {
            if !seen_across.insert(spawn_id.as_str()) && warned.insert(spawn_id.as_str()) {
                issues.push(LintIssue::warn(
                    id,
                    format!(
                        "object id '{spawn_id}' is spawned by more than one handler - fine only if the handlers are mutually exclusive"
                    ),
                ));
            }
        }
    }

    let satisfiable = |target: &str| {
        declared.spawn_ids.iter().any(|s| s == target)
            || declared
                .scatter_prefixes
                .iter()
                .any(|p| target.starts_with(p.as_str()))
    };

    // Pass 2: what the scenario references.
    let mut used_vars: HashSet<String> = HashSet::new();
    for event in &scenario.events {
        for filter in &event.filters {
            check_filter(filter, id, &satisfiable, &mut used_vars, &mut issues);
        }
        for action in &event.actions {
            check_action(
                action,
                id,
                sections,
                known_scenarios,
                &satisfiable,
                &mut used_vars,
                &mut issues,
            );
        }
    }

    for completed in &declared.completed_objectives {
        if !declared.objective_ids.contains(completed) {
            issues.push(LintIssue::warn(
                id,
                format!(
                    "ObjectiveComplete '{completed}' has no matching Objective in this scenario"
                ),
            ));
        }
    }

    // The beat-sheet convention, mechanized: (a) one story line per beat - a
    // multi-line handler reads as one burst even through the paced queue; (b) a
    // StoryMessage beside an Outcome is a DEAD line - the overlay pauses the
    // comms queue and the chained teardown drops it unread. Fold it into the
    // overlay message or move it to an earlier beat.
    for event in &scenario.events {
        let story_lines = event
            .actions
            .iter()
            .filter(|a| matches!(a, EventActionConfig::StoryMessage(_)))
            .count();
        if story_lines > 1 {
            issues.push(LintIssue::warn(
                id,
                format!(
                    "{story_lines} StoryMessages in one handler: space beats with the \
                     scenario clock (one line per beat; the comms queue is the safety \
                     net, not the style)"
                ),
            ));
        }
        if story_lines > 0
            && event
                .actions
                .iter()
                .any(|a| matches!(a, EventActionConfig::Outcome(_)))
        {
            issues.push(LintIssue::warn(
                id,
                "a StoryMessage beside an Outcome is never read (frozen behind the \
                 overlay, dropped by the chained teardown) - fold it into the \
                 overlay's message or move it to an earlier beat"
                    .to_string(),
            ));
        }
    }

    // Outcome + non-lingering NextScenario in ONE handler is an authoring trap
    // either way: undelayed, the instant switch tears the world down and
    // SWALLOWS the overlay before it can show (NovaEventWorld::clear's
    // documented footgun); delayed, the overlay's pause freezes the delay clock
    // so the cut never comes while the player reads. Pair Outcome with linger:
    // true (+ auto_advance_secs for a timed banner), or drop the Outcome for a
    // pure delayed cut.
    for event in &scenario.events {
        let has_outcome = event
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::Outcome(_)));
        let hard_switch = event
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::NextScenario(next) if !next.linger));
        if has_outcome && hard_switch {
            issues.push(LintIssue::warn(
                id,
                "an Outcome and a non-lingering NextScenario in one handler: the \
                 switch swallows (or, delayed, is frozen under) the overlay - use \
                 linger: true with the Outcome, or drop the Outcome for a pure cut"
                    .to_string(),
            ));
        }
    }

    for var in &used_vars {
        // The reserved engine variables (the scenario clock, the player-speed
        // readout) are ENGINE-set every live unpaused tick (loader's
        // tick_scenario_clock / track_player_speed); reading one needs no
        // VariableSet.
        if crate::loader::is_reserved_engine_var(var) {
            continue;
        }
        if !declared.set_vars.contains(var) {
            issues.push(LintIssue::warn(
                id,
                format!(
                    "expression variable '{var}' is never set in this scenario \
                     (filters on it fail closed)"
                ),
            ));
        }
    }

    issues
}

fn collect_declared(action: &EventActionConfig, declared: &mut Declared) {
    match action {
        EventActionConfig::SpawnScenarioObject(config) => {
            declared.spawn_ids.push(config.base.id.clone());
        }
        EventActionConfig::ScatterObjects(config) => {
            declared.scatter_prefixes.push(config.id_prefix.clone());
        }
        EventActionConfig::CreateScenarioArea(config) => {
            declared.spawn_ids.push(config.id.clone());
        }
        EventActionConfig::VariableSet(config) => {
            declared.set_vars.insert(config.key.clone());
        }
        EventActionConfig::Objective(config) => {
            declared.objective_ids.insert(config.id.clone());
        }
        EventActionConfig::ObjectiveComplete(config) => {
            declared.completed_objectives.insert(config.id.clone());
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn check_action(
    action: &EventActionConfig,
    scenario: &str,
    sections: &KnownSections,
    known_scenarios: &HashSet<String>,
    satisfiable: &dyn Fn(&str) -> bool,
    used_vars: &mut HashSet<String>,
    issues: &mut Vec<LintIssue>,
) {
    match action {
        EventActionConfig::SpawnScenarioObject(config) => {
            check_object_prototypes(config, scenario, sections, issues);
        }
        EventActionConfig::ScatterObjects(config) => {
            // The template is a full object config too - a scattered ship with
            // a bad prototype is the same bug one wrapper deeper.
            check_object_prototypes(&config.template, scenario, sections, issues);
            // The runtime clamps rather than OOMs, but a clamped field is not
            // the field the author wrote - say so before it ships.
            if config.count > MAX_SCATTER_COUNT {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ScatterObjects '{}' count {} exceeds the {MAX_SCATTER_COUNT} cap \
                         and will be clamped at runtime",
                        config.id_prefix, config.count
                    ),
                ));
            }
        }
        EventActionConfig::Outcome(config) => {
            if let Some(secs) = config.auto_advance_secs {
                // Half-open at zero, as the message says: `Some(0.0)` builds a
                // Timer that finishes on its first tick, which is a different
                // scenario from the `None` the author meant.
                if !secs.is_finite() || secs <= 0.0 || secs > OUTCOME_AUTO_ADVANCE_MAX_SECS {
                    issues.push(LintIssue::warn(
                        scenario,
                        format!(
                            "Outcome auto_advance_secs {secs}s is outside (0, \
                             {OUTCOME_AUTO_ADVANCE_MAX_SECS}]s"
                        ),
                    ));
                }
            }
        }
        EventActionConfig::VariableSet(config) => {
            // The reserved engine variables (the scenario clock, the
            // player-speed readout) are engine-owned: their tracker rewrites
            // them every frame, so an authored write is at best a one-frame
            // glitch and at worst a broken gate - always a bug.
            if crate::loader::is_reserved_engine_var(&config.key) {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "VariableSet writes the reserved engine variable '{}' \
                         (gate on it with expression filters instead)",
                        config.key
                    ),
                ));
            }
            collect_expression_vars(&config.expression, used_vars);
        }
        EventActionConfig::TimerStart(config) => {
            if config.key.trim().is_empty() {
                issues.push(LintIssue::error(
                    scenario,
                    "TimerStart has an empty key".to_string(),
                ));
            }
            if let Some(seconds) = direct_number_literal(&config.seconds) {
                if !seconds.is_finite() || seconds <= 0.0 {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "TimerStart '{}' duration must be a positive finite number, got {seconds}",
                            config.key
                        ),
                    ));
                }
            }
            collect_expression_vars(&config.seconds, used_vars);
        }
        EventActionConfig::TimerCancel(config) => {
            if config.key.trim().is_empty() {
                issues.push(LintIssue::error(
                    scenario,
                    "TimerCancel has an empty key".to_string(),
                ));
            }
        }
        EventActionConfig::StoryMessage(config) => {
            // The panel clamps silently; an authored dwell outside the
            // documented range is an authoring slip worth a nudge.
            if let Some(dwell) = config.dwell {
                use nova_hud::prelude::{COMMS_DWELL_MAX_SECS, COMMS_DWELL_MIN_SECS};
                if !(COMMS_DWELL_MIN_SECS..=COMMS_DWELL_MAX_SECS).contains(&dwell) {
                    issues.push(LintIssue::warn(
                        scenario,
                        format!(
                            "StoryMessage dwell {dwell}s is outside the [3, 30]s range \
                             and will be clamped by the comms panel"
                        ),
                    ));
                }
            }
        }
        EventActionConfig::NextScenario(config) => {
            // Pacing-field sanity: non-finite or huge delays are
            // runtime-capped, a delay on a LINGERING request is a silently dead
            // field.
            if let Some(delay) = config.delay {
                if config.linger {
                    issues.push(LintIssue::warn(
                        scenario,
                        "NextScenario delay with linger: true is dead (the overlay's \
                         release is the timing) - drop the field or use linger: false"
                            .to_string(),
                    ));
                } else if !delay.is_finite()
                    || delay <= 0.0
                    || delay > NEXT_SCENARIO_DELAY_WARN_SECS
                {
                    issues.push(LintIssue::warn(
                        scenario,
                        format!(
                            "NextScenario delay {delay}s is outside (0, \
                             {NEXT_SCENARIO_DELAY_WARN_SECS}]s (runtime caps at \
                             {NEXT_SCENARIO_DELAY_MAX_SECS}s)"
                        ),
                    ));
                }
            }
            if !known_scenarios.contains(&config.scenario_id) {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "NextScenario targets unknown scenario '{}'",
                        config.scenario_id
                    ),
                ));
            }
        }
        EventActionConfig::ObjectiveMarkerAttach(config) => {
            check_target(
                &config.target_id,
                "ObjectiveMarkerAttach",
                scenario,
                satisfiable,
                issues,
            );
        }
        EventActionConfig::ObjectiveMarkerDetach(config) => {
            check_target(
                &config.target_id,
                "ObjectiveMarkerDetach",
                scenario,
                satisfiable,
                issues,
            );
        }
        EventActionConfig::DespawnScenarioObject(config) => {
            check_target(
                &config.id,
                "DespawnScenarioObject",
                scenario,
                satisfiable,
                issues,
            );
        }
        EventActionConfig::SetSpeedCap(config) => {
            check_target(&config.id, "SetSpeedCap", scenario, satisfiable, issues);
        }
        EventActionConfig::SetAllegiance(config) => {
            check_target(&config.id, "SetAllegiance", scenario, satisfiable, issues);
        }
        EventActionConfig::SetControllerVerb(config) => {
            check_target(
                &config.id,
                "SetControllerVerb",
                scenario,
                satisfiable,
                issues,
            );
        }
        EventActionConfig::HudReadout(config) => {
            // An empty slot or variable is an authoring typo the sync would
            // silently accept (an empty-slot readout can never be cleared).
            if config.slot.trim().is_empty() {
                issues.push(LintIssue::error(
                    scenario,
                    "HudReadout has an empty slot (it needs a stable id to update or clear)"
                        .to_string(),
                ));
            }
            if config.variable.trim().is_empty() {
                issues.push(LintIssue::error(
                    scenario,
                    "HudReadout has an empty variable (nothing to display)".to_string(),
                ));
            }
            // The bound variable is READ like an expression variable: track it
            // so the "never set" pass warns on a readout of a variable no
            // VariableSet ever writes (the engine clock is exempted there).
            used_vars.insert(config.variable.clone());
        }
        _ => {}
    }
}
fn direct_number_literal(expression: &VariableExpressionNode) -> Option<f64> {
    let VariableExpressionNode::Term(VariableTermNode::Factor(VariableFactorNode::Literal(
        VariableLiteral::Number(value),
    ))) = expression
    else {
        return None;
    };
    Some(*value)
}

fn check_target(
    target: &str,
    what: &str,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    issues: &mut Vec<LintIssue>,
) {
    if !satisfiable(target) {
        issues.push(LintIssue::error(
            scenario,
            format!("{what} targets id '{target}', which nothing in this scenario spawns"),
        ));
    }
}

fn check_filter(
    filter: &EventFilterConfig,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    used_vars: &mut HashSet<String>,
    issues: &mut Vec<LintIssue>,
) {
    match filter {
        EventFilterConfig::Entity(config) => {
            for reference in [&config.id, &config.other_id].into_iter().flatten() {
                if !satisfiable(reference) {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "entity filter references id '{reference}', which nothing in \
                             this scenario spawns"
                        ),
                    ));
                }
            }
        }
        EventFilterConfig::Expression(config) => {
            collect_condition_vars(&config.0, used_vars);
        }
        EventFilterConfig::Timer(config) => {
            if config.key.trim().is_empty() {
                issues.push(LintIssue::error(
                    scenario,
                    "Timer filter has an empty key".to_string(),
                ));
            }
        }
        EventFilterConfig::Conditional(config) => match config {
            ConditionalFilterConfig::Not(inner) => {
                check_filter(inner, scenario, satisfiable, used_vars, issues);
            }
            ConditionalFilterConfig::Or(left, right)
            | ConditionalFilterConfig::And(left, right) => {
                check_filter(left, scenario, satisfiable, used_vars, issues);
                check_filter(right, scenario, satisfiable, used_vars, issues);
            }
        },
    }
}

fn collect_condition_vars(node: &VariableConditionNode, vars: &mut HashSet<String>) {
    match node {
        VariableConditionNode::LessThan(left, right)
        | VariableConditionNode::GreaterThan(left, right)
        | VariableConditionNode::Equal(left, right) => {
            collect_expression_vars(left, vars);
            collect_expression_vars(right, vars);
        }
    }
}

fn collect_expression_vars(node: &VariableExpressionNode, vars: &mut HashSet<String>) {
    match node {
        VariableExpressionNode::Add(term, rest) | VariableExpressionNode::Subtract(term, rest) => {
            collect_term_vars(term, vars);
            collect_expression_vars(rest, vars);
        }
        VariableExpressionNode::Term(term) => collect_term_vars(term, vars),
    }
}

fn collect_term_vars(node: &VariableTermNode, vars: &mut HashSet<String>) {
    match node {
        VariableTermNode::Multiply(factor, rest) | VariableTermNode::Divide(factor, rest) => {
            collect_factor_vars(factor, vars);
            collect_term_vars(rest, vars);
        }
        VariableTermNode::Factor(factor) => collect_factor_vars(factor, vars),
    }
}

fn collect_factor_vars(node: &VariableFactorNode, vars: &mut HashSet<String>) {
    match node {
        VariableFactorNode::Parens(inner) => collect_expression_vars(inner, vars),
        VariableFactorNode::Name(name) => {
            vars.insert(name.clone());
        }
        VariableFactorNode::Literal(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;
    use crate::lint::{fixtures::*, LintSeverity};

    #[test]
    fn timer_keys_and_literal_durations_are_linted() {
        let s = scenario(
            vec![
                EventActionConfig::TimerStart(TimerStartActionConfig {
                    key: String::new(),
                    seconds: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                    )),
                }),
                EventActionConfig::TimerCancel(TimerCancelActionConfig { key: String::new() }),
            ],
            vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: String::new(),
            })],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&[]));
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.severity == LintSeverity::Error)
                .count(),
            4,
            "empty start/cancel/filter keys and zero duration each error: {issues:?}"
        );
    }

    /// A campaign whose every member resolves lints clean - the baseline the
    /// dangling case diverges from.
    #[test]
    fn campaign_with_known_members_lints_clean() {
        let c = campaign(
            "nova_protocol",
            &["shakedown_run", "broadside", "final_tally"],
        );
        let issues = lint_campaign(&c, &known(&["shakedown_run", "broadside", "final_tally"]));
        assert!(
            issues.is_empty(),
            "a campaign naming only real scenarios is clean, got {issues:?}"
        );
    }

    /// A campaign member that no bundle provides is a DANGLING reference: an
    /// Error, so the gate refuses a header row that would launch nothing. Would
    /// pass trivially if lint_campaign did not check membership.
    #[test]
    fn campaign_flags_dangling_member() {
        let c = campaign("nova_protocol", &["shakedown_run", "ghost_chapter"]);
        let issues = lint_campaign(&c, &known(&["shakedown_run"]));
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == LintSeverity::Error)
            .collect();
        assert_eq!(errors.len(), 1, "exactly the one missing member errors");
        assert!(
            errors[0].message.contains("ghost_chapter"),
            "the finding names the dangling member: {}",
            errors[0].message
        );
    }

    /// A member listed twice is a Warn (authoring slip), not an Error - the
    /// campaign still lists.
    #[test]
    fn campaign_warns_on_duplicate_member() {
        let c = campaign("nova_protocol", &["shakedown_run", "shakedown_run"]);
        let issues = lint_campaign(&c, &known(&["shakedown_run"]));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == LintSeverity::Warn && i.message.contains("more than once")),
            "a duplicate member warns, got {issues:?}"
        );
        assert!(
            !issues.iter().any(|i| i.severity == LintSeverity::Error),
            "a duplicate of a KNOWN member is not an error, got {issues:?}"
        );
    }
    /// A well-formed scenario yields ZERO issues (the clean baseline every
    /// would-it-fail case below diverges from).
    #[test]
    fn clean_scenario_lints_clean() {
        let s = scenario(
            vec![
                spawn_ship("player", "known_proto"),
                spawn_object("gate_1"),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "act".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                }),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: "next_chapter".to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
            vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some("gate_1".to_string()),
                    other_id: Some("player".to_string()),
                    ..Default::default()
                }),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_equals(
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_name("act"),
                        )),
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                        )),
                    ),
                )),
            ],
        );
        let issues = lint_scenario(
            &s,
            &sections(&["known_proto"]),
            &known(&["test_scenario", "next_chapter"]),
        );
        assert!(issues.is_empty(), "clean scenario flagged: {issues:?}");
    }

    #[test]
    fn dangling_next_scenario_is_an_error() {
        let s = scenario(
            vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "gone".to_string(),
                linger: true,
                delay: None,
            })],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("gone"));
    }

    /// SetAllegiance references a ship by id like SetSpeedCap does, so a typo'd
    /// id must lint as a dangling target, not silently no-op at runtime.
    #[test]
    fn dangling_set_allegiance_target_is_an_error() {
        use nova_gameplay::prelude::Allegiance;

        let s = scenario(
            vec![EventActionConfig::SetAllegiance(
                SetAllegianceActionConfig {
                    id: "ghost".to_string(),
                    allegiance: Allegiance::Enemy,
                },
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("ghost"));
        assert!(errs[0].message.contains("SetAllegiance"));
    }

    #[test]
    fn duplicate_spawn_ids_in_one_handler_are_an_error() {
        let s = scenario(vec![spawn_object("twin"), spawn_object("twin")], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("twin"));
    }

    /// The choice-fork pattern: two handlers each spawning the same boss id
    /// (only one can fire) is a WARN, not a gate failure.
    #[test]
    fn duplicate_spawn_ids_across_handlers_are_a_warn() {
        let mut s = scenario(vec![spawn_object("boss")], vec![]);
        s.events.push(ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![],
            actions: vec![spawn_object("boss")],
        });
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("mutually exclusive"));
    }

    /// F12: `count` is an unvalidated authored u32 driving a spawn loop. The
    /// runtime clamps it, but the author should never get that far - a field
    /// the engine will not honor is a content error.
    #[test]
    fn an_absurd_scatter_count_is_a_lint_error() {
        let s = scenario(
            vec![EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: "rock_".to_string(),
                count: 50_000_000,
                seed: 1,
                region: ScatterRegion::Ring {
                    center: Vec3::ZERO,
                    inner: 10.0,
                    outer: 20.0,
                    y_min: -1.0,
                    y_max: 1.0,
                },
                template: match spawn_object("rock_") {
                    EventActionConfig::SpawnScenarioObject(config) => config,
                    _ => unreachable!(),
                },
                asteroid_radius: None,
                min_separation: None,
            })],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("exceeds the")),
            "an over-cap scatter count is an error: {issues:?}"
        );
    }

    #[test]
    fn unspawnable_filter_id_is_an_error_but_scatter_prefix_satisfies() {
        let s = scenario(
            vec![EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: "rock_".to_string(),
                count: 3,
                seed: 1,
                region: ScatterRegion::Ring {
                    center: Vec3::ZERO,
                    inner: 10.0,
                    outer: 20.0,
                    y_min: -1.0,
                    y_max: 1.0,
                },
                template: match spawn_object("rock_") {
                    EventActionConfig::SpawnScenarioObject(config) => config,
                    _ => unreachable!(),
                },
                asteroid_radius: None,
                min_separation: None,
            })],
            vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some("rock_2".to_string()),
                    ..Default::default()
                }),
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some("ghost".to_string()),
                    ..Default::default()
                }),
            ],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "only the ghost flags: {issues:?}");
        assert!(errs[0].message.contains("ghost"));
    }

    #[test]
    fn unset_variable_and_unmatched_complete_are_warns() {
        let s = scenario(
            vec![EventActionConfig::ObjectiveComplete(
                ObjectiveCompleteActionConfig {
                    id: "never_posted".to_string(),
                },
            )],
            vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_equals(
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_name("never_set"),
                    )),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            ))],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues.iter().any(|i| i.message.contains("never_set")));
        assert!(issues.iter().any(|i| i.message.contains("never_posted")));
    }

    /// Outcome + non-lingering NextScenario in one handler warns: undelayed it
    /// swallows the overlay, delayed it freezes under the pause. The lingering
    /// pair stays clean.
    #[test]
    fn outcome_with_hard_switch_in_one_handler_warns() {
        let outcome = || {
            EventActionConfig::Outcome(OutcomeActionConfig {
                outcome: ScenarioOutcomeKind::Victory,
                message: None,
                auto_advance_secs: None,
            })
        };
        let next = |linger: bool, delay: Option<f32>| {
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "test_scenario".to_string(),
                linger,
                delay,
            })
        };

        let s = scenario(vec![outcome(), next(false, None)], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("non-lingering"));

        let s = scenario(vec![outcome(), next(false, Some(4.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "delayed is the same trap: {issues:?}");

        let s = scenario(vec![outcome(), next(true, None)], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(
            issues.is_empty(),
            "the lingering pair is the good shape: {issues:?}"
        );
    }

    /// The beat-sheet arms: double lines warn, story-beside-outcome warns, one
    /// line per handler is clean.
    #[test]
    fn beat_sheet_arms_warn() {
        let line = |text: &str| {
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: text.to_string(),
                dwell: None,
                icon: None,
            })
        };
        let outcome = || {
            EventActionConfig::Outcome(OutcomeActionConfig {
                outcome: ScenarioOutcomeKind::Victory,
                message: Some("done".to_string()),
                auto_advance_secs: None,
            })
        };

        let s = scenario(vec![line("one"), line("two")], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("one line per beat"));

        let s = scenario(vec![line("dead"), outcome()], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("never read"));

        let s = scenario(vec![line("solo")], vec![]);
        assert!(lint_scenario(&s, &sections(&[]), &known(&["test_scenario"])).is_empty());
    }

    /// Pacing-field ranges: absurd/non-finite delays warn, a delay on a
    /// lingering request is dead and warns, sane values stay clean.
    #[test]
    fn pacing_field_ranges_warn() {
        let next = |linger: bool, delay: Option<f32>| {
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "test_scenario".to_string(),
                linger,
                delay,
            })
        };
        let outcome_adv = |secs: Option<f64>| {
            EventActionConfig::Outcome(OutcomeActionConfig {
                outcome: ScenarioOutcomeKind::Victory,
                message: None,
                auto_advance_secs: secs,
            })
        };

        // Range/dead-field warns, isolated from the same-handler swallow
        // trap (which is its own test): switches only.
        let s = scenario(vec![next(false, Some(1e30)), next(true, Some(4.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues.iter().any(|i| i.message.contains("outside (0, 60]")));
        assert!(issues.iter().any(|i| i.message.contains("dead")));

        // The outcome range warn, without a hard switch in the handler.
        let s = scenario(vec![outcome_adv(Some(f64::INFINITY))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("auto_advance_secs"));

        // Zero is OUTSIDE the (0, MAX] the messages advertise: it builds a
        // Timer that finishes on tick one, so the banner never shows. Both
        // fields, since both read as "omit the field instead".
        let s = scenario(vec![outcome_adv(Some(0.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("auto_advance_secs"));
        let s = scenario(vec![next(false, Some(0.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("outside (0, 60]"));

        // Sane values, trap-free shapes: clean.
        let s = scenario(vec![next(false, Some(4.0))], vec![]);
        assert!(lint_scenario(&s, &sections(&[]), &known(&["test_scenario"])).is_empty());
        let s = scenario(vec![outcome_adv(Some(6.0))], vec![]);
        assert!(lint_scenario(&s, &sections(&[]), &known(&["test_scenario"])).is_empty());
    }

    /// StoryMessage dwell range: out-of-range warns, in-range and omitted stay
    /// clean.
    #[test]
    fn story_dwell_out_of_range_warns() {
        let line = |dwell| {
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "test".to_string(),
                dwell,
                icon: None,
            })
        };
        // One line per handler so the beat-sheet arm stays out of frame.
        let mut s = scenario(vec![line(Some(120.0))], vec![]);
        for l in [line(Some(12.0)), line(None)] {
            s.events.push(ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                actions: vec![l],
            });
        }
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("120"));
    }

    /// The reserved scenario clock: reading it needs no VariableSet (the engine
    /// ticks it), so no unset-variable warning; WRITING it is always a bug, so
    /// an authored VariableSet errors.
    #[test]
    fn scenario_clock_reads_are_clean_and_writes_are_errors() {
        use crate::loader::SCENARIO_ELAPSED_VAR;

        // A time-gated handler the way an author writes one: no warning.
        let read_only = scenario(
            vec![],
            vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_greater_than(
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_name(SCENARIO_ELAPSED_VAR),
                    )),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(30.0)),
                    )),
                ),
            ))],
        );
        let issues = lint_scenario(&read_only, &sections(&[]), &known(&["test_scenario"]));
        assert!(
            issues.is_empty(),
            "gating on the engine clock is the intended pattern: {issues:?}"
        );

        // An authored write to the clock: an error, not a warning.
        let stomp = scenario(
            vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: SCENARIO_ELAPSED_VAR.to_string(),
                expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                )),
            })],
            vec![],
        );
        let issues = lint_scenario(&stomp, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(
            errors(&issues).len(),
            1,
            "writing the reserved clock is an error: {issues:?}"
        );
        assert!(issues[0].message.contains(SCENARIO_ELAPSED_VAR));
    }

    /// The reserved `player_speed` readout follows the same contract as the
    /// clock: gating on it is clean (the engine writes it each tick), authoring
    /// a VariableSet onto it is an error. Pins the second reserved variable so
    /// the shared `is_reserved_engine_var` list cannot drift from the lint
    /// rules that consume it.
    #[test]
    fn player_speed_reads_are_clean_and_writes_are_errors() {
        use crate::loader::PLAYER_SPEED_VAR;

        // A speed-gated handler the way an author writes one: no warning.
        let read_only = scenario(
            vec![],
            vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_greater_than(
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_name(PLAYER_SPEED_VAR),
                    )),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(8.0)),
                    )),
                ),
            ))],
        );
        let issues = lint_scenario(&read_only, &sections(&[]), &known(&["test_scenario"]));
        assert!(
            issues.is_empty(),
            "gating on the engine speed readout is the intended pattern: {issues:?}"
        );

        // An authored write to the readout: an error, not a warning.
        let stomp = scenario(
            vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: PLAYER_SPEED_VAR.to_string(),
                expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                )),
            })],
            vec![],
        );
        let issues = lint_scenario(&stomp, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(
            errors(&issues).len(),
            1,
            "writing the reserved speed readout is an error: {issues:?}"
        );
        assert!(issues[0].message.contains(PLAYER_SPEED_VAR));
    }
}
