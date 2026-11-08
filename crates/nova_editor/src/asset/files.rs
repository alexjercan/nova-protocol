//! The files in a mod's folder beyond its content: what kind each is, what
//! content names it, and the verbs on them.
//!
//! The package holds the manifest, the content files and the files the
//! manifest declares (see [`store::is_declared`]). Any other file in the folder
//! is left out, so the Assets tab marks it. A declared file that is gone from
//! the folder is marked too: the package cannot hold it.
//!
//! The declaration is part of the mod, so declaring and undeclaring a file is
//! one undo step. A write to the folder is not: an undo puts a declaration
//! back, never the bytes of a file.

use std::path::Path;

use bevy::prelude::*;
use nova_assets::mod_refs::prelude::{string_leaves, SELF_SCHEME};

use super::verbs::{glb_fault, resource_name, shown};
use crate::project::{
    store,
    verbs::{lower_open_document, note, project_step, refuse},
    ModProject, ProjectData,
};

/// A picture.
pub(crate) const IMAGE_KIND: &str = "image";
/// A model.
pub(crate) const MESH_KIND: &str = "mesh";
/// A sound.
pub(crate) const AUDIO_KIND: &str = "audio";
/// Any other file.
pub(crate) const FILE_KIND: &str = "file";

/// The file kinds and their headings, in the order the grid lists them.
pub(crate) const FILE_KINDS: [(&str, &str); 4] = [
    (IMAGE_KIND, "Images"),
    (MESH_KIND, "Meshes"),
    (AUDIO_KIND, "Audio"),
    (FILE_KIND, "Files"),
];

/// The image extensions this build has a loader for. A jpeg lists as an image
/// and shows its glyph: the game does not read jpeg.
const LOADED_IMAGES: [&str; 3] = ["png", "ktx2", "webp"];

/// The sound extensions this build has a loader for.
const LOADED_SOUNDS: [&str; 2] = ["ogg", "wav"];

/// Bumped by a verb that writes the folder and not the mod, so the Assets tab
/// reads the folder again.
#[derive(Resource, Default)]
pub(crate) struct ModFolder(pub(crate) u64);

/// One file of the mod's folder, or one the manifest declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModFile {
    /// The path, relative to the mod's folder.
    pub(crate) path: String,
    /// Whether the package holds it: the manifest declares it.
    pub(crate) packaged: bool,
    /// Whether it is in the folder.
    pub(crate) on_disk: bool,
}

/// The lowercase extension of `path`, or nothing.
fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|found| found.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

/// The kind `path` lists under, by its extension.
pub(crate) fn file_kind(path: &str) -> &'static str {
    match extension(path).as_str() {
        "png" | "jpg" | "jpeg" | "ktx2" | "webp" => IMAGE_KIND,
        "glb" | "gltf" => MESH_KIND,
        "ogg" | "wav" | "flac" | "mp3" => AUDIO_KIND,
        _ => FILE_KIND,
    }
}

/// Whether this build can load `path` as an image.
pub(crate) fn loads_as_image(path: &str) -> bool {
    LOADED_IMAGES.contains(&extension(path).as_str())
}

/// Whether this build can play `path`.
pub(crate) fn plays(path: &str) -> bool {
    LOADED_SOUNDS.contains(&extension(path).as_str())
}

/// The files of the mod: the folder's own (`on_disk`), less the manifest and
/// the content files, and every declared file the folder lacks. Sorted by kind
/// in [`FILE_KINDS`] order, then by path.
pub(crate) fn mod_files(project: &ModProject, on_disk: &[String]) -> Vec<ModFile> {
    let data = &project.data;
    let content =
        |path: &str| path == project.bundle || data.files.iter().any(|file| file.path == path);
    let mut files: Vec<ModFile> = on_disk
        .iter()
        .filter(|path| !content(path))
        .map(|path| ModFile {
            path: path.clone(),
            packaged: store::is_declared(data, path),
            on_disk: true,
        })
        .collect();
    let declared = data
        .resources
        .iter()
        .chain(data.meta.icon.iter())
        .chain(data.meta.screenshots.iter());
    for path in declared {
        if !on_disk.contains(path) && !files.iter().any(|file| file.path == *path) {
            files.push(ModFile {
                path: path.clone(),
                packaged: true,
                on_disk: false,
            });
        }
    }
    let rank = |path: &str| {
        FILE_KINDS
            .iter()
            .position(|(kind, _)| *kind == file_kind(path))
    };
    files.sort_by(|a, b| (rank(&a.path), &a.path).cmp(&(rank(&b.path), &b.path)));
    files
}

/// The items of `data` that name the file `file`, by id.
pub(crate) fn file_users(data: &ProjectData, file: &str) -> Vec<String> {
    let whole = format!("{SELF_SCHEME}{file}");
    let labelled = format!("{whole}#");
    data.items()
        .filter(|item| {
            string_leaves(item)
                .iter()
                .any(|leaf| *leaf == whole || leaf.starts_with(&labelled))
        })
        .map(|item| item.id().to_string())
        .collect()
}

/// The saved mod's id, or a refusal that says why `what` needs one.
fn saved_id(world: &mut World, what: &str) -> Option<String> {
    let id = world.get_resource::<ModProject>()?.id.clone();
    if id.is_none() {
        refuse(world, format!("save the mod first: {what}"));
    }
    id
}

/// What names `file`, refused for `verb` when anything does.
fn refused_in_use(world: &mut World, file: &str, verb: &str) -> bool {
    lower_open_document(world);
    let users = file_users(&world.resource::<ModProject>().data, file);
    if users.is_empty() {
        return false;
    }
    refuse(
        world,
        format!("{file} is in use by {}: it does not {verb}", shown(&users)),
    );
    true
}

/// Copy the file at `path` into the mod's folder and declare it, as one step. A
/// glb that is not whole stops here.
pub(crate) fn import_file(world: &mut World, path: &str) {
    let Some(own) = saved_id(world, "a file is copied into its folder") else {
        return;
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(fault) => {
            refuse(world, format!("{path} could not be read: {fault}"));
            return;
        }
    };
    let kind = extension(path);
    if kind == "glb" {
        if let Some(fault) = glb_fault(&bytes) {
            refuse(world, format!("{path}: {fault}"));
            return;
        }
    }
    let resources = world.resource::<ModProject>().data.resources.clone();
    let differs = |file: &str| store::mod_file(&own, file).is_some_and(|held| held != bytes);
    let Some(file) = resource_name(path, &kind, &resources, differs) else {
        refuse(world, format!("{path} has no usable file name"));
        return;
    };
    if let Err(fault) = store::write_mod_file(&own, &file, bytes) {
        refuse(world, format!("the file was not copied: {fault}"));
        return;
    }
    note(world, format!("imported {file}"));
    project_step(world, "Import File", move |world| {
        world
            .resource_mut::<ModProject>()
            .edit()
            .resources
            .push(file);
    });
}

/// Declare `file`, so the package holds it, as one step.
pub(crate) fn include_file(world: &mut World, file: &str) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    if store::is_declared(&project.data, file) {
        return;
    }
    let file = file.to_string();
    project_step(world, "Include File", move |world| {
        world
            .resource_mut::<ModProject>()
            .edit()
            .resources
            .push(file);
    });
}

/// Delete `file`, unless content names it. A declared file leaves the package
/// as one step and stays in the folder; a file the package leaves out is
/// removed from the folder, which no undo puts back.
pub(crate) fn delete_file(world: &mut World, file: &str) {
    if refused_in_use(world, file, "delete") {
        return;
    }
    let data = &world.resource::<ModProject>().data;
    let resource = data.resources.iter().any(|held| held == file);
    let declared = store::is_declared(data, file);
    if resource {
        let file = file.to_string();
        note(
            world,
            format!("{file} left the package and stays in the folder"),
        );
        project_step(world, "Remove File", move |world| {
            world
                .resource_mut::<ModProject>()
                .edit()
                .resources
                .retain(|held| *held != file);
        });
        return;
    }
    if declared {
        refuse(
            world,
            format!("{file} is the mod's icon or a screenshot: change that in Mod Settings"),
        );
        return;
    }
    let Some(own) = saved_id(world, "an unsaved mod has no folder") else {
        return;
    };
    match store::remove_mod_file(&own, file) {
        Ok(()) => {
            note(
                world,
                format!("deleted {file} from the folder: this does not undo"),
            );
            world.resource_mut::<ModFolder>().0 += 1;
        }
        Err(fault) => refuse(world, format!("{file} was not deleted: {fault}")),
    }
}

/// The path `typed` asks for in place of `file`: the folder `file` stands in,
/// and `file`'s extension when `typed` gives none.
fn typed_file(file: &str, typed: &str) -> Option<String> {
    let typed = typed.trim();
    if typed.is_empty() || typed.contains(['/', '\\']) {
        return None;
    }
    let named = if Path::new(typed).extension().is_some() {
        typed.to_string()
    } else {
        match extension(file).as_str() {
            "" => typed.to_string(),
            found => format!("{typed}.{found}"),
        }
    };
    Some(match file.rsplit_once('/') {
        Some((folder, _)) => format!("{folder}/{named}"),
        None => named,
    })
}

/// Rename `file` in the folder to `typed`, unless content names it. A declared
/// file keeps its declaration under the new name, as one step.
pub(crate) fn rename_file(world: &mut World, file: &str, typed: &str) {
    let Some(new) = typed_file(file, typed) else {
        refuse(world, "a file name needs a letter and no slash");
        return;
    };
    if new == file {
        return;
    }
    if refused_in_use(world, file, "rename") {
        return;
    }
    let Some(own) = saved_id(world, "an unsaved mod has no folder") else {
        return;
    };
    if store::mod_file(&own, &new).is_some() {
        refuse(world, format!("{new} is already in the folder"));
        return;
    }
    let Some(bytes) = store::mod_file(&own, file) else {
        refuse(world, format!("{file} is not in the folder"));
        return;
    };
    if let Err(fault) = store::write_mod_file(&own, &new, bytes) {
        refuse(world, format!("{file} was not renamed: {fault}"));
        return;
    }
    if let Err(fault) = store::remove_mod_file(&own, file) {
        refuse(world, format!("{new} is a copy: {file} stays: {fault}"));
    }
    note(world, format!("renamed {file} to {new}"));
    let declared = world
        .resource::<ModProject>()
        .data
        .resources
        .iter()
        .any(|held| held == file);
    if !declared {
        world.resource_mut::<ModFolder>().0 += 1;
        return;
    }
    let from = file.to_string();
    project_step(world, "Rename File", move |world| {
        for held in &mut world.resource_mut::<ModProject>().edit().resources {
            if *held == from {
                held.clone_from(&new);
            }
        }
    });
}

/// Where `file` stands, on the status line.
pub(crate) fn reveal_file(world: &mut World, file: &str) {
    let id = world
        .get_resource::<ModProject>()
        .and_then(|project| project.id.clone());
    let path = id.as_deref().and_then(|id| store::mod_file_path(id, file));
    match path {
        Some(path) => note(world, path),
        None => note(world, format!("{file}: the mod is not saved to a folder")),
    }
}
