//! Static content lint (task 20260716-191543, spike 20260716-193858): the
//! identifier-level checks no load or publish gate can make, because these
//! references resolve at SPAWN time (a scenario naming a section prototype
//! that does not exist loads green and ships a half-spawning ship).
//!
//! Pure functions over parsed config - no assets, no ECS - so one core
//! serves every consumer: the `content_lint` author CLI (nova_assets bin),
//! the CI gate test, and the runtime merge sweep (task 20260716-193949).
//!
//! Static approximations, documented: a reference matching a
//! `ScatterObjects` id prefix counts as satisfiable (the actual `<prefix><n>`
//! ids exist only at runtime); variable set/use is checked scenario-wide,
//! not in firing order.

use std::collections::HashSet;

use crate::prelude::*;

pub mod prelude {
    pub use super::{lint_scenario, LintIssue, LintSeverity};
}

/// How bad a finding is: `Error` fails gates (the content WILL misbehave),
/// `Warn` is reported but does not fail (almost certainly an authoring bug,
/// but the scenario still runs - e.g. a fails-closed unset variable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warn,
}

/// One finding, human-readable and self-contained ("scenario 'x': unknown
/// section prototype 'y'").
#[derive(Clone, Debug)]
pub struct LintIssue {
    pub severity: LintSeverity,
    /// The scenario the finding is about.
    pub scenario: ScenarioId,
    pub message: String,
}

impl LintIssue {
    fn error(scenario: &str, message: String) -> Self {
        Self {
            severity: LintSeverity::Error,
            scenario: scenario.to_string(),
            message,
        }
    }

    fn warn(scenario: &str, message: String) -> Self {
        Self {
            severity: LintSeverity::Warn,
            scenario: scenario.to_string(),
            message,
        }
    }
}

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

/// Lint one scenario against the identifier sets the caller knows about:
/// `known_sections` (the section-prototype catalog visible to this
/// scenario's bundle) and `known_scenarios` (every scenario id a
/// `NextScenario` may target, normally base + all installed bundles).
pub fn lint_scenario(
    scenario: &ScenarioConfig,
    known_sections: &HashSet<String>,
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
                known_sections,
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

    for var in &used_vars {
        // The scenario clock is ENGINE-set every live unpaused tick
        // (loader::tick_scenario_clock); reading it needs no VariableSet.
        if var == crate::loader::SCENARIO_ELAPSED_VAR {
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
    known_sections: &HashSet<String>,
    known_scenarios: &HashSet<String>,
    satisfiable: &dyn Fn(&str) -> bool,
    used_vars: &mut HashSet<String>,
    issues: &mut Vec<LintIssue>,
) {
    match action {
        EventActionConfig::SpawnScenarioObject(config) => {
            check_object_prototypes(config, scenario, known_sections, issues);
        }
        EventActionConfig::ScatterObjects(config) => {
            // The template is a full object config too - a scattered ship
            // with a bad prototype is the same bug one wrapper deeper
            // (review R1.1).
            check_object_prototypes(&config.template, scenario, known_sections, issues);
        }
        EventActionConfig::VariableSet(config) => {
            // The scenario clock is engine-owned: the tick system rewrites
            // it every frame, so an authored write is at best a one-frame
            // glitch and at worst a broken time gate - always a bug.
            if config.key == crate::loader::SCENARIO_ELAPSED_VAR {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "VariableSet writes the reserved engine clock '{}' \
                         (gate on it with expression filters instead)",
                        crate::loader::SCENARIO_ELAPSED_VAR
                    ),
                ));
            }
            collect_expression_vars(&config.expression, used_vars);
        }
        EventActionConfig::StoryMessage(config) => {
            // The panel clamps silently; an authored dwell outside the
            // documented range is an authoring slip worth a nudge.
            if let Some(dwell) = config.dwell {
                use nova_gameplay::prelude::{COMMS_DWELL_MAX_SECS, COMMS_DWELL_MIN_SECS};
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
        EventActionConfig::SetControllerVerb(config) => {
            check_target(
                &config.id,
                "SetControllerVerb",
                scenario,
                satisfiable,
                issues,
            );
        }
        _ => {}
    }
}

/// Every section prototype a spawned (or scatter-template) ship references
/// must exist in the caller's known set.
fn check_object_prototypes(
    config: &ScenarioObjectConfig,
    scenario: &str,
    known_sections: &HashSet<String>,
    issues: &mut Vec<LintIssue>,
) {
    if let ScenarioObjectKind::Spaceship(ship) = &config.kind {
        for section in &ship.sections {
            if let SectionSource::Prototype(proto) = &section.source {
                if !known_sections.contains(proto) {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "ship '{}' section '{}': unknown section prototype '{proto}'",
                            config.base.id, section.id
                        ),
                    ));
                }
            }
        }
        check_section_overlaps(config.base.id.as_str(), ship, scenario, issues);
    }
}

/// Sections are unit cubes centered on their authored grid position
/// (base_section's `Collider::cuboid(1.0, 1.0, 1.0)`), so two sections of
/// one ship OVERLAP - clip visually and double up their colliders in the
/// same space - iff their centers are strictly closer than 1.0 on EVERY
/// axis. Flush contact (distance exactly 1.0 on some axis) is the normal
/// spine/side-mount layout and passes. The check ignores section ROTATION:
/// exact for the quarter-turn rotations all shipped content uses (a unit
/// cube is symmetric under them), conservative-only for exotic angles.
/// Caught in the wild by the Auditor's torpedo bay authored at z 0.5,
/// embedded between two spine sections (task 20260717-151208).
fn check_section_overlaps(
    ship_id: &str,
    ship: &SpaceshipConfig,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    for i in 0..ship.sections.len() {
        for j in (i + 1)..ship.sections.len() {
            let (a, b) = (&ship.sections[i], &ship.sections[j]);
            let d = a.position - b.position;
            if d.x.abs() < 1.0 && d.y.abs() < 1.0 && d.z.abs() < 1.0 {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ship '{ship_id}': sections '{}' at {:?} and '{}' at {:?} overlap (unit-cube grid: centers must be >= 1.0 apart on some axis)",
                        a.id, a.position, b.id, b.position
                    ),
                ));
            }
        }
    }
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
    use nova_gameplay::prelude::AssetRef;

    use super::*;

    fn known(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    fn spawn_object(id: &str) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Beacon(BeaconConfig {
                label: id.to_uppercase(),
                radius: 1.0,
                color: Color::WHITE,
                area_radius: Some(5.0),
                lock_signature: None,
            }),
        })
    }

    fn spawn_ship(id: &str, proto: &str) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                allegiance: None,
                controller: SpaceshipController::AI(AIControllerConfig::default()),
                sections: vec![SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype(proto.to_string()),
                    modifications: vec![],
                }],
            }),
        })
    }

    fn scenario(
        actions: Vec<EventActionConfig>,
        filters: Vec<EventFilterConfig>,
    ) -> ScenarioConfig {
        ScenarioConfig {
            id: "test_scenario".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            cubemap: AssetRef::default(),
            events: vec![ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters,
                actions,
            }],
            ..Default::default()
        }
    }

    fn errors(issues: &[LintIssue]) -> Vec<&LintIssue> {
        issues
            .iter()
            .filter(|i| i.severity == LintSeverity::Error)
            .collect()
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
            &known(&["known_proto"]),
            &known(&["test_scenario", "next_chapter"]),
        );
        assert!(issues.is_empty(), "clean scenario flagged: {issues:?}");
    }

    #[test]
    fn unknown_prototype_is_an_error() {
        let s = scenario(vec![spawn_ship("player", "no_such_proto")], vec![]);
        let issues = lint_scenario(&s, &known(&["known_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
    }

    /// R1.1: a scatter TEMPLATE ship with a bad prototype must flag like a
    /// directly spawned one.
    #[test]
    fn unknown_prototype_in_a_scatter_template_is_an_error() {
        let template = match spawn_ship("swarm_", "no_such_proto") {
            EventActionConfig::SpawnScenarioObject(config) => config,
            _ => unreachable!(),
        };
        let s = scenario(
            vec![EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: "swarm_".to_string(),
                count: 2,
                seed: 1,
                region: ScatterRegion::Ring {
                    inner: 10.0,
                    outer: 20.0,
                    y_min: -1.0,
                    y_max: 1.0,
                },
                template,
                asteroid_radius: None,
            })],
            vec![],
        );
        let issues = lint_scenario(&s, &known(&["known_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
    }

    #[test]
    fn dangling_next_scenario_is_an_error() {
        let s = scenario(
            vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "gone".to_string(),
                linger: true,
            })],
            vec![],
        );
        let issues = lint_scenario(&s, &known(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("gone"));
    }

    #[test]
    fn duplicate_spawn_ids_in_one_handler_are_an_error() {
        let s = scenario(vec![spawn_object("twin"), spawn_object("twin")], vec![]);
        let issues = lint_scenario(&s, &known(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &known(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("mutually exclusive"));
    }

    #[test]
    fn unspawnable_filter_id_is_an_error_but_scatter_prefix_satisfies() {
        let s = scenario(
            vec![EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: "rock_".to_string(),
                count: 3,
                seed: 1,
                region: ScatterRegion::Ring {
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
        let issues = lint_scenario(&s, &known(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &known(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues.iter().any(|i| i.message.contains("never_set")));
        assert!(issues.iter().any(|i| i.message.contains("never_posted")));
    }

    /// StoryMessage dwell range (task 20260717-163033): out-of-range warns,
    /// in-range and omitted stay clean.
    #[test]
    fn story_dwell_out_of_range_warns() {
        let line = |dwell| {
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "test".to_string(),
                dwell,
            })
        };
        let s = scenario(
            vec![line(Some(120.0)), line(Some(12.0)), line(None)],
            vec![],
        );
        let issues = lint_scenario(&s, &known(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("120"));
    }

    /// Section overlaps (task 20260717-151208): strictly-inside-the-cube
    /// errors; flush spine/side mounts pass (the fail-first is the shipped
    /// Auditor tube at z 0.5 this check was born from).
    #[test]
    fn overlapping_sections_error_and_flush_sections_pass() {
        let ship_with = |tube_pos: Vec3| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "ship".to_string(),
                    name: "ship".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::None,
                    allegiance: None,
                    sections: vec![
                        SpaceshipSectionConfig {
                            id: "a".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Prototype("known".to_string()),
                            modifications: vec![],
                        },
                        SpaceshipSectionConfig {
                            id: "b".to_string(),
                            position: tube_pos,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Prototype("known".to_string()),
                            modifications: vec![],
                        },
                    ],
                }),
            })
        };

        // The Auditor shape: half-embedded on the spine.
        let s = scenario(vec![ship_with(Vec3::new(0.0, 0.0, 0.5))], vec![]);
        let issues = lint_scenario(&s, &known(&["known"]), &known(&["test_scenario"]));
        assert_eq!(errors(&issues).len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("overlap"));

        // Flush side mount: legal.
        let s = scenario(vec![ship_with(Vec3::new(1.0, 0.0, 0.0))], vec![]);
        let issues = lint_scenario(&s, &known(&["known"]), &known(&["test_scenario"]));
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// The reserved scenario clock (task 20260717-112647): reading it needs
    /// no VariableSet (the engine ticks it), so no unset-variable warning;
    /// WRITING it is always a bug, so an authored VariableSet errors.
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
        let issues = lint_scenario(&read_only, &known(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&stomp, &known(&[]), &known(&["test_scenario"]));
        assert_eq!(
            errors(&issues).len(),
            1,
            "writing the reserved clock is an error: {issues:?}"
        );
        assert!(issues[0].message.contains(SCENARIO_ELAPSED_VAR));
    }
}
