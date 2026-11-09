//! What a builder does to the mod's non-scenario items: add one from a
//! prototype, copy, rename, delete, and work on the open section's sockets and
//! mesh. Each is one undo step.

use std::sync::Arc;

use bevy::prelude::*;
use nova_assets::mod_refs::prelude::{map_string_leaves, names_string, SELF_SCHEME};
use nova_gameplay::prelude::ItemDesignId;
use nova_modding::prelude::Content;
use nova_ship::prelude::{LinkPoint, LinkPointSkinType};
use nova_ui::theme::PHOSPHOR_THEME_ID;

use super::{
    blank::{blank_item, spaced},
    set_mesh, spawn_socket, AssetNode, SocketNode,
};
use crate::{
    config::SelectedNode,
    history::commit_edit_now,
    inspect::{set_item_text, AssetTextType},
    node::{EditContext, NodeId},
    project::{
        ids::{scenario_id, slug},
        store,
        verbs::{
            absorb_document, lower_open_document, note, project_step, rebuild_open_document, refuse,
        },
        ModProject, OpenAsset, ProjectData,
    },
};

/// The six faces of a cell, as outward normals, in the order sockets are offered.
const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// The word a builder reads for `kind`, capitalised.
fn word(kind: &str) -> String {
    let mut letters = kind.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// `item` under `id`, and nothing else about it changed.
pub(crate) fn with_id(item: &mut Content, id: &str) {
    match item {
        Content::Section(config) => config.base.id = id.to_string(),
        Content::Campaign(config) => config.id = id.to_string(),
        Content::Style(config) => config.id = id.to_string(),
        Content::UiTheme(config) => config.id = id.to_string(),
        Content::Item(design) => design.id = ItemDesignId::from(id),
        Content::StartRole(role) => role.id = id.into(),
        Content::TradeProfile(profile) => profile.id = id.into(),
        Content::Ship(design) => design.id = id.into(),
        Content::Scenario(_)
        | Content::Station(_)
        | Content::Lesson(_)
        | Content::PriceTable(_) => {}
    }
}

/// The id a new item takes: `<namespace>__<slug>`, the one spelling a mod's
/// own ids have, free among the items of every kind. A string does not say which
/// kind it means, so a rename could not move one of two that shared an id.
fn mint_id(project: &ModProject, stem: &str) -> String {
    scenario_id(&project.namespace, stem, &project.data.all_ids())
}

/// Add `item` to the mod and open it, as one step.
pub(crate) fn add_and_open(world: &mut World, label: String, item: Content) {
    project_step(world, label, move |world| {
        crate::project::trail::leave_stop(world);
        let mut project = world.resource_mut::<ModProject>();
        let file = project.default_file();
        let open = OpenAsset {
            kind: item.kind(),
            id: item.id().to_string(),
        };
        let name = item.id().to_string();
        let data = project.edit();
        data.add_item(item, &file);
        data.open = Some(open);
        rebuild_open_document(world);
        note(world, format!("added {name}"));
    });
}

/// `item` under the name `name`, for the kinds that carry one.
fn with_name(item: &mut Content, name: &str) {
    let held = match item {
        Content::Section(config) => &mut config.base.name,
        Content::Campaign(config) => &mut config.name,
        Content::Style(config) => &mut config.name,
        Content::UiTheme(config) => &mut config.name,
        Content::Item(design) => &mut design.name,
        Content::StartRole(role) => &mut role.name,
        Content::Ship(design) => &mut design.name,
        _ => return,
    };
    *held = name.to_string();
}

/// What a new item of `kind` is called: `New UI Theme`, or for a section
/// of kind `CargoIntake`, `New Cargo Intake`.
fn new_name(kind: &str, section: Option<&str>) -> String {
    let words: Vec<String> = match section {
        Some(section) => vec![spaced(section)],
        None => kind
            .split(' ')
            .map(|part| match part {
                "ui" => "UI".to_string(),
                other => word(other),
            })
            .collect(),
    };
    format!("New {}", words.join(" "))
}

/// `+ Add`: a blank item of `kind`, open. A section is of the kind `section`
/// names; see [`blank_item`] for what a blank holds.
pub(crate) fn add_asset(world: &mut World, kind: &'static str, section: Option<&str>) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    if kind == "price table" && !project.data.ids_of(kind).is_empty() {
        refuse(world, "a mod may state one price table");
        return;
    }
    let mut item = match blank_item(kind, section) {
        Ok(item) => item,
        Err(fault) => {
            refuse(world, fault);
            return;
        }
    };
    let name = new_name(kind, section);
    // A root theme must set every role, and the form cannot write roles, so
    // a new theme derives from the stock one and overrides what it changes.
    if let Content::UiTheme(theme) = &mut item {
        theme.inherit = Some(PHOSPHOR_THEME_ID.to_string());
    }
    let id = mint_id(world.resource::<ModProject>(), &name);
    with_id(&mut item, &id);
    with_name(&mut item, &name);
    add_and_open(world, format!("Add {}", word(kind)), item);
}

/// A copy of the item beside it, open.
pub(crate) fn duplicate_asset(world: &mut World, kind: &str, id: &str) {
    if kind == "price table" {
        refuse(world, "a mod may state one price table");
        return;
    }
    lower_open_document(world);
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let Some(mut item) = project.data.item(kind, id).cloned() else {
        return;
    };
    let stem = format!("{}_copy", id.rsplit("__").next().unwrap_or(id));
    let copy = mint_id(project, &stem);
    with_id(&mut item, &copy);
    add_and_open(world, format!("Duplicate {}", word(kind)), item);
}

/// The id `typed` asks for: the mod's namespace on the front, whether or not it
/// was typed, and the one spelling mod ids have.
fn typed_id(namespace: &str, typed: &str) -> Option<String> {
    let prefix = format!("{}__", namespace.replace('-', "_"));
    let stem = slug(typed.trim().strip_prefix(&prefix).unwrap_or(typed.trim())).replace('-', "_");
    (!stem.is_empty()).then(|| format!("{prefix}{stem}"))
}

/// Give the item a new id, and every reference inside the project with it. A
/// clash is refused, and so is an id held as a map key, which no leaf walk
/// rewrites: a build recipe holds its parts by key.
pub(crate) fn rename_asset(world: &mut World, kind: &'static str, id: &str, typed: &str) {
    lower_open_document(world);
    if kind == "price table" {
        refuse(world, "a price table has no id of its own");
        return;
    }
    if typed.trim() == id {
        return;
    }
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let Some(new) = typed_id(&project.namespace, typed) else {
        refuse(world, "an id needs a letter or a digit");
        return;
    };
    if new == id {
        return;
    }
    if project.data.all_ids().contains(&new) {
        refuse(world, format!("{new} is already in this mod"));
        return;
    }
    let also = project
        .data
        .items()
        .find(|item| item.id() == id && item.kind() != kind)
        .map(Content::kind);
    if let Some(other) = also {
        refuse(
            world,
            format!("{id} is also a {other}: a rename would move both"),
        );
        return;
    }
    let renamed = match renamed_items(&project.data, id, &new) {
        Ok(renamed) => renamed,
        Err(line) => {
            refuse(world, line);
            return;
        }
    };
    let (from, label) = (id.to_string(), format!("Rename {}", word(kind)));
    project_step(world, label, move |world| {
        let mut project = world.resource_mut::<ModProject>();
        let data = project.edit();
        for (file, place, item) in renamed {
            data.files[file].items[place] = Arc::new(item);
        }
        if let Some(open) = data.open.as_mut() {
            if open.kind == kind && open.id == from {
                open.id.clone_from(&new);
            }
        }
        // The open asset's nodes hold the old id too, and any of them may.
        rebuild_open_document(world);
    });
}

/// Rename the item on the bench to `typed`.
pub(crate) fn rename_open_asset(world: &mut World, typed: &str) {
    let open = world
        .get_resource::<ModProject>()
        .and_then(|project| project.data.open_bench().cloned());
    if let Some(open) = open {
        rename_asset(world, open.kind, &open.id, typed);
    }
}

/// Every item that names `id`, with `new` in its place, by file and place. An
/// item that still names it after the walk holds it as a map key: refused.
fn renamed_items(
    data: &ProjectData,
    id: &str,
    new: &str,
) -> Result<Vec<(usize, usize, Content)>, String> {
    let change = |leaf: &str| (leaf == id).then(|| new.to_string());
    let mut renamed = Vec::new();
    let mut keyed = Vec::new();
    for (at_file, file) in data.files.iter().enumerate() {
        for (place, item) in file.items.iter().enumerate() {
            if !names_string(item, id) {
                continue;
            }
            let item = map_string_leaves(item, &change)
                .map_err(|fault| format!("{id} did not rename: {fault}"))?;
            if names_string(&item, id) {
                keyed.push(format!("{} {}", item.kind(), item.id()));
            }
            renamed.push((at_file, place, item));
        }
    }
    if keyed.is_empty() {
        Ok(renamed)
    } else {
        Err(format!("{id} is a key in {}", shown(&keyed)))
    }
}

/// Three of `lines`, and how many more.
pub(crate) fn shown(lines: &[String]) -> String {
    let first = lines.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
    match lines.len() {
        0..=3 => first,
        more => format!("{first} (+{} more)", more - 3),
    }
}

/// Delete the item, unless something in the mod names it. Answers what does.
pub(crate) fn delete_asset(world: &mut World, kind: &str, id: &str) -> Vec<String> {
    lower_open_document(world);
    let Some(project) = world.get_resource::<ModProject>() else {
        return vec![];
    };
    let users = project.data.users_of(kind, id);
    if !users.is_empty() {
        refuse(world, format!("{id} is in use by {}", shown(&users)));
        return users;
    }
    let (kind, id) = (kind.to_string(), id.to_string());
    let label = format!("Delete {}", word(&kind));
    project_step(world, label, move |world| {
        let mut project = world.resource_mut::<ModProject>();
        let data = project.edit();
        data.remove_item(&kind, &id);
        let open = data
            .open
            .as_ref()
            .is_some_and(|open| open.kind == kind && open.id == id);
        if open {
            let next = data.scenarios().next().map(|scenario| scenario.id.clone());
            data.open = next.map(OpenAsset::scenario);
            rebuild_open_document(world);
        }
    });
    vec![]
}

/// The first face of `half` no socket stands on, or the front face.
fn free_face(half: Vec3, taken: &[Vec3]) -> (Vec3, Vec3) {
    FACES
        .iter()
        .map(|normal| (*normal * half, *normal))
        .find(|(position, _)| !taken.iter().any(|held| held.distance(*position) < 1.0e-3))
        .unwrap_or((Vec3::Z * half, Vec3::Z))
}

/// Add a socket to the section on the bench, selected, as one step.
pub(crate) fn add_socket(world: &mut World) {
    let Some(root) = world.resource::<EditContext>().scenario() else {
        return;
    };
    let Some(Content::Section(config)) = world.get::<AssetNode>(root).map(|node| &node.0) else {
        refuse(world, "only a section has sockets");
        return;
    };
    let half = config.base.collider.unwrap_or_default().aabb_half_extents();
    let held: Vec<(String, Vec3, u32)> = world
        .get::<Children>(root)
        .into_iter()
        .flatten()
        .filter_map(|child| {
            Some((
                world.get::<NodeId>(*child)?.0.clone(),
                world.get::<Transform>(*child)?.translation,
                world.get::<SocketNode>(*child)?.order,
            ))
        })
        .collect();
    let taken: Vec<Vec3> = held.iter().map(|(_, at, _)| *at).collect();
    let (position, normal) = free_face(half, &taken);
    let id = (1..)
        .map(|at| format!("socket_{at}"))
        .find(|id| !held.iter().any(|(held, ..)| held == id))
        .unwrap_or_default();
    let order = held.iter().map(|(.., order)| order + 1).max().unwrap_or(0);
    let socket = LinkPoint {
        id,
        position,
        normal,
        skin: LinkPointSkinType::Clad,
    };
    let node = spawn_socket(world, root, &socket, order);
    world.resource_mut::<SelectedNode>().set(Some(node));
    commit_edit_now(world, "Add Socket");
}

/// Make the open section's collider a `shape`, about as big as the one it
/// replaces, as one step: Add for a section with none, Replace for one with
/// one. A section holds at most one collider.
pub(crate) fn set_collider(world: &mut World, shape: &str) {
    let Some(root) = world.resource::<EditContext>().scenario() else {
        return;
    };
    let mut held = match world.get::<AssetNode>(root) {
        Some(node @ AssetNode(Content::Section(_))) => node.clone(),
        _ => {
            refuse(world, "open a section to set its collider");
            return;
        }
    };
    let had = matches!(&held.0, Content::Section(config) if config.base.collider.is_some());
    if let Err(fault) = set_item_text(&mut held.0, AssetTextType::ColliderShape, shape) {
        refuse(world, fault);
        return;
    }
    world.entity_mut(root).insert(held);
    commit_edit_now(
        world,
        if had {
            "Replace Collider"
        } else {
            "Add Collider"
        },
    );
}

/// What is wrong with `bytes` as a binary glTF, or nothing.
pub(crate) fn glb_fault(bytes: &[u8]) -> Option<&'static str> {
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .and_then(|held| <[u8; 4]>::try_from(held).ok())
            .map(u32::from_le_bytes)
    };
    if bytes.get(..4) != Some(b"glTF".as_slice()) {
        return Some("it is not a binary glTF file");
    }
    if word(4) != Some(2) {
        return Some("it is not glTF version 2");
    }
    if word(8).and_then(|length| usize::try_from(length).ok()) != Some(bytes.len()) {
        return Some("the file is cut short");
    }
    None
}

/// The file name the file at `path` takes in the mod, ending in `.extension`
/// (nothing when it is empty): free among `resources`, and not one `differs`
/// says holds other bytes. A saved manifest may name a file the resources have
/// forgotten, and a write to it is outside any undo.
pub(crate) fn resource_name(
    path: &str,
    extension: &str,
    resources: &[String],
    differs: impl Fn(&str) -> bool,
) -> Option<String> {
    let stem = std::path::Path::new(path).file_stem()?.to_str()?;
    let stem = slug(stem).replace('-', "_");
    if stem.is_empty() {
        return None;
    }
    let dot = if extension.is_empty() { "" } else { "." };
    (0..)
        .map(|at| match at {
            0 => format!("{stem}{dot}{extension}"),
            at => format!("{stem}_{at}{dot}{extension}"),
        })
        .find(|file| !resources.contains(file) && !differs(file))
}

/// Copy the `.glb` at `path` into the mod, declare it, and point the open
/// section's mesh at it, as one step. Any file that is not a whole glb stops
/// here, so a bad import never reaches the document.
pub(crate) fn import_mesh(world: &mut World, path: &str) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let Some(own) = project.id.clone() else {
        refuse(
            world,
            "save the mod first: a mesh is copied into its folder",
        );
        return;
    };
    let root = world.resource::<EditContext>().scenario();
    let bench = root.filter(|root| {
        world
            .get::<AssetNode>(*root)
            .is_some_and(|node| matches!(node.0, Content::Section(_)))
    });
    let Some(root) = bench else {
        refuse(world, "open a section to import a mesh");
        return;
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(fault) => {
            refuse(world, format!("{path} could not be read: {fault}"));
            return;
        }
    };
    if let Some(fault) = glb_fault(&bytes) {
        refuse(world, format!("{path}: {fault}"));
        return;
    }
    let resources = world.resource::<ModProject>().data.resources.clone();
    let differs = |file: &str| store::mod_file(&own, file).is_some_and(|held| held != bytes);
    let Some(file) = resource_name(path, "glb", &resources, differs) else {
        refuse(world, format!("{path} has no usable file name"));
        return;
    };
    let mesh = format!("{SELF_SCHEME}{file}#Scene0");
    let Some(mut held) = world.get::<AssetNode>(root).cloned() else {
        return;
    };
    if let Content::Section(config) = &mut held.0 {
        if let Err(fault) = set_mesh(&mut config.kind, &mesh) {
            refuse(world, fault);
            return;
        }
    }
    if let Err(fault) = store::write_mod_file(&own, &file, bytes) {
        refuse(world, format!("the mesh was not copied: {fault}"));
        return;
    }
    note(world, format!("imported {file}"));
    project_step(world, "Import Mesh", move |world| {
        world
            .resource_mut::<ModProject>()
            .edit()
            .resources
            .push(file);
        world.entity_mut(root).insert(held);
        absorb_document(world);
    });
}
