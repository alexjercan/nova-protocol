//! Content read from a LOOSE `*.content.ron` file on disk, outside any bundle.
//!
//! The bundle pipeline is how installed content reaches a registry; this is the
//! door for a file that is not installed at all - a scenario somebody is
//! authoring or measuring. Change it when the on-disk content format changes.
//!
//! Native only: it reads the filesystem directly instead of going through the
//! asset server, which is what lets a caller name a path outside the asset root.

/// Glob-import surface for loose-file content.
pub mod prelude {
    pub use super::read_loose_scenarios;
}

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use nova_mod_format::BundleManifest;
use nova_modding::prelude::Content;
use nova_scenario::prelude::ScenarioConfig;

use crate::mod_refs::prelude::*;

/// Read one `*.content.ron` file and return the [`ScenarioConfig`]s in it, in
/// file order.
///
/// `self://` references resolve against the nearest ENCLOSING bundle, found by
/// walking up for a `*.bundle.ron` - the same folder the merge would have
/// resolved them against, so a file that ships inside a bundle measures with
/// its own art. With no enclosing bundle (or with the file outside any `assets`
/// dir) the refs are left literal and fail loudly at load, which is the honest
/// answer: nothing on disk says where they point.
///
/// `dep://` references are always left literal. Reaching into another mod needs
/// that mod's declared, resolved dependency graph, and a loose file has none.
pub fn read_loose_scenarios(path: &Path) -> Result<Vec<ScenarioConfig>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let items: Vec<Content> = ron::de::from_bytes(&bytes)
        .map_err(|e| format!("{}: not a RON Vec<Content>: {e}", path.display()))?;
    let owner = enclosing_bundle(path);
    let no_deps_declared = HashSet::new();
    let no_deps = HashMap::new();
    let scenarios: Vec<ScenarioConfig> = items
        .iter()
        .map(|item| match &owner {
            Some(owner) => rewrite_refs(
                item,
                &RefScope {
                    self_base: &owner.resource_base,
                    self_resources: &owner.resources,
                    declared_deps: &no_deps_declared,
                    deps: &no_deps,
                },
            ),
            None => item.clone(),
        })
        .filter_map(|item| match item {
            Content::Scenario(scenario) => Some(scenario),
            _ => None,
        })
        .collect();
    if scenarios.is_empty() {
        return Err(format!("{}: holds no Scenario item", path.display()));
    }
    Ok(scenarios)
}

/// The bundle a loose file sits inside: where its `self://` refs point, and
/// which files that bundle declares.
struct LooseOwner {
    /// Asset-path prefix of the bundle folder, e.g. `base` or `mods/<id>`.
    resource_base: String,
    /// The manifest's declared `resources`, the same membership gate the merge
    /// applies - an undeclared `self://` stays literal here too.
    resources: Vec<String>,
}

/// Walk up from `path` for a directory holding a `*.bundle.ron`, and describe it
/// as the file's owner.
fn enclosing_bundle(path: &Path) -> Option<LooseOwner> {
    let mut dir = path.parent()?.to_path_buf();
    loop {
        if let Some(manifest) = bundle_manifest_in(&dir) {
            return Some(LooseOwner {
                resource_base: asset_prefix(&dir)?,
                resources: manifest.resources,
            });
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn bundle_manifest_in(dir: &Path) -> Option<BundleManifest> {
    let entry = std::fs::read_dir(dir).ok()?.flatten().find(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(".bundle.ron"))
    })?;
    let bytes = std::fs::read(entry.path()).ok()?;
    ron::de::from_bytes(&bytes).ok()
}

/// The asset-path prefix of `dir`: everything after the LAST `assets`
/// component. `None` when the path holds no `assets` component, because then
/// nothing says how the asset server would address the folder.
fn asset_prefix(dir: &Path) -> Option<String> {
    let parts: Vec<&str> = dir
        .components()
        .filter_map(|part| match part {
            std::path::Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    let assets = parts.iter().rposition(|part| *part == "assets")?;
    Some(parts[assets + 1..].join("/"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A bundle tree on disk: `<tmp>/assets/<id>/` with a manifest and a
    /// content file, mirroring how shipped content is laid out.
    fn bundle_tree(name: &str, resources: &[&str], content: &str) -> PathBuf {
        let root = std::env::temp_dir()
            .join(format!("nova_loose_{}", std::process::id()))
            .join(name);
        let dir = root.join("assets").join("some_bundle");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(dir.join("scenarios")).unwrap();
        let resources = resources
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            dir.join("some_bundle.bundle.ron"),
            format!("(content: [], resources: [{resources}])"),
        )
        .unwrap();
        let file = dir.join("scenarios").join("thing.content.ron");
        std::fs::write(&file, content).unwrap();
        file
    }

    const TWO: &str = r#"[
        Scenario((
            id: "first", name: "First", description: "",
            cubemap: "self://textures/sky.png",
        )),
        Scenario((
            id: "second", name: "Second", description: "",
            cubemap: "base/textures/cubemap.png",
        )),
    ]"#;

    #[test]
    fn a_loose_file_yields_its_scenarios_in_file_order() {
        let file = bundle_tree("order", &["textures/sky.png"], TWO);
        let scenarios = read_loose_scenarios(&file).unwrap();
        let ids: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["first", "second"]);
    }

    /// The whole reason a shipped scenario is measurable by path: its art is
    /// named `self://`, and a loose read that left those literal would measure
    /// a scene with no textures in it.
    #[test]
    fn self_refs_resolve_against_the_enclosing_bundle() {
        let file = bundle_tree("refs", &["textures/sky.png"], TWO);
        let scenarios = read_loose_scenarios(&file).unwrap();
        let skybox = format!("{:?}", scenarios[0].cubemap);
        assert!(
            skybox.contains("some_bundle/textures/sky.png"),
            "the ref resolves against the bundle folder's asset prefix: {skybox}"
        );
        assert!(!skybox.contains("self://"), "{skybox}");
    }

    /// The merge's membership gate applies here too: an undeclared resource is
    /// left literal rather than silently pointed somewhere.
    #[test]
    fn an_undeclared_self_ref_stays_literal() {
        let file = bundle_tree("undeclared", &[], TWO);
        let scenarios = read_loose_scenarios(&file).unwrap();
        assert!(
            format!("{:?}", scenarios[0].cubemap).contains("self://textures/sky.png"),
            "{:?}",
            scenarios[0].cubemap
        );
    }

    #[test]
    fn a_file_without_a_scenario_is_an_error_not_an_empty_list() {
        let file = bundle_tree("empty", &[], "[]");
        let error = read_loose_scenarios(&file).unwrap_err();
        assert!(error.contains("holds no Scenario item"), "{error}");
    }

    #[test]
    fn a_missing_or_malformed_file_says_which() {
        let missing = std::env::temp_dir().join("nova_loose_absent.content.ron");
        let _ = std::fs::remove_file(&missing);
        assert!(read_loose_scenarios(&missing).is_err());

        let file = bundle_tree("bad", &[], "not ron at all");
        let error = read_loose_scenarios(&file).unwrap_err();
        assert!(error.contains("not a RON Vec<Content>"), "{error}");
    }

    #[test]
    fn the_asset_prefix_is_everything_after_the_last_assets_dir() {
        assert_eq!(
            asset_prefix(Path::new("/repo/assets/base")).as_deref(),
            Some("base")
        );
        assert_eq!(
            asset_prefix(Path::new("/repo/assets/mods/my_mod")).as_deref(),
            Some("mods/my_mod")
        );
        assert_eq!(asset_prefix(Path::new("/tmp/loose")), None);
    }
}
