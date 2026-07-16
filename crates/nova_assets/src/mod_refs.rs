//! Mod resource references: the `self://` (own folder) and `dep://<id>/` (a
//! declared dependency's folder) sentinels, their rewrite to concrete asset
//! paths, and membership validation. See `docs/design/mod-binary-resources.md`.
//!
//! Two reserved schemes, both resolved at bundle-merge time where the owning mod
//! and its loaded dependencies are known, and both rewritten away before the path
//! ever reaches the `AssetServer` (neither is a real bevy asset source):
//!
//! - `self://X` -> the file `X` THIS mod ships, against its own folder (shipped
//!   `assets/mods/<id>/` or downloaded `mods://<id>/`).
//! - `dep://<id>/X` -> the file `X` that DEPENDENCY `<id>` ships, against `<id>`'s
//!   own folder - so a shared "art pack" mod (or the base game) can be referenced
//!   without copying its bytes. `<id>` must be a DECLARED dependency
//!   (`meta.dependencies`) of the referencing bundle, OR the special id `base`:
//!   `base` is the implicit universal dependency, so `dep://base/X` is always
//!   allowed (base need not be declared).
//!
//! Both are gated in every domain: the ref must name a DECLARED resource of the
//! mod it points at, and a `dep://` target must be a declared dependency (or
//! `base`).

use std::collections::{HashMap, HashSet};

use nova_modding::prelude::Content;

/// The reserved scheme for a ref to the owning mod's OWN folder.
pub const SELF_SCHEME: &str = "self://";
/// The reserved scheme for a ref to a declared DEPENDENCY's folder
/// (`dep://<id>/<path>`).
pub const DEP_SCHEME: &str = "dep://";

/// What one declared, AVAILABLE dependency contributes to ref resolution: where
/// its files live (for the rewrite) and which it declares (for membership).
#[derive(Clone, Copy)]
pub struct DepRef<'a> {
    /// The dependency's `resource_base`, used to rewrite `dep://<id>/X`. `None`
    /// in domains that only VALIDATE and never rewrite (the static lint), where
    /// the base is irrelevant.
    pub base: Option<&'a str>,
    /// The dependency's declared `resources`, for membership validation. `None`
    /// when this domain cannot see them (a SHIPPED dependency in the portal
    /// generator, which only knows portal-mod resources) - membership is then
    /// skipped, backstopped by the runtime gate and the repo lint.
    pub resources: Option<&'a [String]>,
}

/// The resolution context for ONE owning bundle's content: its own folder plus
/// the declared dependencies it may reach into. Shared by the rewrite (which
/// uses the base fields) and the violation scan (which uses the resource fields).
pub struct RefScope<'a> {
    /// The owning bundle's `resource_base` (target of `self://`).
    pub self_base: &'a str,
    /// The owning bundle's declared `resources` (for `self://` membership).
    pub self_resources: &'a [String],
    /// Every id the owning bundle DECLARES as a dependency (`base` excluded from
    /// this set - it is the implicit universal dependency, allowed for `dep://`
    /// without being declared, and supplied in `deps` instead).
    pub declared_deps: &'a HashSet<String>,
    /// The available `dep://` targets by id - the declared deps that are loaded,
    /// PLUS `base` (the implicit universal dep). A declared id ABSENT here is
    /// declared-but-unavailable - `dep://` to it is a
    /// violation and its ref is left literal.
    pub deps: &'a HashMap<String, DepRef<'a>>,
}

impl RefScope<'_> {
    /// The concrete asset path a resource-ref leaf rewrites to, or `None` to
    /// leave the leaf literal. `self://` ALWAYS rewrites (an undeclared own
    /// resource still resolves to the mod folder and fails to load loudly, so the
    /// violation scan - not the rewrite - is what gates it). `dep://` rewrites
    /// ONLY when the target is a declared, available dependency; an ungated
    /// `dep://` is left literal (an unknown `dep` source that fails loudly).
    fn rewrite_leaf(&self, s: &str) -> Option<String> {
        if let Some(rest) = s.strip_prefix(SELF_SCHEME) {
            return Some(join_base(self.self_base, rest));
        }
        if let Some(rest) = s.strip_prefix(DEP_SCHEME) {
            let (id, path) = rest.split_once('/')?;
            if id.is_empty() || path.is_empty() {
                return None;
            }
            // `base` is the implicit universal dependency: allowed without being
            // in `declared_deps`. Every other id must be declared.
            if id != "base" && !self.declared_deps.contains(id) {
                return None;
            }
            let base = self.deps.get(id)?.base?;
            return Some(join_base(base, path));
        }
        None
    }

    /// The reason `r` is invalid in this scope, or `None` if it is a valid ref.
    /// Each reason opens with a verb ("references .." / "has ..") so a caller can
    /// prefix a subject ("scenario X ..", "section Y ..").
    fn violation(&self, r: &ResourceRef) -> Option<String> {
        match r {
            ResourceRef::SelfRef { file } => {
                (!self.self_resources.iter().any(|x| x == file)).then(|| {
                    format!(
                        "references undeclared mod resource '{SELF_SCHEME}{file}' - add it to \
                         the bundle manifest's `resources` list"
                    )
                })
            }
            ResourceRef::DepRef { id, file } => {
                // `base` is the implicit universal dependency - `dep://base/X` is
                // always allowed (base need not be in `meta.dependencies`). Every
                // other id must be a declared dependency.
                if id != "base" && !self.declared_deps.contains(id) {
                    return Some(format!(
                        "references resource '{DEP_SCHEME}{id}/{file}' but '{id}' is not a \
                         declared dependency - add '{id}' to the bundle manifest's \
                         `meta.dependencies`"
                    ));
                }
                match self.deps.get(id) {
                    None => Some(format!(
                        "references resource '{DEP_SCHEME}{id}/{file}' but dependency '{id}' is \
                         not available (not installed, not loaded, or not enabled)"
                    )),
                    Some(dep) => match dep.resources {
                        Some(res) if !res.iter().any(|x| x == file) => Some(format!(
                            "references undeclared resource '{DEP_SCHEME}{id}/{file}' of \
                             dependency '{id}' - add it to that mod's `resources` list"
                        )),
                        _ => None,
                    },
                }
            }
            ResourceRef::MalformedDep { raw } => Some(format!(
                "has a malformed dependency resource ref '{raw}' - expected \
                 '{DEP_SCHEME}<id>/<path>'"
            )),
        }
    }
}

/// A parsed resource reference found in content, scheme-detected and (for the
/// file part) `#label`-stripped - the form the membership gate checks.
enum ResourceRef {
    /// `self://<file>` - the owning mod's own folder.
    SelfRef { file: String },
    /// `dep://<id>/<file>` - dependency `<id>`'s folder.
    DepRef { id: String, file: String },
    /// A `dep://` leaf that is not `dep://<id>/<path>` shaped.
    MalformedDep { raw: String },
}

/// Rewrite every resource-ref leaf (`self://` and gated `dep://`) in `content` to
/// its concrete asset path per `scope`. Returns the rewritten owned item; a
/// `content` with nothing to rewrite is cloned unchanged.
///
/// Generic over the whole content tree: it serializes to a `serde_json::Value`,
/// rewrites every matching string leaf, and deserializes back, so it catches
/// EVERY `AssetRef` field (all serialize as bare strings) with no per-field code
/// and no maintenance as fields are added. The round-trip is lossless - the
/// content tree has only string map keys (`SectionId = String`) and f32<->f64 is
/// exact. On the rare (de)serialization failure it logs and returns a clone of
/// the original, so bad data never panics the merge.
///
/// That failure is UNREACHABLE for RON-authored content (the only thing
/// `register_bundles` ever feeds): `to_value`/`from_value` only `Err` for
/// programmatically-built configs - an `AssetRef::Handle` (a code default), a
/// non-authorable `Binding`, or a non-finite `f32` - none of which a parsed
/// `*.content.ron` produces. When it does fire, the item's refs are left literal
/// (they fail to load loudly as an unknown source), never silently mis-resolved.
pub fn rewrite_refs(content: &Content, scope: &RefScope) -> Content {
    let mut value = match serde_json::to_value(content) {
        Ok(value) => value,
        Err(err) => {
            bevy::log::error!(
                "mod asset-ref rewrite: content did not serialize ({err}); \
                 leaving its refs unresolved"
            );
            return content.clone();
        }
    };
    if !rewrite_value(&mut value, scope) {
        // No resource-ref leaf anywhere - skip the deserialize round-trip.
        return content.clone();
    }
    match serde_json::from_value(value) {
        Ok(rewritten) => rewritten,
        Err(err) => {
            bevy::log::error!(
                "mod asset-ref rewrite: rewritten content did not deserialize ({err}); \
                 leaving its refs unresolved"
            );
            content.clone()
        }
    }
}

/// Rewrite resource-ref string leaves in place. Returns whether anything changed.
fn rewrite_value(value: &mut serde_json::Value, scope: &RefScope) -> bool {
    match value {
        serde_json::Value::String(s) => match scope.rewrite_leaf(s) {
            Some(rewritten) => {
                *s = rewritten;
                true
            }
            None => false,
        },
        serde_json::Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= rewrite_value(item, scope);
            }
            changed
        }
        serde_json::Value::Object(map) => {
            let mut changed = false;
            for v in map.values_mut() {
                changed |= rewrite_value(v, scope);
            }
            changed
        }
        _ => false,
    }
}

/// Join a `resource_base` prefix with the path after the scheme. An empty base (a
/// bundle at the asset root) yields the bare relative path.
fn join_base(base: &str, rest: &str) -> String {
    if base.is_empty() {
        rest.to_string()
    } else {
        format!("{base}/{rest}")
    }
}

/// The FILE part of a resource ref, with any asset label (`#Scene0`) stripped -
/// the form to check against a declared `resources` list.
fn strip_label(s: &str) -> &str {
    s.split('#').next().unwrap_or(s)
}

/// Parse one string leaf into a resource ref, or `None` if it names no scheme.
fn parse_leaf(s: &str) -> Option<ResourceRef> {
    if let Some(rest) = s.strip_prefix(SELF_SCHEME) {
        return Some(ResourceRef::SelfRef {
            file: strip_label(rest).to_string(),
        });
    }
    if let Some(rest) = s.strip_prefix(DEP_SCHEME) {
        return Some(match rest.split_once('/') {
            Some((id, path)) if !id.is_empty() && !path.is_empty() => ResourceRef::DepRef {
                id: id.to_string(),
                file: strip_label(path).to_string(),
            },
            _ => ResourceRef::MalformedDep { raw: s.to_string() },
        });
    }
    None
}

/// Every resource ref (`self://` or `dep://`) a content item names.
fn collect_refs(content: &Content) -> Vec<ResourceRef> {
    let mut out = Vec::new();
    if let Ok(value) = serde_json::to_value(content) {
        collect_value(&value, &mut out);
    }
    out
}

fn collect_value(value: &serde_json::Value, out: &mut Vec<ResourceRef>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(r) = parse_leaf(s) {
                out.push(r);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|i| collect_value(i, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| collect_value(v, out)),
        _ => {}
    }
}

/// The reasons a content item's resource refs are invalid in `scope` - the
/// validation gate's finding set, sorted and deduplicated. Empty means every
/// `self://` and `dep://` ref names a declared resource of a valid target.
pub fn resource_ref_violations(content: &Content, scope: &RefScope) -> Vec<String> {
    let mut out: Vec<String> = collect_refs(content)
        .iter()
        .filter_map(|r| scope.violation(r))
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::AssetRef;
    use nova_scenario::prelude::ScenarioConfig;

    use super::*;

    /// A scope with no declared dependencies - only `self://` is resolvable.
    fn self_only_scope<'a>(base: &'a str, resources: &'a [String]) -> RefScope<'a> {
        // Leak-free statics for the empty dep collections.
        static NO_DEPS: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
        static NO_DEP_MAP: std::sync::OnceLock<HashMap<String, DepRef<'static>>> =
            std::sync::OnceLock::new();
        RefScope {
            self_base: base,
            self_resources: resources,
            declared_deps: NO_DEPS.get_or_init(HashSet::new),
            deps: NO_DEP_MAP.get_or_init(HashMap::new),
        }
    }

    /// A scenario whose skybox is a mod-relative ref and whose asteroid texture
    /// (buried in a spawn action) is a base ref - exercises both the rewrite and
    /// the "leave base refs alone" rule across a nested tree.
    fn scenario_with_refs() -> ScenarioConfig {
        let ron = r#"(
            id: "example_scenario",
            name: "Example Demo",
            description: "",
            cubemap: "self://textures/nebula.png",
            events: [
                (
                    name: OnStart,
                    filters: [],
                    actions: [
                        SpawnScenarioObject((
                            base: (
                                id: "rock",
                                name: "Rock",
                                position: (0.0, 0.0, -10.0),
                                rotation: (0.0, 0.0, 0.0, 1.0),
                            ),
                            kind: Asteroid((
                                radius: 2.0,
                                texture: "self://textures/rock.png",
                                health: 50.0,
                                surface_gravity: None,
                                invulnerable: false,
                                lock_signature: None,
                            )),
                        )),
                    ],
                ),
            ],
        )"#;
        ron::from_str(ron).expect("scenario parses")
    }

    #[test]
    fn rewrites_self_refs_against_the_bundle_base() {
        let content = Content::Scenario(scenario_with_refs());
        let resources = vec![
            "textures/nebula.png".to_string(),
            "textures/rock.png".to_string(),
        ];
        let rewritten = rewrite_refs(&content, &self_only_scope("mods/example", &resources));
        let Content::Scenario(cfg) = rewritten else {
            panic!("still a scenario");
        };
        assert_eq!(
            cfg.cubemap.path(),
            Some("mods/example/textures/nebula.png"),
            "the skybox self:// ref resolves against the mod folder"
        );
    }

    #[test]
    fn downloaded_base_keeps_the_mods_scheme() {
        let content = Content::Scenario(scenario_with_refs());
        let resources = vec![
            "textures/nebula.png".to_string(),
            "textures/rock.png".to_string(),
        ];
        let rewritten = rewrite_refs(&content, &self_only_scope("mods://example", &resources));
        let Content::Scenario(cfg) = rewritten else {
            panic!("still a scenario");
        };
        assert_eq!(
            cfg.cubemap.path(),
            Some("mods://example/textures/nebula.png"),
            "a downloaded bundle's self:// ref keeps the mods:// source"
        );
    }

    #[test]
    fn a_base_relative_ref_is_untouched() {
        // `cubemap` is base-relative here (no scheme): it must survive verbatim.
        let mut cfg = scenario_with_refs();
        cfg.cubemap = AssetRef::from("textures/cubemap.png".to_string());
        let content = Content::Scenario(cfg);
        let resources = vec!["textures/rock.png".to_string()];
        let rewritten = rewrite_refs(&content, &self_only_scope("mods/example", &resources));
        let Content::Scenario(cfg) = rewritten else {
            panic!("still a scenario");
        };
        assert_eq!(cfg.cubemap.path(), Some("textures/cubemap.png"));
    }

    #[test]
    fn a_handle_bearing_config_falls_back_gracefully() {
        // An `AssetRef::Handle` makes `serde_json::to_value` fail (the Handle
        // variant errors on serialize). The rewrite must NOT panic; it returns
        // the item unchanged (the Handle survives, still not a path).
        let mut cfg = scenario_with_refs();
        cfg.cubemap = AssetRef::from(bevy::prelude::Handle::<bevy::prelude::Image>::default());
        assert_eq!(
            cfg.cubemap.path(),
            None,
            "precondition: a handle, not a path"
        );
        let content = Content::Scenario(cfg);
        let rewritten = rewrite_refs(&content, &self_only_scope("mods/example", &[]));
        let Content::Scenario(cfg) = rewritten else {
            panic!("still a scenario");
        };
        assert_eq!(
            cfg.cubemap.path(),
            None,
            "a handle-bearing config is returned unchanged, not mis-rewritten or panicked"
        );
    }

    #[test]
    fn undeclared_self_refs_are_flagged() {
        let content = Content::Scenario(scenario_with_refs());
        // Only the skybox is declared; the rock texture is not.
        let declared = vec!["textures/nebula.png".to_string()];
        let violations =
            resource_ref_violations(&content, &self_only_scope("mods/example", &declared));
        assert_eq!(
            violations.len(),
            1,
            "one undeclared self ref: {violations:?}"
        );
        assert!(
            violations[0].contains("self://textures/rock.png"),
            "names the undeclared file: {}",
            violations[0]
        );
        // Declare both: nothing undeclared.
        let declared = vec![
            "textures/nebula.png".to_string(),
            "textures/rock.png".to_string(),
        ];
        assert!(
            resource_ref_violations(&content, &self_only_scope("mods/example", &declared))
                .is_empty()
        );
    }

    #[test]
    fn a_labeled_glb_ref_validates_against_the_bare_file() {
        // A section render mesh with a `#Scene0` label: validation strips the
        // label, rewrite keeps it.
        let ron = r#"Section((
            base: (id: "hull", name: "Hull", description: "", mass: 1.0, health: 100.0),
            kind: Hull((render_mesh: Some("self://models/hull.glb#Scene0"))),
        ))"#;
        let content: Content = ron::from_str(ron).expect("section parses");
        let declared = vec!["models/hull.glb".to_string()];
        assert!(
            resource_ref_violations(&content, &self_only_scope("mods/example", &declared))
                .is_empty(),
            "the bare file path satisfies the label-bearing ref"
        );
        let rewritten = rewrite_refs(&content, &self_only_scope("mods/example", &declared));
        let ron_out = ron::to_string(&rewritten).expect("re-serializes");
        assert!(
            ron_out.contains("mods/example/models/hull.glb#Scene0"),
            "the rewrite keeps the #Scene0 label: {ron_out}"
        );
    }

    /// A scenario whose skybox references dependency `art`'s texture.
    fn scenario_with_dep_ref(reference: &str) -> Content {
        let ron = format!(
            r#"Scenario((
                id: "consumer",
                name: "Consumer",
                description: "",
                cubemap: "{reference}",
                events: [],
            ))"#
        );
        ron::from_str(&ron).expect("scenario parses")
    }

    /// A scope declaring dependency `art` with the given availability/resources.
    fn dep_scope<'a>(
        declared: &'a HashSet<String>,
        deps: &'a HashMap<String, DepRef<'a>>,
    ) -> RefScope<'a> {
        RefScope {
            self_base: "mods/consumer",
            self_resources: &[],
            declared_deps: declared,
            deps,
        }
    }

    #[test]
    fn dep_ref_rewrites_against_the_dependency_folder() {
        let content = scenario_with_dep_ref("dep://art/textures/sky.png");
        let declared: HashSet<String> = ["art".to_string()].into_iter().collect();
        let art_resources = vec!["textures/sky.png".to_string()];
        let mut deps = HashMap::new();
        deps.insert(
            "art".to_string(),
            DepRef {
                base: Some("mods/art"),
                resources: Some(&art_resources),
            },
        );
        let scope = dep_scope(&declared, &deps);
        assert!(
            resource_ref_violations(&content, &scope).is_empty(),
            "a declared dep + declared resource is valid"
        );
        let Content::Scenario(cfg) = rewrite_refs(&content, &scope) else {
            panic!("still a scenario");
        };
        assert_eq!(
            cfg.cubemap.path(),
            Some("mods/art/textures/sky.png"),
            "the dep:// ref resolves against the dependency's folder"
        );
    }

    #[test]
    fn dep_ref_to_a_downloaded_dependency_keeps_the_mods_scheme() {
        let content = scenario_with_dep_ref("dep://art/textures/sky.png");
        let declared: HashSet<String> = ["art".to_string()].into_iter().collect();
        let art_resources = vec!["textures/sky.png".to_string()];
        let mut deps = HashMap::new();
        deps.insert(
            "art".to_string(),
            DepRef {
                base: Some("mods://art"),
                resources: Some(&art_resources),
            },
        );
        let Content::Scenario(cfg) = rewrite_refs(&content, &dep_scope(&declared, &deps)) else {
            panic!("still a scenario");
        };
        assert_eq!(
            cfg.cubemap.path(),
            Some("mods://art/textures/sky.png"),
            "a downloaded dependency's dep:// ref keeps the mods:// source"
        );
    }

    #[test]
    fn dep_ref_to_a_non_declared_mod_is_a_violation_and_left_literal() {
        let content = scenario_with_dep_ref("dep://art/textures/sky.png");
        // `art` is NOT declared as a dependency.
        let declared: HashSet<String> = HashSet::new();
        let deps: HashMap<String, DepRef> = HashMap::new();
        let scope = dep_scope(&declared, &deps);
        let violations = resource_ref_violations(&content, &scope);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations[0].contains("not a declared dependency"),
            "{}",
            violations[0]
        );
        // Ungated dep refs are left literal so they fail to load loudly.
        let Content::Scenario(cfg) = rewrite_refs(&content, &scope) else {
            panic!("still a scenario");
        };
        assert_eq!(cfg.cubemap.path(), Some("dep://art/textures/sky.png"));
    }

    #[test]
    fn dep_ref_to_an_undeclared_resource_of_a_dependency_is_a_violation() {
        let content = scenario_with_dep_ref("dep://art/textures/missing.png");
        let declared: HashSet<String> = ["art".to_string()].into_iter().collect();
        let art_resources = vec!["textures/sky.png".to_string()];
        let mut deps = HashMap::new();
        deps.insert(
            "art".to_string(),
            DepRef {
                base: Some("mods/art"),
                resources: Some(&art_resources),
            },
        );
        let violations = resource_ref_violations(&content, &dep_scope(&declared, &deps));
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations[0].contains("undeclared resource 'dep://art/textures/missing.png'"),
            "{}",
            violations[0]
        );
    }

    #[test]
    fn dep_ref_to_a_declared_but_unavailable_dependency_is_a_violation() {
        let content = scenario_with_dep_ref("dep://art/textures/sky.png");
        // `art` is declared but not present in the available `deps` map.
        let declared: HashSet<String> = ["art".to_string()].into_iter().collect();
        let deps: HashMap<String, DepRef> = HashMap::new();
        let violations = resource_ref_violations(&content, &dep_scope(&declared, &deps));
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("not available"), "{}", violations[0]);
    }

    #[test]
    fn dep_ref_to_base_resolves_without_being_declared() {
        // `base` is the implicit universal dependency: `dep://base/X` is allowed
        // even though `base` is NOT in `declared_deps`. Base is supplied in the
        // `deps` map (as callers always do), with base's own `resource_base`.
        let content = scenario_with_dep_ref("dep://base/textures/cubemap.png");
        let declared: HashSet<String> = HashSet::new(); // base NOT declared
        let base_resources = vec!["textures/cubemap.png".to_string()];
        let mut deps = HashMap::new();
        deps.insert(
            "base".to_string(),
            DepRef {
                base: Some("base"),
                resources: Some(&base_resources),
            },
        );
        let scope = dep_scope(&declared, &deps);
        assert!(
            resource_ref_violations(&content, &scope).is_empty(),
            "dep://base with a declared base resource is valid without declaring base",
        );
        let Content::Scenario(cfg) = rewrite_refs(&content, &scope) else {
            panic!("still a scenario");
        };
        assert_eq!(
            cfg.cubemap.path(),
            Some("base/textures/cubemap.png"),
            "dep://base resolves against base's own folder",
        );
    }

    #[test]
    fn dep_ref_to_an_undeclared_base_resource_is_a_violation() {
        // base is available but does not declare the referenced file.
        let content = scenario_with_dep_ref("dep://base/textures/missing.png");
        let declared: HashSet<String> = HashSet::new();
        let base_resources = vec!["textures/cubemap.png".to_string()];
        let mut deps = HashMap::new();
        deps.insert(
            "base".to_string(),
            DepRef {
                base: Some("base"),
                resources: Some(&base_resources),
            },
        );
        let violations = resource_ref_violations(&content, &dep_scope(&declared, &deps));
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations[0].contains("undeclared resource 'dep://base/textures/missing.png'"),
            "{}",
            violations[0]
        );
    }

    #[test]
    fn a_malformed_dep_ref_is_a_violation() {
        let content = scenario_with_dep_ref("dep://art");
        let declared: HashSet<String> = ["art".to_string()].into_iter().collect();
        let deps: HashMap<String, DepRef> = HashMap::new();
        let violations = resource_ref_violations(&content, &dep_scope(&declared, &deps));
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("malformed"), "{}", violations[0]);
    }
}
