//! The content MERGE: flatten every enabled bundle's `Content` in dependency
//! order and overlay it by id into the game's registries (`GameSections`,
//! `GameScenarios`, `GameCampaigns`), linting the result as it goes.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_modding::prelude::{BundleAsset, Content, ContentAsset, InstalledCatalog};
use nova_scenario::prelude::{GameCampaigns, GameScenarios, NewGameStart};

use crate::{
    collections::GameAssets,
    mod_refs,
    mod_set::{DownloadedMods, EnabledMods},
};

/// Route every ENABLED cataloged bundle's content into the id-keyed game registries,
/// with load-order overlay.
///
/// It walks the catalog in order, keeps the entries whose id is in [`EnabledMods`]
/// (base first, by catalog order), flattens each kept bundle's content (in manifest
/// order, across its content files), and hands the whole ordered list to
/// [`merge_bundles`]. A LATER (mod) bundle wins on an id collision with the base
/// (load-order overlay); a duplicate id WITHIN one bundle is a conflict, logged and
/// skipped. Both resources are always inserted (empty if nothing enabled/loaded).
///
/// The catalog is part of the `GameAssets` collection and visits every installed
/// bundle as a dependency, so bevy_asset_loader gates the collection on the whole
/// tree's RECURSIVE load state - every installed bundle + content file is loaded
/// before this first runs `OnEnter(Processing)`, regardless of which are enabled. A
/// handle whose asset is somehow not loaded is logged and skipped (never a panic).
/// Re-runs whenever `EnabledMods` changes so a menu toggle applies live.
///
/// ENABLED DOWNLOADED bundles ([`DownloadedMods`]) merge AFTER the shipped ones,
/// in cache-index order, through the same overlay rules. They sit outside the
/// collection gate (loaded async via `mods://`), so a still-loading bundle is
/// skipped with a warning;
/// [`mark_downloaded_bundles_loaded`](crate::mark_downloaded_bundles_loaded)
/// re-triggers this system when the load lands, and a `DownloadedMods` change
/// (install/uninstall) re-triggers it too.
pub fn register_bundles(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    enabled: Res<EnabledMods>,
    downloaded: Res<DownloadedMods>,
    catalogs: Res<Assets<InstalledCatalog>>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
) {
    // Ordered ENABLED bundle handles: catalog order (base first), keeping only
    // entries whose id is enabled.
    let catalog = catalogs.get(&game_assets.catalog);
    if catalog.is_none() {
        error!("register_bundles: the mods catalog was not loaded; registering nothing");
    }
    // Enabled (id, bundle) pairs in catalog order (base first) then downloaded
    // order - the stable tiebreak the dependency sort keeps below.
    let mut ordered: Vec<(&str, &Handle<BundleAsset>)> = Vec::new();
    if let Some(catalog) = catalog {
        for entry in &catalog.entries {
            if enabled.0.contains(&entry.decl.id) {
                ordered.push((entry.decl.id.as_str(), &entry.bundle));
            }
        }
    }
    for m in &downloaded.0 {
        if !enabled.0.contains(&m.record.id) {
            continue;
        }
        // A downloaded id shadowing a SHIPPED catalog entry is skipped (the
        // portal generator's no-shadowing rule, enforced again at the merge
        // because the index is downloaded input) - otherwise one enabled id
        // would merge two bundles. `build_mod_catalog` hides the same records
        // from the rows.
        if catalog.is_some_and(|c| c.entries.iter().any(|e| e.decl.id == m.record.id)) {
            warn!(
                "register_bundles: downloaded mod '{}' shadows a shipped mod id; \
                 skipping the downloaded copy",
                m.record.id
            );
            continue;
        }
        // Unlike the shipped entries above (gated loaded by the collection), a
        // downloaded bundle may still be in flight; skipping it here is a
        // TRANSIENT state, not the shared "somehow not loaded" error below -
        // the loaded-event re-run merges it in.
        if bundles.contains(&m.bundle) {
            ordered.push((m.record.id.as_str(), &m.bundle));
        } else {
            warn!(
                "register_bundles: downloaded mod '{}' is enabled but its bundle has not \
                 loaded yet; it merges when the load completes",
                m.record.id
            );
        }
    }

    // Dependency-respecting merge order: a mod's Content overlays its
    // dependencies' (last-wins by id), so a dependency must merge BEFORE its
    // dependents. Build the id->deps graph from the loaded bundles' meta and
    // topologically sort, keeping the catalog-then-download order as the stable
    // tiebreak. `base` is implicit (first in catalog order, no incoming edges)
    // so it stays first. A cycle - which the portal generator rejects at
    // publish, but a hand-installed set could carry - warns and falls back to
    // input order.
    //
    // The graph only carries edges for bundles that are LOADED (`bundles.get`);
    // an enabled dependent whose bundle is still loading contributes no edges and
    // may briefly merge before its dependency, but that is transient - the
    // loaded-event re-run of this system (above) rebuilds with the full graph.
    let graph: nova_mod_format::deps::DepGraph = ordered
        .iter()
        .filter_map(|(id, handle)| {
            bundles
                .get(*handle)
                .map(|b| (id.to_string(), b.meta.dependencies.clone()))
        })
        .collect();
    let ids: Vec<String> = ordered.iter().map(|(id, _)| id.to_string()).collect();
    let topo = nova_mod_format::deps::topological_order(&ids, &graph);
    if topo.cycle {
        warn!(
            "register_bundles: a dependency cycle among enabled mods prevents a full \
             topological order; merging the cyclic mods in catalog order"
        );
    }
    // `ordered`'s ids are unique (a downloaded id that shadows a shipped one is
    // skipped above), so this id->handle map never drops a bundle.
    let by_id: HashMap<&str, &Handle<BundleAsset>> =
        ordered.iter().map(|(id, h)| (*id, *h)).collect();
    let bundle_handles: Vec<&Handle<BundleAsset>> = topo
        .order
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .collect();

    // Flatten each enabled bundle into its ordered `Content` items (missing
    // content is logged and skipped), rewriting the bundle's resource refs -
    // `self:/` against its own folder and `dep:/<id>/` against a declared
    // dependency's folder. Items are OWNED because the rewrite produces new
    // configs. Kept as one Vec per bundle so `merge_bundles` can tell
    // intra-bundle duplicates from cross-bundle overlay.
    //
    // A ref that names no declared resource (a `self:/` file the mod does not
    // ship, or a `dep:/` file/dependency it may not reach) is recorded as an
    // Error content issue against the owning item's id - section, scenario or
    // campaign alike (below) - so the runtime gate refuses it, mirroring the
    // portal generator's and static lint's checks.
    // The topological order guarantees a dependency is flattened before its
    // dependents, so its `resource_base`/`resources` are already loaded here.
    let mut bundle_items: Vec<Vec<Content>> = Vec::new();
    let mut undeclared_ref_issues: Vec<(String, String)> = Vec::new();
    for bundle_handle in bundle_handles {
        let Some(bundle) = bundles.get(bundle_handle) else {
            error!(
                "register_bundles: a bundle asset was not loaded; skipping it \
                 (the other bundles still register)"
            );
            continue;
        };
        // The owning bundle's resolution context: its own folder + the DECLARED
        // dependencies whose bundles are enabled+loaded (looked up in the merge
        // set `by_id`), PLUS `base` (the implicit universal `dep://base` target).
        // A declared dep absent here is unavailable, so a `dep://` to it is a
        // violation.
        let declared_deps: HashSet<String> = bundle.meta.dependencies.iter().cloned().collect();
        let mut dep_refs: HashMap<String, mod_refs::DepRef> = HashMap::new();
        // `base` is always enabled (the `base: true` catalog entry) and in `by_id`;
        // supply it so `dep://base/X` resolves and is membership-checked without a
        // `meta.dependencies` entry. Any explicit `base` in `meta.dependencies` is
        // subsumed here (and skipped below).
        if let Some(base_bundle) = by_id.get("base").and_then(|h| bundles.get(*h)) {
            dep_refs.insert(
                "base".to_string(),
                mod_refs::DepRef {
                    base: Some(base_bundle.resource_base.as_str()),
                    resources: Some(base_bundle.resources.as_slice()),
                },
            );
        }
        for dep_id in &bundle.meta.dependencies {
            if dep_id == "base" {
                continue;
            }
            if let Some(dep_bundle) = by_id.get(dep_id.as_str()).and_then(|h| bundles.get(*h)) {
                dep_refs.insert(
                    dep_id.clone(),
                    mod_refs::DepRef {
                        base: Some(dep_bundle.resource_base.as_str()),
                        resources: Some(dep_bundle.resources.as_slice()),
                    },
                );
            }
        }
        let scope = mod_refs::RefScope {
            self_base: &bundle.resource_base,
            self_resources: &bundle.resources,
            declared_deps: &declared_deps,
            deps: &dep_refs,
        };

        let mut items: Vec<Content> = Vec::new();
        for content_handle in &bundle.content {
            let Some(content) = contents.get(content_handle) else {
                error!(
                    "register_bundles: a content asset was not loaded; skipping it \
                     (the other content still registers)"
                );
                continue;
            };
            for item in &content.0 {
                for message in mod_refs::resource_ref_violations(item, &scope) {
                    error!("register_bundles: content {message}");
                    // Every kind, not just scenarios: a section or campaign
                    // with a bad ref was logged and merged anyway, so the
                    // runtime gate never saw it.
                    let id = match item {
                        Content::Section(cfg) => cfg.base.id.clone(),
                        Content::Scenario(cfg) => cfg.id.clone(),
                        Content::Campaign(cfg) => cfg.id.clone(),
                    };
                    undeclared_ref_issues.push((id, message));
                }
                items.push(mod_refs::rewrite_refs(item, &scope));
            }
        }
        bundle_items.push(items);
    }

    let outcome = merge_bundles(bundle_items.iter().map(|items| items.iter()));
    for conflict in &outcome.conflicts {
        error!("register_bundles: {conflict}");
    }

    // The New Game start comes from the BASE bundle's manifest and ONLY from
    // it: any other bundle declaring `new_game_scenario` - shipped or
    // downloaded - is warned about and ignored, so a mod can never redirect
    // what New Game launches.
    let mut new_game: Option<String> = None;
    if let Some(catalog) = catalog {
        for entry in &catalog.entries {
            let Some(bundle) = bundles.get(&entry.bundle) else {
                continue;
            };
            let Some(declared) = &bundle.new_game_scenario else {
                continue;
            };
            if entry.decl.base {
                new_game = Some(declared.clone());
            } else {
                warn!(
                    "register_bundles: mod '{}' declares new_game_scenario '{declared}'; \
                     ignored - only the base bundle picks the New Game start",
                    entry.decl.id
                );
            }
        }
    }
    for m in &downloaded.0 {
        if let Some(declared) = bundles
            .get(&m.bundle)
            .and_then(|b| b.new_game_scenario.as_ref())
        {
            warn!(
                "register_bundles: downloaded mod '{}' declares new_game_scenario '{declared}'; \
                 ignored - only the base bundle picks the New Game start",
                m.record.id
            );
        }
    }
    commands.insert_resource(NewGameStart(new_game));

    // The runtime content gate: lint every registered scenario against the
    // MERGED registries - the only place cross-mod references are decidable.
    // `on_load_scenario` refuses Error-flagged scenarios; the menu filters them
    // out of the backdrop draw.
    let merged_sections =
        nova_scenario::prelude::KnownSections::from_configs(outcome.sections.iter());
    let merged_scenarios: std::collections::HashSet<String> =
        outcome.scenarios.keys().cloned().collect();
    let mut content_issues = nova_scenario::prelude::ContentIssues::default();
    for scenario in outcome.scenarios.values() {
        let found =
            nova_scenario::prelude::lint_scenario(scenario, &merged_sections, &merged_scenarios);
        for issue in &found {
            warn!(
                "register_bundles: content lint [{:?}] scenario '{}': {}",
                issue.severity, issue.scenario, issue.message
            );
        }
        if !found.is_empty() {
            content_issues.0.insert(scenario.id.clone(), found);
        }
    }
    // Campaign membership: every member id a campaign lists must resolve to a
    // merged scenario, or the picker renders a header row that launches nothing.
    // Findings are keyed by the campaign id in the shared ContentIssues channel.
    for campaign in outcome.campaigns.values() {
        let found = nova_scenario::prelude::lint_campaign(campaign, &merged_scenarios);
        for issue in &found {
            warn!(
                "register_bundles: content lint [{:?}] campaign '{}': {}",
                issue.severity, issue.scenario, issue.message
            );
        }
        if !found.is_empty() {
            content_issues
                .0
                .entry(campaign.id.clone())
                .or_default()
                .extend(found);
        }
    }
    // Fold in the resource-ref findings gathered while flattening (undeclared
    // `self://` and ungated `dep://<id>/` refs): an Error per (content id,
    // message) so the gate refuses that item.
    for (content_id, message) in undeclared_ref_issues {
        content_issues
            .0
            .entry(content_id.clone())
            .or_default()
            .push(nova_scenario::prelude::LintIssue {
                severity: nova_scenario::prelude::LintSeverity::Error,
                scenario: content_id,
                message,
            });
    }
    commands.insert_resource(content_issues);

    commands.insert_resource(GameSections(outcome.sections));
    commands.insert_resource(outcome.scenarios);
    commands.insert_resource(outcome.campaigns);
}

/// The result of merging an ordered list of bundles: the id-keyed registries plus
/// any intra-bundle id conflicts that were detected (and skipped).
pub struct MergeOutcome {
    /// Sections in registration order (base then mods), overlaid last-wins by id.
    pub sections: Vec<SectionConfig>,
    /// Scenarios keyed by id, overlaid last-wins.
    pub scenarios: GameScenarios,
    /// Campaigns keyed by id, overlaid last-wins (a later bundle may replace an
    /// earlier campaign's membership by declaring the same id).
    pub campaigns: GameCampaigns,
    /// Human-readable messages, one per intra-bundle duplicate id that was
    /// skipped. Empty on clean data.
    pub conflicts: Vec<String>,
}

/// Merge an ORDERED list of bundles into the id-keyed registries. Each bundle is
/// an ordered list of its `&Content` items (already flattened across the bundle's
/// content files).
///
/// Two overlay rules, mirroring Wesnoth's base+addons model:
/// - CROSS-bundle (a later bundle vs an earlier one): last-wins overlay by id -
///   a mod's `Content` with the same id as the base REPLACES it. This is the
///   whole point of mods.
/// - INTRA-bundle (the same id twice in ONE bundle - including the BASE bundle,
///   whose content files flatten into one bundle): a conflict. The first item is
///   kept, the duplicate is skipped, and a message is recorded. This is an
///   authoring error in any pack, surfaced loudly (the caller logs it) rather than
///   silently last-wins-overlaid like the cross-bundle case - but NOT a panic, so
///   bad mod (or base) data cannot crash the app.
pub fn merge_bundles<'a, B, I>(bundles: B) -> MergeOutcome
where
    B: IntoIterator<Item = I>,
    I: IntoIterator<Item = &'a Content>,
{
    let mut sections: Vec<SectionConfig> = Vec::new();
    let mut scenarios = GameScenarios::default();
    let mut campaigns = GameCampaigns::default();
    let mut conflicts: Vec<String> = Vec::new();

    for bundle in bundles {
        // Ids seen in THIS bundle, per kind - reset each bundle so a later bundle
        // may overlay an earlier one, while a repeat within one bundle conflicts.
        let mut seen_sections: HashSet<&str> = HashSet::new();
        let mut seen_scenarios: HashSet<&str> = HashSet::new();
        let mut seen_campaigns: HashSet<&str> = HashSet::new();

        for item in bundle {
            match item {
                Content::Section(cfg) => {
                    if !seen_sections.insert(cfg.base.id.as_str()) {
                        conflicts.push(format!(
                            "section id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.base.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios, &mut campaigns);
                }
                Content::Scenario(cfg) => {
                    if !seen_scenarios.insert(cfg.id.as_str()) {
                        conflicts.push(format!(
                            "scenario id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios, &mut campaigns);
                }
                Content::Campaign(cfg) => {
                    if !seen_campaigns.insert(cfg.id.as_str()) {
                        conflicts.push(format!(
                            "campaign id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios, &mut campaigns);
                }
            }
        }
    }

    MergeOutcome {
        sections,
        scenarios,
        campaigns,
        conflicts,
    }
}

/// Route one content item into the accumulating registries with last-wins
/// overlay by id. All kinds overlay identically: a later item (from a later
/// bundle) with the same id replaces the earlier one rather than appending a
/// shadowed duplicate. Sections keep a Vec (order matters for the editor palette)
/// so overlay is a linear replace-in-place; scenarios and campaigns are maps so
/// overlay is a plain `insert`. Called by [`merge_bundles`] once per accepted
/// item.
fn merge_content_item(
    item: &Content,
    sections: &mut Vec<SectionConfig>,
    scenarios: &mut GameScenarios,
    campaigns: &mut GameCampaigns,
) {
    match item {
        Content::Section(cfg) => match sections.iter_mut().find(|s| s.base.id == cfg.base.id) {
            Some(existing) => *existing = cfg.as_ref().clone(),
            None => sections.push(cfg.as_ref().clone()),
        },
        Content::Scenario(cfg) => {
            scenarios.insert(cfg.id.clone(), cfg.clone());
        }
        Content::Campaign(cfg) => {
            campaigns.insert(cfg.id.clone(), cfg.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{BaseSectionConfig, HullSectionConfig, SectionKind};

    use super::*;
    use crate::scenario_generation;

    fn section(id: &str, health: f32) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                ..Default::default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A later content item with the same section id overlays the earlier one
    /// (last-wins) instead of appending a shadowed duplicate, and does so
    /// in-place so the palette order is preserved. This is the seam mods rely
    /// on, mirroring the scenario map's insert-overlay.
    #[test]
    fn later_section_overlays_earlier_by_id_in_place() {
        let mut sections: Vec<SectionConfig> = Vec::new();
        let mut scenarios = GameScenarios::default();
        let mut campaigns = GameCampaigns::default();

        // Base bundle: two sections in palette order.
        merge_content_item(
            &Content::Section(Box::new(section("hull", 100.0))),
            &mut sections,
            &mut scenarios,
            &mut campaigns,
        );
        merge_content_item(
            &Content::Section(Box::new(section("thruster", 50.0))),
            &mut sections,
            &mut scenarios,
            &mut campaigns,
        );

        // Mod bundle: overlays "hull" with a new health, leaves "thruster".
        merge_content_item(
            &Content::Section(Box::new(section("hull", 999.0))),
            &mut sections,
            &mut scenarios,
            &mut campaigns,
        );

        // No duplicate appended: still two sections, original order kept.
        assert_eq!(sections.len(), 2, "overlay must replace, not append");
        assert_eq!(sections[0].base.id, "hull", "palette order preserved");
        assert_eq!(sections[1].base.id, "thruster");
        // Last-wins: the overlaid value took effect.
        assert_eq!(sections[0].base.health, 999.0, "later section must win");
    }

    /// A later scenario with the same id overlays the earlier one, same as
    /// sections - the two kinds must behave identically under overlay.
    #[test]
    fn later_scenario_overlays_earlier_by_id() {
        let mut sections: Vec<SectionConfig> = Vec::new();
        let mut scenarios = GameScenarios::default();
        let mut campaigns = GameCampaigns::default();

        // Reuse a real built scenario (no Default on ScenarioConfig) and overlay
        // a second config sharing its id but with a different name.
        let mut base = scenario_generation::build_scenarios()
            .into_iter()
            .next()
            .expect("build_scenarios yields at least one scenario");
        let id = base.id.clone();
        base.name = "base".to_string();
        let mut modded = base.clone();
        modded.name = "modded".to_string();

        merge_content_item(
            &Content::Scenario(base),
            &mut sections,
            &mut scenarios,
            &mut campaigns,
        );
        merge_content_item(
            &Content::Scenario(modded),
            &mut sections,
            &mut scenarios,
            &mut campaigns,
        );

        assert_eq!(scenarios.len(), 1, "overlay must replace, not add");
        assert_eq!(
            scenarios.get(&id).unwrap().name,
            "modded",
            "later scenario must win"
        );
    }

    /// End to end over the REAL generated base content: merge the built-in
    /// scenarios and the built-in campaign, then resolve the "nova_protocol"
    /// campaign's membership. It must list its five chapters in play order -
    /// including the two `hidden` chained members (broadside_gunship, the
    /// phase-two wave; final_tally, the epilogue) - and every member must resolve
    /// to a merged scenario, hidden ones included. This is the "real mapping, not
    /// display-name parsing" contract: the order comes from the campaign's own
    /// list, and a hidden member is reachable despite being filtered from the
    /// flat picker.
    #[test]
    fn merged_campaign_resolves_members_in_order_including_hidden() {
        let mut items: Vec<Content> = Vec::new();
        for (_, content) in scenario_generation::build_scenario_contents() {
            items.extend(content);
        }
        for (_, content) in scenario_generation::build_campaign_contents() {
            items.extend(content);
        }

        let outcome = merge_bundles([items.iter()]);
        assert!(
            outcome.conflicts.is_empty(),
            "base content merges clean: {:?}",
            outcome.conflicts
        );

        let campaign = outcome
            .campaigns
            .get("nova_protocol")
            .expect("the base Nova Protocol campaign registers");
        assert_eq!(
            campaign.scenarios,
            vec![
                "shakedown_run",
                "broadside",
                "broadside_gunship",
                "lifeline",
                "final_tally",
            ],
            "members resolve in the campaign's declared play order"
        );

        // Every member resolves to a merged scenario - the mapping never lists a
        // ghost - and the two chained chapters are genuinely `hidden` (so they
        // are reachable ONLY via this mapping, not the flat picker).
        for member in &campaign.scenarios {
            assert!(
                outcome.scenarios.contains_key(member),
                "campaign member '{member}' resolves to a real scenario"
            );
        }
        for hidden_member in ["broadside_gunship", "final_tally"] {
            assert!(
                outcome.scenarios[hidden_member].hidden,
                "'{hidden_member}' is hidden from the flat picker yet listed for replay"
            );
        }
    }

    /// A later bundle (a mod) overlays an earlier bundle (the base) by id:
    /// last-wins across bundles, with a fresh section left added. No conflicts -
    /// same id in DIFFERENT bundles is the intended overlay, not an error.
    #[test]
    fn merge_bundles_overlays_later_bundle_by_id() {
        let base = [
            Content::Section(Box::new(section("hull", 100.0))),
            Content::Section(Box::new(section("thruster", 50.0))),
        ];
        let modded = [
            // Overrides the base hull by id.
            Content::Section(Box::new(section("hull", 999.0))),
            // Adds a brand-new section.
            Content::Section(Box::new(section("shield", 25.0))),
        ];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert!(
            outcome.conflicts.is_empty(),
            "same id across bundles is overlay, not a conflict: {:?}",
            outcome.conflicts
        );
        // hull overlaid in place (order preserved), thruster kept, shield appended.
        assert_eq!(
            outcome
                .sections
                .iter()
                .map(|s| s.base.id.as_str())
                .collect::<Vec<_>>(),
            vec!["hull", "thruster", "shield"]
        );
        assert_eq!(
            outcome.sections[0].base.health, 999.0,
            "the mod's hull must win over the base's"
        );
    }

    /// Dependency order drives the merge: a DEPENDENT mod overlays its
    /// DEPENDENCY, so the topological order (dependency before dependent) must
    /// merge the dependent LAST even when it comes FIRST in catalog order. This
    /// is `register_bundles`'s ordering step (`topological_order` +
    /// `merge_bundles`) in miniature; without the topo reorder the merge would
    /// keep catalog order and the dependency would wrongly win.
    #[test]
    fn dependency_order_merges_a_dependent_after_its_dependency() {
        use nova_mod_format::deps::{topological_order, DepGraph};

        // `dependent` (id "mod") overrides the `hull` section that `dependency`
        // (id "dep") defines. Catalog/input order lists the dependent FIRST.
        let dependency = vec![Content::Section(Box::new(section("hull", 100.0)))];
        let dependent = vec![Content::Section(Box::new(section("hull", 999.0)))];
        let bundles: HashMap<&str, &Vec<Content>> =
            HashMap::from([("dep", &dependency), ("mod", &dependent)]);
        let ids = vec!["mod".to_string(), "dep".to_string()];
        let graph: DepGraph = HashMap::from([("mod".to_string(), vec!["dep".to_string()])]);

        let topo = topological_order(&ids, &graph);
        assert!(!topo.cycle);
        assert_eq!(topo.order, vec!["dep".to_string(), "mod".to_string()]);

        let outcome = merge_bundles(topo.order.iter().map(|id| bundles[id.as_str()].iter()));
        assert_eq!(outcome.sections.len(), 1);
        assert_eq!(
            outcome.sections[0].base.health, 999.0,
            "the dependent overlays its dependency regardless of catalog order"
        );
    }

    /// The SAME id twice within ONE bundle is a conflict: the first is kept, the
    /// duplicate is skipped and recorded. This is the "intra-bundle duplicate is
    /// an error" rule (surfaced loudly by the caller), distinct from cross-bundle
    /// overlay.
    #[test]
    fn merge_bundles_intra_bundle_duplicate_is_a_conflict() {
        let bundle = [
            Content::Section(Box::new(section("hull", 100.0))),
            // Duplicate id in the SAME bundle - a conflict, not an overlay.
            Content::Section(Box::new(section("hull", 999.0))),
        ];

        let outcome = merge_bundles([bundle.iter()]);

        assert_eq!(outcome.sections.len(), 1, "the duplicate must be skipped");
        assert_eq!(
            outcome.sections[0].base.health, 100.0,
            "the FIRST occurrence is kept within a bundle"
        );
        assert_eq!(outcome.conflicts.len(), 1, "the conflict is recorded");
        assert!(
            outcome.conflicts[0].contains("hull"),
            "the conflict names the offending id: {}",
            outcome.conflicts[0]
        );
    }
}
