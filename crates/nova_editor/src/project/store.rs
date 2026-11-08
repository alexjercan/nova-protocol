//! A mod project on disk: reading one, writing one, copying one, and the list
//! the Mod Hub shows.
//!
//! The web has no synchronous store to write into, so every write here refuses
//! there with a line fit to show the builder, and the hub says so up front.
#![cfg_attr(
    target_arch = "wasm32",
    expect(dead_code, reason = "the web has no store, so nothing reads one")
)]

use std::{collections::HashMap, sync::Arc};

use bevy::prelude::*;
use nova_assets::mod_refs::prelude::{map_prototype_refs, map_strings_and_keys};
use nova_modding::prelude::{parse_content, serialize_content, BundleManifest, Content, ModMeta};
#[cfg(not(target_arch = "wasm32"))]
use nova_modding::prelude::{parse_manifest, serialize_manifest};

use super::{
    ids::{is_taken, namespace_of, project_id, scenario_id, slug, NameProblem, EDITOR_ID_PREFIX},
    ContentFile, ModProject, OpenAsset, ProjectData,
};

/// The key the hub's opened-at times are remembered under.
#[cfg(not(target_arch = "wasm32"))]
const OPENED_KEY: &str = "editor_opened_projects";

/// One local mod, as the hub lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HubEntry {
    /// The local id.
    pub(crate) id: String,
    /// What the manifest calls it, or the id when it cannot be read.
    pub(crate) name: String,
    /// The manifest's version.
    pub(crate) version: String,
    /// How many items of each kind, in file order.
    pub(crate) counts: Vec<(&'static str, usize)>,
    /// Seconds since the epoch the mod was last opened or written, if the
    /// store can say.
    pub(crate) opened: Option<u64>,
    /// Why it cannot be opened. Listed anyway: a row that hid a broken mod
    /// would leave a directory nobody could see or delete.
    pub(crate) fault: Option<String>,
}

/// The bundle manifest of a mod first written by the editor.
pub(crate) fn bundle_file(id: &str) -> String {
    format!("{id}.bundle.ron")
}

/// The one content file a mod first written by the editor lists.
pub(crate) fn content_file(id: &str) -> String {
    format!("{id}.content.ron")
}

/// The manifest `data` writes, listing its content files.
pub(crate) fn manifest_of(data: &ProjectData) -> BundleManifest {
    BundleManifest {
        content: data.files.iter().map(|file| file.path.clone()).collect(),
        resources: data.resources.clone(),
        meta: data.meta.clone(),
        new_game_scenario: data.new_game_scenario.clone(),
    }
}

/// The bytes of one content file.
pub(crate) fn file_bytes(file: &ContentFile) -> Result<Vec<u8>, String> {
    let items: Vec<Content> = file.items.iter().map(|item| (**item).clone()).collect();
    serialize_content(&items)
        .map(String::into_bytes)
        .map_err(|error| format!("{}: {error}", file.path))
}

/// What a manifest and its content files read back into.
pub(crate) fn project_of(
    manifest: BundleManifest,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<ProjectData, String> {
    let mut files = Vec::with_capacity(manifest.content.len());
    for path in manifest.content {
        // Refused rather than skipped: a save after a skip would write the
        // manifest without the file, and the items in it would be gone.
        let bytes = read(&path).ok_or_else(|| format!("{path} is missing"))?;
        let items = parse_content(&bytes).map_err(|error| format!("{path}: {error}"))?;
        files.push(ContentFile {
            path,
            items: items.into_iter().map(Arc::new).collect(),
        });
    }
    let mut data = ProjectData {
        meta: manifest.meta,
        resources: manifest.resources,
        new_game_scenario: manifest.new_game_scenario,
        files,
        open: None,
    };
    let first = data.scenarios().next().map(|scenario| scenario.id.clone());
    data.open = first.map(OpenAsset::scenario);
    Ok(data)
}

/// Whether `id`'s files are the one-scenario layout saves used to write: the
/// scenario under the mod's own id, in the one content file, its designs bare.
pub(crate) fn is_old_save(id: &str, data: &ProjectData) -> bool {
    id.starts_with(EDITOR_ID_PREFIX)
        && data.files.len() == 1
        && data.files[0].path == content_file(id)
        && data.scenario(id).is_some()
}

/// The ids of every mod id `of`'s content or manifest would rewrite in a copy:
/// each scenario to one under `namespace`, the designs it owns with it, and each
/// other item minted under `of`'s namespace, which both mods could not define.
fn copy_renames(data: &ProjectData, of: &str, namespace: &str) -> HashMap<String, String> {
    let old_prefix = format!("{}__", namespace_of(of).replace('-', "_"));
    let mut taken = std::collections::HashSet::new();
    let mut renames = HashMap::new();
    for scenario in data.scenarios() {
        let tail = scenario
            .id
            .strip_prefix(&old_prefix)
            .map_or_else(|| slug(&scenario.id), str::to_string);
        let id = scenario_id(namespace, &tail, &taken);
        taken.insert(id.clone());
        renames.insert(scenario.id.clone(), id);
    }
    let new_prefix = format!("{}__", namespace.replace('-', "_"));
    for item in data.items() {
        if let Some(tail) = item.id().strip_prefix(&old_prefix) {
            if !matches!(
                item,
                Content::Scenario(_) | Content::Ship(_) | Content::Station(_)
            ) {
                renames.insert(item.id().to_string(), format!("{new_prefix}{tail}"));
            }
        }
    }
    renames
}

/// The designs of an old save that still stand bare, each with the id a copy
/// gives it. Apart from [`copy_renames`] because the old layout gave a spawn and
/// its design one id: a whole-leaf rewrite would move the spawn and everything
/// the script calls it, too.
fn bare_design_renames(
    data: &ProjectData,
    of: &str,
    renames: &HashMap<String, String>,
) -> HashMap<String, String> {
    let Some(to) = renames.get(of).filter(|_| is_old_save(of, data)) else {
        return HashMap::new();
    };
    let owned = format!("{of}.");
    data.items()
        .filter(|item| matches!(item, Content::Ship(_) | Content::Station(_)))
        .filter(|item| !item.id().starts_with(&owned))
        .map(|item| (item.id().to_string(), format!("{to}.{}", item.id())))
        .collect()
}

/// `item` with the bare designs `bare` names moved: its own id, and each design
/// it spawns.
fn with_bare_designs_moved(
    item: &Content,
    bare: &HashMap<String, String>,
) -> Result<Content, String> {
    let mut item = map_prototype_refs(item, &|id| bare.get(id).cloned())?;
    match &mut item {
        Content::Ship(ship) => {
            if let Some(to) = bare.get(ship.id.as_str()) {
                ship.id = to.as_str().into();
            }
        }
        Content::Station(station) => {
            if let Some(to) = bare.get(station.id.as_str()) {
                station.id = to.as_str().into();
            }
        }
        _ => {}
    }
    Ok(item)
}

/// Whether the manifest of `data` names the file `path` beyond its content, or
/// `path` is the sidecar of one it names. What else a folder holds is no part of
/// the mod: an import undone leaves its file behind.
pub(crate) fn is_declared(data: &ProjectData, path: &str) -> bool {
    let named = path.strip_suffix(".meta").unwrap_or(path);
    data.resources.iter().any(|held| held == named)
        || data.meta.icon.as_deref() == Some(named)
        || data.meta.screenshots.iter().any(|held| held == named)
}

/// `leaf` as a copy writes it: an exact id, or a design under a renamed scenario.
fn renamed(leaf: &str, renames: &HashMap<String, String>) -> Option<String> {
    if let Some(to) = renames.get(leaf) {
        return Some(to.clone());
    }
    let (scenario, node) = renames
        .keys()
        .find_map(|from| Some((from, leaf.strip_prefix(from)?.strip_prefix('.')?)))?;
    Some(format!("{}.{node}", renames[scenario]))
}

/// `data` under new content ids, for a mod with its own namespace.
pub(crate) fn renamed_copy(
    data: &ProjectData,
    of: &str,
    namespace: &str,
    name: &str,
) -> Result<ProjectData, String> {
    let renames = copy_renames(data, of, namespace);
    let bare = bare_design_renames(data, of, &renames);
    let mut copy = data.clone();
    copy.meta.name = name.to_string();
    copy.open = copy.open.as_ref().map(|open| OpenAsset {
        kind: open.kind,
        id: renames.get(&open.id).unwrap_or(&open.id).clone(),
    });
    for file in &mut copy.files {
        for item in &mut file.items {
            let moved = map_strings_and_keys(item, &|leaf| renamed(leaf, &renames))?;
            *item = Arc::new(with_bare_designs_moved(&moved, &bare)?);
        }
    }
    Ok(copy)
}

/// The id a copy named `name` takes, or the line that says why it cannot.
pub(crate) fn copy_id(name: &str) -> Result<String, String> {
    let id = project_id(name).map_err(|problem| match problem {
        NameProblem::Unusable => "a name needs a letter or a digit".to_string(),
        NameProblem::Reserved => "that name is the game's own - pick another".to_string(),
    })?;
    if is_taken(&id, &local_ids()) {
        return Err(format!("{id} already exists"));
    }
    Ok(id)
}

/// The metadata a new mod opens with.
pub(crate) fn new_meta(name: &str, author: &str) -> ModMeta {
    ModMeta {
        name: name.to_string(),
        description: String::new(),
        author: author.to_string(),
        version: "0.1.0".to_string(),
        dependencies: vec![],
        icon: None,
        screenshots: vec![],
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::time::{SystemTime, UNIX_EPOCH};

    use nova_assets::{
        mod_cache::prelude::{
            install_local, list_mod_files, mod_file_modified, read_index, read_mod_file,
            remove_index_record, remove_mod, remove_mod_files, store_mod_files,
        },
        persist,
    };

    use super::*;

    /// Whether this build can write mods at all.
    pub(crate) const CAN_SAVE: bool = true;

    /// Every local mod id on disk.
    pub(crate) fn local_ids() -> Vec<String> {
        read_index()
            .unwrap_or_default()
            .into_iter()
            .map(|record| record.id)
            .filter(|id| id.starts_with(EDITOR_ID_PREFIX))
            .collect()
    }

    /// Read `id` into a project, or say why it cannot be opened.
    pub(crate) fn read_project(id: &str) -> Result<ModProject, String> {
        let record = read_index()
            .unwrap_or_default()
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| format!("{id} is not installed"))?;
        let bytes = read_mod_file(id, &record.bundle)
            .ok_or_else(|| format!("{} is missing", record.bundle))?;
        let manifest = parse_manifest(&bytes).map_err(|error| error.to_string())?;
        let data = project_of(manifest, |path| read_mod_file(id, path))?;
        let migrate = is_old_save(id, &data).then(|| id.to_string());
        let data = Arc::new(data);
        Ok(ModProject {
            id: Some(id.to_string()),
            namespace: namespace_of(id).to_string(),
            bundle: record.bundle,
            on_disk: Some(Arc::clone(&data)),
            data,
            migrate,
            seed: None,
        })
    }

    /// Write `project`: every content file that changed, the manifest, and the
    /// index record. A file no edit touched keeps its bytes, comments and all.
    ///
    /// Content, then manifest, then index, so a failed write never leaves a
    /// manifest naming a file that is not there.
    pub(crate) fn write_project(project: &ModProject) -> Result<(), String> {
        let id = project
            .id
            .as_deref()
            .ok_or_else(|| "this mod has no id yet".to_string())?;
        let manifest = serialize_manifest(&manifest_of(&project.data))
            .map_err(|error| format!("manifest: {error}"))?;
        let mut files = Vec::new();
        for file in &project.data.files {
            if !project.file_is_written(file) {
                files.push((file.path.clone(), file_bytes(file)?));
            }
        }
        files.push((project.bundle.clone(), manifest.into_bytes()));
        install_local(id, &project.data.meta.version, &project.bundle, &files)
            .map_err(|error| error.to_string())
    }

    /// Write `data` as the new mod `id`: its content under a manifest of its own,
    /// and the files its manifest names from `from`, so resources survive a copy.
    pub(crate) fn write_copy(id: &str, data: &ProjectData, from: &str) -> Result<(), String> {
        let bundle = bundle_file(id);
        let manifest =
            serialize_manifest(&manifest_of(data)).map_err(|error| format!("manifest: {error}"))?;
        let mut files = Vec::new();
        for file in &data.files {
            files.push((file.path.clone(), file_bytes(file)?));
        }
        for path in list_mod_files(from) {
            if !is_declared(data, &path) || files.iter().any(|(held, _)| *held == path) {
                continue;
            }
            let bytes =
                read_mod_file(from, &path).ok_or_else(|| format!("{path} cannot be read"))?;
            files.push((path, bytes));
        }
        files.push((bundle.clone(), manifest.into_bytes()));
        install_local(id, &data.meta.version, &bundle, &files).map_err(|error| error.to_string())
    }

    /// Copy one binary file into the mod `id`.
    pub(crate) fn write_mod_file(id: &str, file: &str, bytes: Vec<u8>) -> Result<(), String> {
        store_mod_files(id, &[(file.to_string(), bytes)]).map_err(|error| error.to_string())
    }

    /// The binary file `file` of mod `id`.
    pub(crate) fn mod_file(id: &str, file: &str) -> Option<Vec<u8>> {
        read_mod_file(id, file)
    }

    /// Copy the mod on disk `from` under a new name.
    pub(crate) fn duplicate_mod(from: &str, name: &str) -> Result<String, String> {
        let id = copy_id(name)?;
        let source = read_project(from)?;
        let copy = renamed_copy(&source.data, from, namespace_of(&id), name)?;
        write_copy(&id, &copy, from)?;
        Ok(id)
    }

    /// Delete a local mod: its files, then its index record. Refuses any id the
    /// editor does not own.
    pub(crate) fn delete_mod(id: &str) -> Result<(), String> {
        if !id.starts_with(EDITOR_ID_PREFIX) {
            return Err(format!("{id} is not a mod the editor owns"));
        }
        remove_mod(id).map_err(|error| error.to_string())?;
        remove_index_record(id);
        Ok(())
    }

    /// The files mod `id` holds on disk.
    pub(crate) fn files_of(id: &str) -> Vec<String> {
        list_mod_files(id)
    }

    /// Remove the file `file` from mod `id`'s folder.
    pub(crate) fn remove_mod_file(id: &str, file: &str) -> Result<(), String> {
        remove_mod_files(id, &[file.to_string()]).map_err(|error| error.to_string())
    }

    /// Where the file `file` of mod `id` stands on disk.
    pub(crate) fn mod_file_path(id: &str, file: &str) -> Option<String> {
        nova_assets::mod_cache::prelude::mod_file_path(id, file)
            .map(|path| path.display().to_string())
    }

    /// What every installed mod declares, by id.
    pub(crate) fn installed_deps() -> Vec<(String, Vec<String>)> {
        read_index()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|record| {
                let bytes = read_mod_file(&record.id, &record.bundle)?;
                let manifest = parse_manifest(&bytes).ok()?;
                Some((record.id, manifest.meta.dependencies))
            })
            .collect()
    }

    /// When each mod was last opened.
    fn opened_times() -> HashMap<String, u64> {
        persist::load(OPENED_KEY).unwrap_or_default()
    }

    /// Every local mod, most recent first.
    pub(crate) fn hub_entries() -> Vec<HubEntry> {
        let opened_times = opened_times();
        let mut entries: Vec<HubEntry> = read_index()
            .unwrap_or_default()
            .into_iter()
            .filter(|record| record.id.starts_with(EDITOR_ID_PREFIX))
            .map(|record| {
                let written = mod_file_modified(&record.id, &record.bundle)
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|elapsed| elapsed.as_secs());
                let opened = written.max(opened_times.get(&record.id).copied());
                match read_project(&record.id) {
                    Ok(project) => HubEntry {
                        name: project.data.meta.name.clone(),
                        version: project.data.meta.version.clone(),
                        counts: project.data.kind_counts(),
                        id: record.id,
                        opened,
                        fault: None,
                    },
                    Err(fault) => HubEntry {
                        name: record.id.clone(),
                        version: record.version,
                        counts: vec![],
                        id: record.id,
                        opened,
                        fault: Some(fault),
                    },
                }
            })
            .collect();
        entries.sort_by(|a, b| b.opened.cmp(&a.opened).then_with(|| a.name.cmp(&b.name)));
        entries
    }

    /// Remember that `id` was opened now.
    pub(crate) fn remember_project(id: &str) {
        let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return;
        };
        let mut opened = opened_times();
        opened.insert(id.to_string(), now.as_secs());
        persist::save(OPENED_KEY, &opened);
    }
}

/// Refusals in place of the native store: the web's is asynchronous and the
/// editor's saves are not.
#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;

    const REFUSED: &str = "saving is not available on the web yet";

    /// Whether this build can write mods at all.
    pub(crate) const CAN_SAVE: bool = false;

    pub(crate) fn local_ids() -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn read_project(_id: &str) -> Result<ModProject, String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn write_project(_project: &ModProject) -> Result<(), String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn write_copy(_id: &str, _data: &ProjectData, _from: &str) -> Result<(), String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn write_mod_file(_id: &str, _file: &str, _bytes: Vec<u8>) -> Result<(), String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn mod_file(_id: &str, _file: &str) -> Option<Vec<u8>> {
        None
    }

    pub(crate) fn duplicate_mod(_from: &str, _name: &str) -> Result<String, String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn delete_mod(_id: &str) -> Result<(), String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn remove_mod_file(_id: &str, _file: &str) -> Result<(), String> {
        Err(REFUSED.to_string())
    }

    pub(crate) fn mod_file_path(_id: &str, _file: &str) -> Option<String> {
        None
    }

    pub(crate) fn files_of(_id: &str) -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn installed_deps() -> Vec<(String, Vec<String>)> {
        Vec::new()
    }

    pub(crate) fn hub_entries() -> Vec<HubEntry> {
        Vec::new()
    }

    pub(crate) fn remember_project(_id: &str) {}
}
