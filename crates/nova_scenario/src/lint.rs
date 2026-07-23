//! Static content lint (task 20260716-191543, spike 20260716-193858): the
//! identifier-level checks no load or publish gate can make, because these
//! references resolve at SPAWN time (a scenario naming a section prototype
//! that does not exist loads green and ships a half-spawning ship).
//!
//! Pure functions over parsed config - no assets, no ECS - so one core
//! serves every consumer: the `content` author CLI's `lint` subcommand
//! (nova_assets bin), the CI gate test, and the runtime merge sweep (task
//! 20260716-193949).
//!
//! Static approximations, documented: a reference matching a
//! `ScatterObjects` id prefix counts as satisfiable (the actual `<prefix><n>`
//! ids exist only at runtime); variable set/use is checked scenario-wide,
//! not in firing order.

use std::collections::HashSet;

use bevy::prelude::Vec3;
use nova_gameplay::prelude::{
    SectionCollider, SectionConfig, SectionKind, TurretJoint, TurretSectionConfig,
};

use crate::prelude::*;

/// Glob-import surface: `use nova_scenario::lint::prelude::*` brings the
/// content-lint entry points and result types into scope.
pub mod prelude {
    pub use super::{lint_scenario, lint_section_config, KnownSections, LintIssue, LintSeverity};
}

/// The section-prototype view a caller lints against: every visible
/// prototype id, plus the subset that MOUNTS - kinds whose model has a base
/// face at local -Y that must sit flush against a neighboring section
/// (turrets and torpedo bays; established from the GLB vertex data in task
/// 20260717-151208's review - the turret turntable and the bay hatch sit at
/// +Y). Built from full configs via [`KnownSections::from_configs`] so the
/// kind classification lives in ONE place for every caller (author CLI
/// walk, CI gate, runtime merge sweep).
#[derive(Clone, Debug, Default)]
pub struct KnownSections {
    /// Every visible section-prototype id.
    pub ids: HashSet<String>,
    /// The ids whose every visible definition is a mount kind. Conservative
    /// on cross-bundle id conflicts (a mod overriding a mount id with a
    /// hull, say): a contested id is NOT treated as a mount, so the
    /// adjacency check can under-flag but never false-fail. The static
    /// walk unions every VISIBLE definition and is where a contested id
    /// can under-flag; the runtime merge gate classifies from the actual
    /// last-wins overlay, so it is the accurate one - conflicting content
    /// can pass CI yet still be refused in-game.
    pub mounts: HashSet<String>,
}

impl KnownSections {
    /// Whether a section kind mounts by its -Y base face.
    pub fn kind_mounts(kind: &SectionKind) -> bool {
        matches!(kind, SectionKind::Turret(_) | SectionKind::Torpedo(_))
    }

    /// Classify full section configs into the catalog view.
    pub fn from_configs<'a>(configs: impl IntoIterator<Item = &'a SectionConfig>) -> Self {
        let mut ids = HashSet::new();
        let mut mounts = HashSet::new();
        let mut non_mounts = HashSet::new();
        for config in configs {
            let id = &config.base.id;
            if Self::kind_mounts(&config.kind) {
                mounts.insert(id.clone());
            } else {
                non_mounts.insert(id.clone());
            }
            ids.insert(id.clone());
        }
        mounts.retain(|id| !non_mounts.contains(id));
        Self { ids, mounts }
    }
}

/// How bad a finding is: `Error` fails gates (the content WILL misbehave),
/// `Warn` is reported but does not fail (almost certainly an authoring bug,
/// but the scenario still runs - e.g. a fails-closed unset variable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LintSeverity {
    /// Fails gates: the content will misbehave at runtime.
    Error,
    /// Reported but non-fatal: almost certainly an authoring bug, but the
    /// scenario still runs.
    Warn,
}

/// One finding, human-readable and self-contained ("scenario 'x': unknown
/// section prototype 'y'").
#[derive(Clone, Debug)]
pub struct LintIssue {
    /// How bad the finding is.
    pub severity: LintSeverity,
    /// The scenario the finding is about.
    pub scenario: ScenarioId,
    /// The human-readable, self-contained description of the finding.
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

    // The beat-sheet convention, mechanized (task 20260717-163058):
    // (a) one story line per beat - a multi-line handler reads as one
    // burst even through the paced queue; (b) a StoryMessage beside an
    // Outcome is a DEAD line - the overlay pauses the comms queue and the
    // chained teardown drops it unread. Fold it into the overlay message
    // or move it to an earlier beat.
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

    // Outcome + non-lingering NextScenario in ONE handler is an authoring
    // trap either way (task 20260717-163050): undelayed, the instant
    // switch tears the world down and SWALLOWS the overlay before it can
    // show (NovaEventWorld::clear's documented footgun); delayed, the
    // overlay's pause freezes the delay clock so the cut never comes
    // while the player reads. Pair Outcome with linger: true (+
    // auto_advance_secs for a timed banner), or drop the Outcome for a
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
            // The template is a full object config too - a scattered ship
            // with a bad prototype is the same bug one wrapper deeper
            // (review R1.1).
            check_object_prototypes(&config.template, scenario, sections, issues);
        }
        EventActionConfig::Outcome(config) => {
            if let Some(secs) = config.auto_advance_secs {
                if !secs.is_finite() || !(0.0..=OUTCOME_AUTO_ADVANCE_MAX_SECS).contains(&secs) {
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
            // Pacing-field sanity (reviews R1.1/R1.5 of 20260717-163050):
            // non-finite or huge delays are runtime-capped, a delay on a
            // LINGERING request is a silently dead field.
            if let Some(delay) = config.delay {
                if config.linger {
                    issues.push(LintIssue::warn(
                        scenario,
                        "NextScenario delay with linger: true is dead (the overlay's \
                         release is the timing) - drop the field or use linger: false"
                            .to_string(),
                    ));
                } else if !delay.is_finite()
                    || !(0.0..=NEXT_SCENARIO_DELAY_WARN_SECS).contains(&delay)
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

/// Every section prototype a spawned (or scatter-template) ship references
/// must exist in the caller's known set.
fn check_object_prototypes(
    config: &ScenarioObjectConfig,
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    if let ScenarioObjectKind::Spaceship(ship) = &config.kind {
        for section in &ship.sections {
            if let SectionSource::Prototype(proto) = &section.source {
                if !sections.ids.contains(proto) {
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
        check_mount_adjacency(config.base.id.as_str(), ship, scenario, sections, issues);
        check_controller_durations(config.base.id.as_str(), ship, scenario, issues);
        // Inline section configs a scenario writes directly (a Prototype ref
        // resolves to a catalog section, which is linted where the catalog is
        // walked - lint_bundle - so it is not re-linted here).
        for section in &ship.sections {
            if let SectionSource::Inline(inline) = &section.source {
                issues.extend(lint_section_config(inline, scenario));
            }
        }
    }
}

/// Author-supplied event-window overrides must be a positive, finite number of
/// seconds - a zero/negative/NaN window would fire the event every frame, so it
/// fails closed (the runtime ignores such a value and uses the engine default,
/// but the content is still wrong). Also warns when an orbit-hold override is
/// set on a ship with no `orbit` directive, where it can never take effect.
/// Task 20260717-165031.
fn check_controller_durations(
    ship_id: &str,
    ship: &SpaceshipConfig,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    let bad = |secs: f64| !secs.is_finite() || secs <= 0.0;
    match &ship.controller {
        SpaceshipController::AI(ai) => {
            if let Some(secs) = ai.orbit_hold_secs {
                if bad(secs) {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "ship '{ship_id}': orbit_hold_secs must be a positive number of seconds, got {secs}"
                        ),
                    ));
                } else if ai.orbit.is_none() {
                    issues.push(LintIssue::warn(
                        scenario,
                        format!(
                            "ship '{ship_id}': orbit_hold_secs is set but the ship has no `orbit` directive, so it never takes effect"
                        ),
                    ));
                }
            }
        }
        SpaceshipController::Player(player) => {
            if let Some(secs) = player.lock_refire_secs {
                if bad(secs) {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "ship '{ship_id}': lock_refire_secs must be a positive number of seconds, got {secs}"
                        ),
                    ));
                }
            }
        }
        SpaceshipController::None => {}
    }
}

/// Static well-formedness of one section's config that the RON parser cannot
/// catch (a well-typed field can still be nonsense). Currently the turret joint
/// tree; other kinds pass. Pure over the config, so every consumer - the author
/// CLI's `lint`, the CI gate, the runtime merge - runs the SAME check on base +
/// mod section catalogs, and `lint_scenario` runs it on inline turret sections.
pub fn lint_section_config(config: &SectionConfig, source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    if let SectionKind::Turret(turret) = &config.kind {
        check_turret_tree(config.base.id.as_str(), turret, source, &mut issues);
    }
    issues
}

/// Walk a turret's joint tree and flag authoring mistakes the parser accepts but
/// the runtime cannot use: a hinge with a degenerate (zero or non-finite) axis
/// or a non-positive traverse speed can never aim, min > max locks the hinge
/// shut, and a tree with no muzzle can never fire (the spawn observer rejects it
/// at runtime). Cheap: one DFS. `min`/`max`/a non-default `speed` on a FIXED
/// node (no `axis`) is a soft warning - harmless (the runtime ignores them) but
/// usually a forgotten `axis`.
fn check_turret_tree(
    section_id: &str,
    config: &TurretSectionConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    fn walk(
        section_id: &str,
        joint: &TurretJoint,
        source: &str,
        issues: &mut Vec<LintIssue>,
    ) -> usize {
        let mut muzzles = usize::from(joint.muzzle.is_some());
        match joint.axis {
            Some(axis) => {
                if !axis.is_finite() || axis.length_squared() < 1e-12 {
                    issues.push(LintIssue::error(
                        source,
                        format!(
                            "section '{section_id}': turret joint has a degenerate hinge axis \
                             {axis:?} - a hinge axis must be a non-zero, finite vector"
                        ),
                    ));
                }
                if !joint.speed.is_finite() || joint.speed <= 0.0 {
                    issues.push(LintIssue::error(
                        source,
                        format!(
                            "section '{section_id}': turret hinge speed must be a positive, \
                             finite number of rad/s, got {}",
                            joint.speed
                        ),
                    ));
                }
                if let (Some(min), Some(max)) = (joint.min, joint.max) {
                    if min > max {
                        issues.push(LintIssue::error(
                            source,
                            format!(
                                "section '{section_id}': turret hinge min {min} exceeds max {max} \
                                 - the hinge is locked shut"
                            ),
                        ));
                    }
                }
            }
            None => {
                if joint.min.is_some() || joint.max.is_some() {
                    issues.push(LintIssue::warn(
                        source,
                        format!(
                            "section '{section_id}': turret joint sets rotation limits but has no \
                             `axis`, so it never rotates - did you forget the hinge axis?"
                        ),
                    ));
                }
            }
        }
        for child in &joint.children {
            muzzles += walk(section_id, child, source, issues);
        }
        muzzles
    }

    let muzzles = walk(section_id, &config.root, source, issues);
    if muzzles == 0 {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': turret has no muzzle joint - it can never fire \
                 (add a `muzzle:` to a leaf joint)"
            ),
        ));
    }
}

/// Two sections of one ship OVERLAP - clip visually and double up their
/// colliders in the same space - iff their axis-aligned collider boxes
/// interpenetrate: centers strictly closer than the sum of their half-extents
/// on EVERY axis. For the default unit-cube sections (half-extent 0.5 each)
/// that is the classic "closer than 1.0 on every axis"; authorable colliders
/// ([`SectionCollider`]) widen or narrow the threshold per section. Flush
/// contact (distance exactly the half-extent sum on some axis) is the normal
/// spine/side-mount layout and passes. The check ignores section ROTATION:
/// exact for the quarter-turn rotations all shipped content uses (a unit cube
/// is symmetric under them; a non-cube box's AABB is a conservative
/// over-approximation), conservative-only for exotic angles. Only INLINE
/// colliders are resolved; a `Prototype` section falls back to the unit cube
/// (the catalog is not in scope here), matching pre-config behavior. Caught in
/// the wild by the Auditor's torpedo bay authored at z 0.5, embedded between
/// two spine sections (task 20260717-151208).
fn check_section_overlaps(
    ship_id: &str,
    ship: &SpaceshipConfig,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    /// AABB half-extents of a section's collider, ignoring rotation. Inline
    /// sources use their authored collider (unit cube when unset); Prototype
    /// sources fall back to the unit cube since the catalog is not resolvable
    /// here.
    fn half_extents(section: &SpaceshipSectionConfig) -> Vec3 {
        match &section.source {
            SectionSource::Inline(config) => {
                config.base.collider.unwrap_or_default().aabb_half_extents()
            }
            SectionSource::Prototype(_) => SectionCollider::default().aabb_half_extents(),
        }
    }

    for i in 0..ship.sections.len() {
        for j in (i + 1)..ship.sections.len() {
            let (a, b) = (&ship.sections[i], &ship.sections[j]);
            let d = a.position - b.position;
            let sum = half_extents(a) + half_extents(b);
            if d.x.abs() < sum.x && d.y.abs() < sum.y && d.z.abs() < sum.z {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ship '{ship_id}': sections '{}' at {:?} and '{}' at {:?} overlap (collider boxes interpenetrate: centers must be >= {:?} apart on some axis)",
                        a.id, a.position, b.id, b.position, sum
                    ),
                ));
            }
        }
    }
}

/// Mount-base adjacency (task 20260717-162121, seeded by review R1.2 of
/// 20260717-151214): a mount section's base face (local -Y) must sit flush
/// against an occupied neighbor cell - `position + rotation * -Y` lands on
/// a sibling section's cell. ANY sibling counts, not just hulls (most
/// shipped ships seat the aft turret's base against the controller cell).
/// All shipped content rotates sections by quarter-turns, so the base
/// direction is axis-aligned; a mount with a non-quarter rotation gets a
/// Warn note and is otherwise skipped (conservative, like the overlap
/// check's rotation caveat). This check would have caught both shipped
/// wrong-roll bugs at authoring time: the Auditor bay bottom-down at a
/// flank cell (task 20260717-151208) and all four gunship side mounts
/// with spine-end rolls (task 20260717-151214).
fn check_mount_adjacency(
    ship_id: &str,
    ship: &SpaceshipConfig,
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    // f32 quat error on authored quarter-turns is ~1e-7 and authored
    // positions sit on the unit grid; the smallest shipped slip (the
    // Auditor bay's 0.5-cell offset) is orders above both epsilons.
    const AXIS_EPS: f32 = 1e-4;
    const CELL_EPS: f32 = 1e-3;
    for section in &ship.sections {
        let is_mount = match &section.source {
            SectionSource::Inline(config) => KnownSections::kind_mounts(&config.kind),
            SectionSource::Prototype(proto) => sections.mounts.contains(proto),
        };
        if !is_mount {
            continue;
        }
        let base_dir = section.rotation * Vec3::NEG_Y;
        let snapped = base_dir.round();
        // A quarter-turn of a UNIT quat sends -Y to a unit axis vector.
        // Anything else - a free angle, or a non-unit hand-typed quat, for
        // which `q * v` is not a rotation at all (a sqrt(2)-scaled
        // quarter-turn yields an INTEGER base direction like (-2, 1, 0)
        // that would pass the deviation test alone; review R1.1) - is
        // statically uncheckable: note and skip. `snapped` components are
        // exact integers, so the length comparison is exact.
        if (base_dir - snapped).abs().max_element() > AXIS_EPS || snapped.length_squared() != 1.0 {
            issues.push(LintIssue::warn(
                scenario,
                format!(
                    "ship '{ship_id}' section '{}': non-quarter-turn (or non-unit) rotation, \
                     mount-base adjacency unchecked (base direction {base_dir:?})",
                    section.id
                ),
            ));
            continue;
        }
        // The section can never satisfy itself: the target cell is a full
        // unit away from its own position.
        let target = section.position + snapped;
        let occupied = ship
            .sections
            .iter()
            .any(|other| (other.position - target).abs().max_element() < CELL_EPS);
        if !occupied {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "ship '{ship_id}' section '{}' at {:?}: mount base (rotation * -Y = \
                     {snapped:?}) points at empty cell {target:?} - a turret/torpedo bay \
                     must sit base-against an occupied neighbor cell",
                    section.id, section.position
                ),
            ));
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

    /// A catalog of known prototype ids, none of them mounts (the shape
    /// every pre-mount-check test wants: the adjacency arm stays silent).
    fn sections(ids: &[&str]) -> KnownSections {
        KnownSections {
            ids: known(ids),
            mounts: HashSet::new(),
        }
    }

    /// A catalog where `mounts` are mount-kind prototypes (also known).
    fn sections_with_mounts(ids: &[&str], mounts: &[&str]) -> KnownSections {
        let mut catalog = sections(ids);
        for id in mounts {
            catalog.ids.insert(id.to_string());
            catalog.mounts.insert(id.to_string());
        }
        catalog
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
    fn unknown_prototype_is_an_error() {
        let s = scenario(vec![spawn_ship("player", "no_such_proto")], vec![]);
        let issues = lint_scenario(&s, &sections(&["known_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
    }

    /// Spawn a ship with an explicit controller and a known section prototype
    /// (so only the controller-duration check can flag it). Task 20260717-165031.
    fn spawn_ship_with_controller(id: &str, controller: SpaceshipController) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                allegiance: None,
                controller,
                sections: vec![SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype("known_proto".to_string()),
                    modifications: vec![],
                }],
            }),
        })
    }

    /// Task 20260717-165031: a non-positive orbit_hold_secs / lock_refire_secs
    /// would fire the event every frame, so it is a fail-closed error; a valid
    /// positive override lints clean.
    #[test]
    fn non_positive_event_window_overrides_are_errors() {
        let bad_orbit = spawn_ship_with_controller(
            "orbiter",
            SpaceshipController::AI(AIControllerConfig {
                orbit: Some("well".to_string()),
                orbit_hold_secs: Some(0.0),
                ..Default::default()
            }),
        );
        let issues = lint_scenario(
            &scenario(vec![bad_orbit], vec![]),
            &sections(&["known_proto"]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("orbit_hold_secs"));

        let bad_lock = spawn_ship_with_controller(
            "player",
            SpaceshipController::Player(PlayerControllerConfig {
                lock_refire_secs: Some(-1.0),
                ..Default::default()
            }),
        );
        let issues = lint_scenario(
            &scenario(vec![bad_lock], vec![]),
            &sections(&["known_proto"]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("lock_refire_secs"));

        // A valid positive override lints clean.
        let ok = spawn_ship_with_controller(
            "orbiter2",
            SpaceshipController::AI(AIControllerConfig {
                orbit: Some("well".to_string()),
                orbit_hold_secs: Some(8.0),
                ..Default::default()
            }),
        );
        let issues = lint_scenario(
            &scenario(vec![ok], vec![]),
            &sections(&["known_proto"]),
            &known(&["test_scenario"]),
        );
        assert!(
            errors(&issues).is_empty(),
            "a positive override should lint clean: {issues:?}"
        );
    }

    /// orbit_hold_secs with no `orbit` directive can never take effect: a warn,
    /// not an error (the scenario still runs). Task 20260717-165031.
    #[test]
    fn orbit_hold_without_orbit_directive_warns() {
        let s = scenario(
            vec![spawn_ship_with_controller(
                "drifter",
                SpaceshipController::AI(AIControllerConfig {
                    orbit: None,
                    orbit_hold_secs: Some(3.0),
                    ..Default::default()
                }),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&["known_proto"]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "should not error: {issues:?}");
        assert!(
            issues
                .iter()
                .any(|i| i.severity == LintSeverity::Warn && i.message.contains("orbit_hold_secs")),
            "expected a warn about orbit_hold_secs without orbit: {issues:?}"
        );
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
        let issues = lint_scenario(&s, &sections(&["known_proto"]), &known(&["test_scenario"]));
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
                delay: None,
            })],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("gone"));
    }

    /// Review R1.1 (task 20260723-000253): SetAllegiance references a ship by
    /// id like SetSpeedCap does, so a typo'd id must lint as a dangling
    /// target, not silently no-op at runtime.
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

    /// Outcome + non-lingering NextScenario in one handler warns (task
    /// 20260717-163050): undelayed it swallows the overlay, delayed it
    /// freezes under the pause. The lingering pair stays clean.
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

    /// The beat-sheet arms (task 20260717-163058): double lines warn,
    /// story-beside-outcome warns, one line per handler is clean.
    #[test]
    fn beat_sheet_arms_warn() {
        let line = |text: &str| {
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: text.to_string(),
                dwell: None,
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

    /// Pacing-field ranges (reviews R1.1/R1.5 of 20260717-163050):
    /// absurd/non-finite delays warn, a delay on a lingering request is
    /// dead and warns, sane values stay clean.
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

        // Sane values, trap-free shapes: clean.
        let s = scenario(vec![next(false, Some(4.0))], vec![]);
        assert!(lint_scenario(&s, &sections(&[]), &known(&["test_scenario"])).is_empty());
        let s = scenario(vec![outcome_adv(Some(6.0))], vec![]);
        assert!(lint_scenario(&s, &sections(&[]), &known(&["test_scenario"])).is_empty());
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
        let issues = lint_scenario(&s, &sections(&["known"]), &known(&["test_scenario"]));
        assert_eq!(errors(&issues).len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("overlap"));

        // Flush side mount: legal.
        let s = scenario(vec![ship_with(Vec3::new(1.0, 0.0, 0.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&["known"]), &known(&["test_scenario"]));
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// Authorable colliders (task 20260718-102022) move the overlap threshold:
    /// two sections 0.8 apart clip as default unit cubes but sit flush once both
    /// tighten to a 0.8 cube, and oversized colliders clip where unit cubes
    /// would not. Only INLINE colliders are resolved; prototypes fall back to
    /// the unit cube.
    #[test]
    fn overlap_uses_authored_collider_half_extents() {
        use nova_gameplay::prelude::{BaseSectionConfig, HullSectionConfig};

        // An inline hull section at `pos` with the given collider.
        let inline =
            |id: &str, pos: Vec3, collider: Option<SectionCollider>| SpaceshipSectionConfig {
                id: id.to_string(),
                position: pos,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(SectionConfig {
                    base: BaseSectionConfig {
                        collider,
                        ..Default::default()
                    },
                    kind: SectionKind::Hull(HullSectionConfig {
                        render_mesh: None,
                        render_mesh_transform: None,
                    }),
                }),
                modifications: vec![],
            };

        let ship = |a: SpaceshipSectionConfig, b: SpaceshipSectionConfig| {
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
                    sections: vec![a, b],
                }),
            })
        };

        let cube = |n: f32| {
            Some(SectionCollider::Cuboid {
                size: Vec3::splat(n),
            })
        };
        let x = |n: f32| Vec3::new(n, 0.0, 0.0);

        // 0.8 apart, default unit cubes: half-extents sum to 1.0 > 0.8 -> overlap.
        let s = scenario(
            vec![ship(
                inline("a", Vec3::ZERO, None),
                inline("b", x(0.8), None),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(
            errors(&issues).len(),
            1,
            "unit cubes should clip: {issues:?}"
        );
        assert!(issues[0].message.contains("overlap"));

        // Same spacing, both tightened to 0.8 cubes: sum 0.8 == distance -> flush.
        let s = scenario(
            vec![ship(
                inline("a", Vec3::ZERO, cube(0.8)),
                inline("b", x(0.8), cube(0.8)),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert!(issues.is_empty(), "tightened cubes are flush: {issues:?}");

        // 1.5 apart, oversized 2.0 cubes: sum 2.0 > 1.5 -> overlap where unit
        // cubes (sum 1.0) would pass.
        let s = scenario(
            vec![ship(
                inline("a", Vec3::ZERO, cube(2.0)),
                inline("b", x(1.5), cube(2.0)),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(errors(&issues).len(), 1, "oversized cubes clip: {issues:?}");
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

    /// The mount-fixture ship (task 20260717-162121): a hull cell at the
    /// origin plus one MOUNT prototype at `pos`/`rotation`. Catalog for
    /// these tests: `sections_with_mounts(&["hull_proto"], &["mount_proto"])`.
    fn ship_with_mount(pos: Vec3, rotation: Quat) -> EventActionConfig {
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
                        id: "hull".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        source: SectionSource::Prototype("hull_proto".to_string()),
                        modifications: vec![],
                    },
                    SpaceshipSectionConfig {
                        id: "mount".to_string(),
                        position: pos,
                        rotation,
                        source: SectionSource::Prototype("mount_proto".to_string()),
                        modifications: vec![],
                    },
                ],
            }),
        })
    }

    /// Task 20260717-162121: every shipped mount-roll shape lints clean -
    /// flank mounts with inboard Rz rolls, a top mount's identity, and the
    /// bow mount's Rx(-90) (base against the cell astern of it).
    #[test]
    fn mount_bases_against_occupied_cells_are_clean() {
        use std::f32::consts::FRAC_PI_2;
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        for (pos, rotation) in [
            // Starboard flank, base rolled inboard (-Y -> -X).
            (Vec3::new(1.0, 0.0, 0.0), Quat::from_rotation_z(-FRAC_PI_2)),
            // Port flank, the mirror roll (-Y -> +X).
            (Vec3::new(-1.0, 0.0, 0.0), Quat::from_rotation_z(FRAC_PI_2)),
            // Top mount, identity: base straight down at the hull.
            (Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
            // Bow mount, the player-ship roll: base astern (-Y -> +Z).
            (Vec3::new(0.0, 0.0, -1.0), Quat::from_rotation_x(-FRAC_PI_2)),
        ] {
            let s = scenario(vec![ship_with_mount(pos, rotation)], vec![]);
            let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
            assert!(issues.is_empty(), "mount at {pos:?} flagged: {issues:?}");
        }
    }

    /// The two shipped wrong-roll shapes are errors: the Auditor bay
    /// bottom-down on a flank cell (identity roll, base into empty space,
    /// task 20260717-151208) and the gunship side mounts with the spine-end
    /// Rx(-90) roll (base astern instead of inboard, task 20260717-151214).
    #[test]
    fn mount_base_at_an_empty_cell_is_an_error() {
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        for (pos, rotation) in [
            (Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY),
            (
                Vec3::new(1.0, 0.0, 0.0),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
        ] {
            let s = scenario(vec![ship_with_mount(pos, rotation)], vec![]);
            let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
            let errs = errors(&issues);
            assert_eq!(errs.len(), 1, "mount rot {rotation:?}: {issues:?}");
            assert!(errs[0].message.contains("mount base"), "{issues:?}");
        }
    }

    /// A non-quarter-turn - or non-unit (review R1.1: `q * v` is not a
    /// rotation for a non-unit hand-typed quat, and a sqrt(2)-scaled
    /// quarter-turn snaps to an integer NON-UNIT direction that the
    /// deviation test alone would accept) - mount rotation is skipped with
    /// a Warn note, never errored: the static check cannot reason about
    /// either (the same conservative caveat as the overlap check's).
    #[test]
    fn mount_with_non_quarter_rotation_warns_and_skips() {
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        for rotation in [
            Quat::from_rotation_z(0.7),
            // Rz(-90) scaled by sqrt(2): base_dir snaps to (-2, 1, 0).
            Quat::from_xyzw(0.0, 0.0, -1.0, 1.0),
        ] {
            let s = scenario(
                vec![ship_with_mount(Vec3::new(1.0, 0.0, 0.0), rotation)],
                vec![],
            );
            let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
            assert!(
                errors(&issues).is_empty(),
                "warn-only for {rotation:?}: {issues:?}"
            );
            assert_eq!(issues.len(), 1, "{issues:?}");
            assert!(issues[0].message.contains("non-quarter"), "{issues:?}");
        }
    }

    /// Occupancy is kind-blind (review R1.4): a mount seated base-against
    /// ANOTHER MOUNT's cell passes - any sibling section counts, matching
    /// the shipped ships that seat turrets against the controller cell.
    #[test]
    fn mount_seated_against_another_mount_is_clean() {
        use std::f32::consts::FRAC_PI_2;
        let inboard = Quat::from_rotation_z(-FRAC_PI_2);
        let mount = |id: &str, pos: Vec3| SpaceshipSectionConfig {
            id: id.to_string(),
            position: pos,
            rotation: inboard,
            source: SectionSource::Prototype("mount_proto".to_string()),
            modifications: vec![],
        };
        let s = scenario(
            vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
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
                                id: "hull".to_string(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                                source: SectionSource::Prototype("hull_proto".to_string()),
                                modifications: vec![],
                            },
                            // Inner mount seats against the hull; the outer one
                            // seats against the INNER MOUNT.
                            mount("mount_inner", Vec3::new(1.0, 0.0, 0.0)),
                            mount("mount_outer", Vec3::new(2.0, 0.0, 0.0)),
                        ],
                    }),
                },
            )],
            vec![],
        );
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// An INLINE mount section is checked from its own kind - no catalog
    /// membership involved.
    #[test]
    fn inline_mount_sections_are_checked() {
        use nova_gameplay::prelude::{BaseSectionConfig, TurretSectionConfig};

        let mut action = ship_with_mount(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);
        let EventActionConfig::SpawnScenarioObject(config) = &mut action else {
            unreachable!()
        };
        let ScenarioObjectKind::Spaceship(ship) = &mut config.kind else {
            unreachable!()
        };
        ship.sections[1].source = SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig::default(),
            kind: SectionKind::Turret(TurretSectionConfig::default()),
        });
        let s = scenario(vec![action], vec![]);
        let issues = lint_scenario(&s, &sections(&["hull_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("mount base"), "{issues:?}");
    }

    #[test]
    fn turret_joint_tree_wellformedness_is_linted() {
        use nova_gameplay::prelude::{
            BaseSectionConfig, MuzzleConfig, TurretJoint, TurretSectionConfig,
        };

        fn joint(
            axis: Option<Vec3>,
            min: Option<f32>,
            max: Option<f32>,
            muzzle: bool,
            children: Vec<TurretJoint>,
        ) -> TurretJoint {
            TurretJoint {
                offset: Vec3::ZERO,
                axis,
                speed: std::f32::consts::PI,
                min,
                max,
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: muzzle.then(|| MuzzleConfig {
                    fire_rate: 10.0,
                    muzzle_effect: None,
                }),
                children,
            }
        }
        let turret = |root: TurretJoint| SectionConfig {
            base: BaseSectionConfig {
                id: "t".to_string(),
                ..Default::default()
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                root,
                ..Default::default()
            }),
        };

        // Valid: a hinge over a muzzle leaf, and the shipped default, pass clean.
        let ok = joint(
            Some(Vec3::Y),
            None,
            None,
            false,
            vec![joint(None, None, None, true, vec![])],
        );
        assert!(lint_section_config(&turret(ok), "s").is_empty());
        assert!(
            lint_section_config(&turret(TurretSectionConfig::default().root), "s").is_empty(),
            "the shipped default turret must lint clean"
        );

        // No muzzle anywhere -> error (can never fire).
        let none = joint(Some(Vec3::Y), None, None, false, vec![]);
        assert!(errors(&lint_section_config(&turret(none), "s"))
            .iter()
            .any(|i| i.message.contains("no muzzle")));

        // Degenerate hinge axis -> error.
        let zero = joint(
            Some(Vec3::ZERO),
            None,
            None,
            false,
            vec![joint(None, None, None, true, vec![])],
        );
        assert!(errors(&lint_section_config(&turret(zero), "s"))
            .iter()
            .any(|i| i.message.contains("degenerate hinge axis")));

        // min > max -> error (locked shut).
        let inverted = joint(
            Some(Vec3::X),
            Some(1.0),
            Some(-1.0),
            false,
            vec![joint(None, None, None, true, vec![])],
        );
        assert!(errors(&lint_section_config(&turret(inverted), "s"))
            .iter()
            .any(|i| i.message.contains("exceeds max")));

        // Rotation limits on a FIXED node -> warning, not error.
        let limits_no_axis = joint(None, Some(-1.0), Some(1.0), true, vec![]);
        let issues = lint_section_config(&turret(limits_no_axis), "s");
        assert!(errors(&issues).is_empty(), "{issues:?}");
        assert!(
            issues.iter().any(|i| i.message.contains("no `axis`")),
            "{issues:?}"
        );
    }

    /// `KnownSections::from_configs` classifies turret/torpedo kinds as
    /// mounts and everything else as plain sections - and an id CONFLICT
    /// (one definition a mount, another not) conservatively drops the id
    /// from the mount set rather than risking a false Error.
    #[test]
    fn section_catalog_classifies_mount_kinds() {
        use nova_gameplay::prelude::{
            BaseSectionConfig, ControllerSectionConfig, HullSectionConfig, ThrusterSectionConfig,
            TorpedoSectionConfig, TurretSectionConfig,
        };

        let section = |id: &str, kind: SectionKind| SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                ..Default::default()
            },
            kind,
        };
        let configs = vec![
            section("hull", SectionKind::Hull(HullSectionConfig::default())),
            section(
                "thruster",
                SectionKind::Thruster(ThrusterSectionConfig::default()),
            ),
            section(
                "controller",
                SectionKind::Controller(ControllerSectionConfig::default()),
            ),
            section(
                "turret",
                SectionKind::Turret(TurretSectionConfig::default()),
            ),
            section(
                "torpedo",
                SectionKind::Torpedo(TorpedoSectionConfig::default()),
            ),
            // The same id defined as a mount in one bundle, a hull in another.
            section(
                "contested",
                SectionKind::Turret(TurretSectionConfig::default()),
            ),
            section("contested", SectionKind::Hull(HullSectionConfig::default())),
        ];
        let catalog = KnownSections::from_configs(&configs);
        assert_eq!(catalog.ids.len(), 6, "{catalog:?}");
        assert_eq!(
            catalog.mounts,
            known(&["turret", "torpedo"]),
            "only uncontested mount kinds: {catalog:?}"
        );
    }
}
