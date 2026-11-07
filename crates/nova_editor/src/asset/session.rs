//! Project sections in the running catalog, for as long as the project is open.
//!
//! A ship in a mod's scenario places the mod's own section, so the editor, the
//! preview and Play all have to find it in [`GameSections`], saved or not. A
//! remerge is no way to get it there: any merge refusal in Playing is fatal.
//! Sections are written straight into the catalog with their refs resolved, the
//! way Play registers a station design for the session, and what they stood over
//! comes back when the project closes.

use std::{collections::BTreeMap, sync::Arc};

use bevy::prelude::*;
use nova_assets::mod_refs::prelude::rewrite_refs;
use nova_modding::prelude::Content;
use nova_ship::prelude::{GameSections, SectionConfig};

use crate::{
    asset_index::AssetIndex,
    project::{ModProject, ProjectData},
};

/// One section the project holds in the catalog.
#[derive(Debug)]
struct Registered {
    /// What the catalog held under this id before, or `None`.
    prior: Option<SectionConfig>,
    /// What was written, to tell a merge's write from this one's.
    written: SectionConfig,
}

/// The project's sections in the catalog, by id.
#[derive(Resource, Debug, Default)]
pub(crate) struct ProjectSections {
    /// The data the catalog was last brought up to.
    seen: Option<Arc<ProjectData>>,
    held: BTreeMap<String, Registered>,
}

impl ProjectSections {
    /// Whether `id` is one of the open project's own.
    pub(crate) fn holds(&self, id: &str) -> bool {
        self.held.contains_key(id)
    }
}

/// Whether the catalog needs a look: the project moved, or a merge replaced the
/// catalog under it.
pub(crate) fn project_sections_moved(
    project: Option<Res<ModProject>>,
    sections: Option<Res<GameSections>>,
    held: Res<ProjectSections>,
) -> bool {
    let (Some(project), Some(sections)) = (project, sections) else {
        return false;
    };
    let same = held
        .seen
        .as_ref()
        .is_some_and(|seen| Arc::ptr_eq(seen, &project.data));
    !same || sections.is_changed()
}

/// The project's sections with their refs resolved, as the catalog holds them.
fn resolved_sections(index: AssetIndex, project: Res<ModProject>) -> Vec<SectionConfig> {
    index.scoped(|scope| {
        project
            .data
            .items()
            .filter(|item| matches!(item, Content::Section(_)))
            .filter_map(|item| match rewrite_refs(item, scope) {
                Content::Section(config) => Some(*config),
                _ => None,
            })
            .collect()
    })
}

/// Put `id` back as it stood before the project wrote over it.
fn put_back(sections: &mut GameSections, id: &str, prior: Option<SectionConfig>) {
    let at = sections.iter().position(|config| config.base.id == id);
    match (at, prior) {
        (Some(at), Some(prior)) => sections[at] = prior,
        (Some(at), None) => {
            sections.remove(at);
        }
        (None, Some(prior)) => sections.push(prior),
        (None, None) => {}
    }
}

/// Bring the catalog up to the project. Writes only what differs, so running it
/// again finds nothing to do.
pub(crate) fn sync_project_sections(world: &mut World) {
    if !world.contains_resource::<GameSections>() {
        return;
    }
    let Some(data) = world
        .get_resource::<ModProject>()
        .map(|project| Arc::clone(&project.data))
    else {
        return;
    };
    let wanted = match world.run_system_cached(resolved_sections) {
        Ok(wanted) => wanted,
        Err(fault) => {
            error!("editor: the mod's sections were not read: {fault}");
            return;
        }
    };
    world.init_resource::<ProjectSections>();
    world.resource_scope(|world, mut held: Mut<ProjectSections>| {
        held.seen = Some(data);
        let stale: Vec<String> = held
            .held
            .keys()
            .filter(|id| !wanted.iter().any(|config| config.base.id == **id))
            .cloned()
            .collect();
        for id in stale {
            if let Some(gone) = held.held.remove(&id) {
                put_back(&mut world.resource_mut::<GameSections>(), &id, gone.prior);
            }
        }
        for config in wanted {
            let id = config.base.id.clone();
            let live = world.resource::<GameSections>().get_section(&id).cloned();
            let prior = match held.held.get(&id) {
                Some(entry) if live.as_ref() == Some(&entry.written) => entry.prior.clone(),
                // A merge wrote here since: that is what stands under the project.
                _ => live.clone(),
            };
            if live.as_ref() != Some(&config) {
                let mut sections = world.resource_mut::<GameSections>();
                match sections.iter().position(|held| held.base.id == id) {
                    Some(at) => sections[at] = config.clone(),
                    None => sections.push(config.clone()),
                }
            }
            held.held.insert(
                id,
                Registered {
                    prior,
                    written: config,
                },
            );
        }
    });
}

/// Take the project's sections out of the catalog.
pub(crate) fn unregister_project_sections(world: &mut World) {
    let Some(mut held) = world.get_resource_mut::<ProjectSections>() else {
        return;
    };
    held.seen = None;
    let gone = std::mem::take(&mut held.held);
    let Some(mut sections) = world.get_resource_mut::<GameSections>() else {
        return;
    };
    for (id, entry) in gone {
        put_back(&mut sections, &id, entry.prior);
    }
}
