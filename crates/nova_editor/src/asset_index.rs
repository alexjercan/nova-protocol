//! What FILES a row naming an asset can offer, and which of them resolve.
//!
//! The set is not a directory listing. A bundle DECLARES the files it ships in
//! its `resources` list, and a content ref reaches them by the `dep://<id>/`
//! scheme, so the declared lists across the installed bundles are the exact set
//! a saved range may name - the same set the merge would rewrite against. That
//! is also why this needs no filesystem: the web build reads it too.

/// Glob-import surface for the asset picker's source.
pub(crate) mod prelude {
    pub(crate) use super::{AssetIndex, AssetSort};
}

use std::collections::{HashMap, HashSet};

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_assets::{
    mod_cache::prelude::MODS_SOURCE,
    mod_refs::prelude::{rewrite_refs, DepRef, RefScope, SELF_SCHEME},
    prelude::{enabled_bundles, DownloadedMods, EnabledMods, OptionalBundles},
};
use nova_modding::prelude::{BundleAsset, Content, ContentAsset, InstalledCatalog, BASE_MOD_ID};
use nova_scenario::prelude::ScenarioConfig;

use crate::project::ModProject;

/// The scheme a picked file is written under. `dep://<id>/<file>` names another
/// bundle's declared resource, which is what a range built in the editor does
/// with everything the base game ships.
const DEP: &str = "dep://";

/// What kind of file a row wants, taken from the type of the field itself
/// rather than from its name: `AssetRef<Image>` wants an image whatever the
/// field is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetSort {
    /// A texture or an icon.
    Image,
    /// A sound.
    Audio,
    /// A model, offered with the scene label a mesh ref needs.
    Model,
}

impl AssetSort {
    /// The extensions a bundle ships this sort under.
    fn extensions(self) -> &'static [&'static str] {
        match self {
            AssetSort::Image => &["png", "jpg", "jpeg", "ktx2", "basis"],
            AssetSort::Audio => &["ogg", "wav", "flac", "mp3"],
            AssetSort::Model => &["glb", "gltf"],
        }
    }

    /// What a picked file of this sort is written as.
    ///
    /// A model ref addresses a SCENE inside the file, never the file, so the
    /// picker writes the label a mesh field needs rather than leaving a builder
    /// to remember it.
    fn spelling(self, path: &str) -> String {
        match self {
            AssetSort::Model => format!("{path}#Scene0"),
            _ => path.to_string(),
        }
    }

    /// What the picker window is titled.
    pub(crate) fn holds(self) -> &'static str {
        match self {
            AssetSort::Image => "images",
            AssetSort::Audio => "sounds",
            AssetSort::Model => "models",
        }
    }
}

/// The declared resources of every ENABLED bundle, read the way
/// `nova_assets::register_bundles` reads them so the picker offers what the
/// merge would accept.
#[derive(SystemParam)]
pub(crate) struct AssetIndex<'w> {
    enabled: Option<Res<'w, EnabledMods>>,
    downloaded: Option<Res<'w, DownloadedMods>>,
    optional: Option<Res<'w, OptionalBundles>>,
    catalogs: Option<Res<'w, Assets<InstalledCatalog>>>,
    bundles: Option<Res<'w, Assets<BundleAsset>>>,
    contents: Option<Res<'w, Assets<ContentAsset>>>,
    project: Option<Res<'w, ModProject>>,
}

/// One loaded mod a project can see, as the merge would read it.
pub(crate) struct VisiblePack<'a> {
    /// Its id.
    pub(crate) id: &'a str,
    /// What it declares.
    pub(crate) dependencies: &'a [String],
    /// Its content, in load order.
    pub(crate) items: Vec<&'a Content>,
}

impl AssetIndex<'_> {
    /// Every file of `sort` the installed bundles declare, as the ref a
    /// document writes: `dep://<bundle>/<file>`, sorted so the list does not
    /// reshuffle between openings.
    pub(crate) fn offers(&self, sort: AssetSort) -> Vec<String> {
        let mut offered: Vec<String> = self
            .declared()
            .filter(|(_, file)| has_extension(file, sort.extensions()))
            .map(|(id, file)| sort.spelling(&format!("{DEP}{id}/{file}")))
            .collect();
        offered.extend(self.project.iter().flat_map(|project| {
            project
                .data
                .resources
                .iter()
                .filter(|file| has_extension(file, sort.extensions()))
                .map(|file| sort.spelling(&format!("{SELF_SCHEME}{file}")))
        }));
        offered.sort_unstable();
        offered.dedup();
        offered
    }

    /// Whether `text` names a file the installed bundles ship.
    ///
    /// A `dep://` ref is judged against the bundles, and a `self://` ref against
    /// the open mod. With no mod open a `self://` ref belongs to whichever bundle
    /// owns the config, which the panel does not know, and a bare path addresses
    /// the asset root, which no bundle declares. Both of those are left alone
    /// rather than marked wrong on a guess.
    pub(crate) fn resolves(&self, text: &str) -> bool {
        if let (Some(rest), Some(project)) =
            (text.strip_prefix(SELF_SCHEME), self.project.as_deref())
        {
            let file = rest.split('#').next().unwrap_or(rest);
            return project.data.resources.iter().any(|held| held == file);
        }
        let Some(rest) = text.strip_prefix(DEP) else {
            return true;
        };
        let Some((id, file)) = rest.split_once('/') else {
            return false;
        };
        let file = file.split('#').next().unwrap_or(file);
        self.declared()
            .any(|(bundle, declared)| bundle == id && declared == file)
    }

    /// The same rewrite the mod merge performs, for a range that never goes
    /// through it.
    ///
    /// A picked file is written as `dep://<bundle>/<file>`, which is the ref a
    /// SAVED range needs: the merge resolves it against the bundle that ships
    /// the file. Play is built at runtime and merged with nothing, so the same
    /// ref reaches the asset server verbatim and fails as an unknown source -
    /// which is a scenario that comes up with no sky. Resolving here keeps ONE
    /// spelling in the document and lets Play show what a save would.
    pub(crate) fn resolved(&self, scenario: ScenarioConfig) -> ScenarioConfig {
        let content = Content::Scenario(scenario);
        match self.scoped(|scope| rewrite_refs(&content, scope)) {
            Content::Scenario(resolved) => resolved,
            _ => unreachable!("a rewrite keeps the content's kind"),
        }
    }

    /// Read the ref scope the merge gives the open project.
    ///
    /// The project's own dependencies, `base`, and its own folder: the set the
    /// merge gate holds a saved mod to, so what passes here passes there. A rig
    /// with no project may name everything loaded, and it and a mod with no id
    /// own no folder: their `self://` refs are left literal and fail loudly.
    pub(crate) fn scoped<R>(&self, read: impl FnOnce(&RefScope<'_>) -> R) -> R {
        let project = self.project.as_deref();
        let bundles: Vec<(&str, &BundleAsset)> = match project {
            Some(_) => self.installed().collect(),
            None => self.bundles().collect(),
        };
        let declared_deps: HashSet<String> = match project {
            Some(project) => project.data.meta.dependencies.iter().cloned().collect(),
            None => bundles.iter().map(|(id, _)| (*id).to_string()).collect(),
        };
        let deps: HashMap<String, DepRef<'_>> = bundles
            .iter()
            .filter(|(id, _)| *id == BASE_MOD_ID || declared_deps.contains(*id))
            .map(|(id, bundle)| {
                (
                    (*id).to_string(),
                    DepRef {
                        base: Some(bundle.resource_base.as_str()),
                        resources: Some(bundle.resources.as_slice()),
                    },
                )
            })
            .collect();
        let folder = project.filter(|project| project.id.is_some());
        let self_base = folder
            .and_then(|project| project.id.as_deref())
            .map_or_else(String::new, |id| format!("{MODS_SOURCE}://{id}"));
        let scope = RefScope {
            self_base: &self_base,
            self_resources: folder.map_or(&[][..], |project| project.data.resources.as_slice()),
            declared_deps: &declared_deps,
            deps: &deps,
        };
        read(&scope)
    }

    /// Whether a loaded bundle has this id, enabled or not.
    pub(crate) fn has_bundle(&self, id: &str) -> bool {
        self.installed().any(|(held, _)| held == id)
    }

    /// Whether mod `id` is on in the enabled set.
    pub(crate) fn is_enabled(&self, id: &str) -> bool {
        self.bundles().any(|(held, _)| held == id)
    }

    /// Whether the shipped catalog lists `id`, enabled or not.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "only Package reads this, and the web has no Package"
        )
    )]
    pub(crate) fn is_shipped(&self, id: &str) -> bool {
        self.catalog()
            .is_some_and(|catalog| catalog.entries.iter().any(|entry| entry.decl.id == id))
    }

    /// The id of every mod the game ships.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "only Import reads this, and the web has no Import"
        )
    )]
    pub(crate) fn shipped_ids(&self) -> Vec<String> {
        self.catalog().map_or_else(Vec::new, |catalog| {
            catalog
                .entries
                .iter()
                .map(|entry| entry.decl.id.clone())
                .collect()
        })
    }

    /// Every loaded bundle by id, enabled or not, with what it declares.
    pub(crate) fn loaded_deps(&self) -> Vec<(String, Vec<String>)> {
        self.installed()
            .map(|(id, bundle)| (id.to_string(), bundle.meta.dependencies.clone()))
            .collect()
    }

    /// The loaded mod that ships section `part`, when the project does not see
    /// it yet: neither `base` nor a declared mod carries it.
    pub(crate) fn undeclared_owner(&self, part: &str) -> Option<String> {
        let project = self.project.as_deref()?;
        let declared = &project.data.meta.dependencies;
        let mut owner = None;
        for (id, bundle) in self.bundles() {
            let ships_it = self
                .items_of(bundle)
                .any(|item| matches!(item, Content::Section(_)) && item.id() == part);
            if !ships_it {
                continue;
            }
            let open_mod = project.id.as_deref() == Some(id);
            if open_mod || id == BASE_MOD_ID || declared.iter().any(|held| held == id) {
                return None;
            }
            owner = Some(id.to_string());
        }
        owner
    }

    /// Whether `file` is a `dep://` ref into a mod the project does not declare.
    pub(crate) fn is_undeclared(&self, file: &str) -> bool {
        let Some(project) = self.project.as_deref() else {
            return false;
        };
        file.strip_prefix(DEP)
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(id, _)| {
                id != BASE_MOD_ID
                    && project.id.as_deref() != Some(id)
                    && !project.data.meta.dependencies.iter().any(|dep| dep == id)
            })
    }

    /// The content ids mod `dep` ships.
    pub(crate) fn content_ids(&self, dep: &str) -> HashSet<String> {
        self.installed()
            .filter(|(id, _)| *id == dep)
            .flat_map(|(_, bundle)| self.items_of(bundle))
            .map(|item| item.id().to_string())
            .collect()
    }

    /// The loaded mods a project declaring `declared` sees, in load order:
    /// `base`, what it declares, and what those declare.
    pub(crate) fn visible_packs(&self, declared: &[String]) -> Vec<VisiblePack<'_>> {
        let bundles: Vec<(&str, &BundleAsset)> = self.installed().collect();
        let mut seen: HashSet<&str> = HashSet::from([BASE_MOD_ID]);
        let mut queue: Vec<&str> = declared.iter().map(String::as_str).collect();
        while let Some(id) = queue.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some((_, bundle)) = bundles.iter().find(|(held, _)| *held == id) {
                queue.extend(bundle.meta.dependencies.iter().map(String::as_str));
            }
        }
        bundles
            .iter()
            .filter(|(id, _)| seen.contains(*id))
            .map(|(id, bundle)| VisiblePack {
                id,
                dependencies: &bundle.meta.dependencies,
                items: self.items_of(bundle).collect(),
            })
            .collect()
    }

    /// Every item of `bundle`, in file order.
    fn items_of<'a>(&'a self, bundle: &'a BundleAsset) -> impl Iterator<Item = &'a Content> {
        let contents = self.contents.as_deref();
        bundle
            .content
            .iter()
            .filter_map(move |handle| contents?.get(handle))
            .flat_map(|content| content.0.iter())
    }

    /// Whether the index knows anything at all. A rig with no bundles loaded
    /// answers every path with "unknown", which would paint a whole panel wrong.
    pub(crate) fn is_empty(&self) -> bool {
        self.declared().next().is_none()
    }

    /// Every (bundle id, declared file) pair of the enabled set.
    fn declared(&self) -> impl Iterator<Item = (&str, &str)> {
        self.bundles()
            .flat_map(|(id, bundle)| bundle.resources.iter().map(move |file| (id, file.as_str())))
    }

    /// The one catalog asset, read out of `Assets` rather than through
    /// `GameAssets`: there is only ever the one, and reading it this way keeps
    /// the index out of the boot collection, whose thirty handles a picker has
    /// no use for.
    fn catalog(&self) -> Option<&InstalledCatalog> {
        self.catalogs
            .as_deref()
            .and_then(|catalogs| catalogs.iter().next())
            .map(|(_, catalog)| catalog)
    }

    /// Every ENABLED bundle, by id - the merge's own walk.
    ///
    /// `nova_assets::enabled_bundles` owns which bundles are active and in what
    /// order, so the picker offers the set the merge loads. That includes the
    /// NO-SHADOWING rule: a downloaded id that shadows a shipped one is dropped
    /// there, and offering its files here would let a creator name content the
    /// merge resolves against the shipped bundle instead.
    fn bundles(&self) -> impl Iterator<Item = (&str, &BundleAsset)> {
        let catalog = self.catalog();
        // An optional mod's handle lives in `OptionalBundles`, not on its
        // catalog entry - the catalog deliberately does not load one. A rig
        // without that resource sees the mandatory entries only, and one
        // without `EnabledMods` sees everything installed.
        enabled_bundles(
            catalog,
            self.optional.as_deref(),
            self.downloaded.as_deref(),
            self.enabled.as_deref(),
        )
        .filter_map(|active| Some((active.id, self.bundles.as_deref()?.get(active.bundle?)?)))
    }

    /// Every loaded bundle, enabled or not, by the same walk.
    ///
    /// What a project can depend on. A dependency a builder just declared is
    /// switched on only on the way out of the editor, because a change to the
    /// enabled set during Playing re-runs the merge, and its bundle is loaded
    /// either way.
    fn installed(&self) -> impl Iterator<Item = (&str, &BundleAsset)> {
        enabled_bundles(
            self.catalog(),
            self.optional.as_deref(),
            self.downloaded.as_deref(),
            None,
        )
        .filter_map(|active| Some((active.id, self.bundles.as_deref()?.get(active.bundle?)?)))
    }
}

/// Whether `file` ends in one of `extensions`, ignoring case.
fn has_extension(file: &str, extensions: &[&str]) -> bool {
    let Some((_, found)) = file.rsplit_once('.') else {
        return false;
    };
    extensions
        .iter()
        .any(|wanted| found.eq_ignore_ascii_case(wanted))
}

#[cfg(test)]
#[path = "asset_index/tests.rs"]
mod tests;
