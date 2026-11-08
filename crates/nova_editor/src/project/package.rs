//! Package a mod, and import one.
//!
//! A package is the mod's own directory as one zip: the layout the portal
//! generator walks, so the extracted archive is a portal entry and an installable
//! mod at once. [`package_files`] is the one crossing out and [`adopt_package`]
//! the one crossing back, and both go through the id rule in [`super::ids`].
//!
//! The check is the merge's own walk over the content the project can see, so a
//! package that passes is one the gate accepts. Errors block, and go to the
//! Problems tab; warnings do not.
#![cfg_attr(
    target_arch = "wasm32",
    expect(dead_code, reason = "the web has no store, so nothing packages")
)]

use std::collections::{HashMap, HashSet};

use bevy::{ecs::system::RunSystemOnce, prelude::*, ui_widgets::Activate};
use nova_assets::{
    items::{item_pack_faults, ItemPack},
    merge_bundles,
    mod_cache::prelude::is_safe_id,
    mod_refs::prelude::{bare_asset_refs, resource_ref_violations, DEP_SCHEME, SELF_SCHEME},
};
use nova_gameplay::prelude::{
    price_table_faults, trade_profile_faults, GameItems, GameTradeProfiles,
};
use nova_modding::prelude::{Content, BASE_MOD_ID};
use nova_scenario::prelude::*;
use nova_ship::prelude::{GameStyles, SectionConfig};
use nova_training::prelude::{lint_lessons, LessonSeverity};
use nova_ui::theme::lint_theme;
use nova_world_base::prelude::{ShipPartPack, ShipPartSnapshot};

use super::{
    ids::{publish_id, EDITOR_ID_PREFIX},
    ProjectData,
};
use crate::{
    asset::{mesh_fault, mesh_of, section_of},
    asset_index::{AssetIndex, VisiblePack},
    problems::{Problem, ProblemSeverityType},
};

/// The folder a bare path into the base game starts with.
const BASE_FOLDER: &str = "base/";

/// Which part of a version a bump steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VersionBumpType {
    /// The last number: a fix.
    Patch,
    /// The middle number: something added.
    Minor,
    /// The first number: something that breaks.
    Major,
}

impl VersionBumpType {
    /// Every bump, in the order the Package window offers them.
    pub(crate) const ALL: [Self; 3] = [Self::Patch, Self::Minor, Self::Major];

    /// The button's word.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Patch => "Patch",
            Self::Minor => "Minor",
            Self::Major => "Major",
        }
    }
}

/// `version` one `bump` on, as `major.minor.patch`: the stepped number goes up
/// and the ones after it go to 0. A missing part reads as 0, so `1` bumps to
/// `1.0.1`. `None` when a part is not a number: the version rule allows
/// letters, and a bump does not guess what `beta` steps to.
pub(crate) fn bumped(version: &str, bump: VersionBumpType) -> Option<String> {
    let parts: Vec<&str> = version.trim().split('.').collect();
    if parts.len() > 3 {
        return None;
    }
    let mut numbers = [0u64; 3];
    for (slot, part) in numbers.iter_mut().zip(&parts) {
        if part.is_empty() || !part.chars().all(|digit| digit.is_ascii_digit()) {
            return None;
        }
        *slot = part.parse().ok()?;
    }
    let [major, minor, patch] = numbers;
    let [major, minor, patch] = match bump {
        VersionBumpType::Patch => [major, minor, patch.checked_add(1)?],
        VersionBumpType::Minor => [major, minor.checked_add(1)?, 0],
        VersionBumpType::Major => [major.checked_add(1)?, 0, 0],
    };
    Some(format!("{major}.{minor}.{patch}"))
}

/// The archive's file name for mod `id` at `version`: the published id and the
/// version, so two versions sit side by side in one folder.
pub(crate) fn package_name(id: &str, version: &str) -> String {
    format!("{}-{}.zip", publish_id(id), version.trim())
}

/// A size as a reader says it: bytes, then KB and MB at one decimal.
pub(crate) fn shown_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", bytes as f64 / KB),
        _ => format!("{:.1} MB", bytes as f64 / (KB * KB)),
    }
}

/// A finding that names no node.
fn finding(severity: ProblemSeverityType, message: impl Into<String>) -> Problem {
    Problem {
        severity,
        message: message.into(),
        node: None,
    }
}

/// An error.
fn error(message: impl Into<String>) -> Problem {
    finding(ProblemSeverityType::Error, message)
}

/// A warning.
fn warning(message: impl Into<String>) -> Problem {
    finding(ProblemSeverityType::Warning, message)
}

/// What the merge's lints say about `data`, against what `packs` hold.
///
/// Only the project's own items are judged. The merge is what makes that
/// meaningful: an id resolves here exactly when base and the declared mods
/// carry it, so a part from a mod the project does not declare is a finding.
pub(crate) fn lint_content(own: &str, data: &ProjectData, packs: &[VisiblePack]) -> Vec<Problem> {
    lint_judged(own, data, packs, |_| true)
}

/// [`lint_content`], judging only the items `judged` keeps. The merge still sees
/// them all, so what a kept item names still resolves.
fn lint_judged(
    own: &str,
    data: &ProjectData,
    packs: &[VisiblePack],
    judged: impl Fn(&Content) -> bool,
) -> Vec<Problem> {
    let own_items: Vec<&Content> = data.items().collect();
    let lists: Vec<Vec<&Content>> = packs
        .iter()
        .map(|pack| pack.items.clone())
        .chain(std::iter::once(own_items.clone()))
        .collect();
    let outcome = merge_bundles(lists.iter().map(|items| items.iter().copied()));
    let mut found: Vec<Problem> = outcome.conflicts.iter().map(error).collect();

    let sections = KnownSections::from_configs(outcome.sections.iter());
    let ships = KnownShipDesigns::from_configs(outcome.ships.iter());
    let stations = KnownStationDesigns::from_configs(outcome.stations.iter());
    let scenario_ids: HashSet<String> = outcome.scenarios.keys().cloned().collect();
    let by_id: HashMap<String, &ScenarioConfig> = outcome
        .scenarios
        .values()
        .map(|scenario| (scenario.id.clone(), scenario))
        .collect();
    let practice: HashSet<String> = by_id
        .iter()
        .filter(|(_, scenario)| scenario.role.is_lesson())
        .map(|(id, _)| id.clone())
        .collect();
    let launchable: HashSet<String> = by_id
        .iter()
        .filter(|(_, scenario)| !scenario.role.is_backdrop())
        .map(|(id, _)| id.clone())
        .collect();
    let items = GameItems::new(outcome.items.iter().cloned());
    let styles = GameStyles(outcome.styles.clone());
    let profiles = GameTradeProfiles(
        outcome
            .trade_profiles
            .iter()
            .map(|profile| (profile.id.clone(), profile.clone()))
            .collect(),
    );

    let mut issues: Vec<LintIssue> = Vec::new();
    let mut tables = 0;
    for item in own_items.iter().filter(|item| judged(item)) {
        match item {
            Content::Section(config) => {
                issues.extend(lint_section_config(config, own));
                issues.extend(lint_section_recipe(config, &items, own));
            }
            Content::Scenario(scenario) => issues.extend(lint_scenario(
                scenario,
                &sections,
                &ships,
                &stations,
                &scenario_ids,
                &items,
                &styles,
                &profiles,
            )),
            Content::Campaign(campaign) => issues.extend(lint_campaign(campaign, &by_id)),
            Content::Style(style) => {
                if let Err(faults) = style.compile_greebles() {
                    found.extend(
                        faults
                            .iter()
                            .map(|fault| error(format!("style {}: {fault}", style.id))),
                    );
                }
            }
            Content::Ship(ship) => issues.extend(lint_ship_design_config(
                ship,
                &sections,
                &items,
                &styles,
                ship.id.as_str(),
            )),
            Content::Station(station) => issues.extend(lint_station_design(
                station.id.as_str(),
                &station.design,
                &sections,
                &items,
                &styles,
                station.id.as_str(),
            )),
            Content::StartRole(role) => issues.extend(lint_start_role(
                role,
                &ships,
                &sections,
                &items,
                role.id.as_str(),
            )),
            Content::TradeProfile(profile) => found.extend(
                trade_profile_faults(profile, &items)
                    .into_iter()
                    .map(|fault| error(format!("trade profile {}: {fault}", profile.id))),
            ),
            Content::PriceTable(table) => {
                tables += 1;
                found.extend(price_table_faults(table).into_iter().map(error));
            }
            Content::UiTheme(theme) => found.extend(
                lint_theme(theme, &outcome.ui_themes)
                    .into_iter()
                    .map(|issue| error(issue.to_string())),
            ),
            Content::Lesson(_) | Content::Item(_) => {}
        }
    }
    if tables > 1 {
        found.push(error("a mod may state one price table"));
    }
    let lessons = own_items
        .iter()
        .filter(|item| judged(item))
        .filter_map(|item| match item {
            Content::Lesson(lesson) => Some(lesson),
            _ => None,
        });
    found.extend(
        lint_lessons(lessons, &scenario_ids, &practice, &launchable)
            .into_iter()
            .map(|issue| match issue.severity {
                LessonSeverity::Error => error(issue.message),
                LessonSeverity::Warn => warning(issue.message),
            }),
    );
    found.extend(issues.into_iter().map(|issue| match issue.severity {
        LintSeverity::Error => error(issue.message),
        LintSeverity::Warn => warning(issue.message),
    }));
    found.extend(item_findings(own, data, packs));
    found
}

/// What the world generator says of `own`'s sections beside `packs`: the faults
/// its catalog has that the catalog without it does not. A generator-only fault
/// reaches the player as a world that does not arm, so it is said here.
fn ship_part_findings(own: &str, data: &ProjectData, packs: &[VisiblePack]) -> Vec<Problem> {
    let sections_of = |items: &[&Content]| -> Vec<SectionConfig> {
        items
            .iter()
            .filter_map(|item| match item {
                Content::Section(config) => Some(config.as_ref().clone()),
                _ => None,
            })
            .collect()
    };
    let with_base = |dependencies: &[String], id: &str| -> Vec<String> {
        let mut held = dependencies.to_vec();
        if id != BASE_MOD_ID && !held.iter().any(|dep| dep == BASE_MOD_ID) {
            held.push(BASE_MOD_ID.to_string());
        }
        held
    };
    let mut parts: Vec<ShipPartPack> = packs
        .iter()
        .map(|pack| ShipPartPack {
            id: pack.id.to_string(),
            dependencies: with_base(pack.dependencies, pack.id),
            sections: sections_of(&pack.items),
        })
        .collect();
    let faults = |parts: &[ShipPartPack]| -> Vec<String> {
        ShipPartSnapshot::build(parts)
            .err()
            .into_iter()
            .flatten()
            .map(|fault| fault.to_string())
            .collect()
    };
    let before = faults(&parts);
    parts.push(ShipPartPack {
        id: own.to_string(),
        dependencies: with_base(&data.meta.dependencies, own),
        sections: sections_of(&data.items().collect::<Vec<_>>()),
    });
    faults(&parts)
        .into_iter()
        .filter(|fault| !before.contains(fault))
        .map(error)
        .collect()
}

/// What the content rules say about the mod's non-scenario items while they are
/// edited: the lints, the generator's parts, and the files its meshes name.
///
/// A scenario, a ship or a station is the document lint's to judge. With no base
/// game in sight there is nothing to judge against, so a rig with none says
/// nothing rather than every part being unknown.
pub(crate) fn asset_findings(own: &str, data: &ProjectData, index: &AssetIndex) -> Vec<Problem> {
    let packs = index.visible_packs(&data.meta.dependencies);
    if !packs.iter().any(|pack| pack.id == BASE_MOD_ID) {
        return vec![];
    }
    let mut found = lint_judged(own, data, &packs, |item| {
        !matches!(
            item,
            Content::Scenario(_) | Content::Ship(_) | Content::Station(_) | Content::Lesson(_)
        )
    });
    found.extend(ship_part_findings(own, data, &packs));
    found.extend(data.items().filter_map(|item| {
        let config = section_of(item)?;
        let mesh = mesh_of(&config.kind);
        let declared = mesh
            .strip_prefix(SELF_SCHEME)
            .and_then(|file| file.split('#').next())
            .is_some_and(|file| data.resources.iter().any(|held| held == file));
        // An undeclared file is the ref check's row.
        declared
            .then(|| mesh_fault(own, data, &mesh))
            .flatten()
            .map(|fault| error(format!("section {}: {fault}", config.base.id)))
    }));
    found
}

/// The errors [`asset_findings`] has for the project as it stands.
pub(crate) fn asset_faults(world: &mut World) -> Vec<String> {
    let Some(project) = world.get_resource::<super::ModProject>() else {
        return vec![];
    };
    let own = project
        .id
        .clone()
        .unwrap_or_else(|| project.namespace.clone());
    let data = std::sync::Arc::clone(&project.data);
    let found = world.run_system_once(move |index: AssetIndex| {
        asset_findings(&own, &data, &index)
            .into_iter()
            .filter(|problem| problem.severity == ProblemSeverityType::Error)
            .map(|problem| problem.message)
            .collect::<Vec<_>>()
    });
    found.unwrap_or_else(|fault| vec![format!("the check did not run: {fault}")])
}

/// What the item rule says about `own` beside `packs`.
fn item_findings(own: &str, data: &ProjectData, packs: &[VisiblePack]) -> Vec<Problem> {
    let designs = |items: &[&'_ Content]| -> Vec<_> {
        items
            .iter()
            .filter_map(|item| match item {
                Content::Item(design) => Some(design.clone()),
                _ => None,
            })
            .collect()
    };
    let held: Vec<(&str, &[String], Vec<_>)> = packs
        .iter()
        .map(|pack| (pack.id, pack.dependencies, designs(&pack.items)))
        .chain(std::iter::once((
            own,
            data.meta.dependencies.as_slice(),
            designs(&data.items().collect::<Vec<_>>()),
        )))
        .collect();
    let packs: Vec<ItemPack> = held
        .iter()
        .map(|(id, dependencies, items)| ItemPack {
            id,
            dependencies,
            items: items.iter().collect(),
        })
        .collect();
    item_pack_faults(&packs)
        .remove(own)
        .into_iter()
        .flatten()
        .map(error)
        .collect()
}

/// Whether `text` is an asset ref into the base game with no scheme, which the
/// editor's own constants write and the portal refuses.
fn is_bare_base_ref(text: &str) -> bool {
    text.starts_with(BASE_FOLDER)
}

/// What the manifest, dependency and ref rules say about `data`.
///
/// `resources_on_disk` answers whether a declared resource exists, and
/// `installed` whether a dependency can be found at all.
pub(crate) fn manifest_findings(
    own: &str,
    data: &ProjectData,
    shipped: impl Fn(&str) -> bool,
    installed: impl Fn(&str) -> bool,
    resource_exists: impl Fn(&str) -> bool,
) -> Vec<Problem> {
    let mut found = Vec::new();
    if data.meta.name.trim().is_empty() {
        found.push(error("the mod has no name"));
    }
    let version = data.meta.version.trim();
    if version.is_empty() || !is_safe_id(version) {
        found.push(error(format!(
            "the version '{version}' cannot be published: use letters, digits and dots"
        )));
    }
    let published = publish_id(own);
    if shipped(&published) {
        found.push(error(format!(
            "the id {published} is a shipped mod's: rename this mod"
        )));
    }
    for dep in &data.meta.dependencies {
        if !installed(dep) && !shipped(dep) {
            found.push(error(format!("dependency {dep} is not installed")));
        } else if dep.starts_with(EDITOR_ID_PREFIX) {
            found.push(warning(format!(
                "dependency {dep} is a local mod: publish it as {} too",
                publish_id(dep)
            )));
        }
    }
    for resource in &data.resources {
        if !resource_exists(resource) {
            found.push(error(format!("resource {resource} is missing")));
        }
    }
    if data.items().next().is_none() {
        found.push(warning("the mod holds no content"));
    }
    for item in data.items() {
        for bare in bare_asset_refs(item) {
            if !is_bare_base_ref(&bare) {
                found.push(error(format!(
                    "{} {}: {bare} names no mod: pick it again",
                    item.kind(),
                    item.id()
                )));
            }
        }
    }
    found
}

/// Every finding about the open project, worst first.
fn project_findings(world: &mut World) -> Vec<Problem> {
    let Some(project) = world.get_resource::<super::ModProject>() else {
        return vec![];
    };
    let Some(own) = project.id.clone() else {
        return vec![];
    };
    let data = std::sync::Arc::clone(&project.data);
    let on_disk: HashSet<String> = super::store::files_of(&own).into_iter().collect();
    let installed: HashSet<String> = super::store::installed_mods()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let lint = {
        let (own, data) = (own.clone(), std::sync::Arc::clone(&data));
        move |index: AssetIndex| {
            let packs = index.visible_packs(&data.meta.dependencies);
            let mut found = lint_content(&own, &data, &packs);
            found.extend(manifest_findings(
                &own,
                &data,
                |id| id == BASE_MOD_ID || index.is_shipped(id),
                |id| installed.contains(id) || index.has_bundle(id),
                |file| on_disk.contains(file),
            ));
            found.extend(index.scoped(|scope| {
                data.items()
                    .flat_map(|item| {
                        resource_ref_violations(item, scope)
                            .into_iter()
                            .map(move |fault| format!("{} {}: {fault}", item.kind(), item.id()))
                    })
                    .map(error)
                    .collect::<Vec<_>>()
            }));
            found
        }
    };
    let mut found = match world.run_system_once(lint) {
        Ok(found) => found,
        Err(fault) => vec![error(format!("the check did not run: {fault}"))],
    };
    found.sort_by_key(|problem| problem.severity);
    found.dedup();
    found
}

/// What in the open project the merge gate would refuse, by item. Each stops
/// Play, because a refusal while Playing is fatal. A rig with no project has
/// nothing to refuse.
pub(crate) fn play_faults(world: &mut World) -> Vec<String> {
    let Some(data) = world
        .get_resource::<super::ModProject>()
        .map(|project| std::sync::Arc::clone(&project.data))
    else {
        return vec![];
    };
    let faults = world.run_system_once(move |index: AssetIndex| {
        index.scoped(|scope| {
            data.items()
                .flat_map(|item| {
                    resource_ref_violations(item, scope)
                        .into_iter()
                        .map(move |fault| format!("{} {}: {fault}", item.kind(), item.id()))
                })
                .collect::<Vec<_>>()
        })
    });
    let mut faults = faults.unwrap_or_else(|fault| vec![format!("the check did not run: {fault}")]);
    faults.extend(asset_faults(world));
    faults
}

/// `leaf` as a package writes it, when it changes.
fn published_leaf(leaf: &str, bare: &HashSet<String>) -> Option<String> {
    if bare.contains(leaf) && is_bare_base_ref(leaf) {
        return Some(format!("{DEP_SCHEME}{leaf}"));
    }
    let rest = leaf.strip_prefix(DEP_SCHEME)?;
    let (id, file) = rest.split_once('/')?;
    id.starts_with(EDITOR_ID_PREFIX)
        .then(|| format!("{DEP_SCHEME}{}/{file}", publish_id(id)))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

/// File > Package: the Package window, or a refusal that says why not.
pub(crate) fn on_package_item(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| super::verbs::pressed(world, ask_to_package));
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        io::{Cursor, Read, Write},
        path::{Path, PathBuf},
        sync::Arc,
    };

    use nova_assets::{
        mod_cache::prelude::{
            exports_dir, install_local, is_safe_rel_path, list_mod_files, read_mod_file,
            replace_local,
        },
        mod_refs::prelude::{map_string_leaves, string_leaves},
    };
    use nova_modding::prelude::{
        parse_content, parse_manifest, serialize_content, serialize_manifest,
    };
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

    use super::{
        super::{
            ids::{is_taken, local_id, project_id},
            store::{self, bundle_file, file_bytes, is_declared, manifest_of},
            verbs::{lower_open_document, note, refuse},
            ModProject, PendingMods,
        },
        *,
    };
    use crate::{
        asset::files::mod_files,
        bundle::{FileWindowKind, FileWindowRequest},
        event::ScriptNodes,
        node::EditContext,
        problems::Problems,
        ui::{
            browser::BrowserTabType,
            hub::HubForm,
            package::open_package_modal,
            window::{ConfirmAsk, DestructiveVerb},
        },
    };

    /// The most an import may unpack. A bound rather than trust: a zip's sizes
    /// are the archive's own claim.
    const MAX_IMPORT_BYTES: u64 = 1 << 30;

    /// One file of a package.
    pub(crate) type PackageFile = (String, Vec<u8>);

    /// The files of mod `id`, as a package holds them. `other` is what the mod
    /// holds on disk beyond its manifest and content.
    pub(crate) fn package_files(
        id: &str,
        data: &ProjectData,
        other: Vec<PackageFile>,
    ) -> Result<Vec<PackageFile>, String> {
        let mut published = data.clone();
        published.open = None;
        for dep in &mut published.meta.dependencies {
            if dep.starts_with(EDITOR_ID_PREFIX) {
                *dep = publish_id(dep);
            }
        }
        for file in &mut published.files {
            for item in &mut file.items {
                let bare: HashSet<String> = bare_asset_refs(item).into_iter().collect();
                let changes = string_leaves(item)
                    .iter()
                    .any(|leaf| published_leaf(leaf, &bare).is_some());
                if changes {
                    *item = Arc::new(map_string_leaves(item, &|leaf| {
                        published_leaf(leaf, &bare)
                    })?);
                }
            }
        }
        let manifest = serialize_manifest(&manifest_of(&published))
            .map_err(|error| format!("manifest: {error}"))?;
        let mut files = vec![(
            format!("{}.bundle.ron", publish_id(id)),
            manifest.into_bytes(),
        )];
        for file in &published.files {
            files.push((file.path.clone(), file_bytes(file)?));
        }
        files.extend(other);
        Ok(files)
    }

    /// `files` as one archive.
    pub(crate) fn zip_bytes(files: &[PackageFile]) -> Result<Vec<u8>, String> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (path, bytes) in files {
            writer
                .start_file(path.as_str(), options)
                .map_err(|error| format!("{path}: {error}"))?;
            writer
                .write_all(bytes)
                .map_err(|error| format!("{path}: {error}"))?;
        }
        writer
            .finish()
            .map(Cursor::into_inner)
            .map_err(|error| error.to_string())
    }

    /// The files of an archive. Refuses the whole archive on one entry that
    /// could land outside the mod: a partly unpacked mod is not a mod.
    pub(crate) fn unzip_bytes(bytes: &[u8]) -> Result<Vec<PackageFile>, String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|error| format!("not a package: {error}"))?;
        let mut files = Vec::with_capacity(archive.len());
        let mut left = MAX_IMPORT_BYTES;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| format!("not a package: {error}"))?;
            let path = entry.name().to_string();
            if entry.is_dir() {
                continue;
            }
            if !is_safe_rel_path(&path) {
                return Err(format!("the package holds an unsafe path: {path}"));
            }
            let mut held = Vec::new();
            let read = (&mut entry)
                .take(left + 1)
                .read_to_end(&mut held)
                .map_err(|error| format!("{path}: {error}"))?;
            left = left
                .checked_sub(read as u64)
                .ok_or_else(|| "the package unpacks too large".to_string())?;
            files.push((path, held));
        }
        Ok(files)
    }

    /// A package made ready to install.
    #[derive(Debug)]
    pub(crate) struct Adopted {
        /// The local id it installs under.
        pub(crate) id: String,
        /// The manifest's version.
        pub(crate) version: String,
        /// The manifest's new path.
        pub(crate) bundle: String,
        /// Every file, the manifest and content rewritten.
        pub(crate) files: Vec<PackageFile>,
    }

    impl Adopted {
        /// This package under the spelling of the local mod it publishes as,
        /// if there is one: `editor_a_b` and `editor_a-b` are one portal entry,
        /// and two folders would share a namespace.
        fn onto_held_spelling(mut self) -> Self {
            let published = publish_id(&self.id);
            let held = store::local_ids()
                .into_iter()
                .find(|held| publish_id(held) == published);
            let Some(held) = held else {
                return self;
            };
            let bundle = bundle_file(&held);
            for (path, _) in &mut self.files {
                if *path == self.bundle {
                    path.clone_from(&bundle);
                }
            }
            self.id = held;
            self.bundle = bundle;
            self
        }
    }

    /// Turn a package into a local mod: the reverse of [`package_files`].
    ///
    /// A dependency on `x` becomes `editor_x` when that mod is here and `x` is
    /// not, which is how two local mods packaged together stay joined.
    pub(crate) fn adopt_package(
        mut files: Vec<PackageFile>,
        is_local: impl Fn(&str) -> bool,
        is_installed: impl Fn(&str) -> bool,
    ) -> Result<Adopted, String> {
        let mut roots = files
            .iter()
            .filter(|(path, _)| !path.contains('/') && path.ends_with(".bundle.ron"));
        let (root, _) = roots
            .next()
            .ok_or_else(|| "the package holds no manifest".to_string())?;
        if roots.next().is_some() {
            return Err("the package holds two manifests".to_string());
        }
        let root = root.clone();
        let published = root.trim_end_matches(".bundle.ron").to_string();
        if published.is_empty()
            || !published.chars().all(|letter| {
                letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-'
            })
        {
            return Err(format!("{published} is not a mod id"));
        }
        let id = project_id(&published)
            .map_err(|_| format!("{published} cannot be a mod's id: it is the game's own"))?;
        let at = files
            .iter()
            .position(|(path, _)| *path == root)
            .ok_or_else(|| "the package holds no manifest".to_string())?;
        let (_, bytes) = files.remove(at);
        let mut manifest = parse_manifest(&bytes).map_err(|error| error.to_string())?;
        for path in manifest.content.iter().chain(&manifest.resources) {
            if !files.iter().any(|(held, _)| held == path) {
                return Err(format!("the package is missing {path}"));
            }
        }
        let local_dep = |dep: &str| {
            let local = local_id(dep);
            (dep != BASE_MOD_ID && is_local(&local) && !is_installed(dep)).then_some(local)
        };
        for dep in &mut manifest.meta.dependencies {
            if let Some(local) = local_dep(dep) {
                *dep = local;
            }
        }
        for (path, bytes) in &mut files {
            if !manifest.content.contains(path) {
                continue;
            }
            let items = parse_content(bytes).map_err(|error| format!("{path}: {error}"))?;
            let adopted = items
                .iter()
                .map(|item| {
                    map_string_leaves(item, &|leaf| {
                        let (dep, file) = leaf.strip_prefix(DEP_SCHEME)?.split_once('/')?;
                        Some(format!("{DEP_SCHEME}{}/{file}", local_dep(dep)?))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            *bytes = serialize_content(&adopted)
                .map_err(|error| format!("{path}: {error}"))?
                .into_bytes();
        }
        let bundle = bundle_file(&id);
        let bytes = serialize_manifest(&manifest)
            .map_err(|error| format!("manifest: {error}"))?
            .into_bytes();
        files.insert(0, (bundle.clone(), bytes));
        Ok(Adopted {
            id,
            version: manifest.meta.version,
            bundle,
            files,
        })
    }

    /// Read and adopt the package at `path`.
    fn adopted_from(world: &mut World, path: &str) -> Result<Adopted, String> {
        let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
        let shipped = world
            .run_system_once(|index: AssetIndex| index.shipped_ids())
            .map_err(|fault| format!("the shipped mods could not be read: {fault}"))?;
        let installed: HashSet<String> = store::installed_mods()
            .into_iter()
            .map(|(id, _)| id)
            .chain(shipped)
            .collect();
        let local = store::local_ids();
        adopt_package(
            unzip_bytes(&bytes)?,
            |id| local.iter().any(|held| held == id),
            |id| installed.contains(id),
        )
    }

    /// Install what [`adopted_from`] made. `over` puts it in the place of the
    /// mod of that id, which stands as it was if the install fails.
    fn install_adopted(world: &mut World, adopted: &Adopted, over: bool) {
        let install = if over { replace_local } else { install_local };
        match install(
            &adopted.id,
            &adopted.version,
            &adopted.bundle,
            &adopted.files,
        ) {
            Ok(()) => {
                world
                    .get_resource_or_init::<PendingMods>()
                    .enable([adopted.id.clone()]);
                hub_says(world, format!("imported {}", adopted.id));
            }
            Err(error) => hub_says(world, format!("import failed: {error}")),
        }
    }

    /// Say `line` on the hub and make it list what is there now.
    fn hub_says(world: &mut World, line: String) {
        let mut form = world.resource_mut::<HubForm>();
        form.line = line;
        form.revision += 1;
    }

    /// Import Package: install the zip at `path`, or ask before replacing a mod
    /// of the same id.
    pub(crate) fn import_package(world: &mut World, path: &str) {
        let adopted = match adopted_from(world, path) {
            Ok(adopted) => adopted,
            Err(error) => {
                hub_says(world, error);
                return;
            }
        };
        if is_taken(&adopted.id, &store::local_ids()) {
            let verb = DestructiveVerb::OverwriteImport(path.to_string());
            let asked = world.run_system_cached_with(
                |In(verb): In<DestructiveVerb>, mut confirm: ConfirmAsk| confirm.ask(verb),
                verb,
            );
            if let Err(fault) = asked {
                error!("editor: the import question did not go up: {fault}");
            }
            return;
        }
        install_adopted(world, &adopted, false);
    }

    /// The answer to [`import_package`]'s question: replace the mod.
    pub(crate) fn import_over(world: &mut World, path: &str) {
        match adopted_from(world, path) {
            Ok(adopted) => install_adopted(world, &adopted.onto_held_spelling(), true),
            Err(error) => hub_says(world, error),
        }
    }

    /// The folder every package lands in.
    fn package_folder() -> Result<PathBuf, String> {
        exports_dir().ok_or_else(|| "this machine has no data folder".to_string())
    }

    /// The files mod `id` holds that its manifest names beyond the content.
    pub(crate) fn other_files(id: &str, data: &ProjectData) -> Result<Vec<PackageFile>, String> {
        list_mod_files(id)
            .into_iter()
            .filter(|path| {
                is_declared(data, path) && !data.files.iter().any(|file| file.path == *path)
            })
            .map(|path| {
                let bytes =
                    read_mod_file(id, &path).ok_or_else(|| format!("{path} cannot be read"))?;
                Ok((path, bytes))
            })
            .collect()
    }

    /// Write `bytes` to `path`, through a temporary file so a failed write
    /// never leaves half an archive under the real name.
    fn write_archive(path: &Path, bytes: &[u8]) -> Result<(), String> {
        let show = |error: std::io::Error| format!("{}: {error}", path.display());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(show)?;
        }
        let partial = path.with_extension("zip.partial");
        std::fs::write(&partial, bytes).map_err(show)?;
        std::fs::rename(&partial, path).map_err(show)
    }

    /// One file of the mod as the Package window lists it.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct PlannedFile {
        /// The path inside the mod and the archive.
        pub(crate) path: String,
        /// Its size, for a file the package holds.
        pub(crate) size: Option<u64>,
    }

    /// What Package would do now: the window's settings and the check.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct PackagePlan {
        /// The manifest's version.
        pub(crate) version: String,
        /// The folder the archive lands in, or why there is none.
        pub(crate) folder: Result<PathBuf, String>,
        /// The archive's file name.
        pub(crate) file_name: String,
        /// What the package holds, then the mod's files it leaves out.
        pub(crate) files: Vec<PlannedFile>,
        /// What blocks the package, worst first.
        pub(crate) errors: Vec<String>,
        /// How many warnings the check found. They do not block.
        pub(crate) warnings: usize,
    }

    /// A written package.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct PackageDone {
        /// Where the archive is.
        pub(crate) path: PathBuf,
        /// Its size on disk.
        pub(crate) size: u64,
    }

    /// What the open scenario's Events blocks leave unfilled, as errors.
    fn gap_faults(world: &mut World) -> Vec<String> {
        let Some(scenario) = world
            .get_resource::<EditContext>()
            .and_then(EditContext::scenario)
        else {
            return vec![];
        };
        world
            .run_system_once(move |script: ScriptNodes| script.gaps(scenario))
            .unwrap_or_default()
            .into_iter()
            .map(|(_, reason)| format!("events: {reason}"))
            .collect()
    }

    /// Check the open mod as Package would and say what it would write. The
    /// findings go to the Problems tab as well. `Err` when the mod has no id.
    pub(crate) fn plan_package(world: &mut World) -> Result<PackagePlan, String> {
        lower_open_document(world);
        let Some(project) = world.get_resource::<ModProject>() else {
            return Err("no mod is open".to_string());
        };
        let Some(id) = project.id.clone() else {
            return Err("name the mod before packaging it".to_string());
        };
        let data = Arc::clone(&project.data);
        let on_disk = store::files_of(&id);
        let left_out: Vec<PlannedFile> = mod_files(project, &on_disk)
            .into_iter()
            .filter(|file| file.on_disk && !file.packaged)
            .map(|file| PlannedFile {
                path: file.path,
                size: None,
            })
            .collect();
        let mut errors = gap_faults(world);
        errors.extend(asset_faults(world));
        let found = project_findings(world);
        let warnings = found
            .iter()
            .filter(|problem| problem.severity == ProblemSeverityType::Warning)
            .count();
        errors.extend(
            found
                .iter()
                .filter(|problem| problem.severity == ProblemSeverityType::Error)
                .map(|problem| problem.message.clone()),
        );
        world.resource_mut::<Problems>().set_project(found);
        let mut files =
            match other_files(&id, &data).and_then(|other| package_files(&id, &data, other)) {
                Ok(files) => files
                    .into_iter()
                    .map(|(path, bytes)| PlannedFile {
                        path,
                        size: Some(bytes.len() as u64),
                    })
                    .collect(),
                Err(fault) => {
                    errors.push(format!("the files cannot be read: {fault}"));
                    vec![]
                }
            };
        files.extend(left_out);
        let version = data.meta.version.trim().to_string();
        Ok(PackagePlan {
            folder: package_folder(),
            file_name: package_name(&id, &version),
            version,
            files,
            errors,
            warnings,
        })
    }

    /// File > Package: the window, once the mod has an id to package under.
    pub(crate) fn ask_to_package(world: &mut World) {
        if world
            .get_resource::<ModProject>()
            .is_some_and(|project| project.id.is_none())
        {
            refuse(world, "name the mod before packaging it");
            world.resource_mut::<FileWindowRequest>().0 = Some(FileWindowKind::NameMod);
            return;
        }
        open_package_modal(world);
    }

    /// The window's Package: check the mod again and write the archive if
    /// nothing blocks. The refusal or the fault is the `Err`, and the status
    /// line says it too.
    pub(crate) fn package_project(world: &mut World) -> Result<PackageDone, String> {
        let written = plan_package(world).and_then(|plan| {
            if let Some(first) = plan.errors.first() {
                if let Some(mut tab) = world.get_resource_mut::<BrowserTabType>() {
                    *tab = BrowserTabType::Problems;
                }
                return Err(match plan.errors.len() {
                    1 => format!("package refused: {first}"),
                    count => format!("package refused: {count} errors, first: {first}"),
                });
            }
            let id = world
                .resource::<ModProject>()
                .id
                .clone()
                .ok_or_else(|| "name the mod before packaging it".to_string())?;
            let data = Arc::clone(&world.resource::<ModProject>().data);
            let bytes = other_files(&id, &data)
                .and_then(|other| package_files(&id, &data, other))
                .and_then(|files| zip_bytes(&files))?;
            let path = plan.folder?.join(&plan.file_name);
            write_archive(&path, &bytes)?;
            Ok(PackageDone {
                size: bytes.len() as u64,
                path,
            })
        });
        match &written {
            Ok(done) => {
                let shown = done.path.display().to_string();
                info!("editor: packaged to {shown}, {} bytes", done.size);
                note(world, format!("packaged {shown}"));
            }
            Err(reason) => refuse(world, reason.clone()),
        }
        written
    }

    /// Show `folder` in the desktop's file manager. The opener runs on its own;
    /// a thread waits on it so it leaves no zombie behind.
    pub(crate) fn open_folder(folder: &Path) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        const OPENER: Option<&str> = Some("explorer");
        #[cfg(target_os = "macos")]
        const OPENER: Option<&str> = Some("open");
        #[cfg(all(unix, not(target_os = "macos")))]
        const OPENER: Option<&str> = Some("xdg-open");
        #[cfg(not(any(unix, target_os = "windows")))]
        const OPENER: Option<&str> = None;
        let opener = OPENER.ok_or_else(|| "this system has no file manager to open".to_string())?;
        let mut child = std::process::Command::new(opener)
            .arg(folder)
            .spawn()
            .map_err(|fault| format!("{opener} did not start: {fault}"))?;
        std::thread::spawn(move || child.wait());
        Ok(())
    }
}

/// The web has no file store to package from or import into.
#[cfg(target_arch = "wasm32")]
mod web {
    use super::{super::verbs::refuse, *};

    pub(crate) fn ask_to_package(world: &mut World) {
        refuse(world, "packaging is not available on the web yet");
    }

    pub(crate) fn import_package(_world: &mut World, _path: &str) {}

    pub(crate) fn import_over(_world: &mut World, _path: &str) {}
}
