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
    mod_refs::prelude::{rewrite_refs, DepRef, RefScope},
    prelude::{DownloadedMods, EnabledMods},
};
use nova_modding::prelude::{BundleAsset, Content, InstalledCatalog};
use nova_scenario::prelude::ScenarioConfig;

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
    catalogs: Option<Res<'w, Assets<InstalledCatalog>>>,
    bundles: Option<Res<'w, Assets<BundleAsset>>>,
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
        offered.sort_unstable();
        offered.dedup();
        offered
    }

    /// Whether `text` names a file the installed bundles ship.
    ///
    /// Only a `dep://` ref can be answered: `self://` resolves against whichever
    /// bundle owns the config, which the panel does not know, and a bare path
    /// addresses the asset root, which no bundle declares. Both are left alone
    /// rather than marked wrong on a guess.
    pub(crate) fn resolves(&self, text: &str) -> bool {
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
    /// the file. The sandbox is built at runtime and merged with nothing, so
    /// the same ref reaches the asset server verbatim and fails as an unknown
    /// source - which is a scenario that comes up with no sky. Resolving here
    /// keeps ONE spelling in the document and lets Play show what a save would.
    ///
    /// Every enabled bundle is a legal target, `base` included. There is no
    /// dependency graph to honour: the range is not a mod yet, and what it may
    /// name is what is installed. A save writes the refs unresolved, so the
    /// merge still gates them when the range becomes a mod.
    pub(crate) fn resolved(&self, scenario: ScenarioConfig) -> ScenarioConfig {
        let bundles: Vec<(&str, &BundleAsset)> = self.bundles().collect();
        let declared_deps: HashSet<String> =
            bundles.iter().map(|(id, _)| (*id).to_string()).collect();
        let deps: HashMap<String, DepRef<'_>> = bundles
            .iter()
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
        // No `self://` target: the sandbox owns no folder of its own, so such a
        // ref is left literal and fails loudly rather than resolving somewhere
        // it was never pointed at.
        let scope = RefScope {
            self_base: "",
            self_resources: &[],
            declared_deps: &declared_deps,
            deps: &deps,
        };
        let Content::Scenario(resolved) =
            rewrite_refs(&Content::Scenario(scenario.clone()), &scope)
        else {
            return scenario;
        };
        resolved
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

    /// Every ENABLED bundle, by id.
    fn bundles(&self) -> impl Iterator<Item = (&str, &BundleAsset)> {
        let enabled = self.enabled.as_deref().map(|enabled| &enabled.0);
        // Every catalog asset loaded rather than the one `GameAssets` points
        // at: there is only ever the one, and reading it this way keeps the
        // index out of the boot collection, whose thirty handles a picker has
        // no use for.
        let shipped = self
            .catalogs
            .as_deref()
            .into_iter()
            .flat_map(Assets::iter)
            .flat_map(|(_, catalog)| catalog.entries.iter())
            .map(|entry| (entry.decl.id.as_str(), &entry.bundle));
        let downloaded = self
            .downloaded
            .as_deref()
            .into_iter()
            .flat_map(|downloaded| downloaded.0.iter())
            .map(|installed| (installed.record.id.as_str(), &installed.bundle));
        shipped
            .chain(downloaded)
            .filter(move |(id, _)| enabled.is_none_or(|enabled| enabled.contains(*id)))
            .filter_map(|(id, handle)| Some((id, self.bundles.as_deref()?.get(handle)?)))
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
