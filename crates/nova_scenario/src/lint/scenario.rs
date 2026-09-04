//! Reference and pacing checks over one scenario or campaign config.

use std::collections::{HashMap, HashSet};

use nova_gameplay::prelude::SectionClass;

use super::{ship::check_object_prototypes, KnownSections, KnownShips, LintIssue};
use crate::prelude::*;

/// Everything a scenario's actions can DECLARE, collected in one pass:
/// spawnable entity ids (spawns + areas), scatter prefixes, set variables,
/// posted objective ids.
#[derive(Default)]
struct Declared {
    spawn_ids: Vec<String>,
    scatter_prefixes: Vec<String>,
    set_vars: HashSet<String>,
    timer_keys: HashSet<String>,
    order_keys: HashSet<String>,
    objective_ids: HashSet<String>,
    completed_objectives: HashSet<String>,
    /// The ships this scenario spawns BY NAME, so a check can ask what one of
    /// them is built from and who drives it. Scattered ships are absent on
    /// purpose: their ids are `<prefix><n>`, minted at runtime, so nothing
    /// static can resolve one.
    spawned_ships: HashMap<String, SpawnedShip>,
}

/// What the lint knows about one ship a spawn declares.
struct SpawnedShip {
    hull: ShipSource,
    controller: SpaceshipController,
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
/// bundle), `ships` (the ship catalog it may spawn by id) and
/// `known_scenarios` (every scenario id a `NextScenario` may target, normally
/// base + all installed bundles).
pub fn lint_scenario(
    scenario: &ScenarioConfig,
    sections: &KnownSections,
    ships: &KnownShips,
    known_scenarios: &HashSet<String>,
) -> Vec<LintIssue> {
    let id = scenario.id.as_str();
    let mut issues = Vec::new();

    // A menu backdrop poses its OWN camera (the SetCamera contract): the
    // menu strips the loader's flyable camera and shows nothing until the
    // backdrop's scripted pose lands, so a poseless backdrop would sit on a
    // blank camera forever. An ERROR on purpose: erroring scenarios are
    // filtered out of the menu draw, so the broken backdrop degrades to
    // "not in the rotation" instead of "menu with no picture".
    if scenario.menu_backdrop
        && !scenario
            .events
            .iter()
            .flat_map(|event| &event.actions)
            .any(|action| matches!(action, EventActionConfig::SetCamera(_)))
    {
        issues.push(LintIssue::error(
            id,
            "menu backdrop authors no SetCamera; the menu camera would never be posed".to_string(),
        ));
    }

    let mut watch_names = HashSet::new();
    for watch in &scenario.watches {
        if watch.variable.trim().is_empty() {
            issues.push(LintIssue::error(
                id,
                "watch has an empty variable name".to_string(),
            ));
        }
        if !watch_names.insert(watch.variable.clone()) {
            issues.push(LintIssue::error(
                id,
                format!("duplicate watch variable '{}'", watch.variable),
            ));
        }
        if let QueryConfig::Entity(query) = &watch.query {
            if query.filter.id.trim().is_empty() {
                issues.push(LintIssue::error(
                    id,
                    format!("watch '{}' has an empty entity id", watch.variable),
                ));
            }
        }
    }

    // Pass 1: what the scenario declares. Spawn ids and sequence keys are
    // tracked per event so the duplicate checks can tell a definite bug from a
    // branch pattern.
    let mut declared = Declared::default();
    let mut spawns_per_event: Vec<Vec<String>> = Vec::new();
    let mut sequences_per_event: Vec<Vec<String>> = Vec::new();
    for event in &scenario.events {
        let mut event_spawns = Vec::new();
        let mut event_sequences = Vec::new();
        for action in &event.actions {
            action.walk(&mut |action| {
                collect_declared(action, &mut declared);
                match action {
                    EventActionConfig::SpawnScenarioObject(config) => {
                        event_spawns.push(config.base.id.clone());
                    }
                    EventActionConfig::CreateScenarioArea(config) => {
                        event_spawns.push(config.id.clone());
                    }
                    EventActionConfig::Sequence(config) => {
                        check_sequence(config, id, &mut issues);
                        if !config.key.trim().is_empty() {
                            event_sequences.push(config.key.clone());
                        }
                    }
                    _ => {}
                }
            });
        }
        spawns_per_event.push(event_spawns);
        sequences_per_event.push(event_sequences);
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

    // Duplicate sequence keys, within ONE handler's action list only: two starts
    // there definitely race for one cursor and the engine refuses the second.
    //
    // Across handlers a shared key is the INTENDED shape, not a smell: every win
    // variant of a scenario starts the same outro chain, and only one of them
    // can ever fire. Nothing static can tell that apart from a real collision,
    // so the runtime holds that half - `start_sequence` refuses a live key and
    // says so - and the gate stays quiet rather than warning on the pattern the
    // mainline uses everywhere.
    for event_sequences in &sequences_per_event {
        let mut seen = HashSet::new();
        for key in event_sequences {
            if !seen.insert(key.as_str()) {
                issues.push(LintIssue::error(
                    id,
                    format!(
                        "duplicate Sequence key '{key}' within one handler; the engine \
                         holds ONE cursor per key, so the second start is refused"
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

    for watch in &scenario.watches {
        check_query(&watch.query, id, &satisfiable, &mut issues);
    }
    for query in scenario.inline_queries() {
        check_query(query, id, &satisfiable, &mut issues);
    }

    // Pass 2: what the scenario references.
    let mut used_vars: HashSet<String> = HashSet::new();
    for event in &scenario.events {
        for filter in &event.filters {
            check_filter(
                filter,
                id,
                &satisfiable,
                &declared,
                &mut used_vars,
                &mut issues,
            );
        }
        for action in &event.actions {
            action.walk_filters(&mut |filter| {
                check_filter(
                    filter,
                    id,
                    &satisfiable,
                    &declared,
                    &mut used_vars,
                    &mut issues,
                );
            });
            action.walk(&mut |action| {
                check_action(
                    action,
                    id,
                    sections,
                    ships,
                    known_scenarios,
                    &satisfiable,
                    &declared,
                    &mut used_vars,
                    &mut issues,
                );
            });
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
    for group in scenario.events.iter().flat_map(|e| e.action_groups()) {
        let story_lines = group
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
            && group
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
    for group in scenario.events.iter().flat_map(|e| e.action_groups()) {
        let has_outcome = group
            .iter()
            .any(|a| matches!(a, EventActionConfig::Outcome(_)));
        let hard_switch = group
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

    // The regression rule stage 2 pays for. An `OnUpdate` handler whose filters
    // read NOTHING but the scenario clock is a hand-rolled delay: the pulse
    // walks it every frame to ask a question the engine now answers itself -
    // `after:` on a `Sequence` step for a beat in a chain, a keyed timer for a
    // one-off wait. Only pure clock polling is flagged; a value-gated milestone
    // (a tally, a distance) is a legitimate `OnUpdate` and stays silent.
    let clock_watches: HashSet<&str> = scenario
        .watches
        .iter()
        .filter(|watch| {
            matches!(&watch.query, QueryConfig::Scenario(query)
                if query.property == ScenarioProperty::Elapsed)
        })
        .map(|watch| watch.variable.as_str())
        .collect();
    for event in &scenario.events {
        if !matches!(event.name, EventConfig::OnUpdate) || event.filters.is_empty() {
            continue;
        }
        let mut names = HashSet::new();
        let mut queries = Vec::new();
        let mut expressions_only = true;
        for filter in &event.filters {
            match filter {
                EventFilterConfig::Expression(config) => {
                    collect_condition_vars(&config.0, &mut names);
                    collect_condition_queries(&config.0, &mut queries);
                }
                _ => expressions_only = false,
            }
        }
        let reads_only_the_clock = names
            .iter()
            .all(|name| clock_watches.contains(name.as_str()))
            && queries.iter().all(|query| {
                matches!(query, QueryConfig::Scenario(query)
                    if query.property == ScenarioProperty::Elapsed)
            });
        if expressions_only && reads_only_the_clock && !(names.is_empty() && queries.is_empty()) {
            issues.push(LintIssue::warn(
                id,
                "an OnUpdate handler filtered on nothing but the scenario clock is a \
                 hand-rolled delay walked every frame - use a Sequence step's `after`, \
                 or a keyed timer with OnTimerEnd"
                    .to_string(),
            ));
        }

        // The other half of the same regression, which the rule above cannot
        // see because these handlers read real state too: comparing the clock
        // against anything but a LITERAL is a stopwatch written by hand - a
        // `deadline = now + 3.5` stamp somewhere else, and this filter watching
        // for it. A keyed timer says it in one action and one event, and
        // nothing has to poll the clock to find out.
        if event
            .filters
            .iter()
            .any(|filter| compares_the_clock_to_a_non_literal(filter, &clock_watches))
        {
            issues.push(LintIssue::warn(
                id,
                "an OnUpdate handler comparing the scenario clock against a variable is a \
                 hand-rolled stopwatch - start a keyed timer and react to OnTimerEnd"
                    .to_string(),
            ));
        }
    }

    for var in &used_vars {
        if matches!(var.as_str(), "scenario_elapsed" | "player_speed") && !watch_names.contains(var)
        {
            issues.push(LintIssue::error(
                id,
                format!(
                    "legacy engine variable '{var}'; declare a typed watch with this variable name"
                ),
            ));
            continue;
        }
        if !declared.set_vars.contains(var) && !watch_names.contains(var) {
            issues.push(LintIssue::warn(
                id,
                format!(
                    "expression variable '{var}' is never set in this scenario \
                     (filters on it fail closed)"
                ),
            ));
        }
    }

    for name in declared.set_vars.intersection(&watch_names) {
        issues.push(LintIssue::error(
            id,
            format!("VariableSet writes watched variable '{name}'"),
        ));
    }

    issues
}

/// Every action list that runs as ONE beat: a handler's own list, and each
/// `Sequence` step's, at any depth.
///
/// The beat-sheet checks count per group rather than per handler. A five-step
/// sequence is five beats spaced by the scenario clock, so its five story lines
/// are the convention being followed, not broken.
/// Check one `Sequence`'s own shape: a non-empty key, at least one step, and a
/// deadline on every step that waits for an event.
///
/// Key UNIQUENESS is not decided here - it needs the whole scenario, and it
/// reads per handler (see the duplicate rules in [`lint_scenario`]).
/// One spawned rock says what it is made of, and says something real.
///
/// There is no default kind and no fallback for an unknown one, so an id this
/// build does not ship is a body that renders as nothing. Catching it here is
/// the difference between a message naming the object and a hole in a frame.
fn check_asteroid_kind(config: &ScenarioObjectConfig, scenario: &str, issues: &mut Vec<LintIssue>) {
    let ScenarioObjectKind::Asteroid(asteroid) = &config.kind else {
        return;
    };
    if !is_asteroid_kind(&asteroid.material) {
        issues.push(LintIssue::error(
            scenario,
            format!(
                "asteroid '{}': '{}' is not a kind - author one of {:?}",
                config.base.id, asteroid.material, ASTEROID_KINDS
            ),
        ));
    }
}

/// A scattered asteroid field's kind mix: present, weighted, and made of kinds
/// that exist.
///
/// A mix on a template that is not an asteroid is an error too, not a harmless
/// no-op: it is an author who believes they said something about their field.
fn check_scatter_kind_mix(
    config: &ScatterObjectsConfig,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    let field = &config.id_prefix;
    if !matches!(config.template.kind, ScenarioObjectKind::Asteroid(_)) {
        if !config.asteroid_kinds.is_empty() {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "ScatterObjects '{field}' authors an asteroid kind mix on a template \
                     that is not an asteroid, so the mix does nothing"
                ),
            ));
        }
        return;
    }

    if config.asteroid_kinds.is_empty() {
        issues.push(LintIssue::error(
            scenario,
            format!(
                "ScatterObjects '{field}' scatters asteroids with no `asteroid_kinds` mix - \
                 a field says what it is made of; author at least one of {ASTEROID_KINDS:?}"
            ),
        ));
        return;
    }

    for (kind, weight) in &config.asteroid_kinds {
        if !is_asteroid_kind(kind) {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "ScatterObjects '{field}': '{kind}' is not a kind - \
                     author one of {ASTEROID_KINDS:?}"
                ),
            ));
        }
        if *weight == 0 {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "ScatterObjects '{field}': '{kind}' is weighted 0, so it never appears - \
                     give it a share or take it out"
                ),
            ));
        }
    }
}

fn check_sequence(config: &SequenceActionConfig, scenario: &str, issues: &mut Vec<LintIssue>) {
    if config.key.trim().is_empty() {
        issues.push(LintIssue::error(
            scenario,
            "Sequence has an empty key".to_string(),
        ));
    }
    if config.steps.is_empty() {
        issues.push(LintIssue::error(
            scenario,
            format!("Sequence '{}' has no steps", config.key),
        ));
    }
    for (index, step) in config.steps.iter().enumerate() {
        if step.until.is_some() && step.deadline.is_none() {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "Sequence '{}' step {index} waits for an event with no deadline; \
                     a gate that never opens is a silent soft-lock",
                    config.key
                ),
            ));
        }
        for field in [("after", step.after), ("deadline", step.deadline)] {
            if let (name, Some(secs)) = field {
                if !secs.is_finite() || secs < 0.0 {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "Sequence '{}' step {index} has a {name} of {secs}s; \
                             it must be a finite, non-negative number of seconds",
                            config.key
                        ),
                    ));
                }
            }
        }
        if step.until.is_none() && step.deadline.is_some() {
            issues.push(LintIssue::warn(
                scenario,
                format!(
                    "Sequence '{}' step {index} has a deadline but nothing to wait \
                     for; the deadline is dead unless the step has an `until` gate",
                    config.key
                ),
            ));
        }
    }
}

fn collect_declared(action: &EventActionConfig, declared: &mut Declared) {
    match action {
        EventActionConfig::SpawnScenarioObject(config) => {
            declared.spawn_ids.push(config.base.id.clone());
            if let ScenarioObjectKind::Spaceship(ship) = &config.kind {
                declared.spawned_ships.insert(
                    config.base.id.clone(),
                    SpawnedShip {
                        hull: ship.hull.clone(),
                        controller: ship.controller.clone(),
                    },
                );
            }
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
        EventActionConfig::TimerStart(config) => {
            declared.timer_keys.insert(config.key.clone());
        }
        EventActionConfig::MoveShipTo(config) => {
            declared.order_keys.insert(config.order.clone());
        }
        EventActionConfig::ForceAlign(config) => {
            declared.order_keys.insert(config.order.clone());
        }
        EventActionConfig::StopShip(config) => {
            declared.order_keys.insert(config.order.clone());
        }
        EventActionConfig::PatrolShip(config) => {
            declared.order_keys.insert(config.order.clone());
        }
        EventActionConfig::OrbitShip(config) => {
            declared.order_keys.insert(config.order.clone());
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

#[expect(
    clippy::too_many_arguments,
    reason = "one pass over every catalog the lint reads"
)]
fn check_action(
    action: &EventActionConfig,
    scenario: &str,
    sections: &KnownSections,
    ships: &KnownShips,
    known_scenarios: &HashSet<String>,
    satisfiable: &dyn Fn(&str) -> bool,
    declared: &Declared,
    used_vars: &mut HashSet<String>,
    issues: &mut Vec<LintIssue>,
) {
    match action {
        EventActionConfig::SpawnScenarioObject(config) => {
            check_object_prototypes(config, scenario, sections, ships, issues);
            check_spawned_arrival_standoff(config, scenario, issues);
            check_asteroid_kind(config, scenario, issues);
            check_planet(config, scenario, issues);
        }
        EventActionConfig::ScatterObjects(config) => {
            // The template is a full object config too - a scattered ship with
            // a bad prototype is the same bug one wrapper deeper.
            check_object_prototypes(&config.template, scenario, sections, ships, issues);
            check_spawned_arrival_standoff(&config.template, scenario, issues);
            check_scatter_kind_mix(config, scenario, issues);
            check_planet(&config.template, scenario, issues);
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
            } else if !declared.timer_keys.contains(&config.key) {
                issues.push(LintIssue::warn(
                    scenario,
                    format!(
                        "TimerCancel references timer '{}', which no TimerStart creates",
                        config.key
                    ),
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
        EventActionConfig::MoveShipTo(config) => {
            check_orderable_ship(
                &config.ship,
                "MoveShipTo",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_order_key(&config.order, "MoveShipTo", scenario, issues);
            check_arrival_standoff(
                config.arrival_standoff,
                &format!("MoveShipTo '{}'", config.order),
                scenario,
                issues,
            );
        }
        EventActionConfig::ForceAlign(config) => {
            check_orderable_ship(
                &config.ship,
                "ForceAlign",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_order_key(&config.order, "ForceAlign", scenario, issues);
            // A negative or non-finite tolerance can never be met, so the
            // order never completes and every beat chained off it stalls
            // until its deadline. Zero is legal but asks for a perfect aim.
            if !config.tolerance_degrees.is_finite() || config.tolerance_degrees < 0.0 {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ForceAlign '{}' tolerance_degrees must be a non-negative finite \
                         number, got {}; the order could never complete",
                        config.order, config.tolerance_degrees
                    ),
                ));
            }
        }
        EventActionConfig::StopShip(config) => {
            check_orderable_ship(
                &config.ship,
                "StopShip",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_order_key(&config.order, "StopShip", scenario, issues);
        }
        EventActionConfig::PatrolShip(config) => {
            check_orderable_ship(
                &config.ship,
                "PatrolShip",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_order_key(&config.order, "PatrolShip", scenario, issues);
            // No waypoints is no loop, so the order is refused at runtime and
            // every beat chained off its completion stalls. Catchable here.
            if config.waypoints.is_empty() {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "PatrolShip '{}' has no waypoints; there is no loop to fly and \
                         nothing could wait for it",
                        config.order
                    ),
                ));
            }
        }
        EventActionConfig::OrbitShip(config) => {
            check_orderable_ship(
                &config.ship,
                "OrbitShip",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_order_key(&config.order, "OrbitShip", scenario, issues);
            check_target(&config.well, "OrbitShip", scenario, satisfiable, issues);
        }
        EventActionConfig::ClearShipOrder(config) => {
            check_orderable_ship(
                &config.ship,
                "ClearShipOrder",
                scenario,
                satisfiable,
                declared,
                issues,
            );
        }
        EventActionConfig::SetAILeash(config) => {
            check_ai_ship(
                &config.ship,
                "SetAILeash",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            if let Some(leash) = &config.leash {
                if !leash.radius.0.is_finite() || leash.radius.0 <= 0.0 {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "SetAILeash on '{}' needs a positive finite radius in meters, got {}",
                            config.ship, leash.radius.0
                        ),
                    ));
                }
            }
        }
        EventActionConfig::SetAIEngageRange(config) => {
            check_ai_ship(
                &config.ship,
                "SetAIEngageRange",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_ai_range(
                config.range,
                "SetAIEngageRange",
                &config.ship,
                scenario,
                issues,
            );
        }
        EventActionConfig::SetAIPointDefenseRange(config) => {
            check_ai_ship(
                &config.ship,
                "SetAIPointDefenseRange",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_ai_range(
                config.range,
                "SetAIPointDefenseRange",
                &config.ship,
                scenario,
                issues,
            );
        }
        EventActionConfig::ForceRailgunFire(config) => {
            check_scripted_ship(
                &config.ship,
                "ForceRailgunFire",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_ship_section(
                &config.ship,
                &config.section,
                SectionClass::Railgun,
                "ForceRailgunFire",
                scenario,
                sections,
                ships,
                declared,
                issues,
            );
        }
        EventActionConfig::ForceTorpedoFire(config) => {
            check_scripted_ship(
                &config.ship,
                "ForceTorpedoFire",
                scenario,
                satisfiable,
                declared,
                issues,
            );
            check_target(
                &config.target,
                "ForceTorpedoFire",
                scenario,
                satisfiable,
                issues,
            );
            check_ship_section(
                &config.ship,
                &config.section,
                SectionClass::Torpedo,
                "ForceTorpedoFire",
                scenario,
                sections,
                ships,
                declared,
                issues,
            );
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

/// A scripted action's actor: the id must resolve, and nothing else may be
/// driving it.
///
/// The controller half is catchable at AUTHOR time and so is checked here
/// rather than left to a runtime error: the id is already proven to be spawned
/// by this scenario, which means its `SpaceshipController` is sitting in the
/// same file. A ship this scenario does not spawn by name (a scattered one)
/// gets the id check only - there is nothing static to read.
fn check_scripted_ship(
    ship: &str,
    what: &str,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    declared: &Declared,
    issues: &mut Vec<LintIssue>,
) {
    check_target(ship, what, scenario, satisfiable, issues);
    let Some(spawned) = declared.spawned_ships.get(ship) else {
        return;
    };
    let driver = match &spawned.controller {
        SpaceshipController::None => return,
        SpaceshipController::Player(_) => "player-driven",
        SpaceshipController::AI(_) => "AI-driven",
    };
    issues.push(LintIssue::error(
        scenario,
        format!(
            "{what} targets ship '{ship}', which is {driver}; a scripted action only \
             drives a `SpaceshipController::None` ship (use `non_combatant` for an \
             armed ship that flies itself and never shoots)"
        ),
    ));
}

/// A helm order's actor: the id must resolve, and the player must not be
/// flying it.
///
/// Only the player is refused. An order works the same on a
/// `SpaceshipController::None` actor and on an AI ship - that is the whole
/// point of the shared mission layer - so the AI case that used to be an error
/// here is now ordinary authoring. The player case stays catchable at AUTHOR
/// time because the id is already proven to be spawned by this scenario, which
/// means its `SpaceshipController` is sitting in the same file.
fn check_orderable_ship(
    ship: &str,
    what: &str,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    declared: &Declared,
    issues: &mut Vec<LintIssue>,
) {
    check_target(ship, what, scenario, satisfiable, issues);
    let Some(spawned) = declared.spawned_ships.get(ship) else {
        return;
    };
    if !matches!(spawned.controller, SpaceshipController::Player(_)) {
        return;
    }
    issues.push(LintIssue::error(
        scenario,
        format!(
            "{what} targets ship '{ship}', which is player-driven; a helm order cannot \
             share a helm with live input (give the ship `SpaceshipController::None` if \
             the scenario is meant to fly it)"
        ),
    ));
}

/// An AI constraint's actor: the id must resolve, and the ship must actually
/// have judgement to constrain.
///
/// The mirror of [`check_orderable_ship`]. A leash on a `None`-controller
/// actor installs a component nothing reads, which is silent at runtime and
/// exactly the kind of thing an author reads back as "the leash is broken".
fn check_ai_ship(
    ship: &str,
    what: &str,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    declared: &Declared,
    issues: &mut Vec<LintIssue>,
) {
    check_target(ship, what, scenario, satisfiable, issues);
    let Some(spawned) = declared.spawned_ships.get(ship) else {
        return;
    };
    let driver = match &spawned.controller {
        SpaceshipController::AI(_) => return,
        SpaceshipController::None => "driven by nothing",
        SpaceshipController::Player(_) => "player-driven",
    };
    issues.push(LintIssue::error(
        scenario,
        format!(
            "{what} targets ship '{ship}', which is {driver}; an AI constraint only means \
             something on a ship that flies itself"
        ),
    ));
}

/// An AI range override must be a real distance: a negative or non-finite one
/// would either never trigger or trigger always, both silently.
fn check_ai_range(
    range: Option<nova_events::prelude::Meters>,
    what: &str,
    ship: &str,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    let Some(range) = range else {
        return;
    };
    if !range.0.is_finite() || range.0 < 0.0 {
        issues.push(LintIssue::error(
            scenario,
            format!(
                "{what} on '{ship}' needs a non-negative finite range in meters, got {}",
                range.0
            ),
        ));
    }
}

/// A helm order's key must be a real key: it is what a `ShipOrder` filter
/// matches on, and an empty one cannot be told apart from an unset field.
fn check_order_key(order: &str, what: &str, scenario: &str, issues: &mut Vec<LintIssue>) {
    if order.trim().is_empty() {
        issues.push(LintIssue::error(
            scenario,
            format!("{what} has an empty order key (nothing could wait for its completion)"),
        ));
    }
}

/// An authored arrival standoff must be a distance the autopilot can fly to.
///
/// Zero is legal - it means the hull's own face on the mark - so only negative
/// and non-finite values are rejected. Unsafe-but-flyable values are the
/// creator's to choose: a margin is a parking rule, never an obstacle
/// guarantee, and clamping one would quietly move a mark the author staged.
fn check_arrival_standoff(
    standoff: Option<nova_events::prelude::Meters>,
    what: &str,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    let Some(standoff) = standoff else {
        return;
    };
    if !standoff.0.is_finite() || standoff.0 < 0.0 {
        issues.push(LintIssue::error(
            scenario,
            format!(
                "{what} arrival_standoff must be a non-negative finite number of meters, \
                 got {}",
                standoff.0
            ),
        ));
    }
}

/// The same check on the spawn side: an AI ship authors the identical field,
/// and it went unlinted while the order path was covered.
fn check_spawned_arrival_standoff(
    config: &ScenarioObjectConfig,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    let ScenarioObjectKind::Spaceship(ship) = &config.kind else {
        return;
    };
    let SpaceshipController::AI(ai) = &ship.controller else {
        return;
    };
    check_arrival_standoff(
        ai.arrival_standoff,
        &format!("ship '{}'", config.base.id),
        scenario,
        issues,
    );
}

/// Every authored figure on a planet has to be one the generator can use.
///
/// A planet's radius is its REAL size, and the whole body - the mesh range,
/// the derived body radius, the well clamp, the sphere of influence, an orbit
/// ring - is measured off it. A zero or negative radius does not draw a small
/// planet; it divides the authored relief by nothing. The generator will not
/// paper over any of this, so the lint has to name the file first.
fn check_planet(config: &ScenarioObjectConfig, scenario: &str, issues: &mut Vec<LintIssue>) {
    let ScenarioObjectKind::Planet(planet) = &config.kind else {
        return;
    };
    let id = &config.base.id;

    if !planet.radius.0.is_finite() || planet.radius.0 <= 0.0 {
        issues.push(LintIssue::error(
            scenario,
            format!(
                "planet '{id}' needs a positive finite mean radius in meters, got {}",
                planet.radius.0
            ),
        ));
    }
    if let Some(relief) = planet.relief {
        if !relief.0.is_finite() || relief.0 <= 0.0 || relief.0 >= planet.radius.0 {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "planet '{id}' authors {} m of relief against a {} m radius; relief is \
                     a height above the mean surface, so it must be positive and smaller \
                     than the radius",
                    relief.0, planet.radius.0
                ),
            ));
        }
    }
    if let Some(sea_level) = planet.sea_level {
        if !(0.0..=1.0).contains(&sea_level) {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "planet '{id}' authors a sea level of {sea_level}; it is a fraction of \
                     the height range, so it runs 0 to 1"
                ),
            ));
        }
    }
    if let Some(mass) = planet.mass {
        if !mass.is_finite() || mass <= 0.0 {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "planet '{id}' authors a mass of {mass}; a gravity well needs a \
                     positive one"
                ),
            ));
        }
    }
    if let Some(signature) = planet.lock_signature {
        if !signature.0.is_finite() || signature.0 <= 0.0 {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "planet '{id}' authors a lock signature of {} m; drop the override to \
                     read at the mean radius instead",
                    signature.0
                ),
            ));
        }
    }
}

/// A section-addressed weapon action must name a section the ship carries, of
/// the class the action fires.
///
/// The class check is the point. Naming a hull block where a railgun was meant
/// is an ordinary authoring slip, and at runtime it either fires nothing or -
/// worse, before this - fired some other mount. Both are the kind of failure a
/// set piece hides until someone watches it play.
#[expect(
    clippy::too_many_arguments,
    reason = "one section reference, checked against every catalog that can resolve it"
)]
fn check_ship_section(
    ship: &str,
    section: &str,
    wanted: SectionClass,
    what: &str,
    scenario: &str,
    sections: &KnownSections,
    ships: &KnownShips,
    declared: &Declared,
    issues: &mut Vec<LintIssue>,
) {
    if section.trim().is_empty() {
        issues.push(LintIssue::error(
            scenario,
            format!("{what} on ship '{ship}' has an empty section id"),
        ));
        return;
    }
    // A ship this scenario does not spawn by name, or a catalog hull the
    // caller cannot see, leaves nothing to resolve the section against.
    let Some(spawned) = declared.spawned_ships.get(ship) else {
        return;
    };
    let hull = match &spawned.hull {
        ShipSource::Inline(hull) => hull,
        ShipSource::Prototype(id) => match ships.get(id) {
            Some(hull) => hull,
            None => return,
        },
    };
    let Some(placed) = hull.sections.iter().find(|placed| placed.id == section) else {
        issues.push(LintIssue::error(
            scenario,
            format!("{what} names section '{section}', which ship '{ship}' does not carry"),
        ));
        return;
    };
    // An unresolvable prototype is already an error where the hull is linted;
    // saying so twice from here would only add noise.
    let class = match &placed.source {
        SectionSource::Inline(config) => config.kind.class(),
        SectionSource::Prototype(proto) => match sections.get(proto) {
            Some(known) => known.class,
            None => return,
        },
    };
    if class != wanted {
        issues.push(LintIssue::error(
            scenario,
            format!(
                "{what} names section '{section}' of ship '{ship}', which is a \
                 {class:?} and not a {wanted:?}"
            ),
        ));
    }
}

fn check_filter(
    filter: &EventFilterConfig,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    declared: &Declared,
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
            } else if !declared.timer_keys.contains(&config.key) {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "Timer filter references timer '{}', which no TimerStart creates",
                        config.key
                    ),
                ));
            }
        }
        EventFilterConfig::ShipOrder(config) => {
            // An unset field matches any completion, which is a legitimate
            // authoring choice - so only a SET key is checked, and only for
            // being a key at all and for naming an order some helm action
            // installs. A filter waiting on an order nothing ever issues can
            // never open, which for a sequence gate is a soft-lock.
            if let Some(order) = &config.order {
                if order.trim().is_empty() {
                    issues.push(LintIssue::error(
                        scenario,
                        "ShipOrder filter has an empty order key".to_string(),
                    ));
                } else if !declared.order_keys.contains(order) {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "ShipOrder filter waits for order '{order}', which no helm \
                             action in this scenario issues"
                        ),
                    ));
                }
            }
            if let Some(ship) = &config.ship {
                check_target(ship, "ShipOrder filter", scenario, satisfiable, issues);
            }
        }
        EventFilterConfig::Conditional(config) => match config {
            ConditionalFilterConfig::Not(inner) => {
                check_filter(inner, scenario, satisfiable, declared, used_vars, issues);
            }
            ConditionalFilterConfig::Or(left, right)
            | ConditionalFilterConfig::And(left, right) => {
                check_filter(left, scenario, satisfiable, declared, used_vars, issues);
                check_filter(right, scenario, satisfiable, declared, used_vars, issues);
            }
        },
    }
}

fn check_query(
    query: &QueryConfig,
    scenario: &str,
    satisfiable: &dyn Fn(&str) -> bool,
    issues: &mut Vec<LintIssue>,
) {
    if let QueryConfig::Entity(query) = query {
        if query.filter.id.trim().is_empty() {
            return;
        }
        if !satisfiable(&query.filter.id) {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "entity query targets '{}', but no scenario action can spawn that id",
                    query.filter.id
                ),
            ));
        }
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
        VariableFactorNode::Query(_) | VariableFactorNode::Literal(_) => {}
    }
}

/// A condition of the shape `<clock> < <not a literal>` (either way round).
///
/// Only the bare `Term(Factor(..))` spelling counts on the clock side, because
/// that is the only shape that reads as a stopwatch; `elapsed * 2 > x` is
/// arithmetic and is somebody else's problem.
fn compares_the_clock_to_a_non_literal(
    filter: &EventFilterConfig,
    clock_watches: &HashSet<&str>,
) -> bool {
    let EventFilterConfig::Expression(config) = filter else {
        return false;
    };
    let (left, right) = match &config.0 {
        VariableConditionNode::LessThan(left, right)
        | VariableConditionNode::GreaterThan(left, right) => (left.as_ref(), right.as_ref()),
        VariableConditionNode::Equal(_, _) => return false,
    };
    let clock_side = |node: &VariableExpressionNode| match node {
        VariableExpressionNode::Term(VariableTermNode::Factor(VariableFactorNode::Name(name))) => {
            clock_watches.contains(name.as_str())
        }
        VariableExpressionNode::Term(VariableTermNode::Factor(VariableFactorNode::Query(
            QueryConfig::Scenario(query),
        ))) => query.property == ScenarioProperty::Elapsed,
        _ => false,
    };
    let literal_side = |node: &VariableExpressionNode| {
        matches!(
            node,
            VariableExpressionNode::Term(VariableTermNode::Factor(VariableFactorNode::Literal(
                VariableLiteral::Number(_)
            )))
        )
    };
    (clock_side(left) && !literal_side(right)) || (clock_side(right) && !literal_side(left))
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use nova_events::prelude::*;

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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.severity == LintSeverity::Error)
                .count(),
            4,
            "empty start/cancel/filter keys and zero duration each error: {issues:?}"
        );
    }

    #[test]
    fn timer_references_must_match_a_started_key() {
        let s = scenario(
            vec![
                EventActionConfig::TimerStart(TimerStartActionConfig {
                    key: "orbit_hold".to_string(),
                    seconds: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(8.0)),
                    )),
                }),
                EventActionConfig::TimerCancel(TimerCancelActionConfig {
                    key: "oribt_hold".to_string(),
                }),
            ],
            vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "oribt_hold".to_string(),
            })],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            issues.iter().any(|issue| {
                issue.severity == LintSeverity::Error
                    && issue.message.contains("Timer filter")
                    && issue.message.contains("oribt_hold")
            }),
            "a timer-filter typo is an impossible event and must error: {issues:?}"
        );
        assert!(
            issues.iter().any(|issue| {
                issue.severity == LintSeverity::Warn
                    && issue.message.contains("TimerCancel")
                    && issue.message.contains("oribt_hold")
            }),
            "cancelling a timer never started in the scenario must warn: {issues:?}"
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
            &ships(&[]),
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("ghost"));
        assert!(errs[0].message.contains("SetAllegiance"));
    }

    /// The backdrop camera contract: a menu backdrop must pose its own
    /// camera (SetCamera) - the menu never derives a pose. An ERROR so the
    /// draw filters the broken backdrop out of the rotation.
    #[test]
    fn a_backdrop_without_a_camera_pose_is_an_error() {
        let mut poseless = scenario(vec![], vec![]);
        poseless.menu_backdrop = true;
        let issues = lint_scenario(
            &poseless,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("SetCamera"));

        let mut posed = scenario(
            vec![EventActionConfig::SetCamera(SetCameraActionConfig {
                position: Meters3::new(0.0, 900.0, 3_000.0),
                look_at: Meters3::ZERO,
            })],
            vec![],
        );
        posed.menu_backdrop = true;
        let issues = lint_scenario(
            &posed,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(errors(&issues).is_empty(), "{issues:?}");

        // Non-backdrop scenarios owe no camera.
        let plain = scenario(vec![], vec![]);
        let issues = lint_scenario(
            &plain,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(errors(&issues).is_empty(), "{issues:?}");
    }

    /// A helm order on a PLAYER's ship is an authoring error - the controller
    /// is sitting in the same file the action is, and the input layer would
    /// win the fight silently. An AI ship is not: taking an AI hull's helm for
    /// a mission is the point of the shared order family.
    #[test]
    fn a_helm_order_refuses_the_players_ship_and_accepts_an_ai_one() {
        let refusals = |controller| {
            let s = scenario(
                vec![
                    spawn_armed_ship("warship", controller),
                    EventActionConfig::StopShip(StopShipActionConfig {
                        order: "halt".to_string(),
                        ship: "warship".to_string(),
                    }),
                ],
                vec![],
            );
            let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
            errors(&issues)
                .into_iter()
                .filter(|issue| issue.message.contains("StopShip"))
                .map(|issue| issue.message.clone())
                .collect::<Vec<_>>()
        };

        let player = refusals(SpaceshipController::Player(
            PlayerControllerConfig::default(),
        ));
        assert_eq!(player.len(), 1, "{player:?}");
        assert!(player[0].contains("warship"));

        assert!(
            refusals(SpaceshipController::AI(AIControllerConfig::default())).is_empty(),
            "an AI ship takes a mission"
        );
    }

    /// A forced SHOT is stricter than a helm order and keeps refusing an AI
    /// ship: the shot leaves down whatever line the hull holds, and an AI hull
    /// rewrites its own aim every frame.
    #[test]
    fn a_forced_shot_still_refuses_a_driven_ship() {
        for controller in [
            SpaceshipController::Player(PlayerControllerConfig::default()),
            SpaceshipController::AI(AIControllerConfig::default()),
        ] {
            let s = scenario(
                vec![
                    spawn_armed_ship("warship", controller),
                    EventActionConfig::ForceRailgunFire(ForceRailgunFireActionConfig {
                        ship: "warship".to_string(),
                        section: "spinal".to_string(),
                    }),
                ],
                vec![],
            );
            let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
            let refusals: Vec<_> = errors(&issues)
                .into_iter()
                .filter(|issue| {
                    issue.message.contains("is player-driven")
                        || issue.message.contains("is AI-driven")
                })
                .collect();
            assert_eq!(refusals.len(), 1, "{issues:?}");
            assert!(refusals[0].message.contains("warship"));
        }
    }

    /// An AI constraint is the mirror: it only means something on a ship that
    /// flies itself, so a `None`-controller actor is refused.
    #[test]
    fn an_ai_constraint_refuses_a_ship_with_no_judgement() {
        let s = scenario(
            vec![
                spawn_armed_ship("hulk", SpaceshipController::None),
                EventActionConfig::SetAIEngageRange(SetAIEngageRangeActionConfig {
                    ship: "hulk".to_string(),
                    range: Some(Meters(1_200.0)),
                }),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let refusals: Vec<_> = errors(&issues)
            .into_iter()
            .filter(|issue| issue.message.contains("SetAIEngageRange"))
            .collect();
        assert_eq!(refusals.len(), 1, "{issues:?}");
    }

    /// A patrol with no waypoints has no loop to fly, so nothing could ever
    /// wait for its completion. Caught at author time rather than as a
    /// runtime refusal nobody sees.
    #[test]
    fn an_empty_patrol_route_is_an_error() {
        let s = scenario(
            vec![
                spawn_armed_ship(
                    "picket",
                    SpaceshipController::AI(AIControllerConfig::default()),
                ),
                EventActionConfig::PatrolShip(PatrolShipActionConfig {
                    order: "sweep".to_string(),
                    ship: "picket".to_string(),
                    waypoints: Vec::new(),
                }),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(
            errors(&issues)
                .iter()
                .any(|issue| issue.message.contains("PatrolShip")
                    && issue.message.contains("waypoints")),
            "{issues:?}"
        );
    }

    /// The same action on a `None`-controller ship is clean - that is the ship
    /// the scripted vocabulary is for.
    #[test]
    fn a_scripted_action_on_a_none_controller_ship_is_clean() {
        let s = scenario(
            vec![
                spawn_armed_ship("warship", SpaceshipController::None),
                EventActionConfig::MoveShipTo(MoveShipToActionConfig {
                    order: "approach".to_string(),
                    ship: "warship".to_string(),
                    position: Meters3::new(0.0, 0.0, -1_200.0),
                    arrival_standoff: Some(Meters(60.0)),
                }),
                EventActionConfig::ForceRailgunFire(ForceRailgunFireActionConfig {
                    ship: "warship".to_string(),
                    section: "spinal".to_string(),
                }),
                EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
                    ship: "warship".to_string(),
                    section: "bay".to_string(),
                    target: "warship".to_string(),
                }),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "{issues:?}");
    }

    /// Both authored margins are the same field with the same rule, so both
    /// are linted the same way: negative and non-finite are errors, zero is
    /// legal. The AI one used to be silently unchecked - and its guard used to
    /// throw a zero away instead.
    #[test]
    fn a_negative_arrival_standoff_is_an_error_on_either_authored_path() {
        let s = scenario(
            vec![
                spawn_armed_ship(
                    "picket",
                    SpaceshipController::AI(AIControllerConfig {
                        arrival_standoff: Some(Meters(-10.0)),
                        ..default()
                    }),
                ),
                spawn_armed_ship("warship", SpaceshipController::None),
                EventActionConfig::MoveShipTo(MoveShipToActionConfig {
                    order: "approach".to_string(),
                    ship: "warship".to_string(),
                    position: Meters3::new(0.0, 0.0, -1_200.0),
                    arrival_standoff: Some(Meters(f32::NAN)),
                }),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert!(
            errs.iter()
                .any(|issue| issue.message.contains("ship 'picket' arrival_standoff")),
            "the AI spawn field is linted: {issues:?}"
        );
        assert!(
            errs.iter().any(|issue| issue
                .message
                .contains("MoveShipTo 'approach' arrival_standoff")),
            "and the order field still is: {issues:?}"
        );
    }

    /// Nothing on a planet is allowed to be nonsense-but-tolerated.
    ///
    /// The generator divides the authored relief by the radius and measures
    /// the whole body off the result, so a zero radius does not draw a small
    /// world - it produces a body radius of NaN and takes the gravity well,
    /// the sphere of influence and every orbit inside it with it. The lint is
    /// the first place that can name the file, so it is where this fails.
    #[test]
    fn a_planet_authored_with_impossible_figures_is_an_error() {
        let planet = |id: &str, config: PlanetConfig| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: id.to_string(),
                    name: id.to_string(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Planet(config),
            })
        };
        let sound = || PlanetConfig::new(PlanetType::DustWorld, Meters(900.0), 7);

        let s = scenario(
            vec![
                planet(
                    "no_size",
                    PlanetConfig::new(PlanetType::DustWorld, Meters::ZERO, 7),
                ),
                planet(
                    "relief_past_the_radius",
                    PlanetConfig {
                        relief: Some(Meters(2_000.0)),
                        ..sound()
                    },
                ),
                planet(
                    "sea_above_the_peaks",
                    PlanetConfig {
                        sea_level: Some(1.4),
                        ..sound()
                    },
                ),
                planet(
                    "weightless_well",
                    PlanetConfig {
                        mass: Some(0.0),
                        ..sound()
                    },
                ),
                planet("sound", sound()),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        for id in [
            "no_size",
            "relief_past_the_radius",
            "sea_above_the_peaks",
            "weightless_well",
        ] {
            assert!(
                errs.iter()
                    .any(|issue| issue.message.contains(&format!("planet '{id}'"))),
                "'{id}' must be an error: {issues:?}"
            );
        }
        assert!(
            !errs
                .iter()
                .any(|issue| issue.message.contains("planet 'sound'")),
            "a well-authored planet stays clean: {issues:?}"
        );
    }

    /// Zero is a legal margin - the hull's own face on the mark - so it must
    /// not be linted as if it were missing or wrong.
    #[test]
    fn a_zero_arrival_standoff_is_clean_on_either_authored_path() {
        let s = scenario(
            vec![
                spawn_armed_ship(
                    "picket",
                    SpaceshipController::AI(AIControllerConfig {
                        arrival_standoff: Some(Meters::ZERO),
                        ..default()
                    }),
                ),
                spawn_armed_ship("warship", SpaceshipController::None),
                EventActionConfig::MoveShipTo(MoveShipToActionConfig {
                    order: "approach".to_string(),
                    ship: "warship".to_string(),
                    position: Meters3::new(0.0, 0.0, -1_200.0),
                    arrival_standoff: Some(Meters::ZERO),
                }),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "{issues:?}");
    }

    /// A section id the hull does not carry, and a section of the WRONG class,
    /// are both errors: at runtime the first fires nothing and the second
    /// would have fired some other mount.
    #[test]
    fn a_missing_or_wrong_class_section_is_an_error() {
        let s = scenario(
            vec![
                spawn_armed_ship("warship", SpaceshipController::None),
                EventActionConfig::ForceRailgunFire(ForceRailgunFireActionConfig {
                    ship: "warship".to_string(),
                    section: "dorsal".to_string(),
                }),
                EventActionConfig::ForceRailgunFire(ForceRailgunFireActionConfig {
                    ship: "warship".to_string(),
                    section: "nose".to_string(),
                }),
                EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
                    ship: "warship".to_string(),
                    section: "spinal".to_string(),
                    target: "warship".to_string(),
                }),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 3, "{issues:?}");
        assert!(
            errs[0].message.contains("does not carry"),
            "{:?}",
            errs[0].message
        );
        assert!(
            errs[1].message.contains("Hull") && errs[1].message.contains("Railgun"),
            "{:?}",
            errs[1].message
        );
        assert!(
            errs[2].message.contains("Railgun") && errs[2].message.contains("Torpedo"),
            "{:?}",
            errs[2].message
        );
    }

    /// An order key is what a waiting beat matches on, so an empty one is an
    /// error at both ends - the action that could never be waited for, and the
    /// filter that could never match.
    #[test]
    fn an_empty_order_key_is_an_error_at_both_ends() {
        let s = scenario(
            vec![
                spawn_armed_ship("warship", SpaceshipController::None),
                EventActionConfig::ForceAlign(ForceAlignActionConfig {
                    order: String::new(),
                    ship: "warship".to_string(),
                    look_at: Meters3::ZERO,
                    tolerance_degrees: 2.0,
                }),
            ],
            vec![EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: Some(String::new()),
                ..default()
            })],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 2, "{issues:?}");
        assert!(errs.iter().any(|e| e.message.contains("ForceAlign")));
        assert!(errs.iter().any(|e| e.message.contains("ShipOrder filter")));
    }

    /// A filter waiting on a key nothing ever issues is a handler that can
    /// never run, which is worth an error rather than a silent dead beat. A
    /// key some helm action DOES issue is clean, and so is a filter that
    /// constrains nothing.
    #[test]
    fn a_ship_order_filter_must_name_a_key_some_action_issues() {
        let issue_order = || {
            EventActionConfig::StopShip(StopShipActionConfig {
                order: "halt".to_string(),
                ship: "warship".to_string(),
            })
        };
        let waits_for = |key: Option<&str>| {
            scenario(
                vec![
                    spawn_armed_ship("warship", SpaceshipController::None),
                    issue_order(),
                ],
                vec![EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                    order: key.map(str::to_string),
                    ..default()
                })],
            )
        };

        let issues = lint_scenario(
            &waits_for(Some("halt")),
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(errors(&issues).is_empty(), "{issues:?}");

        let issues = lint_scenario(
            &waits_for(None),
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(
            issues.is_empty(),
            "an unconstrained filter is a choice, not a finding: {issues:?}"
        );

        let issues = lint_scenario(
            &waits_for(Some("never_issued")),
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("never_issued"));
    }

    /// ForceTorpedoFire references TWO ships by id (launcher and target);
    /// both must lint as dangling targets on a typo, not no-op at runtime.
    #[test]
    fn dangling_force_torpedo_fire_ids_are_errors() {
        let s = scenario(
            vec![EventActionConfig::ForceTorpedoFire(
                ForceTorpedoFireActionConfig {
                    ship: "ghost_battery".to_string(),
                    section: "bay_port".to_string(),
                    target: "ghost_prey".to_string(),
                },
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 2, "{issues:?}");
        assert!(errs[0].message.contains("ghost_battery"));
        assert!(errs[1].message.contains("ghost_prey"));
        assert!(errs[0].message.contains("ForceTorpedoFire"));
    }

    #[test]
    fn duplicate_spawn_ids_in_one_handler_are_an_error() {
        let s = scenario(vec![spawn_object("twin"), spawn_object("twin")], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
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
            label: None,
            name: EventConfig::OnDestroyed,
            once: false,
            filters: vec![],
            actions: vec![spawn_object("boss")],
        });
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("mutually exclusive"));
    }

    /// A rock says what it is made of, and the id has to be real. There is no
    /// default kind and no fallback, so an unknown one renders as nothing -
    /// which is exactly the failure a static check should beat the frame to.
    #[test]
    fn an_unknown_asteroid_kind_is_a_lint_error() {
        let rock = |material: &str| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "lone_rock".to_string(),
                    name: "Lone Rock".to_string(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    material: material.to_string(),
                    destroy_sound: None,
                    radius: Meters(20.0),
                    texture: nova_gameplay::prelude::AssetRef::default(),
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            })
        };

        let bad = scenario(vec![rock("granite")], vec![]);
        let issues = lint_scenario(
            &bad,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("'granite' is not a kind")),
            "an unshipped kind is an error: {issues:?}"
        );

        let good = scenario(vec![rock(KIND_ICE)], vec![]);
        let issues = lint_scenario(
            &good,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(
            errors(&issues).is_empty(),
            "a shipped kind is fine: {issues:?}"
        );
    }

    /// A scattered field says what it is made of too, in a mix that has weight
    /// and names kinds that exist. All three failures are the same content bug
    /// wearing different hats: a field nobody decided the composition of.
    #[test]
    fn a_scattered_field_must_author_a_real_kind_mix() {
        let field = |mix: Vec<(String, u32)>| {
            EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: "rock_".to_string(),
                count: 8,
                seed: 1,
                region: ScatterRegion::Box {
                    min: Meters3::new(-100.0, -100.0, -100.0),
                    max: Meters3::new(100.0, 100.0, 100.0),
                },
                template: ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "rock_".to_string(),
                        name: "Rock".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        material: KIND_ROCK.to_string(),
                        destroy_sound: None,
                        radius: Meters(20.0),
                        texture: nova_gameplay::prelude::AssetRef::default(),
                        mass: None,
                        invulnerable: false,
                        seed: None,
                        lock_signature: None,
                    }),
                },
                asteroid_radius: None,
                asteroid_kinds: mix,
                min_separation: None,
            })
        };
        let lint_of = |action| {
            let s = scenario(vec![action], vec![]);
            lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]))
        };

        let issues = lint_of(field(vec![]));
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("no `asteroid_kinds` mix")),
            "a field with no mix is an error: {issues:?}"
        );

        let issues = lint_of(field(vec![("granite".to_string(), 3)]));
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("'granite' is not a kind")),
            "an unshipped kind in the mix is an error: {issues:?}"
        );

        let issues = lint_of(field(vec![
            (KIND_ROCK.to_string(), 3),
            (KIND_METAL.to_string(), 0),
        ]));
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("is weighted 0")),
            "a kind that can never be drawn is an error: {issues:?}"
        );

        let issues = lint_of(field(vec![
            (KIND_ROCK.to_string(), 6),
            (KIND_METAL.to_string(), 1),
        ]));
        assert!(errors(&issues).is_empty(), "a real mix is fine: {issues:?}");
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
                    center: Meters3::ZERO,
                    inner: Meters(100.0),
                    outer: Meters(200.0),
                    y_min: Meters(-10.0),
                    y_max: Meters(10.0),
                },
                template: match spawn_object("rock_") {
                    EventActionConfig::SpawnScenarioObject(config) => config,
                    _ => unreachable!(),
                },
                asteroid_radius: None,
                asteroid_kinds: vec![],
                min_separation: None,
            })],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
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
                    center: Meters3::ZERO,
                    inner: Meters(100.0),
                    outer: Meters(200.0),
                    y_min: Meters(-10.0),
                    y_max: Meters(10.0),
                },
                template: match spawn_object("rock_") {
                    EventActionConfig::SpawnScenarioObject(config) => config,
                    _ => unreachable!(),
                },
                asteroid_radius: None,
                asteroid_kinds: vec![],
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("non-lingering"));

        let s = scenario(vec![outcome(), next(false, Some(4.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "delayed is the same trap: {issues:?}");

        let s = scenario(vec![outcome(), next(true, None)], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
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
                speaker: "Alpha".to_string(),
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("one line per beat"));

        let s = scenario(vec![line("dead"), outcome()], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("never read"));

        let s = scenario(vec![line("solo")], vec![]);
        assert!(
            lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"])).is_empty()
        );
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
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues.iter().any(|i| i.message.contains("outside (0, 60]")));
        assert!(issues.iter().any(|i| i.message.contains("dead")));

        // The outcome range warn, without a hard switch in the handler.
        let s = scenario(vec![outcome_adv(Some(f64::INFINITY))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("auto_advance_secs"));

        // Zero is OUTSIDE the (0, MAX] the messages advertise: it builds a
        // Timer that finishes on tick one, so the banner never shows. Both
        // fields, since both read as "omit the field instead".
        let s = scenario(vec![outcome_adv(Some(0.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("auto_advance_secs"));
        let s = scenario(vec![next(false, Some(0.0))], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("outside (0, 60]"));

        // Sane values, trap-free shapes: clean.
        let s = scenario(vec![next(false, Some(4.0))], vec![]);
        assert!(
            lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"])).is_empty()
        );
        let s = scenario(vec![outcome_adv(Some(6.0))], vec![]);
        assert!(
            lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"])).is_empty()
        );
    }

    /// StoryMessage dwell range: out-of-range warns, in-range and omitted stay
    /// clean.
    #[test]
    fn story_dwell_out_of_range_warns() {
        let line = |dwell| {
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: "Alpha".to_string(),
                text: "test".to_string(),
                dwell,
                icon: None,
            })
        };
        // One line per handler so the beat-sheet arm stays out of frame.
        let mut s = scenario(vec![line(Some(120.0))], vec![]);
        for l in [line(Some(12.0)), line(None)] {
            s.events.push(ScenarioEventConfig {
                label: None,
                name: EventConfig::OnStart,
                once: false,
                filters: vec![],
                actions: vec![l],
            });
        }
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(errors(&issues).is_empty(), "warn-only: {issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("120"));
    }

    /// A declared elapsed watch is readable without a mutable seed, and writing
    /// its owned name is an error.
    #[test]
    fn scenario_clock_reads_are_clean_and_writes_are_errors() {
        const SCENARIO_ELAPSED_VAR: &str = "scenario_elapsed";

        // A time-gated handler the way an author writes one: no warning.
        let mut read_only = scenario(
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
        read_only.watches.push(WatchConfig {
            variable: SCENARIO_ELAPSED_VAR.to_string(),
            query: QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
        });
        let issues = lint_scenario(
            &read_only,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(
            issues.is_empty(),
            "gating on the engine clock is the intended pattern: {issues:?}"
        );

        // An authored write to the clock: an error, not a warning.
        let mut stomp = scenario(
            vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: SCENARIO_ELAPSED_VAR.to_string(),
                expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                )),
            })],
            vec![],
        );
        stomp.watches = read_only.watches.clone();
        let issues = lint_scenario(
            &stomp,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert_eq!(
            errors(&issues).len(),
            1,
            "writing the watched clock is an error: {issues:?}"
        );
        assert!(issues[0].message.contains(SCENARIO_ELAPSED_VAR));
    }

    /// A declared entity-speed watch follows the same ownership contract as
    /// elapsed: reads are clean and mutable writes are errors.
    #[test]
    fn player_speed_reads_are_clean_and_writes_are_errors() {
        const PLAYER_SPEED_VAR: &str = "player_speed";

        // A speed-gated handler the way an author writes one: no warning.
        let mut read_only = scenario(
            vec![spawn_ship("player_spaceship", "hull")],
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
        read_only.watches.push(WatchConfig {
            variable: PLAYER_SPEED_VAR.to_string(),
            query: QueryConfig::Entity(EntityQuery {
                filter: EntityQueryFilter {
                    id: "player_spaceship".to_string(),
                },
                property: EntityProperty::Speed,
            }),
        });
        let issues = lint_scenario(
            &read_only,
            &sections(&["hull"]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(
            issues.is_empty(),
            "gating on a watched speed is the intended pattern: {issues:?}"
        );

        // An authored write to the readout: an error, not a warning.
        let mut stomp = scenario(
            vec![
                spawn_ship("player_spaceship", "hull"),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: PLAYER_SPEED_VAR.to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                    )),
                }),
            ],
            vec![],
        );
        stomp.watches = read_only.watches.clone();
        let issues = lint_scenario(
            &stomp,
            &sections(&["hull"]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert_eq!(
            errors(&issues).len(),
            1,
            "writing a watched speed is an error: {issues:?}"
        );
        assert!(issues[0].message.contains(PLAYER_SPEED_VAR));
    }

    fn number(n: f64) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(n)),
        ))
    }

    fn name(n: &str) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_name(n),
        ))
    }

    fn story(text: &str) -> EventActionConfig {
        EventActionConfig::StoryMessage(StoryMessageActionConfig {
            speaker: "Control".to_string(),
            text: text.to_string(),
            dwell: None,
            icon: None,
        })
    }

    fn sequence(key: &str, steps: Vec<SequenceStepConfig>) -> EventActionConfig {
        EventActionConfig::Sequence(SequenceActionConfig {
            key: key.to_string(),
            steps,
        })
    }

    fn enter_gate(id: &str) -> SequenceGateConfig {
        SequenceGateConfig {
            name: EventConfig::OnEnter,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(id.to_string()),
                ..default()
            })],
        }
    }

    /// A gate is a runtime question, so whether it can EVER open is not
    /// decidable here. The deadline is what turns an unanswerable gate into a
    /// loud stop instead of a silent soft-lock, so authoring one without it is
    /// an error.
    #[test]
    fn a_gated_step_without_a_deadline_errors() {
        let s = scenario(
            vec![
                spawn_object("beacon_1"),
                sequence(
                    "run",
                    vec![SequenceStepConfig {
                        until: Some(enter_gate("beacon_1")),
                        ..default()
                    }],
                ),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("no deadline")),
            "a gate with no deadline must error: {issues:?}"
        );
    }

    /// The engine holds ONE cursor per key, so the second start is refused at
    /// runtime - a beat chain that silently never plays. Catch it on the gate.
    #[test]
    fn a_duplicate_sequence_key_errors() {
        let s = scenario(
            vec![
                sequence("run", vec![SequenceStepConfig::default()]),
                sequence("run", vec![SequenceStepConfig::default()]),
            ],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            errors(&issues)
                .iter()
                .any(|i| i.message.contains("duplicate Sequence key")),
            "two sequences on one key must error: {issues:?}"
        );
    }

    /// The shared outro pattern: every win variant starts the SAME chain, and
    /// only one of them can ever fire. Nothing static can tell that apart from
    /// a real collision, so the gate stays quiet and the runtime holds that
    /// half - otherwise the mainline's own shape warns five times a run.
    #[test]
    fn a_sequence_key_started_by_two_handlers_is_not_flagged() {
        let mut s = scenario(
            vec![sequence("outro", vec![SequenceStepConfig::default()])],
            vec![],
        );
        s.events.push(ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![],
            actions: vec![sequence("outro", vec![SequenceStepConfig::default()])],
        });
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            !issues
                .iter()
                .any(|i| i.message.contains("Sequence key 'outro'")),
            "mutually exclusive starts must not be flagged at all: {issues:?}"
        );
    }

    /// The walkers recurse. A spawn that only a STEP performs still declares
    /// its id, so a filter and a gate that name it are satisfiable - without
    /// the recursion both would read as dangling references.
    #[test]
    fn a_spawn_inside_a_step_declares_its_id_to_the_whole_scenario() {
        let s = scenario(
            vec![sequence(
                "run",
                vec![
                    SequenceStepConfig {
                        actions: vec![spawn_object("beacon_1")],
                        ..default()
                    },
                    SequenceStepConfig {
                        until: Some(enter_gate("beacon_1")),
                        deadline: Some(120.0),
                        ..default()
                    },
                ],
            )],
            vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("beacon_1".to_string()),
                ..default()
            })],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            errors(&issues).is_empty(),
            "a step's spawn declares its id like a handler's would: {issues:?}"
        );
    }

    /// A sequence step is a BEAT, so the one-line-per-beat convention counts
    /// per step. Three single-line steps are the convention being followed;
    /// two lines inside one step are the burst it exists to stop.
    #[test]
    fn story_lines_are_counted_per_step_not_per_handler() {
        let paced = scenario(
            vec![sequence(
                "run",
                (0..3)
                    .map(|i| SequenceStepConfig {
                        after: Some(4.0),
                        actions: vec![story(&format!("line {i}"))],
                        ..default()
                    })
                    .collect(),
            )],
            vec![],
        );
        let issues = lint_scenario(&paced, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            !issues.iter().any(|i| i.message.contains("StoryMessages")),
            "clock-spaced steps are one line per beat: {issues:?}"
        );

        let burst = scenario(
            vec![sequence(
                "run",
                vec![SequenceStepConfig {
                    actions: vec![story("one"), story("two")],
                    ..default()
                }],
            )],
            vec![],
        );
        let issues = lint_scenario(&burst, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            issues.iter().any(|i| i.message.contains("StoryMessages")),
            "two lines inside ONE step are still a burst: {issues:?}"
        );
    }

    /// The regression rule: once `after` exists, an `OnUpdate` handler filtered
    /// on nothing but the clock is a hand-rolled delay walked every frame.
    #[test]
    fn an_onupdate_filtered_only_on_the_clock_warns() {
        let mut s = scenario(vec![], vec![]);
        s.watches = vec![WatchConfig {
            variable: "elapsed".to_string(),
            query: QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
        }];
        s.events = vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::GreaterThan(
                    Box::new(name("elapsed")),
                    Box::new(number(12.0)),
                ),
            ))],
            actions: vec![story("late")],
        }];
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("hand-rolled delay")),
            "pure clock polling must be flagged: {issues:?}"
        );
    }

    /// The stopwatch half: the clock against a VARIABLE deadline is a keyed
    /// timer written by hand, and the pure-clock rule cannot see it because
    /// the handler reads real state too.
    #[test]
    fn an_onupdate_comparing_the_clock_to_a_variable_warns() {
        let mut s = scenario(vec![], vec![]);
        s.watches = vec![WatchConfig {
            variable: "elapsed".to_string(),
            query: QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
        }];
        s.events = vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::Equal(Box::new(name("armed")), Box::new(number(1.0))),
                )),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::GreaterThan(
                        Box::new(name("elapsed")),
                        Box::new(name("deadline")),
                    ),
                )),
            ],
            actions: vec![story("too late")],
        }];
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("hand-rolled stopwatch")),
            "a stamped deadline must be flagged: {issues:?}"
        );
    }

    /// A literal threshold is an ordinary clock gate and stays quiet - it is
    /// schedulable, and it is what a paced beat looks like.
    #[test]
    fn an_onupdate_comparing_the_clock_to_a_literal_stays_quiet() {
        let mut s = scenario(vec![], vec![]);
        s.watches = vec![WatchConfig {
            variable: "elapsed".to_string(),
            query: QueryConfig::Scenario(ScenarioQuery {
                property: ScenarioProperty::Elapsed,
            }),
        }];
        s.events = vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::Equal(Box::new(name("armed")), Box::new(number(1.0))),
                )),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::GreaterThan(
                        Box::new(name("elapsed")),
                        Box::new(number(95.0)),
                    ),
                )),
            ],
            actions: vec![story("wave two")],
        }];
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            !issues
                .iter()
                .any(|i| i.message.contains("hand-rolled stopwatch")),
            "a literal threshold is a legitimate clock gate: {issues:?}"
        );
    }

    /// The same rule must stay quiet on a value-gated milestone - a tally, a
    /// distance - which is what `OnUpdate` is legitimately for.
    #[test]
    fn an_onupdate_gated_on_a_value_stays_quiet() {
        let mut s = scenario(vec![spawn_object("beacon_1")], vec![]);
        s.watches = vec![WatchConfig {
            variable: "crates_left".to_string(),
            query: QueryConfig::Entity(EntityQuery {
                filter: EntityQueryFilter {
                    id: "beacon_1".to_string(),
                },
                property: EntityProperty::Speed,
            }),
        }];
        s.events.push(ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::LessThan(
                    Box::new(name("crates_left")),
                    Box::new(number(1.0)),
                ),
            ))],
            actions: vec![story("all aboard")],
        });
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&[]));
        assert!(
            !issues
                .iter()
                .any(|i| i.message.contains("hand-rolled delay")),
            "a value-gated milestone is a legitimate OnUpdate: {issues:?}"
        );
    }
}
