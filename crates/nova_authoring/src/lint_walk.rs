//! The content-lint walk: reads a mod tree (or single target) off disk, parses
//! every bundle's content, runs the reference/geometry, balance and
//! input-overlap checks, and assembles the unified
//! [`ContentReport`]. Used by the
//! `content lint` CLI and the CI gate test; not part of the game runtime.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use nova_mod_format::BundleManifest;
use nova_modding::prelude::Content;
use nova_scenario::prelude::{
    lint_campaign, lint_scenario, CampaignConfig, EventActionConfig, KnownSections, LintIssue,
    LintSeverity, ScenarioConfig, ScenarioObjectKind, SpaceshipController,
};
use nova_ship::prelude::{binding_source, flight_rig_reserved_sources, InputSource, SectionConfig};

use crate::content_report::{
    AckedFinding, Category, ContentReport, Finding, Severity as ReportSeverity,
};

/// The two report walks (`collect_*`), the two issue-only walks (`lint_*`),
/// the target resolver, and the balance-input view of a walked tree.
pub mod prelude {
    pub use super::{
        audit_bundles, collect_target, collect_tree, lint_content_tree, lint_target,
        resolve_target, AuditBundle,
    };
}

/// One walked bundle: its id (directory-derived), manifest, parsed content
/// items, and the section / scenario views derived from them.
struct WalkedBundle {
    id: String,
    manifest: BundleManifest,
    sections: Vec<SectionConfig>,
    scenarios: Vec<ScenarioConfig>,
    campaigns: Vec<CampaignConfig>,
    /// Every parsed content item paired with the bundle-relative file it was
    /// read from (a bundle lists several content files). Kept so the
    /// mod-relative `self://` resource-ref check can see every kind, and so
    /// the unified report can point each finding at its source file.
    content: Vec<(String, Content)>,
}

impl WalkedBundle {
    /// The bundle-relative file a content element (scenario, section or
    /// campaign id) was authored in, for report provenance. `None` when no
    /// content item carries that id (e.g. a finding about a missing/foreign
    /// prototype).
    fn file_of(&self, element_id: &str) -> Option<&str> {
        self.content.iter().find_map(|(file, item)| {
            let id = match item {
                Content::Scenario(cfg) => cfg.id.as_str(),
                Content::Section(cfg) => cfg.base.id.as_str(),
                Content::Campaign(cfg) => cfg.id.as_str(),
            };
            (id == element_id).then_some(file.as_str())
        })
    }
}

/// The workspace root (this crate sits at `crates/nova_assets`).
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
    let mut scenarios = Vec::new();
    let mut campaigns = Vec::new();
    for (_, item) in &content {
        match item {
            Content::Section(section) => sections.push(section.as_ref().clone()),
            Content::Scenario(scenario) => scenarios.push(scenario.clone()),
            Content::Campaign(campaign) => campaigns.push(campaign.clone()),
        }
    }
    WalkedBundle {
        id: id.to_string(),
        manifest,
        sections,
        scenarios,
        campaigns,
        content,
    }
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
    let mut bundles = vec![read_bundle("base", &root.join("assets/base"))];
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

    // Visible prototypes: base + this bundle's own + its declared
    // dependencies' ('base' is implicit and never declared). Full
    // configs, so the catalog can classify mount kinds.
    let mut visible: Vec<&SectionConfig> = sections_by_bundle
        .get("base")
        .map(|s| s.iter().collect())
        .unwrap_or_default();
    visible.extend(bundle.sections.iter());
    for dep in &bundle.manifest.meta.dependencies {
        if let Some(dep_sections) = sections_by_bundle.get(dep.as_str()) {
            visible.extend(dep_sections.iter());
        }
    }
    let known_sections = KnownSections::from_configs(visible);

    let mut issues = Vec::new();
    for scenario in &bundle.scenarios {
        for issue in lint_scenario(scenario, &known_sections, &known_scenarios) {
            issues.push((bundle.id.clone(), issue));
        }
    }

    // Campaign membership: every scenario a campaign lists must resolve to a
    // known scenario (base + all bundles + this bundle's own), or the picker
    // renders a header row launching nothing.
    for campaign in &bundle.campaigns {
        for issue in lint_campaign(campaign, &known_scenarios) {
            issues.push((bundle.id.clone(), issue));
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
    if let Some(base_bundle) = bundles_by_id.get("base") {
        dep_refs.insert(
            "base".to_string(),
            nova_assets::mod_refs::DepRef {
                base: None,
                resources: Some(base_bundle.manifest.resources.as_slice()),
            },
        );
    }
    for dep_id in &bundle.manifest.meta.dependencies {
        if dep_id == "base" {
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
        let (scenario, kind) = match item {
            Content::Scenario(cfg) => (cfg.id.clone(), "scenario"),
            Content::Section(cfg) => (cfg.base.id.clone(), "section"),
            Content::Campaign(cfg) => (cfg.id.clone(), "campaign"),
        };
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
    /// The bundle's parsed scenario configs.
    pub scenarios: Vec<ScenarioConfig>,
}

/// Every bundle in the repo tree for the balance audit, base first.
pub fn audit_bundles() -> Vec<AuditBundle> {
    walk_repo_bundles()
        .into_iter()
        .map(|bundle| AuditBundle {
            dependencies: bundle.manifest.meta.dependencies.clone(),
            sections: bundle.sections,
            scenarios: bundle.scenarios,
            id: bundle.id,
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
    if arg == "base" {
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

/// The flight-rig input overlaps in one scenario: every player
/// `input_mapping` binding whose physical source the always-on flight rig
/// already reserves (`consume_input: false`, so both fire). Returns
/// `(section id, colliding source, the flight verb it drives)` - the raw
/// material for an `input-overlap` finding. See
/// `nova_gameplay::flight_rig_reserved_sources` and lesson
/// `input-mapping-overlays-flight-rig`.
fn scenario_input_overlaps(scenario: &ScenarioConfig) -> Vec<(String, InputSource, String)> {
    let reserved: HashMap<InputSource, &'static str> =
        flight_rig_reserved_sources().into_iter().collect();
    let mut out = Vec::new();
    for event in &scenario.events {
        for action in &event.actions {
            let EventActionConfig::SpawnScenarioObject(config) = action else {
                continue;
            };
            let ScenarioObjectKind::Spaceship(ship) = &config.kind else {
                continue;
            };
            let SpaceshipController::Player(player) = &ship.controller else {
                continue;
            };
            // Sorted: `input_mapping` is a `HashMap`, so iterating it raw makes
            // the findings (and the report file the CI diff compares) come out
            // in a different order every run.
            let mut sections: Vec<_> = player.input_mapping.iter().collect();
            sections.sort_by_key(|(id, _)| *id);
            for (section_id, bindings) in sections {
                for binding in bindings {
                    if let Some(source) = binding_source(binding) {
                        if let Some(verb) = reserved.get(&source) {
                            out.push((section_id.clone(), source, (*verb).to_string()));
                        }
                    }
                }
            }
        }
    }
    out
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

    // 2. Balance / fairness audit (nova_authoring::balance), acks applied.
    let audit_bundles: Vec<AuditBundle> = all
        .iter()
        .map(|b| AuditBundle {
            id: b.id.clone(),
            dependencies: b.manifest.meta.dependencies.clone(),
            sections: b.sections.clone(),
            scenarios: b.scenarios.clone(),
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
    let acks: Vec<crate::balance::BalanceAck> = crate::balance::shipped_acks()
        .into_iter()
        .filter(|ack| report_ids.contains(&ack.bundle))
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
                "distance or delay '{}', or record it in balance_acks.ron if the close \
                 reinforcement is intended",
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
    for ack in stale {
        findings.push(Finding {
            bundle: ack.bundle.clone(),
            file: file_of(&ack.bundle, &ack.scenario),
            severity: ReportSeverity::Error,
            category: Category::Balance,
            element: format!("{} > {}", ack.scenario, ack.hostile),
            message: format!(
                "stale ack ({}): task {} acknowledged a {} finding for '{}' that no live \
                 audit raises - the content was rebalanced; prune it from balance_acks.ron",
                ack.kind, ack.task, ack.kind, ack.hostile
            ),
            suggestion: Some("remove the dead entry from balance_acks.ron".to_string()),
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

    let bundles: Vec<String> = all
        .iter()
        .map(|b| b.id.clone())
        .filter(|id| report_ids.contains(id))
        .collect();

    ContentReport {
        target,
        bundles,
        scenarios_audited,
        findings,
        acked: acked_findings,
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

    use super::{lint_bundle, WalkedBundle};

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
            r#"Section((base: (id: "{id}", name: "{id}", description: "", mass: 1.0, health: 100.0), kind: Hull((render_mesh: Some("{render_mesh}")))))"#
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
            scenarios,
            campaigns,
            // The tests do not exercise multi-file provenance; a single
            // synthetic file name carries every item.
            content: content
                .into_iter()
                .map(|c| ("content.ron".to_string(), c))
                .collect(),
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
}
