//! What a builder does to a mod: open an asset, add, duplicate, rename and
//! delete a scenario, edit the manifest, save.
//!
//! Every verb is a function of the world, queued from a press, because each one
//! crosses the document, the project and history in one go. A step is
//! [`project_step`]: lower what is open, change the project, commit once.

use std::sync::Arc;

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_assets::mod_refs::prelude::{map_string_leaves, string_leaves, DEP_SCHEME};
use nova_mod_format::deps::{transitive_deps, DepGraph};
use nova_modding::prelude::{Content, ModMeta, BASE_MOD_ID};
use nova_scenario::prelude::{
    GeneratedWorldConfig, ScenarioConfig, ScenarioObjectConfig, ScenarioObjectKind,
    ScenarioWorldType, BOUNDS_AREA_ID,
};
use nova_ship::prelude::{GameSections, TargetingSettings};
use nova_ui::theme::UiColor;

use super::{
    ids::namespace_of,
    mint_scenario_id,
    package::asset_faults,
    store::{self, bundle_file, renamed_copy, write_project},
    ModProject, OpenAsset, PendingMods,
};
use crate::{
    asset::{frame_bench, lift_item, lowered_item, session::sync_project_sections},
    asset_index::AssetIndex,
    bundle::{
        document_content, fill_document, lift_scenario, spawned_in_layout, FileWindowKind,
        FileWindowRequest,
    },
    config::{EditorOverlays, EditorStatus, RailTab, SelectedNode},
    cues::EditorCues,
    event::{reveal, ScriptNodes},
    history::{
        commit_project_now, document_is_lowered, forget_history_now, mark_document_lowered,
        mark_document_switched, saved_now,
    },
    node::{
        found_document, found_empty_document, rebuild_node_views, EditContext, NodeId, ObjectNodes,
        ScenarioNode, SectionNodes, ShipNode,
    },
    scenario::{lower_fleet, world_objects, world_script, world_settings, FileFacts, HullForm},
    station::StationCatalogs,
    template::ScenarioTemplate,
    ui::browser::BrowserTabType,
};

/// Say `line` on the status line, in `colour`.
fn say(world: &mut World, line: impl Into<String>, colour: UiColor) {
    let now = world
        .get_resource::<Time>()
        .map_or(0.0, Time::elapsed_secs_f64);
    if let Some(mut status) = world.get_resource_mut::<EditorStatus>() {
        status.say(line.into(), colour, now);
    }
}

/// Say a refusal, with the buzz. The world verbs' [`EditorSays::refuse`].
///
/// [`EditorSays::refuse`]: crate::config::EditorSays::refuse
pub(crate) fn refuse(world: &mut World, line: impl Into<String>) {
    say(world, line, UiColor::Danger);
    if let Err(fault) = world.run_system_once(|mut cues: EditorCues| cues.deny()) {
        error!("editor: a refusal could not buzz: {fault}");
    }
}

/// The click of a press that went through.
pub(crate) fn accept(world: &mut World) {
    if let Err(fault) = world.run_system_once(|mut cues: EditorCues| cues.select()) {
        error!("editor: an accepted press could not click: {fault}");
    }
}

/// Run `verb` for a press on a [`ButtonCueType::Owned`] button: it clicks unless
/// the verb refused, and a refusal has already buzzed.
///
/// Here and not in the verb, because the same verb also runs from a key, and a
/// key has no click to play.
///
/// [`ButtonCueType::Owned`]: nova_ui::widget::ButtonCueType::Owned
pub(crate) fn pressed(world: &mut World, verb: impl FnOnce(&mut World)) {
    let refusals = |world: &World| {
        world
            .get_resource::<EditorStatus>()
            .map_or(0, EditorStatus::refusals)
    };
    let before = refusals(world);
    verb(world);
    if refusals(world) == before {
        accept(world);
    }
}

/// Say what happened.
pub(crate) fn note(world: &mut World, line: impl Into<String>) {
    say(world, line, UiColor::Primary);
}

/// The open document as items, the scenario under `id`.
fn lowered_items(
    In(id): In<String>,
    context: Res<EditContext>,
    nodes: SectionNodes,
    q_objects: ObjectNodes,
    q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>,
    script: ScriptNodes,
    q_settings: Query<&ScenarioNode>,
    targeting: Option<Res<TargetingSettings>>,
    catalogs: StationCatalogs,
    project: Option<Res<ModProject>>,
) -> Vec<Content> {
    let targeting = targeting.as_deref().cloned().unwrap_or_default();
    let world = world_objects(&context, &q_objects);
    let held = project
        .as_deref()
        .and_then(|project| project.data.scenario(&id));
    let unsigned = held.map_or_else(Vec::new, |held| unsigned_beacons(held, &world));
    let world_ids = held.map_or_else(Vec::new, world_ids);
    // A station design the lint refuses is still written: the file is the
    // builder's work, and Play is where a refused station is stopped.
    let fleet = lower_fleet(&q_ships, &nodes);
    let stations = catalogs
        .designs(&fleet, HullForm::Prototype { scenario: &id })
        .into_iter()
        .map(|(design, _)| design)
        .collect();
    document_content(
        &world_settings(&context, &q_settings),
        &id,
        FileFacts {
            unsigned: &unsigned,
            world_ids: &world_ids,
        },
        world,
        &fleet,
        stations,
        world_script(&context, &script),
        &targeting,
    )
}

/// The ids `held`'s world brings with it.
fn world_ids(held: &ScenarioConfig) -> Vec<String> {
    match &held.world {
        ScenarioWorldType::Generated(GeneratedWorldConfig {
            bounds: Some(_), ..
        }) => vec![BOUNDS_AREA_ID.to_string()],
        _ => Vec::new(),
    }
}

/// The beacons of `world` that state no signature and whose spawn in `held`
/// states none either. See [`FileFacts::unsigned`].
fn unsigned_beacons(held: &ScenarioConfig, world: &[ScenarioObjectConfig]) -> Vec<String> {
    let is_unsigned = |object: &ScenarioObjectConfig| {
        matches!(
            &object.kind,
            ScenarioObjectKind::Beacon(beacon) if beacon.lock_signature.is_none()
        )
    };
    spawned_in_layout(held)
        .filter(|spawn| is_unsigned(spawn))
        .filter(|spawn| {
            world
                .iter()
                .any(|object| object.base.id == spawn.base.id && is_unsigned(object))
        })
        .map(|spawn| spawn.base.id.clone())
        .collect()
}

/// Write the item on the bench back where it stands in the mod.
fn absorb_bench(world: &mut World, id: &str) {
    let root = world.resource::<EditContext>().scenario();
    let Some(item) = root.and_then(|root| lowered_item(world, root)) else {
        return;
    };
    world
        .resource_mut::<ModProject>()
        .edit()
        .replace_item(id, item);
}

/// Write the document into the project, under the open asset's id.
///
/// With nothing open the document is new work, so it mints a scenario: nothing a
/// builder placed is ever left outside the mod.
pub(crate) fn absorb_document(world: &mut World) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    if let Some(open) = project.data.open_bench() {
        let id = open.id.clone();
        absorb_bench(world, &id);
        return;
    }
    let open = project.data.open_scenario().map(str::to_string);
    let id = open.unwrap_or_else(|| {
        let name = world
            .get_resource::<EditContext>()
            .and_then(EditContext::scenario)
            .and_then(|root| world.get::<ScenarioNode>(root))
            .map_or_else(String::new, |settings| settings.name.clone());
        mint_scenario_id(project, &name)
    });
    let migrating = project.migrate.as_deref() == Some(id.as_str());
    let file = project.default_file();
    let items = match world.run_system_cached_with(lowered_items, id.clone()) {
        Ok(items) => items,
        Err(fault) => {
            error!("editor: the document could not be lowered: {fault}");
            return;
        }
    };
    let mut project = world.resource_mut::<ModProject>();
    let data = project.edit();
    data.absorb(&id, items, migrating, &file);
    data.open = Some(OpenAsset::scenario(id));
    if migrating {
        project.migrate = None;
    }
}

/// Refuse `verb` while the document holds a block the file cannot spell, and
/// answer whether it did.
///
/// Every lowering would lose that block, so a verb that writes or leaves the
/// document stops here, says why and marks the block. The block stays on the
/// canvas for the builder to finish or delete.
pub(crate) fn refused_by_gaps(world: &mut World, verb: &str) -> bool {
    let Some(scenario) = world
        .get_resource::<EditContext>()
        .and_then(EditContext::scenario)
    else {
        return false;
    };
    let gaps = world
        .run_system_once(move |script: ScriptNodes| script.gaps(scenario))
        .unwrap_or_default();
    let Some((node, reason)) = gaps.first().cloned() else {
        return false;
    };
    reveal(world, node);
    if let Some(mut tab) = world.get_resource_mut::<RailTab>() {
        *tab = RailTab::Events;
    }
    world.resource_mut::<SelectedNode>().set(Some(node));
    let line = match gaps.len() {
        1 => format!("{verb} stopped: {reason}"),
        more => format!("{verb} stopped: {reason} (+{} more)", more - 1),
    };
    refuse(world, line);
    true
}

/// Bring the project up to date with the document. Not a step, and a no-op when
/// nothing changed since the last time.
pub(crate) fn lower_open_document(world: &mut World) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let migrating =
        project.migrate.is_some() && project.migrate.as_deref() == project.data.open_scenario();
    let has_document = world
        .get_resource::<EditContext>()
        .is_some_and(|context| context.scenario().is_some());
    if !has_document || (!migrating && document_is_lowered(world)) {
        return;
    }
    absorb_document(world);
    mark_document_lowered(world);
}

/// Throw the nodes away and stand `open` up in their place, or an empty document.
fn rebuild_document(world: &mut World, open: Option<&OpenAsset>) {
    sync_project_sections(world);
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<ScenarioNode>>()
        .iter(world)
        .collect();
    for root in roots {
        world.entity_mut(root).despawn();
    }
    if let Some(mut selected) = world.get_resource_mut::<SelectedNode>() {
        selected.set(None);
    }
    let scenario_id = open.filter(|open| open.is_scenario()).map(|open| &open.id);
    let lifted = world.get_resource::<ModProject>().and_then(|project| {
        let id = scenario_id?;
        let old_save = project.migrate.as_ref() == Some(id);
        lift_scenario(project.data.items(), id, old_save)
    });
    let bench = open.filter(|open| !open.is_scenario()).and_then(|open| {
        let project = world.get_resource::<ModProject>()?;
        project.data.item(open.kind, &open.id).cloned()
    });
    let mut context = world.resource_mut::<EditContext>();
    let mut held = std::mem::take(&mut *context);
    let scenario = {
        let mut commands = world.commands();
        found_empty_document(&mut commands, &mut held)
    };
    *world.resource_mut::<EditContext>() = held;
    world.flush();
    if let Some(lifted) = lifted {
        fill_document(world, scenario, lifted);
    }
    if let Some(item) = bench {
        lift_item(world, scenario, &item);
        frame_bench(world, scenario);
        // A bench has no script for the Events tab to show.
        if let Some(mut tab) = world.get_resource_mut::<RailTab>() {
            *tab = RailTab::Scene;
        }
    }
}

/// Whether an Error the content rules raise stops `verb`. A mod holding one
/// would be refused at the next merge, and that refusal is fatal in Playing, so
/// the stop is here and it names the fault in the Problems tab.
pub(crate) fn refused_by_asset_faults(world: &mut World, verb: &str) -> bool {
    let faults = asset_faults(world);
    let Some(first) = faults.first() else {
        return false;
    };
    let line = match faults.len() {
        1 => format!("{verb} stopped: {first}"),
        more => format!("{verb} stopped: {first} (+{} more)", more - 1),
    };
    if let Some(mut tab) = world.get_resource_mut::<BrowserTabType>() {
        *tab = BrowserTabType::Problems;
    }
    refuse(world, line);
    true
}

/// Stand the asset the project has open up in the document, with its bodies.
pub(crate) fn rebuild_open_document(world: &mut World) {
    let open = world.resource::<ModProject>().data.open.clone();
    rebuild_document(world, open.as_ref());
    views(world);
}

/// Give the document its bodies.
fn views(world: &mut World) {
    if let Err(fault) = world.run_system_cached(rebuild_node_views) {
        error!("editor: the document's views were not built: {fault}");
    }
}

/// Found a document from `template`, for a scenario that does not exist yet.
fn found_from_template(world: &mut World, template: ScenarioTemplate) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<ScenarioNode>>()
        .iter(world)
        .collect();
    for root in roots {
        world.entity_mut(root).despawn();
    }
    if let Some(mut selected) = world.get_resource_mut::<SelectedNode>() {
        selected.set(None);
    }
    // Taken out and put back: the catalog is read while the commands hold the
    // world.
    let sections = world.remove_resource::<GameSections>();
    let mut held = std::mem::take(&mut *world.resource_mut::<EditContext>());
    {
        let mut commands = world.commands();
        found_document(&mut commands, sections.as_ref(), &mut held, template);
    }
    *world.resource_mut::<EditContext>() = held;
    world.flush();
    if let Some(sections) = sections {
        world.insert_resource(sections);
    }
}

/// Stand the document up on entering the editor, from what the project holds.
///
/// A project with a seed is a new mod: its first scenario is founded here,
/// because a template is nodes and the hub has no document to found it in.
pub(crate) fn open_project_document(world: &mut World) {
    let seed = world
        .get_resource_mut::<ModProject>()
        .and_then(|mut project| project.seed.take());
    match seed {
        Some(template) => {
            found_from_template(world, template);
            absorb_document(world);
            forget_history_now(world);
            let saved = world
                .resource::<ModProject>()
                .id
                .is_some()
                .then(|| write_first(world));
            if let Some(Err(error)) = saved {
                refuse(world, format!("save failed: {error}"));
            }
        }
        None => {
            let open = world.resource::<ModProject>().data.open.clone();
            rebuild_document(world, open.as_ref());
            forget_history_now(world);
        }
    }
}

/// Write a project founded a moment ago, so it is on disk before any edit.
fn write_first(world: &mut World) -> Result<(), String> {
    let written = write_project(world.resource::<ModProject>());
    if written.is_ok() {
        let mut project = world.resource_mut::<ModProject>();
        project.on_disk = Some(Arc::clone(&project.data));
        if let Some(id) = project.id.clone() {
            store::remember_project(&id);
            world.get_resource_or_init::<PendingMods>().enable([id]);
        }
    }
    written
}

/// Run `work` and record it as one step. The document is lowered first, so a
/// step taken back returns every edit made before it.
pub(crate) fn project_step(
    world: &mut World,
    label: impl Into<std::borrow::Cow<'static, str>>,
    work: impl FnOnce(&mut World),
) {
    let label = label.into();
    if refused_by_gaps(world, &label) {
        return;
    }
    lower_open_document(world);
    work(world);
    commit_project_now(world, label);
}

/// Open the item of `kind` and `id`. Never a step, and never loses an edit: the
/// document is lowered first.
pub(crate) fn open_asset(world: &mut World, kind: &'static str, id: &str) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let wanted = OpenAsset {
        kind,
        id: id.to_string(),
    };
    if project.data.open.as_ref() == Some(&wanted) {
        return;
    }
    if project.data.item(kind, id).is_none() {
        refuse(world, format!("{id} is not in this mod"));
        return;
    }
    if refused_by_gaps(world, "Open") {
        return;
    }
    lower_open_document(world);
    world.resource_mut::<ModProject>().edit().open = Some(wanted);
    rebuild_open_document(world);
    mark_document_switched(world);
    if kind == "section" {
        if let Some(mut overlays) = world.get_resource_mut::<EditorOverlays>() {
            overlays.link_points = true;
        }
    }
    let name = world
        .resource::<ModProject>()
        .data
        .open_name()
        .unwrap_or_default();
    note(world, format!("opened {name}"));
}

/// File > New Scenario, and the Assets tab's Add: a new scenario from
/// `template`, open.
pub(crate) fn add_scenario(world: &mut World, template: ScenarioTemplate) {
    if !world.contains_resource::<ModProject>() {
        return;
    }
    project_step(world, "Add Scenario", |world| {
        let project = world.resource::<ModProject>();
        let id = mint_scenario_id(project, template.settings().name.as_str());
        world.resource_mut::<ModProject>().edit().open = Some(OpenAsset::scenario(id));
        found_from_template(world, template);
        views(world);
        absorb_document(world);
    });
}

/// A copy of scenario `id` beside it, with its own designs. The open document
/// stays open.
pub(crate) fn duplicate_scenario(world: &mut World, id: &str) {
    if !world.contains_resource::<ModProject>() {
        return;
    }
    let id = id.to_string();
    project_step(world, "Duplicate Scenario", move |world| {
        let project = world.resource::<ModProject>();
        let Some(source) = project.data.scenario(&id) else {
            return;
        };
        let name = format!("{} Copy", source.name);
        let copy = mint_scenario_id(project, &name);
        let (from, to) = (format!("{id}."), format!("{copy}."));
        let rename = |leaf: &str| {
            if leaf == id {
                return Some(copy.clone());
            }
            leaf.strip_prefix(&from).map(|node| format!("{to}{node}"))
        };
        let mut items = Vec::new();
        for item in project.data.items() {
            let owned = matches!(item, Content::Ship(_) | Content::Station(_))
                && item.id().starts_with(&from);
            if !owned && item.id() != id {
                continue;
            }
            match map_string_leaves(item, &rename) {
                Ok(renamed) => items.push(renamed),
                Err(fault) => {
                    error!("editor: {} did not copy: {fault}", item.id());
                    return;
                }
            }
        }
        for item in &mut items {
            if let Content::Scenario(scenario) = item {
                scenario.name.clone_from(&name);
            }
        }
        world
            .resource_mut::<ModProject>()
            .edit()
            .insert_after(&id, items);
    });
}

/// Rename scenario `id`. Its id stays: other content may name it.
pub(crate) fn rename_scenario(world: &mut World, id: &str, name: &str) {
    lower_open_document(world);
    let same = world.get_resource::<ModProject>().is_some_and(|project| {
        project
            .data
            .scenarios()
            .any(|held| held.id == id && held.name == name)
    });
    if same {
        return;
    }
    let (id, name) = (id.to_string(), name.to_string());
    project_step(world, "Rename Scenario", move |world| {
        let mut project = world.resource_mut::<ModProject>();
        let open = project.data.open_scenario() == Some(id.as_str());
        project
            .edit()
            .change_scenario(&id, |scenario| scenario.name.clone_from(&name));
        if !open {
            return;
        }
        let root = world.resource::<EditContext>().scenario();
        if let Some(mut settings) = root.and_then(|root| world.get_mut::<ScenarioNode>(root)) {
            settings.name = name;
        }
    });
}

/// Delete scenario `id`, and the designs only it owned. Undo returns it whole.
pub(crate) fn delete_scenario(world: &mut World, id: &str) {
    let id = id.to_string();
    project_step(world, "Delete Scenario", move |world| {
        let mut project = world.resource_mut::<ModProject>();
        let open = project.data.open_scenario() == Some(id.as_str());
        let data = project.edit();
        data.remove_scenario(&id);
        if !open {
            return;
        }
        let next = data.scenarios().next().map(|scenario| scenario.id.clone());
        data.open = next.map(OpenAsset::scenario);
        rebuild_open_document(world);
    });
}

/// Change the manifest's description of the mod as one step.
pub(crate) fn edit_meta(
    world: &mut World,
    label: impl Into<std::borrow::Cow<'static, str>>,
    change: impl FnOnce(&mut ModMeta),
) {
    project_step(world, label, move |world| {
        let mut project = world.resource_mut::<ModProject>();
        let mut meta = project.data.meta.clone();
        change(&mut meta);
        if format!("{meta:?}") != format!("{:?}", project.data.meta) {
            project.edit().meta = meta;
        }
    });
}

/// Ctrl+S and File > Save: write the mod, or ask a project with no id yet for a
/// name.
pub(crate) fn save_or_ask(world: &mut World) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    if project.id.is_none() {
        world.resource_mut::<FileWindowRequest>().0 = Some(FileWindowKind::NameMod);
        return;
    }
    save_project(world);
}

/// Write the mod to its own id.
pub(crate) fn save_project(world: &mut World) {
    if refused_by_gaps(world, "Save") {
        return;
    }
    lower_open_document(world);
    if refused_by_asset_faults(world, "Save") {
        return;
    }
    let project = world.resource::<ModProject>();
    let Some(id) = project.id.clone() else {
        return;
    };
    if let Err(error) = write_project(project) {
        error!("editor: the save failed - {error}");
        refuse(world, format!("save failed: {error}"));
        return;
    }
    let dependencies = project.data.meta.dependencies.clone();
    let name = project.data.meta.name.clone();
    let mut project = world.resource_mut::<ModProject>();
    project.on_disk = Some(Arc::clone(&project.data));
    saved_now(world);
    // Enabled as well as installed: a save is the builder's own work, and it
    // reaches the Scenarios list on the way out.
    world.get_resource_or_init::<PendingMods>().enable(
        std::iter::once(id.clone())
            .chain(dependencies.into_iter().filter(|dep| dep != BASE_MOD_ID)),
    );
    store::remember_project(&id);
    info!("editor: saved mod '{id}'");
    note(world, format!("saved {name}"));
    accept(world);
}

/// Give a project with no id its first one, and write it. Its scenario ids keep
/// the namespace they were minted under: history holds them.
pub(crate) fn name_project(world: &mut World, name: &str) {
    let id = match store::copy_id(name) {
        Ok(id) => id,
        Err(line) => {
            refuse(world, line);
            return;
        }
    };
    let mut project = world.resource_mut::<ModProject>();
    project.bundle = bundle_file(&id);
    project.id = Some(id);
    if project.data.meta.name.is_empty() || project.data.meta.name != name {
        project.edit().meta.name = name.to_string();
    }
    save_project(world);
}

/// File > Save As Copy: write a new mod under `name` and stay in this one, so
/// every step on the stack still means this project.
pub(crate) fn save_copy(world: &mut World, name: &str) {
    if refused_by_gaps(world, "Save") {
        return;
    }
    lower_open_document(world);
    if refused_by_asset_faults(world, "Save") {
        return;
    }
    let id = match store::copy_id(name) {
        Ok(id) => id,
        Err(line) => {
            refuse(world, line);
            return;
        }
    };
    let project = world.resource::<ModProject>();
    let from = project.id.clone().unwrap_or_default();
    let written = renamed_copy(&project.data, &from, namespace_of(&id), name)
        .and_then(|copy| store::write_copy(&id, &copy, &from));
    match written {
        Ok(()) => {
            world
                .get_resource_or_init::<PendingMods>()
                .enable([id.clone()]);
            note(world, format!("saved a copy as {id}"));
            accept(world);
        }
        Err(error) => refuse(world, format!("copy failed: {error}")),
    }
}

/// Every mod id a `dep://` in `text` names.
fn dep_ids(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(DEP_SCHEME)
        .filter_map(|(at, _)| text[at + DEP_SCHEME.len()..].split_once('/'))
        .map(|(id, _)| id)
}

/// Declare every mod the changed records reach into and the project does not
/// declare yet, called from the commit so it lands in the same step.
///
/// One hook rather than one per picker: a ref arrives by the asset window, by
/// typing and by paste, a part by the catalog and the gallery, and a pick the
/// project does not declare passes Play and then fails the merge gate.
pub(crate) fn declare_used(world: &mut World, keys: &[&str], parts: &[String]) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let declared = &project.data.meta.dependencies;
    let own = project.id.clone();
    let wanted: Vec<String> = keys
        .iter()
        .flat_map(|key| dep_ids(key))
        .map(str::to_string)
        .collect();
    let parts = parts.to_vec();
    let declared = declared.clone();
    let owners = world.run_system_cached_with(
        |In((wanted, parts)): In<(Vec<String>, Vec<String>)>, index: AssetIndex| {
            let mut owners: Vec<String> = wanted
                .into_iter()
                .filter(|id| index.has_bundle(id))
                .collect();
            owners.extend(parts.iter().filter_map(|part| index.undeclared_owner(part)));
            owners
        },
        (wanted, parts),
    );
    let Ok(mut owners) = owners else {
        return;
    };
    owners.retain(|id| {
        id != BASE_MOD_ID && !declared.contains(id) && own.as_deref() != Some(id.as_str())
    });
    owners.sort();
    owners.dedup();
    if owners.is_empty() {
        return;
    }
    let mut project = world.resource_mut::<ModProject>();
    project
        .edit()
        .meta
        .dependencies
        .extend(owners.iter().cloned());
    note(world, format!("added dependency: {}", owners.join(", ")));
}

/// What in this project references mod `dep`, by item.
pub(crate) fn references_to(world: &mut World, dep: &str) -> Vec<String> {
    let scheme = format!("{DEP_SCHEME}{dep}/");
    let ids = world
        .run_system_cached_with(
            |In(dep): In<String>, index: AssetIndex| index.content_ids(&dep),
            dep.to_string(),
        )
        .unwrap_or_default();
    let project = world.resource::<ModProject>();
    let mut found: Vec<String> = project
        .data
        .items()
        .filter(|item| {
            string_leaves(item)
                .iter()
                .any(|leaf| leaf.starts_with(&scheme) || ids.contains(leaf))
        })
        .map(|item| format!("{} {}", item.kind(), item.id()))
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Whether adding `dep` to mod `own` closes a cycle, given every mod's own list.
pub(crate) fn closes_cycle(graph: &DepGraph, own: &str, dep: &str) -> bool {
    dep == own || transitive_deps(graph, dep).map_or(true, |deps| deps.iter().any(|id| id == own))
}

/// Every mod's own dependency list: the loaded bundles and the installed ones.
pub(crate) fn dep_graph(world: &mut World) -> DepGraph {
    let mut graph: DepGraph = store::installed_deps().into_iter().collect();
    let loaded = world.run_system_cached(|index: AssetIndex| index.loaded_deps());
    graph.extend(loaded.unwrap_or_default());
    graph
}

/// Add `dep` to the project's dependencies, as one step. Refuses the cycle.
pub(crate) fn add_dependency(world: &mut World, dep: &str) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let own = project.id.clone().unwrap_or_default();
    let mut graph = dep_graph(world);
    let project = world.resource::<ModProject>();
    graph.insert(own.clone(), project.data.meta.dependencies.clone());
    if closes_cycle(&graph, &own, dep) {
        refuse(world, format!("{dep} depends on this mod: that is a cycle"));
        return;
    }
    let dep = dep.to_string();
    edit_meta(world, "Add Dependency", move |meta| {
        if !meta.dependencies.contains(&dep) {
            meta.dependencies.push(dep);
        }
    });
}

/// Remove `dep` from the project's dependencies, as one step, unless something
/// in the mod still reaches into it. Answers what does.
pub(crate) fn remove_dependency(world: &mut World, dep: &str) -> Vec<String> {
    lower_open_document(world);
    let references = references_to(world, dep);
    if !references.is_empty() {
        let shown = references.iter().take(3).cloned().collect::<Vec<_>>();
        refuse(world, format!("{dep} is in use by {}", shown.join(", ")));
        return references;
    }
    let dep = dep.to_string();
    edit_meta(world, "Remove Dependency", move |meta| {
        meta.dependencies.retain(|held| *held != dep);
    });
    vec![]
}
