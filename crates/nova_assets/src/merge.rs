//! The content MERGE: flatten every enabled bundle's `Content` in dependency
//! order and overlay it by id into the game's registries (`GameSections`,
//! `GameShipDesigns`, `GameScenarios`, `GameCampaigns`, `GameStyles`), linting
//! the result as it goes.

/// Glob-import surface: `use nova_assets::merge::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{merge_bundles, register_bundles, MergeOutcome};
}

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use nova_modding::prelude::{BundleAsset, Content, ContentAsset, InstalledCatalog, BASE_MOD_ID};
use nova_scenario::prelude::{
    GameCampaigns, GameScenarios, GameShipDesigns, NewGameStart, ScenarioRole, ShipDesignPrototype,
};
use nova_ship::prelude::*;
use nova_training::prelude::{Lesson, LessonSeverity, TrainingCatalog};
use nova_ui::theme::{GameUiThemes, UiThemeConfig};

use crate::{
    collections::GameAssets,
    mod_refs,
    mod_set::{enabled_bundles, shadows_shipped, DownloadedMods, EnabledMods},
    safe_mode::{catalog_bundle, OptionalBundles},
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
/// The catalog is part of the `GameAssets` collection, but it visits only its
/// MANDATORY bundle (the base game's) as a dependency, so what the collection
/// gates on is the base content alone. The OPTIONAL cataloged bundles load
/// beside it (`crate::safe_mode`) and are settled before this first runs in
/// `Processing`; a bundle that is still in flight, or that safe mode quarantined,
/// is warned about and skipped (never a panic). Re-runs whenever the installed
/// set changes, so a menu toggle applies live and a late bundle merges when it
/// lands.
///
/// ENABLED DOWNLOADED bundles ([`DownloadedMods`]) merge AFTER the shipped ones,
/// in cache-index order, through the same overlay rules. They sit outside the
/// collection gate (loaded async via `mods://`), so a still-loading bundle is
/// skipped with a warning;
/// [`mark_installed_bundles_loaded`](crate::mark_installed_bundles_loaded)
/// re-triggers this system when the load lands, and a `DownloadedMods` change
/// (install/uninstall) re-triggers it too.
pub fn register_bundles(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    enabled: Res<EnabledMods>,
    downloaded: Res<DownloadedMods>,
    optional: Res<OptionalBundles>,
    catalogs: Res<Assets<InstalledCatalog>>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
) {
    let catalog = catalogs.get(&game_assets.catalog);
    if catalog.is_none() {
        error!("register_bundles: the mods catalog was not loaded; registering nothing");
    }
    // The merge SAYS what the walk below drops: the cache index is downloaded
    // input, and a mod that vanishes without a word reaches the player as
    // missing content. `build_mod_catalog` hides the same records from the rows.
    for m in &downloaded.0 {
        if enabled.0.contains(&m.record.id)
            && catalog.is_some_and(|shipped| shadows_shipped(shipped, &m.record.id))
        {
            warn!(
                "register_bundles: downloaded mod '{}' shadows a shipped mod id; \
                 skipping the downloaded copy",
                m.record.id
            );
        }
    }
    // Enabled (id, bundle) pairs in merge order - catalog order (base first)
    // then downloaded order, the stable tiebreak the dependency sort keeps
    // below. `mod_set::enabled_bundles` owns that walk, so the merge, the mods
    // rows and the editor's asset index cannot disagree about what is active.
    let mut ordered: Vec<(&str, &Handle<BundleAsset>)> = Vec::new();
    for active in enabled_bundles(catalog, Some(&optional), Some(&downloaded), Some(&enabled)) {
        // An OPTIONAL entry reads through its runtime load, which - like a
        // downloaded bundle - may still be in flight or may have failed and
        // been quarantined. Both are handled below by the same loaded-or-skip
        // rule; only `base` is guaranteed here.
        let Some(handle) = active.bundle else {
            warn!(
                "register_bundles: mod '{}' is enabled but its bundle has not started \
                 loading; it merges when the load completes",
                active.id
            );
            continue;
        };
        // Unlike the shipped entries (gated loaded by the collection), a
        // downloaded bundle may still be in flight; skipping it here is a
        // TRANSIENT state, not the shared "somehow not loaded" error below -
        // the loaded-event re-run merges it in.
        if active.downloaded && !bundles.contains(handle) {
            warn!(
                "register_bundles: downloaded mod '{}' is enabled but its bundle has not \
                 loaded yet; it merges when the load completes",
                active.id
            );
            continue;
        }
        ordered.push((active.id, handle));
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
    // So a bundle or content file that fails to load can name the mod it
    // belongs to.
    let bundle_handles: Vec<(&str, &Handle<BundleAsset>)> = topo
        .order
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).map(|handle| (id.as_str(), *handle)))
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
    for (mod_id, bundle_handle) in bundle_handles {
        let Some(bundle) = bundles.get(bundle_handle) else {
            // WARN, not ERROR: an optional bundle that has not settled yet is a
            // transient state. The change-driven re-merge registers it as soon
            // as it lands, and safe mode quarantines it if it never does.
            warn!(
                "register_bundles: mod '{mod_id}' has no loaded bundle asset; skipping it \
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
        if let Some(base_bundle) = by_id.get(BASE_MOD_ID).and_then(|h| bundles.get(*h)) {
            dep_refs.insert(
                BASE_MOD_ID.to_string(),
                mod_refs::DepRef {
                    base: Some(base_bundle.resource_base.as_str()),
                    resources: Some(base_bundle.resources.as_slice()),
                },
            );
        }
        for dep_id in &bundle.meta.dependencies {
            if dep_id == BASE_MOD_ID {
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
                let path = content_handle
                    .path()
                    .map_or_else(|| "<unknown>".to_string(), ToString::to_string);
                error!(
                    "register_bundles: mod '{mod_id}' content '{path}' did not load (the \
                     loader error above says why); skipping it (the other content still \
                     registers). A downloaded mod that no longer parses was built for an \
                     older game version: update it in Mods > Explore."
                );
                continue;
            };
            for item in &content.0 {
                for message in mod_refs::resource_ref_violations(item, &scope) {
                    error!("register_bundles: content {message}");
                    // Every kind, not just scenarios: a section or campaign
                    // with a bad ref was logged and merged anyway, so the
                    // runtime gate never saw it.
                    undeclared_ref_issues.push((item.id().to_string(), message));
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
            let Some(bundle) = catalog_bundle(entry, &optional).and_then(|h| bundles.get(h)) else {
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
    let merged_ships = nova_scenario::prelude::KnownShipDesigns::from_configs(outcome.ships.iter());
    let merged_scenarios: std::collections::HashSet<String> =
        outcome.scenarios.keys().cloned().collect();
    // What each merged scenario declares itself to be, so campaign lint can
    // refuse a member that is scenery or a lesson's range rather than a
    // launchable chapter - and so lesson lint can check the other direction.
    let merged_roles: HashMap<String, ScenarioRole> = outcome
        .scenarios
        .values()
        .map(|scenario| (scenario.id.clone(), scenario.role))
        .collect();
    let merged_practice_ranges: std::collections::HashSet<String> = merged_roles
        .iter()
        .filter(|(_, role)| role.is_lesson())
        .map(|(id, _)| id.clone())
        .collect();
    // Everything a player can be sent into, which is what may PROVE a lesson:
    // a chapter or a range, never scenery.
    let merged_launchable: std::collections::HashSet<String> = merged_roles
        .iter()
        .filter(|(_, role)| !role.is_backdrop())
        .map(|(id, _)| id.clone())
        .collect();
    let mut content_issues = nova_scenario::prelude::ContentIssues::default();
    // Every MERGED ship, checked where it is authored: a scenario referencing
    // one only checks that the id resolves, so this is the pass that sees the
    // hull's own geometry. Findings key on the ship id in the shared channel.
    for ship in &outcome.ships {
        let found = nova_scenario::prelude::lint_ship_design_config(
            ship,
            &merged_sections,
            ship.id.as_str(),
        );
        for issue in &found {
            warn!(
                "register_bundles: content lint [{:?}] ship '{}': {}",
                issue.severity, issue.scenario, issue.message
            );
        }
        if !found.is_empty() {
            content_issues
                .0
                .entry(ship.id.to_string())
                .or_default()
                .extend(found);
        }
    }
    for scenario in outcome.scenarios.values() {
        let found = nova_scenario::prelude::lint_scenario(
            scenario,
            &merged_sections,
            &merged_ships,
            &merged_scenarios,
        );
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
        let found =
            nova_scenario::prelude::lint_campaign(campaign, &merged_scenarios, &merged_roles);
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
    // Lesson references: a Practice button may only hand off to a scenario
    // that declares itself a practice range, and this is the only place that is
    // decidable across mods - a mod's lesson may practise in a base range, and
    // `content lint` never sees an installed mod. Findings are keyed by the
    // LESSON id in the same channel, so the handbook can refuse to offer one.
    let lesson_findings = nova_training::prelude::lint_lessons(
        outcome.lessons.iter(),
        &merged_scenarios,
        &merged_practice_ranges,
        &merged_launchable,
    )
    .into_iter()
    .chain(nova_training::prelude::unused_practice_ranges(
        outcome.lessons.iter(),
        &merged_practice_ranges,
    ));
    for issue in lesson_findings {
        warn!(
            "register_bundles: content lint [{:?}] lesson '{}': {}",
            issue.severity, issue.lesson, issue.message
        );
        content_issues
            .0
            .entry(issue.lesson.clone())
            .or_default()
            .push(nova_scenario::prelude::LintIssue {
                severity: match issue.severity {
                    LessonSeverity::Error => nova_scenario::prelude::LintSeverity::Error,
                    LessonSeverity::Warn => nova_scenario::prelude::LintSeverity::Warn,
                },
                scenario: issue.lesson,
                message: issue.message,
            });
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
    publish_scenarios(&mut commands, outcome.scenarios);
    commands.insert_resource(outcome.campaigns);
    commands.insert_resource(GameStyles(outcome.styles));
    commands.insert_resource(GameShipDesigns(outcome.ships));
    commands.insert_resource(TrainingCatalog::new(outcome.lessons));
    commands.insert_resource(GameUiThemes(outcome.ui_themes));
}

/// The scenario ids the last merge published, so the next one knows which
/// entries of [`GameScenarios`] are ITS to replace.
#[derive(Resource, Default)]
pub(crate) struct ContentScenarioIds(HashSet<String>);

/// Replace what CONTENT declares in [`GameScenarios`] and keep the rest.
///
/// Scenarios are the one merged registry code also writes into: the editor's
/// sandbox and a probe fixture have no content file behind them, and a re-merge
/// that rebuilt the whole resource deleted them - which a live re-merge does
/// whenever an enabled mod's bundle lands after the first pass. What a previous
/// merge published is dropped (a removed mod's chapters really do go), and
/// everything else survives.
fn publish_scenarios(commands: &mut Commands, published: GameScenarios) {
    commands.queue(move |world: &mut World| {
        let mine = world
            .remove_resource::<ContentScenarioIds>()
            .unwrap_or_default();
        let mut registry = world.remove_resource::<GameScenarios>().unwrap_or_default();
        registry.0.retain(|id, _| !mine.0.contains(id));
        world.insert_resource(ContentScenarioIds(published.0.keys().cloned().collect()));
        registry.0.extend(published.0);
        world.insert_resource(registry);
    });
}

/// The result of merging an ordered list of bundles: the id-keyed registries plus
/// any intra-bundle id conflicts that were detected (and skipped).
#[derive(Default)]
pub struct MergeOutcome {
    /// Sections in registration order (base then mods), overlaid last-wins by id.
    pub sections: Vec<SectionConfig>,
    /// Scenarios keyed by id, overlaid last-wins.
    pub scenarios: GameScenarios,
    /// Campaigns keyed by id, overlaid last-wins (a later bundle may replace an
    /// earlier campaign's membership by declaring the same id).
    pub campaigns: GameCampaigns,
    /// Skin styles in registration order, overlaid last-wins by id - so a mod
    /// restyles a base look by declaring the same id.
    pub styles: Vec<ShipStyleConfig>,
    /// Ships in registration order, overlaid last-wins by id - so a mod
    /// rebuilds a base hull by declaring the same id.
    pub ships: Vec<ShipDesignPrototype>,
    /// Handbook lessons in registration order, overlaid last-wins by id - so a
    /// mod re-teaches a base lesson by declaring its id, and adds one to a
    /// category by declaring a new one. The registration order is NOT the draw
    /// order: `TrainingCatalog::new` sorts by category, authored order and id,
    /// so which mod arrived first never decides how the handbook reads.
    pub lessons: Vec<Lesson>,
    /// UI themes in registration order, overlaid last-wins by id - so a mod
    /// restyles a shipped look by declaring its id and adds one by declaring a
    /// new id. The order is what the Settings picker lists, so the base mod's
    /// default stays first.
    pub ui_themes: Vec<UiThemeConfig>,
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
    let mut outcome = MergeOutcome::default();

    for bundle in bundles {
        // Ids seen in THIS bundle, per kind - reset each bundle so a later
        // bundle may overlay an earlier one, while a repeat within one bundle
        // conflicts. Keyed by `Content::kind` rather than one set per kind: the
        // seven near-identical blocks this replaces all did the same thing, and
        // an eighth content kind meant writing the block again.
        let mut seen: HashMap<&'static str, HashSet<&str>> = HashMap::new();

        for item in bundle {
            if !seen.entry(item.kind()).or_default().insert(item.id()) {
                outcome.conflicts.push(format!(
                    "{} id '{}' appears more than once in one bundle; \
                     keeping the first, skipping the duplicate",
                    item.kind(),
                    item.id()
                ));
                continue;
            }
            merge_content_item(item, &mut outcome);
        }
    }

    outcome
}

/// Route one content item into the accumulating registries with last-wins
/// overlay by id. All kinds overlay identically: a later item (from a later
/// bundle) with the same id replaces the earlier one rather than appending a
/// shadowed duplicate. Sections keep a Vec (order matters for the editor palette)
/// so overlay is a linear replace-in-place; scenarios and campaigns are maps so
/// overlay is a plain `insert`. Called by [`merge_bundles`] once per accepted
/// item.
fn merge_content_item(item: &Content, into: &mut MergeOutcome) {
    match item {
        Content::Section(cfg) => {
            match into.sections.iter_mut().find(|s| s.base.id == cfg.base.id) {
                Some(existing) => *existing = cfg.as_ref().clone(),
                None => into.sections.push(cfg.as_ref().clone()),
            }
        }
        Content::Scenario(cfg) => {
            into.scenarios.insert(cfg.id.clone(), cfg.clone());
        }
        Content::Campaign(cfg) => {
            into.campaigns.insert(cfg.id.clone(), cfg.clone());
        }
        // A Vec like the sections, for the same reason: a style catalog has an
        // order, and overlaying in place keeps a mod's restyle where the base
        // one stood.
        Content::Style(cfg) => match into.styles.iter_mut().find(|s| s.id == cfg.id) {
            Some(existing) => *existing = cfg.clone(),
            None => into.styles.push(cfg.clone()),
        },
        // A Vec for the same reason again: the ship catalog has an order a
        // picker reads, and overlaying in place keeps a mod's rebuild where the
        // base hull stood.
        Content::Ship(cfg) => match into.ships.iter_mut().find(|s| s.id == cfg.id) {
            Some(existing) => *existing = cfg.clone(),
            None => into.ships.push(cfg.clone()),
        },
        // A Vec, overlaid in place, like the rest - though the handbook's own
        // order comes from the lessons' `category`/`order` rather than from
        // this Vec, so an overlay here is about REPLACING a lesson, never
        // about where it draws.
        Content::Lesson(lesson) => match into.lessons.iter_mut().find(|l| l.id == lesson.id) {
            Some(existing) => *existing = lesson.clone(),
            None => into.lessons.push(lesson.clone()),
        },
        // A Vec, overlaid in place, because this order IS the Settings picker's
        // order: a mod that restyles `base/phosphor` must leave the default
        // where the player expects to find it.
        Content::UiTheme(cfg) => match into.ui_themes.iter_mut().find(|t| t.id == cfg.id) {
            Some(existing) => *existing = cfg.as_ref().clone(),
            None => into.ui_themes.push(cfg.as_ref().clone()),
        },
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::CommandQueue;
    use nova_gameplay::prelude::AssetRef;
    use nova_scenario::prelude::ScenarioConfig;
    use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, SectionKind};

    use super::*;

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
        // Base bundle: two sections in palette order. Mod bundle: overlays
        // "hull" with a new health, leaves "thruster" alone.
        let base = [
            Content::Section(Box::new(section("hull", 100.0))),
            Content::Section(Box::new(section("thruster", 50.0))),
        ];
        let modded = [Content::Section(Box::new(section("hull", 999.0)))];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        // No duplicate appended: still two sections, original order kept.
        let sections = outcome.sections;
        assert_eq!(sections.len(), 2, "overlay must replace, not append");
        assert_eq!(sections[0].base.id, "hull", "palette order preserved");
        assert_eq!(sections[1].base.id, "thruster");
        // Last-wins: the overlaid value took effect.
        assert_eq!(sections[0].base.health, 999.0, "later section must win");
    }

    /// A live re-merge replaces what CONTENT publishes and leaves everything
    /// else alone.
    ///
    /// Scenarios are the one merged registry code also writes into - the
    /// editor's sandbox, a probe fixture - and a re-merge runs whenever an
    /// enabled mod's bundle lands after the first pass. Rebuilding the whole
    /// resource deleted those entries, and the scenario they named stopped
    /// existing mid-run. A chapter a previous merge published still goes when
    /// the mod behind it does.
    #[test]
    fn a_re_merge_keeps_the_scenarios_no_bundle_published() {
        let scenario = |id: &str, name: &str| {
            ScenarioConfig::new(
                id.to_string(),
                name,
                AssetRef::from("dep://base/textures/cubemap.png".to_string()),
            )
        };
        let publish = |world: &mut World, ids: &[&str]| {
            let mut queue = CommandQueue::default();
            let mut scenarios = GameScenarios::default();
            for id in ids {
                scenarios.insert(id.to_string(), scenario(id, "content"));
            }
            publish_scenarios(&mut Commands::new(&mut queue, world), scenarios);
            queue.apply(world);
        };

        let mut world = World::new();
        publish(&mut world, &["chapter_one", "from_a_mod"]);

        // Code registers a scenario of its own, beside the merged ones.
        world
            .resource_mut::<GameScenarios>()
            .insert("sandbox".to_string(), scenario("sandbox", "sandbox"));

        // The mod is switched off and its bundle drops out of the re-merge.
        publish(&mut world, &["chapter_one"]);

        let live = world.resource::<GameScenarios>();
        assert!(
            live.contains_key("sandbox"),
            "a scenario no bundle published survives the re-merge: {:?}",
            live.keys().collect::<Vec<_>>(),
        );
        assert!(live.contains_key("chapter_one"), "content is republished");
        assert!(
            !live.contains_key("from_a_mod"),
            "a chapter the last merge published goes when its bundle does",
        );
    }

    /// A later scenario with the same id overlays the earlier one, same as
    /// sections - the two kinds must behave identically under overlay.
    #[test]
    fn later_scenario_overlays_earlier_by_id() {
        let id = "shakedown_run".to_string();
        let base = ScenarioConfig::new(
            id.clone(),
            "base",
            AssetRef::from("dep://base/textures/cubemap.png".to_string()),
        );
        let mut modded = base.clone();
        modded.name = "modded".to_string();
        let base = [Content::Scenario(base)];
        let modded = [Content::Scenario(modded)];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert_eq!(outcome.scenarios.len(), 1, "overlay must replace, not add");
        assert_eq!(
            outcome.scenarios.get(&id).unwrap().name,
            "modded",
            "later scenario must win"
        );
    }

    /// A mod adds a lesson to a base category and replaces a base lesson by
    /// declaring its id.
    ///
    /// A lesson id denotes a stable learning outcome, because persisted
    /// progress is keyed by it: a replacement may change every word and the
    /// picture, but the player who already read that lesson has still read it.
    /// So the overlay is by id, in place, like every other content kind.
    #[test]
    fn a_mod_overlays_a_base_lesson_by_id_and_adds_its_own() {
        let lesson = |id: &str, category, order, title: &str| {
            Content::Lesson(nova_training::prelude::Lesson {
                id: id.to_string(),
                category,
                order,
                title: title.to_string(),
                media: nova_training::prelude::LessonMedia::Image {
                    image: AssetRef::from("self://training/x.png".to_string()),
                    alt: "a still".to_string(),
                },
                body: "A short body.".to_string(),
                actions: vec![],
                wiki_path: "wiki/flight".to_string(),
                practice: None,
                proven_by: vec![],
                field_notes: vec![],
            })
        };
        use nova_training::prelude::LessonCategory::{Advanced, Flight};
        let base = [lesson("flight_stop", Flight, 30, "STOP is an order")];
        let modded = [
            lesson("flight_stop", Flight, 30, "STOP, in this mod"),
            lesson("mod_ritual", Advanced, 10, "The ritual"),
        ];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert!(outcome.conflicts.is_empty(), "{:?}", outcome.conflicts);
        assert_eq!(
            outcome
                .lessons
                .iter()
                .map(|lesson| (lesson.id.as_str(), lesson.title.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("flight_stop", "STOP, in this mod"),
                ("mod_ritual", "The ritual"),
            ],
            "the mod's lesson must win in place, and its new one must be added",
        );
    }

    /// A mod restyles a base look by declaring a style with the same id, and
    /// adds a look of its own by declaring a new one.
    ///
    /// The overlay is what makes a style CONTENT rather than a constant: the
    /// base ships one, a mod replaces it or adds beside it, and every ship that
    /// names it by id wears whatever won.
    #[test]
    fn a_mod_overlays_a_base_style_by_id_and_adds_its_own() {
        let style = |id: &str, name: &str| {
            Content::Style(ShipStyleConfig {
                id: id.to_string(),
                name: name.to_string(),
                ..Default::default()
            })
        };
        let base = [style("industrial", "Industrial")];
        let modded = [style("industrial", "Rusted"), style("raider", "Raider")];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert!(outcome.conflicts.is_empty(), "{:?}", outcome.conflicts);
        assert_eq!(
            outcome
                .styles
                .iter()
                .map(|style| (style.id.as_str(), style.name.as_str()))
                .collect::<Vec<_>>(),
            vec![("industrial", "Rusted"), ("raider", "Raider")],
            "the mod's style must win in place, and its new one must be added",
        );
    }

    /// A mod rebuilds a base hull by declaring a ship with the same id, and
    /// adds a hull of its own by declaring a new one.
    ///
    /// The overlay is what makes a ship CONTENT rather than a copy: every
    /// scenario that names the corvette by id flies whatever won.
    #[test]
    fn a_mod_overlays_a_base_ship_by_id_and_adds_its_own() {
        let ship = |id: &str, name: &str| {
            Content::Ship(nova_scenario::prelude::ShipDesignPrototype {
                id: id.into(),
                name: name.to_string(),
                design: nova_scenario::prelude::ShipDesign::default(),
            })
        };
        let base = [ship("block_gunship", "Gunship")];
        let modded = [
            ship("block_gunship", "Rusted Gunship"),
            ship("raider", "Raider"),
        ];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert!(outcome.conflicts.is_empty(), "{:?}", outcome.conflicts);
        assert_eq!(
            outcome
                .ships
                .iter()
                .map(|ship| (ship.id.as_str(), ship.name.as_str()))
                .collect::<Vec<_>>(),
            vec![("block_gunship", "Rusted Gunship"), ("raider", "Raider")],
            "the mod's hull must win in place, and its new one must be added",
        );
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
