//! The content-lint walk: reads a mod tree (or single target) off disk, parses
//! every bundle's content, runs the reference/geometry, balance and
//! input-overlap checks, and assembles the unified
//! [`ContentReport`]. Used by the
//! `content lint` CLI and the CI gate test; not part of the game runtime.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use nova_input::prelude::InputSource;
use nova_mod_format::{BundleManifest, BASE_MOD_ID};
use nova_modding::prelude::Content;
use nova_scenario::prelude::{
    lint_campaign, lint_scenario, lint_ship_design_config, CampaignConfig, EventActionConfig,
    KnownSections, KnownShipDesigns, LintIssue, LintSeverity, ScenarioConfig, ScenarioObjectKind,
    ScenarioRole, ShipDesignPrototype, SpaceshipController,
};
use nova_ship::prelude::{flight_rig_reserved_sources, SectionConfig};
use nova_training::prelude::{
    lint_lessons, unused_practice_ranges, Lesson, LessonIssue, LessonSeverity,
};
use nova_ui::theme::{lint_theme, UiThemeConfig};
use nova_world_base::prelude::{ShipPartFault, ShipPartPack, ShipPartSnapshot};

use crate::{
    balance::{BalanceAck, BALANCE_ACKS_FILE},
    content_report::{
        AckedFinding, Category, ContentReport, CreativeMap, Finding, Severity as ReportSeverity,
    },
};

/// The two report walks (`collect_*`), the two issue-only walks (`lint_*`),
/// the target resolver, the balance-input view of a walked tree and the
/// generated-ship part packs of repo bundles.
pub mod prelude {
    pub use super::{
        audit_bundles, collect_target, collect_tree, lint_content_tree, lint_target,
        repo_ship_part_packs, resolve_target, tree_acks, AuditBundle,
    };
}

/// One walked bundle: its id (directory-derived), manifest, parsed content
/// items, and the section / scenario views derived from them.
struct WalkedBundle {
    id: String,
    manifest: BundleManifest,
    sections: Vec<SectionConfig>,
    ships: Vec<ShipDesignPrototype>,
    scenarios: Vec<ScenarioConfig>,
    campaigns: Vec<CampaignConfig>,
    lessons: Vec<Lesson>,
    ui_themes: Vec<UiThemeConfig>,
    /// Every parsed content item paired with the bundle-relative file it was
    /// read from (a bundle lists several content files). Kept so the
    /// mod-relative `self://` resource-ref check can see every kind, and so
    /// the unified report can point each finding at its source file.
    content: Vec<(String, Content)>,
    /// The balance findings this bundle's author declared intended, read from
    /// its own [`BALANCE_ACKS_FILE`]. Empty when the file is absent - most
    /// bundles ack nothing.
    acks: Vec<BalanceAck>,
}

impl WalkedBundle {
    /// The bundle-relative file a content element (scenario, section or
    /// campaign id) was authored in, for report provenance. `None` when no
    /// content item carries that id (e.g. a finding about a missing/foreign
    /// prototype).
    fn file_of(&self, element_id: &str) -> Option<&str> {
        self.content
            .iter()
            .find_map(|(file, item)| (item.id() == element_id).then_some(file.as_str()))
    }
}

/// The workspace root (this crate sits at `crates/nova_authoring`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Parse the single `*.bundle.ron` at `dir`'s root plus its content
/// files. Panics with a readable message on unreadable/unparsable files
/// (the loaders' own gates cover load-ability; the lint walk assumes a
/// tree that passes them).
fn read_bundle(id: &str, dir: &Path) -> WalkedBundle {
    let manifest_path = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .filter_map(|e| Some(e.ok()?.path()))
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".bundle.ron"))
        })
        .unwrap_or_else(|| panic!("{} has no *.bundle.ron at its root", dir.display()));
    let manifest: BundleManifest = ron::de::from_str(
        &std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", manifest_path.display())),
    )
    .unwrap_or_else(|err| panic!("parse {}: {err}", manifest_path.display()));

    let mut content = Vec::new();
    for rel in &manifest.content {
        let path = dir.join(rel);
        let items: Vec<Content> = ron::de::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read {}: {err}", path.display())),
        )
        .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()));
        content.extend(items.into_iter().map(|item| (rel.clone(), item)));
    }
    let mut sections = Vec::new();
    let mut ships = Vec::new();
    let mut scenarios = Vec::new();
    let mut campaigns = Vec::new();
    let mut lessons = Vec::new();
    let mut ui_themes = Vec::new();
    for (_, item) in &content {
        match item {
            Content::Section(section) => sections.push(section.as_ref().clone()),
            Content::Ship(ship) => ships.push(ship.clone()),
            Content::Scenario(scenario) => scenarios.push(scenario.clone()),
            Content::Campaign(campaign) => campaigns.push(campaign.clone()),
            Content::Lesson(lesson) => lessons.push(lesson.clone()),
            Content::UiTheme(theme) => ui_themes.push(theme.as_ref().clone()),
            // A style has no cross-content references of its own - it
            // names asset paths and nothing else - so it is walked for its
            // resource refs (below) and needs no bucket here.
            Content::Style(_) => {}
        }
    }
    WalkedBundle {
        id: id.to_string(),
        manifest,
        sections,
        ships,
        scenarios,
        campaigns,
        lessons,
        ui_themes,
        content,
        acks: read_acks(dir),
    }
}

/// A lesson finding in the scenario checks' own vocabulary: the report groups
/// findings by element, and a lesson id is an element like a scenario id.
fn lesson_issue(issue: LessonIssue) -> LintIssue {
    LintIssue {
        severity: match issue.severity {
            LessonSeverity::Error => LintSeverity::Error,
            LessonSeverity::Warn => LintSeverity::Warn,
        },
        scenario: issue.lesson,
        message: issue.message,
    }
}

/// A bundle's declared balance acknowledgments. Absent file = no acks; an
/// unparsable one panics like every other malformed bundle file, because a
/// silently dropped ack turns an intended exception back into a warning.
fn read_acks(dir: &Path) -> Vec<BalanceAck> {
    let path = dir.join(BALANCE_ACKS_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    ron::de::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

/// Every mod directory under `parent` (one bundle per subdirectory).
fn bundle_dirs(parent: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .filter_map(|e| {
            let path = e.ok()?.path();
            if !path.is_dir() {
                return None;
            }
            let id = path.file_name()?.to_string_lossy().into_owned();
            Some((id, path))
        })
        .collect();
    dirs.sort();
    dirs
}

/// Every bundle in the repo tree, base first.
fn walk_repo_bundles() -> Vec<WalkedBundle> {
    let root = workspace_root();
    let mut bundles = vec![read_bundle(BASE_MOD_ID, &root.join("assets/base"))];
    for (id, dir) in bundle_dirs(&root.join("assets/mods")) {
        bundles.push(read_bundle(&id, &dir));
    }
    for (id, dir) in bundle_dirs(&root.join("webmods")) {
        bundles.push(read_bundle(&id, &dir));
    }
    bundles
}

/// Lint `bundle`'s scenarios given the whole walked set (known sections
/// = base + the bundle's own + its declared dependencies'; known
/// scenarios = every id across `all` plus the bundle's own).
fn lint_bundle(bundle: &WalkedBundle, all: &[WalkedBundle]) -> Vec<(String, LintIssue)> {
    let sections_by_bundle: HashMap<&str, &[SectionConfig]> = all
        .iter()
        .map(|b| (b.id.as_str(), b.sections.as_slice()))
        .collect();
    let bundles_by_id: HashMap<&str, &WalkedBundle> =
        all.iter().map(|b| (b.id.as_str(), b)).collect();
    let mut known_scenarios: HashSet<String> = all
        .iter()
        .flat_map(|b| b.scenarios.iter().map(|s| s.id.clone()))
        .collect();
    known_scenarios.extend(bundle.scenarios.iter().map(|s| s.id.clone()));
    // The same set keyed by declared role: a campaign member must be a
    // launchable chapter, and a lesson's practice target must be a range built
    // for practising. Both checks read this one map.
    let roles: HashMap<String, ScenarioRole> = all
        .iter()
        .flat_map(|b| b.scenarios.iter())
        .chain(bundle.scenarios.iter())
        .map(|s| (s.id.clone(), s.role))
        .collect();
    let practice_ranges: HashSet<String> = roles
        .iter()
        .filter(|(_, role)| role.is_lesson())
        .map(|(id, _)| id.clone())
        .collect();
    // What may PROVE a lesson: a chapter or a range, never a menu backdrop.
    let launchable: HashSet<String> = roles
        .iter()
        .filter(|(_, role)| !role.is_backdrop())
        .map(|(id, _)| id.clone())
        .collect();
    // Visible prototypes: base + this bundle's own + its declared
    // dependencies' ('base' is implicit and never declared). Full
    // configs, so the catalog can classify mount kinds.
    let mut visible: Vec<&SectionConfig> = sections_by_bundle
        .get(BASE_MOD_ID)
        .map(|s| s.iter().collect())
        .unwrap_or_default();
    for dep in &bundle.manifest.meta.dependencies {
        if let Some(dep_sections) = sections_by_bundle.get(dep.as_str()) {
            visible.extend(dep_sections.iter());
        }
    }
    visible.extend(bundle.sections.iter());
    let known_sections = KnownSections::from_configs(visible);

    // Visible ships, same overlay: base + declared dependencies' + this
    // bundle's own, so a scenario may spawn a base hull by id.
    let ships_by_bundle: HashMap<&str, &[ShipDesignPrototype]> = all
        .iter()
        .map(|b| (b.id.as_str(), b.ships.as_slice()))
        .collect();
    let mut visible_ships: Vec<&ShipDesignPrototype> = ships_by_bundle
        .get(BASE_MOD_ID)
        .map(|s| s.iter().collect())
        .unwrap_or_default();
    for dep in &bundle.manifest.meta.dependencies {
        if let Some(dep_ships) = ships_by_bundle.get(dep.as_str()) {
            visible_ships.extend(dep_ships.iter());
        }
    }
    visible_ships.extend(bundle.ships.iter());
    let known_ships = KnownShipDesigns::from_configs(visible_ships);

    let mut issues = Vec::new();
    for scenario in &bundle.scenarios {
        for issue in lint_scenario(scenario, &known_sections, &known_ships, &known_scenarios) {
            issues.push((bundle.id.clone(), issue));
        }
    }

    // Ship well-formedness: validate every hull THIS bundle ships, so a
    // disconnected or unresolvable ship is caught even when no scenario spawns
    // it. Same rule the section catalog follows below.
    for ship in &bundle.ships {
        for issue in lint_ship_design_config(ship, &known_sections, ship.id.as_str()) {
            issues.push((bundle.id.clone(), issue));
        }
    }

    // Campaign membership: every scenario a campaign lists must resolve to a
    // known scenario (base + all bundles + this bundle's own), or the picker
    // renders a header row launching nothing.
    for campaign in &bundle.campaigns {
        for issue in lint_campaign(campaign, &known_scenarios, &roles) {
            issues.push((bundle.id.clone(), issue));
        }
    }

    // Lesson well-formedness: required fields, the body cap, a loop grid that
    // can actually be cut into frames, and a practice target that both exists
    // and declares itself a range. Keyed by lesson id in the same channel the
    // scenario checks use, so a broken lesson is addressable by name.
    //
    // The orphan check runs over a DIFFERENT pair of sets: the ranges judged
    // are the ones this bundle ships, and the lessons that keep one reachable
    // are every lesson in the walked tree, because a mod's lesson may practise
    // in a base range. Judging base's ranges against a mod's own lessons would
    // report all of them as unreachable in every other bundle's report.
    let own_ranges: HashSet<String> = bundle
        .scenarios
        .iter()
        .filter(|s| s.role.is_lesson())
        .map(|s| s.id.clone())
        .collect();
    let all_lessons: Vec<&Lesson> = all.iter().flat_map(|b| b.lessons.iter()).collect();
    let findings = lint_lessons(
        bundle.lessons.iter(),
        &known_scenarios,
        &practice_ranges,
        &launchable,
    )
    .into_iter()
    .chain(unused_practice_ranges(all_lessons, &own_ranges));
    for issue in findings {
        issues.push((bundle.id.clone(), lesson_issue(issue)));
    }

    // UI-theme well-formedness: a theme must RESOLVE - every role reachable
    // through its inheritance chain, every palette variable it names declared,
    // every hex parseable. The chain is walked against every theme visible to
    // this bundle (base + declared dependencies + its own), because a mod's
    // theme may inherit from a base one. A theme that does not resolve is not
    // a theme that looks wrong; it is one the game would refuse and fall back
    // from, so the player would never see it at all.
    let themes_by_bundle: HashMap<&str, &[UiThemeConfig]> = all
        .iter()
        .map(|b| (b.id.as_str(), b.ui_themes.as_slice()))
        .collect();
    let mut visible_themes: Vec<UiThemeConfig> = themes_by_bundle
        .get(BASE_MOD_ID)
        .map(|t| t.to_vec())
        .unwrap_or_default();
    for dep in &bundle.manifest.meta.dependencies {
        if let Some(dep_themes) = themes_by_bundle.get(dep.as_str()) {
            visible_themes.extend(dep_themes.iter().cloned());
        }
    }
    visible_themes.extend(bundle.ui_themes.iter().cloned());
    for theme in &bundle.ui_themes {
        for issue in lint_theme(theme, &visible_themes) {
            issues.push((
                bundle.id.clone(),
                LintIssue {
                    severity: LintSeverity::Error,
                    scenario: theme.id.clone(),
                    message: issue.message,
                },
            ));
        }
    }

    // Section-config well-formedness (turret joint trees today): validate
    // every section THIS bundle ships, so a malformed turret in a base or
    // mod catalog is caught even when no scenario inlines it.
    for section in &bundle.sections {
        for issue in nova_scenario::prelude::lint_section_config(section, bundle.id.as_str()) {
            issues.push((bundle.id.clone(), issue));
        }
    }

    // Resource-ref membership: every `self:/` ref in this bundle's content -
    // section OR scenario - must name a declared `resources` member of THIS
    // bundle, and every `dep:/<id>/` ref must target a DECLARED dependency and
    // name a declared resource of it. Same gate the portal generator and the
    // runtime merge apply, in the static domain (the deps' resources come from
    // the walked set `all`). Validated once over `bundle.content`, independent
    // of the scenario loop.
    let declared_deps: HashSet<String> =
        bundle.manifest.meta.dependencies.iter().cloned().collect();
    let mut dep_refs: HashMap<String, nova_assets::mod_refs::DepRef> = HashMap::new();
    // `base` is the implicit universal `dep://base` target: supply it from the
    // walked set so `dep://base/X` validates without a `meta.dependencies`
    // entry. (`base: None` - the static lint validates but never rewrites.)
    if let Some(base_bundle) = bundles_by_id.get(BASE_MOD_ID) {
        dep_refs.insert(
            BASE_MOD_ID.to_string(),
            nova_assets::mod_refs::DepRef {
                base: None,
                resources: Some(base_bundle.manifest.resources.as_slice()),
            },
        );
    }
    for dep_id in &bundle.manifest.meta.dependencies {
        if dep_id == BASE_MOD_ID {
            continue;
        }
        if let Some(dep_bundle) = bundles_by_id.get(dep_id.as_str()) {
            dep_refs.insert(
                dep_id.clone(),
                nova_assets::mod_refs::DepRef {
                    base: None,
                    resources: Some(dep_bundle.manifest.resources.as_slice()),
                },
            );
        }
    }
    let scope = nova_assets::mod_refs::RefScope {
        self_base: "",
        self_resources: &bundle.manifest.resources,
        declared_deps: &declared_deps,
        deps: &dep_refs,
    };
    for (_, item) in &bundle.content {
        let (scenario, kind) = (item.id().to_string(), item.kind());
        for message in nova_assets::mod_refs::resource_ref_violations(item, &scope) {
            issues.push((
                bundle.id.clone(),
                LintIssue {
                    severity: LintSeverity::Error,
                    scenario: scenario.clone(),
                    message: format!("{kind} {message}"),
                },
            ));
        }
        // Canonical enforcement: every asset ref must carry a scheme. A bare
        // (scheme-less) asset-path ref no longer resolves (base art lives under
        // assets/base), so it is an Error - caught here at author time rather
        // than as a runtime 404.
        for bare in nova_assets::mod_refs::bare_asset_refs(item) {
            issues.push((
                bundle.id.clone(),
                LintIssue {
                    severity: LintSeverity::Error,
                    scenario: scenario.clone(),
                    message: format!(
                        "{kind} references asset '{bare}' with no scheme - use \
                         'self://{bare}' for this mod's own art or 'dep://<id>/{bare}' for a \
                         dependency's (e.g. 'dep://base/{bare}' for a base-game asset)"
                    ),
                },
            ));
        }
    }
    issues
}

/// One walked bundle's full content for the balance audit: the same walk as the
/// lint, with the parsed section CONFIGS (not just ids) so stats can join
/// through the dependency overlay.
pub struct AuditBundle {
    /// The bundle's id (directory-derived).
    pub id: String,
    /// The ids of bundles this one depends on.
    pub dependencies: Vec<String>,
    /// The bundle's parsed section configs.
    pub sections: Vec<SectionConfig>,
    /// The bundle's parsed ship configs.
    pub ships: Vec<ShipDesignPrototype>,
    /// The bundle's parsed scenario configs.
    pub scenarios: Vec<ScenarioConfig>,
    /// The balance findings this bundle DECLARES intended, read from its own
    /// [`BALANCE_ACKS_FILE`].
    pub acks: Vec<BalanceAck>,
}

/// Every bundle in the repo tree for the balance audit, base first.
pub fn audit_bundles() -> Vec<AuditBundle> {
    walk_repo_bundles()
        .into_iter()
        .map(|bundle| AuditBundle {
            dependencies: bundle.manifest.meta.dependencies.clone(),
            sections: bundle.sections,
            ships: bundle.ships,
            scenarios: bundle.scenarios,
            acks: bundle.acks,
            id: bundle.id,
        })
        .collect()
}

/// Every declared ack in the repo tree, paired with the bundle that declares
/// it - the [`partition_findings`](crate::balance::partition_findings) input
/// matching [`crate::balance::audit_content_tree`]'s findings.
pub fn tree_acks() -> Vec<(String, BalanceAck)> {
    audit_bundles()
        .into_iter()
        .flat_map(|bundle| {
            let id = bundle.id;
            bundle.acks.into_iter().map(move |ack| (id.clone(), ack))
        })
        .collect()
}

/// Walk the whole repo content tree and lint every scenario. Returns
/// `(bundle id, issue)` pairs in a stable order.
pub fn lint_content_tree() -> Vec<(String, LintIssue)> {
    let bundles = walk_repo_bundles();
    let mut issues = Vec::new();
    for bundle in &bundles {
        issues.extend(lint_bundle(bundle, &bundles));
    }
    issues
}

/// Resolve a `--target` argument to a mod directory: an existing
/// directory path wins (an external modder's work-in-progress lives
/// anywhere); otherwise an in-repo id under `webmods/<id>` or
/// `assets/mods/<id>`, with `base` -> `assets/base`.
pub fn resolve_target(arg: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(arg);
    if direct.is_dir() {
        return Some(direct);
    }
    let root = workspace_root();
    if arg == BASE_MOD_ID {
        return Some(root.join("assets/base"));
    }
    for parent in ["webmods", "assets/mods"] {
        let candidate = root.join(parent).join(arg);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// Lint ONE mod (the `--target` mode): the target's scenarios only, but with
/// the FULL repo walk as the known set - chains into base or dependency
/// scenarios resolve, and an external mod (a path outside the repo) still sees
/// the base catalog. The target's dir name is its id (the portal rule); an
/// in-repo target is deduped from the walked set by that id.
pub fn lint_target(dir: &Path) -> Vec<(String, LintIssue)> {
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "target".to_string());
    let target = read_bundle(&id, dir);
    let repo: Vec<WalkedBundle> = walk_repo_bundles()
        .into_iter()
        .filter(|b| b.id != target.id)
        .collect();
    lint_bundle(&target, &repo)
}

/// The effective section catalog of the repo bundles `ids` as ship-part
/// snapshot packs: each bundle, base and every bundle it depends on, once
/// each, in id order. The content lint builds its generated-ship snapshot from
/// the same packs, so a caller hashes and draws from what the lint checked.
///
/// # Panics
///
/// If the repo tree holds no bundle with one of the ids.
pub fn repo_ship_part_packs(ids: &[&str]) -> Vec<ShipPartPack> {
    let all = walk_repo_bundles();
    let by_id: HashMap<&str, &WalkedBundle> = all.iter().map(|b| (b.id.as_str(), b)).collect();
    let mut packs = BTreeMap::new();
    for id in ids {
        let bundle = by_id
            .get(id)
            .unwrap_or_else(|| panic!("the repo tree holds no bundle '{id}'"));
        for pack in ship_part_packs(bundle, &by_id) {
            packs.entry(pack.id.clone()).or_insert(pack);
        }
    }
    packs.into_values().collect()
}

/// `bundle`'s effective section catalog as snapshot packs: base, every bundle
/// it depends on directly or through another, and itself. `base` is every
/// mod's implicit dependency. A dependency missing from the walk is left for
/// the snapshot to name.
fn ship_part_packs(
    bundle: &WalkedBundle,
    by_id: &HashMap<&str, &WalkedBundle>,
) -> Vec<ShipPartPack> {
    let dependencies = |walked: &WalkedBundle| -> Vec<String> {
        let mut dependencies: BTreeSet<String> =
            walked.manifest.meta.dependencies.iter().cloned().collect();
        if walked.id != BASE_MOD_ID {
            dependencies.insert(BASE_MOD_ID.to_string());
        }
        dependencies.into_iter().collect()
    };
    let mut packs = Vec::new();
    let mut seen = HashSet::new();
    let mut pending = vec![bundle];
    while let Some(walked) = pending.pop() {
        if !seen.insert(walked.id.as_str()) {
            continue;
        }
        let dependencies = dependencies(walked);
        pending.extend(
            dependencies
                .iter()
                .filter_map(|dependency| by_id.get(dependency.as_str()).copied()),
        );
        packs.push(ShipPartPack {
            id: walked.id.clone(),
            dependencies,
            sections: walked.sections.clone(),
        });
    }
    packs
}

/// The flight-rig input overlaps in one scenario: every player
/// `input_mapping` binding whose physical source the always-on flight rig
/// already reserves (`consume_input: false`, so both fire). Returns
/// `(section id, colliding source, the flight verb it drives)` - the raw
/// material for an `input-overlap` finding. See
/// `nova_gameplay::flight_rig_reserved_sources` and lesson
/// `input-mapping-overlays-flight-rig`.
///
/// Through [`EventActionConfig::walk`]: the flight rig reserves its sources
/// whenever the player's ship exists, so a player staged from inside a
/// `Sequence` or `Cinematic` beat carries the same overlap as one a handler
/// places itself.
fn scenario_input_overlaps(scenario: &ScenarioConfig) -> Vec<(String, InputSource, String)> {
    let reserved: HashMap<InputSource, &'static str> =
        flight_rig_reserved_sources().into_iter().collect();
    let mut out = Vec::new();
    for action in scenario.events.iter().flat_map(|event| &event.actions) {
        action.walk(&mut |action| {
            let EventActionConfig::SpawnScenarioObject(config) = action else {
                return;
            };
            let ScenarioObjectKind::Spaceship(ship) = &config.kind else {
                return;
            };
            let SpaceshipController::Player(player) = &ship.controller else {
                return;
            };
            // `input_mapping` is ordered, so the findings - and the report
            // file the CI diff compares - come out the same every run.
            for (section_id, bindings) in &player.input_mapping {
                for source in bindings {
                    if let Some(verb) = reserved.get(source) {
                        out.push((section_id.clone(), *source, (*verb).to_string()));
                    }
                }
            }
        });
    }
    out
}

/// The injection actions one scenario's script runs, sorted and deduplicated.
///
/// Empty means the scenario only does bookkeeping: objectives, variables,
/// dialogue and camera. Non-empty makes it a creative map. Computed here and
/// never authored, so a scenario cannot declare itself clean.
fn scenario_injections(scenario: &ScenarioConfig) -> Vec<String> {
    let mut names: BTreeSet<&'static str> = BTreeSet::new();
    for event in &scenario.events {
        for action in &event.actions {
            action.collect_injections(&mut names);
        }
    }
    names.into_iter().map(str::to_string).collect()
}

fn lint_severity(severity: LintSeverity) -> ReportSeverity {
    match severity {
        LintSeverity::Error => ReportSeverity::Error,
        LintSeverity::Warn => ReportSeverity::Warn,
    }
}

/// Build the unified [`ContentReport`] over an already-walked bundle set,
/// reporting on the bundles named in `report_ids`. Runs all three checker
/// families in one pass and attaches each finding's source file from the
/// walk's provenance. `collect_tree` and `collect_target` are the two
/// entry points; the split of `all` vs `report_ids` is what lets a
/// `--target` lint see the whole repo for context while reporting only the
/// target's own findings.
fn build_report(
    all: &[WalkedBundle],
    report_ids: &HashSet<String>,
    target: Option<String>,
) -> ContentReport {
    let by_id: HashMap<&str, &WalkedBundle> = all.iter().map(|b| (b.id.as_str(), b)).collect();
    let file_of = |bundle: &str, element: &str| -> Option<String> {
        by_id
            .get(bundle)
            .and_then(|b| b.file_of(element))
            .map(str::to_string)
    };

    let mut findings = Vec::new();

    // 1. Reference / geometry / resource checks (nova_scenario::lint).
    for bundle in all.iter().filter(|b| report_ids.contains(&b.id)) {
        for (bundle_id, issue) in lint_bundle(bundle, all) {
            let file = file_of(&bundle_id, &issue.scenario);
            findings.push(Finding {
                bundle: bundle_id,
                file,
                severity: lint_severity(issue.severity),
                category: Category::Reference,
                element: issue.scenario,
                message: issue.message,
                suggestion: None,
            });
        }
    }

    // 1b. Generated-ship parts: each reported bundle's effective catalog, base
    // and its transitive dependencies included, must build one snapshot. A
    // fault that names a pack is reported against that pack, reported or not,
    // so a `--target` lint names base for a base stat. An unordered duplicate
    // goes to the first of its two packs a report covers. A missing family
    // belongs to base when base's own catalog misses it too, else to the
    // catalog that raises it. Each fault is reported once per owner, so two
    // mods that each lose the same family are both named.
    let base_id = BASE_MOD_ID.to_string();
    let base_ship_part_faults = by_id
        .get(BASE_MOD_ID)
        .and_then(|base| ShipPartSnapshot::build(&ship_part_packs(base, &by_id)).err())
        .unwrap_or_default();
    let mut ship_part_faults: BTreeSet<(String, String, String)> = BTreeSet::new();
    for bundle in all.iter().filter(|b| report_ids.contains(&b.id)) {
        let Err(faults) = ShipPartSnapshot::build(&ship_part_packs(bundle, &by_id)) else {
            continue;
        };
        for fault in faults {
            let (owner, element) = match &fault {
                ShipPartFault::InvalidStat { pack, id, .. }
                | ShipPartFault::DuplicateInPack { pack, id }
                | ShipPartFault::UnlanedExit { pack, id, .. } => (pack, id.as_str()),
                ShipPartFault::DuplicatePack { pack }
                | ShipPartFault::UnknownDependency { pack, .. } => (pack, "ship parts"),
                ShipPartFault::UnorderedDuplicate { id, first, second } => {
                    let owner = if !report_ids.contains(first) && report_ids.contains(second) {
                        second
                    } else {
                        first
                    };
                    (owner, id.as_str())
                }
                ShipPartFault::MissingFamily(_) if base_ship_part_faults.contains(&fault) => {
                    (&base_id, "ship parts")
                }
                ShipPartFault::MissingFamily(_) => (&bundle.id, "ship parts"),
            };
            ship_part_faults.insert((owner.clone(), element.to_string(), fault.to_string()));
        }
    }
    for (bundle, element, message) in ship_part_faults {
        findings.push(Finding {
            file: file_of(&bundle, &element),
            bundle,
            severity: ReportSeverity::Error,
            category: Category::Reference,
            element,
            message,
            suggestion: None,
        });
    }

    // 2. Balance / fairness audit (nova_authoring::balance), acks applied.
    let audit_bundles: Vec<AuditBundle> = all
        .iter()
        .map(|b| AuditBundle {
            id: b.id.clone(),
            dependencies: b.manifest.meta.dependencies.clone(),
            sections: b.sections.clone(),
            ships: b.ships.clone(),
            scenarios: b.scenarios.clone(),
            acks: b.acks.clone(),
        })
        .collect();
    let audits = crate::balance::audit_bundles_to_audits(&audit_bundles);
    let scenarios_audited = audits
        .iter()
        .filter(|(bundle, _)| report_ids.contains(bundle))
        .count();
    let balance_findings: Vec<(String, crate::balance::BalanceFinding)> = audits
        .iter()
        .filter(|(bundle, _)| report_ids.contains(bundle))
        .flat_map(|(bundle, audit)| {
            audit
                .findings()
                .into_iter()
                .map(move |finding| (bundle.clone(), finding))
        })
        .collect();
    // Only acks in the reported scope: a whole-tree lint prunes every ack,
    // a --target lint only the target's, so another mod's ack is not
    // falsely stale against a single-mod walk.
    let acks: Vec<(String, BalanceAck)> = all
        .iter()
        .filter(|b| report_ids.contains(&b.id))
        .flat_map(|b| b.acks.iter().map(|ack| (b.id.clone(), ack.clone())))
        .collect();
    let (active, acked, stale) = crate::balance::partition_findings(balance_findings, &acks);
    for (bundle, finding) in active {
        let file = file_of(&bundle, &finding.scenario);
        let suggestion = match finding.kind {
            crate::balance::FindingKind::SpawnedDead => Some(format!(
                "spawn '{}' outside its threat envelope, or delay it past OnStart so the \
                 player has fired before it opens up",
                finding.hostile
            )),
            crate::balance::FindingKind::CloseSpawn => Some(format!(
                "distance or delay '{}', or record it in this mod's {BALANCE_ACKS_FILE} if \
                 the close reinforcement is intended",
                finding.hostile
            )),
        };
        findings.push(Finding {
            bundle: bundle.clone(),
            file,
            severity: match finding.severity {
                crate::balance::BalanceSeverity::Error => ReportSeverity::Error,
                crate::balance::BalanceSeverity::Warn => ReportSeverity::Warn,
            },
            category: Category::Balance,
            element: format!("{} > {}", finding.scenario, finding.hostile),
            message: finding.message,
            suggestion,
        });
    }
    // A stale ack fails the build (it names an exception the content moved
    // past), so it is an Error-grade finding here - the exit code keys on
    // error count and this preserves the "non-zero on stale ack" rule.
    for (bundle, ack) in stale {
        findings.push(Finding {
            bundle: bundle.to_string(),
            file: file_of(bundle, &ack.scenario),
            severity: ReportSeverity::Error,
            category: Category::Balance,
            element: format!("{} > {}", ack.scenario, ack.hostile),
            message: format!(
                "stale ack ({}): task {} acknowledged a {} finding for '{}' that no live \
                 audit raises - the content was rebalanced; prune it from the mod's \
                 {BALANCE_ACKS_FILE}",
                ack.kind, ack.task, ack.kind, ack.hostile
            ),
            suggestion: Some(format!("remove the dead entry from {BALANCE_ACKS_FILE}")),
        });
    }
    let acked_findings: Vec<AckedFinding> = acked
        .into_iter()
        .map(|(bundle, finding, ack)| AckedFinding {
            file: file_of(&bundle, &finding.scenario),
            element: format!("{} > {}", finding.scenario, finding.hostile),
            message: finding.message,
            ack_task: ack.task.clone(),
            ack_reason: ack.reason.clone(),
            bundle,
        })
        .collect();

    // 3. Flight-rig input-overlap check.
    for bundle in all.iter().filter(|b| report_ids.contains(&b.id)) {
        for scenario in &bundle.scenarios {
            for (section_id, source, verb) in scenario_input_overlaps(scenario) {
                let key = source.label();
                findings.push(Finding {
                    bundle: bundle.id.clone(),
                    file: file_of(&bundle.id, &scenario.id),
                    severity: ReportSeverity::Warn,
                    category: Category::InputOverlap,
                    element: format!("{} > {}", scenario.id, section_id),
                    message: format!(
                        "section '{section_id}' binds {key}, which the flight rig already \
                         drives ({verb}, consume_input: false) - the key silently \
                         double-drives flight"
                    ),
                    suggestion: Some(format!(
                        "rebind '{section_id}' off {key}; LMB and the analog RightTrigger2 \
                         are free of the flight rig"
                    )),
                });
            }
        }
    }

    // 4. Creative-map classification. Context, not a finding: the report says
    // which scenarios reach into the world, and grades none of them.
    let mut creative_maps = Vec::new();
    for bundle in all.iter().filter(|b| report_ids.contains(&b.id)) {
        for scenario in &bundle.scenarios {
            let actions = scenario_injections(scenario);
            if actions.is_empty() {
                continue;
            }
            creative_maps.push(CreativeMap {
                bundle: bundle.id.clone(),
                file: file_of(&bundle.id, &scenario.id),
                scenario: scenario.id.clone(),
                actions,
            });
        }
    }

    // A finding may name a bundle outside the report, such as a base
    // ship-part fault under `--target`; the report groups by this list.
    let bundles: Vec<String> = all
        .iter()
        .map(|b| b.id.clone())
        .filter(|id| report_ids.contains(id) || findings.iter().any(|f| &f.bundle == id))
        .collect();

    ContentReport {
        target,
        bundles,
        scenarios_audited,
        findings,
        acked: acked_findings,
        creative_maps,
    }
}

/// The unified content report over the whole repo tree: reference/geometry,
/// balance/fairness, and input-overlap findings for every bundle, each
/// located to its source file. This is what `content lint` (no `--target`)
/// produces.
pub fn collect_tree() -> ContentReport {
    let all = walk_repo_bundles();
    let report_ids: HashSet<String> = all.iter().map(|b| b.id.clone()).collect();
    build_report(&all, &report_ids, None)
}

/// The unified content report for ONE mod (`--target`): the same three
/// checker families over just the target's content, but with the full repo
/// walk as context so cross-mod chains, dependency sections and base
/// prototypes still resolve. The target's dir name is its id; an in-repo
/// target is deduped from the walked set by that id.
pub fn collect_target(dir: &Path) -> ContentReport {
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "target".to_string());
    let target = read_bundle(&id, dir);
    let target_id = target.id.clone();
    let mut all: Vec<WalkedBundle> = walk_repo_bundles()
        .into_iter()
        .filter(|b| b.id != target_id)
        .collect();
    all.push(target);
    let report_ids: HashSet<String> = std::iter::once(target_id.clone()).collect();
    build_report(&all, &report_ids, Some(target_id))
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::AssetRef;
    use nova_mod_format::{BundleManifest, ModMeta};
    use nova_modding::prelude::Content;
    use nova_scenario::prelude::ScenarioConfig;
    use nova_world_base::prelude::{ShipPartFamilyType, ShipPartFault};

    use super::{build_report, lint_bundle, scenario_input_overlaps, ReportSeverity, WalkedBundle};

    fn scenario(id: &str, cubemap: &str) -> Content {
        Content::Scenario(ScenarioConfig {
            description: String::new(),
            ..ScenarioConfig::new(
                id.to_string(),
                id.to_string(),
                AssetRef::from(cubemap.to_string()),
            )
        })
    }

    /// A hull section whose render mesh is `render_mesh` (a resource ref).
    fn section(id: &str, render_mesh: &str) -> Content {
        let ron = format!(
            r#"Section((base: (id: "{id}", name: "{id}", description: "", health: 100.0), kind: Hull((render_mesh: Some("{render_mesh}")))))"#
        );
        ron::from_str(&ron).expect("section parses")
    }

    /// A walked bundle from its id, declared deps, declared resources, and
    /// content items (its scenario/section views are derived, as `read_bundle`
    /// does).
    fn walked(id: &str, deps: &[&str], resources: &[&str], content: Vec<Content>) -> WalkedBundle {
        let scenarios = content
            .iter()
            .filter_map(|c| match c {
                Content::Scenario(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        let sections = content
            .iter()
            .filter_map(|c| match c {
                Content::Section(s) => Some(s.as_ref().clone()),
                _ => None,
            })
            .collect();
        let campaigns = content
            .iter()
            .filter_map(|c| match c {
                Content::Campaign(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        let ships = content
            .iter()
            .filter_map(|c| match c {
                Content::Ship(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        let lessons = content
            .iter()
            .filter_map(|c| match c {
                Content::Lesson(lesson) => Some(lesson.clone()),
                _ => None,
            })
            .collect();
        let ui_themes = content
            .iter()
            .filter_map(|c| match c {
                Content::UiTheme(theme) => Some(theme.as_ref().clone()),
                _ => None,
            })
            .collect();
        WalkedBundle {
            id: id.to_string(),
            manifest: BundleManifest {
                content: Vec::new(),
                resources: resources.iter().map(|s| s.to_string()).collect(),
                meta: ModMeta {
                    dependencies: deps.iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                },
                new_game_scenario: None,
            },
            sections,
            ships,
            scenarios,
            campaigns,
            lessons,
            ui_themes,
            // The tests do not exercise multi-file provenance; a single
            // synthetic file name carries every item.
            content: content
                .into_iter()
                .map(|c| ("content.ron".to_string(), c))
                .collect(),
            acks: Vec::new(),
        }
    }

    /// The messages `lint_bundle` raised for `bundle` against `all`.
    fn messages(bundle: &WalkedBundle, all: &[WalkedBundle]) -> Vec<String> {
        lint_bundle(bundle, all)
            .into_iter()
            .map(|(_, issue)| issue.message)
            .collect()
    }

    fn count_containing(msgs: &[String], needle: &str) -> usize {
        msgs.iter().filter(|m| m.contains(needle)).count()
    }

    /// A generated-ship part fault belongs to the pack that authored the bad
    /// definition. A `--target` lint must not blame the target for a base
    /// stat, and a whole-tree lint must not repeat the base fault per mod.
    #[test]
    fn a_ship_part_stat_fault_is_reported_once_against_its_authoring_bundle() {
        let hull = |id: &str, health: f32| -> Content {
            let ron = format!(
                r#"Section((base: (id: "{id}", name: "{id}", description: "", health: {health:?}), kind: Hull((render_mesh: None))))"#
            );
            ron::from_str(&ron).expect("section parses")
        };
        let all = vec![
            walked("base", &[], &[], vec![hull("base_hull", -1.0)]),
            walked("mod", &[], &[], vec![hull("mod_hull", -2.0)]),
        ];
        let located = |report_ids: &[&str]| -> Vec<(String, String, Option<String>)> {
            let report_ids = report_ids.iter().map(|id| id.to_string()).collect();
            let mut located: Vec<_> = build_report(&all, &report_ids, None)
                .findings
                .into_iter()
                .filter(|finding| finding.message.contains("generated ship part"))
                .map(|finding| (finding.bundle, finding.element, finding.file))
                .collect();
            located.sort();
            located
        };
        let expected = vec![
            (
                "base".to_string(),
                "base_hull".to_string(),
                Some("content.ron".to_string()),
            ),
            (
                "mod".to_string(),
                "mod_hull".to_string(),
                Some("content.ron".to_string()),
            ),
        ];

        assert_eq!(located(&["mod"]), expected, "target lint");
        assert_eq!(located(&["base", "mod"]), expected, "whole-tree lint");
        let target_ids = std::iter::once("mod".to_string()).collect();
        let markdown = build_report(&all, &target_ids, Some("mod".to_string())).to_markdown();
        assert!(
            markdown.contains("## base") && markdown.contains("`base_hull`"),
            "the target report lists the base fault under base:\n{markdown}"
        );

        // A family base itself lacks is base's fault, not the target's.
        let empty = vec![
            walked("base", &[], &[], Vec::new()),
            walked("mod", &[], &[], Vec::new()),
        ];
        let missing: Vec<String> = build_report(&empty, &target_ids, None)
            .findings
            .into_iter()
            .filter(|finding| finding.message.starts_with("no usable"))
            .map(|finding| finding.bundle)
            .collect();
        assert!(
            !missing.is_empty() && missing.iter().all(|bundle| bundle == "base"),
            "base's missing families are reported against base: {missing:?}"
        );
    }

    /// Two mods that do not depend on each other can each overlay base's only
    /// cargo intake with an unusable one. Each catalog then lacks the family,
    /// and the whole-tree report names both mods and never base.
    #[test]
    fn two_unrelated_mods_missing_same_ship_part_family_are_each_reported() {
        let socketed = |id: &str, sockets: &[[f32; 3]], kind: &str| -> Content {
            let links: Vec<String> = sockets
                .iter()
                .enumerate()
                .map(|(index, [x, y, z])| {
                    format!(
                        "(id: \"link_{index}\", position: ({:?}, {:?}, {:?}), normal: ({x:?}, {y:?}, {z:?}))",
                        x * 0.5,
                        y * 0.5,
                        z * 0.5
                    )
                })
                .collect();
            let ron = format!(
                r#"Section((base: (id: "{id}", name: "{id}", description: "", health: 100.0, link_points: [{}]), kind: {kind}))"#,
                links.join(", ")
            );
            ron::from_str(&ron).expect("section parses")
        };
        let intake = |sockets: &[[f32; 3]]| {
            socketed(
                "intake",
                sockets,
                r#"CargoIntake((render_mesh: "self://intake.glb#Scene0", canister_mesh: "self://canister.glb#Scene0", door_sound: "self://door.wav", eject_sound: "self://eject.wav", take_sound: "self://take.wav", detection_range: 40.0, capture_gap: 1.0, aperture_width: 8.0, aperture_height: 8.0, maximum_capture_speed: 5.0, eject_speed: 3.0))"#,
            )
        };
        let base = vec![
            socketed(
                "hull",
                &[
                    [1.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, -1.0],
                ],
                "Hull((render_mesh: None))",
            ),
            socketed(
                "controller",
                &[[0.0, 0.0, -1.0]],
                "Controller((steering_lag: 0.5, max_torque: 100.0))",
            ),
            socketed(
                "thruster",
                &[[0.0, 0.0, -1.0]],
                "Thruster((magnitude: 1.0))",
            ),
            intake(&[[0.0, 0.0, 1.0]]),
            socketed(
                "dock",
                &[
                    [1.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [0.0, 0.0, 1.0],
                ],
                "Docking((capture_distance: 10.0, capture_angle: 15.0, maximum_relative_speed: 5.0, maximum_relative_angular_speed: 5.0))",
            ),
        ];
        let all = vec![
            walked("base", &[], &[], base),
            walked("left", &["base"], &[], vec![intake(&[])]),
            walked("right", &["base"], &[], vec![intake(&[])]),
        ];
        let report_ids = ["base", "left", "right"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let mut missing: Vec<(String, String)> = build_report(&all, &report_ids, None)
            .findings
            .into_iter()
            .filter(|finding| finding.message.starts_with("no usable"))
            .map(|finding| (finding.bundle, finding.message))
            .collect();
        missing.sort();
        let message = ShipPartFault::MissingFamily(ShipPartFamilyType::CargoIntake).to_string();
        assert_eq!(
            missing,
            [
                ("left".to_string(), message.clone()),
                ("right".to_string(), message),
            ]
        );
    }

    /// The two exit faults ([`CellGridFault::ObliqueExit`],
    /// [`CellGridFault::SocketOnExitFace`]) are `UnlanedExit` faults, reported
    /// against the pack that authors them exactly like an `InvalidStat`.
    #[test]
    fn an_unlaned_exit_is_reported_once_against_its_authoring_bundle() {
        let bay = |id: &str| -> Content {
            let ron = format!(
                r#"Section((base: (id: "{id}", name: "{id}", description: "", health: 100.0, link_points: [(id: "socket", position: (0.0, 0.0, -0.5), normal: (0.0, 0.0, -1.0))]), kind: Torpedo((spawn_offset: (1.0, 1.0, 0.0), spawn_rotation: (0.0, 0.0, 0.0, 1.0), fire_rate: 1.0, spawner_speed: 80.0, projectile_lifetime: 100.0, arm_time: 0.5, arm_distance: 50.0, nav_constant: 3.0, linear_damping: 0.8, blast_radius: 300.0, blast_damage: 750.0))))"#
            );
            ron::from_str(&ron).expect("section parses")
        };
        let all = vec![
            walked("base", &[], &[], vec![bay("base_bay")]),
            walked("mod", &[], &[], vec![bay("mod_bay")]),
        ];
        // The bundles ship no floor parts, so a whole-tree build also raises
        // `MissingFamily` faults; filter down to the exit fault under test.
        let located =
            |report_ids: &[&str]| -> Vec<(String, String, Option<String>, ReportSeverity)> {
                let report_ids = report_ids.iter().map(|id| id.to_string()).collect();
                let mut located: Vec<_> = build_report(&all, &report_ids, None)
                    .findings
                    .into_iter()
                    .filter(|finding| {
                        finding
                            .message
                            .contains("fires down a direction that is not cardinal")
                    })
                    .map(|finding| {
                        (
                            finding.bundle,
                            finding.element,
                            finding.file,
                            finding.severity,
                        )
                    })
                    .collect();
                located.sort_by_key(|(bundle, element, ..)| (bundle.clone(), element.clone()));
                located
            };
        let expected = vec![
            (
                "base".to_string(),
                "base_bay".to_string(),
                Some("content.ron".to_string()),
                ReportSeverity::Error,
            ),
            (
                "mod".to_string(),
                "mod_bay".to_string(),
                Some("content.ron".to_string()),
                ReportSeverity::Error,
            ),
        ];

        assert_eq!(located(&["mod"]), expected, "target lint");
        assert_eq!(located(&["base", "mod"]), expected, "whole-tree lint");
    }

    #[test]
    fn an_undeclared_self_ref_is_reported_once_for_a_multi_scenario_bundle() {
        // Two scenarios; the FIRST carries one undeclared self:// ref. The
        // membership check must fire ONCE, not once per scenario - the
        // pre-refactor loop nested the content walk inside the scenario loop
        // and double-reported.
        let b = walked(
            "mod",
            &[],
            &["textures/base.png"],
            vec![
                scenario("a", "self://textures/missing.png"),
                scenario("b", "self://textures/base.png"),
            ],
        );
        let msgs = messages(&b, std::slice::from_ref(&b));
        assert_eq!(
            count_containing(&msgs, "self://textures/missing.png"),
            1,
            "reported exactly once, not per scenario: {msgs:?}"
        );
    }

    #[test]
    fn a_section_only_bundle_still_gets_its_refs_checked() {
        // A bundle with ZERO scenarios (only a section). The pre-refactor loop
        // was nested in `for scenario in scenarios`, so a scenario-less bundle
        // skipped the membership check entirely; the refactor checks content
        // regardless of scenario count.
        let b = walked(
            "artmod",
            &[],
            &[],
            vec![section("hull", "self://models/hull.glb")],
        );
        let msgs = messages(&b, std::slice::from_ref(&b));
        assert_eq!(
            count_containing(&msgs, "self://models/hull.glb"),
            1,
            "a section-only bundle's undeclared ref is still caught: {msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.starts_with("section ")),
            "the finding is attributed to the section: {msgs:?}"
        );
    }

    #[test]
    fn a_valid_dep_ref_across_the_walked_set_is_clean() {
        let art = walked("art", &[], &["textures/sky.png"], vec![]);
        let consumer = walked(
            "consumer",
            &["art"],
            &[],
            vec![scenario("c", "dep://art/textures/sky.png")],
        );
        let all = vec![art, consumer];
        let msgs = messages(&all[1], &all);
        assert!(
            !msgs.iter().any(|m| m.contains("dep://")),
            "a declared dep + declared resource lints clean: {msgs:?}"
        );
    }

    #[test]
    fn dep_ref_static_lint_flags_every_bad_case() {
        // `WalkedBundle` is not `Clone`, so each sub-case rebuilds the `art`
        // dependency (ships `textures/sky.png`) fresh alongside its consumer.

        // Undeclared resource OF a declared dependency.
        let all = vec![
            walked("art", &[], &["textures/sky.png"], vec![]),
            walked(
                "consumer",
                &["art"],
                &[],
                vec![scenario("c", "dep://art/textures/missing.png")],
            ),
        ];
        assert!(
            messages(&all[1], &all)
                .iter()
                .any(|m| m.contains("undeclared resource 'dep://art/textures/missing.png'")),
            "an undeclared dependency resource is flagged"
        );

        // A dep that is NOT declared as a dependency.
        let all = vec![
            walked("art", &[], &["textures/sky.png"], vec![]),
            walked(
                "consumer",
                &[],
                &[],
                vec![scenario("c", "dep://art/textures/sky.png")],
            ),
        ];
        assert!(
            messages(&all[1], &all)
                .iter()
                .any(|m| m.contains("not a declared dependency")),
            "a ref to a non-declared dependency is flagged"
        );

        // dep://base to a DECLARED base resource is valid WITHOUT the consumer
        // declaring base (base is the implicit universal dependency).
        let all = vec![
            walked("base", &[], &["textures/cubemap.png"], vec![]),
            walked(
                "consumer",
                &[],
                &[],
                vec![scenario("c", "dep://base/textures/cubemap.png")],
            ),
        ];
        assert!(
            !messages(&all[1], &all)
                .iter()
                .any(|m| m.contains("dep://base")),
            "dep://base to a declared base resource lints clean without declaring base: {:?}",
            messages(&all[1], &all)
        );

        // dep://base to an UNDECLARED base resource is still flagged.
        let all = vec![
            walked("base", &[], &["textures/cubemap.png"], vec![]),
            walked(
                "consumer",
                &[],
                &[],
                vec![scenario("c", "dep://base/textures/missing.png")],
            ),
        ];
        assert!(
            messages(&all[1], &all)
                .iter()
                .any(|m| m.contains("undeclared resource 'dep://base/textures/missing.png'")),
            "an undeclared base resource is flagged"
        );
    }

    #[test]
    fn a_bare_asset_ref_is_a_lint_error() {
        // A scheme-less asset ref (the retired bare convention) is now an
        // Error - it no longer resolves, so it is caught at author time.
        let b = walked("mod", &[], &[], vec![scenario("s", "textures/cubemap.png")]);
        let msgs = messages(&b, std::slice::from_ref(&b));
        assert!(
            msgs.iter()
                .any(|m| m.contains("references asset 'textures/cubemap.png' with no scheme")),
            "a bare asset ref is flagged: {msgs:?}"
        );
        // A schemed ref is clean (base ships the resource in the walked set).
        let all = vec![
            walked("base", &[], &["textures/cubemap.png"], vec![]),
            walked(
                "mod",
                &[],
                &[],
                vec![scenario("s", "dep://base/textures/cubemap.png")],
            ),
        ];
        assert!(
            !messages(&all[1], &all)
                .iter()
                .any(|m| m.contains("no scheme")),
            "a schemed ref is not flagged as bare"
        );
    }

    /// The flight rig reserves its sources for as long as the player's ship
    /// exists, so WHERE the spawn is authored cannot change the answer. A
    /// player staged from inside a `Sequence` beat was invisible to the flat
    /// read of each handler's own action list, and a mod could ship a Space
    /// binding that silently fired the burn as well as the guns.
    #[test]
    fn an_input_overlap_is_found_on_a_player_staged_inside_a_sequence_step() {
        let ron = r#"Scenario((
            id: "staged_player",
            name: "Staged Player",
            description: "",
            cubemap: "dep://base/textures/cubemap.png",
            events: [
                (
                    name: OnStart,
                    actions: [
                        Sequence((
                            key: "arrival",
                            steps: [
                                (
                                    after: Some(2.0),
                                    actions: [
                                        SpawnScenarioObject((
                                            base: (id: "player", name: "Player", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                                            kind: Spaceship((
                                                controller: Player((
                                                    input_mapping: {
                                                        "guns": [ Keyboard(Space) ],
                                                    },
                                                )),
                                                design: Inline((sections: [])),
                                                inventory: {},
                                                lootable: false,
                                                credits: 0,
                                            )),
                                        )),
                                    ],
                                ),
                            ],
                        )),
                    ],
                ),
            ],
        ))"#;
        let Content::Scenario(staged) = ron::from_str::<Content>(ron).expect("scenario parses")
        else {
            unreachable!("the fixture is a Scenario");
        };

        let overlaps = scenario_input_overlaps(&staged);

        assert_eq!(
            overlaps.len(),
            1,
            "the staged player's Space binding still collides with the flight rig: {overlaps:?}"
        );
        assert_eq!(overlaps[0].0, "guns");
    }
}
