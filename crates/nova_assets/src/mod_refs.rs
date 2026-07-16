//! Mod-relative asset references: the `self://` sentinel, its rewrite to a
//! concrete asset path, and membership validation against a bundle's declared
//! `resources`. See `docs/design/mod-binary-resources.md`.
//!
//! `self://X` in any content asset path means "the file `X` this mod ships",
//! resolved against the owning mod's own folder (shipped `assets/mods/<id>/` or
//! downloaded `mods://<id>/`) rather than the base game. It is a SENTINEL, never
//! a real bevy asset source: it is always rewritten away before the path reaches
//! the `AssetServer`, at bundle-merge time where the owning mod is known.

use nova_modding::prelude::Content;

/// The reserved scheme marking a mod-relative asset ref in content.
pub const SELF_SCHEME: &str = "self://";

/// Rewrite every `self://X` asset path in `content` to `<base>/X`, where `base`
/// is the owning bundle's `resource_base` (`mods/<id>` for shipped, `mods://<id>`
/// for downloaded). Returns the rewritten owned item; a `content` with no
/// `self://` ref is cloned unchanged.
///
/// Generic over the whole content tree: it serializes to a `serde_json::Value`,
/// rewrites every string leaf beginning with `self://`, and deserializes back,
/// so it catches EVERY `AssetRef` field (all serialize as bare strings) with no
/// per-field code and no maintenance as fields are added. The round-trip is
/// lossless - the content tree has only string map keys (`SectionId = String`)
/// and f32<->f64 is exact. On the rare (de)serialization failure it logs and
/// returns a clone of the original, so bad data never panics the merge.
///
/// That failure is UNREACHABLE for RON-authored content (the only thing
/// `register_bundles` ever feeds): `to_value`/`from_value` only `Err` for
/// programmatically-built configs - an `AssetRef::Handle` (a code default), a
/// non-authorable `Binding`, or a non-finite `f32` - none of which a parsed
/// `*.content.ron` produces. When it does fire, the item's `self://` refs are
/// left literal (they fail to load loudly as an unknown `self` source), never
/// silently mis-resolved.
pub fn rewrite_self_refs(content: &Content, base: &str) -> Content {
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
    if !rewrite_value(&mut value, base) {
        // No `self://` ref anywhere - skip the deserialize round-trip entirely.
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

/// Rewrite `self://` string leaves in place. Returns whether anything changed.
fn rewrite_value(value: &mut serde_json::Value, base: &str) -> bool {
    match value {
        serde_json::Value::String(s) => match s.strip_prefix(SELF_SCHEME) {
            Some(rest) => {
                *s = join_base(base, rest);
                true
            }
            None => false,
        },
        serde_json::Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= rewrite_value(item, base);
            }
            changed
        }
        serde_json::Value::Object(map) => {
            let mut changed = false;
            for v in map.values_mut() {
                changed |= rewrite_value(v, base);
            }
            changed
        }
        _ => false,
    }
}

/// Join a `resource_base` prefix with the path after `self://`. An empty base (a
/// bundle at the asset root) yields the bare relative path.
fn join_base(base: &str, rest: &str) -> String {
    if base.is_empty() {
        rest.to_string()
    } else {
        format!("{base}/{rest}")
    }
}

/// The FILE path a `self://` ref names, with the scheme and any asset label
/// (`#Scene0`) stripped - the form to check against a bundle's declared
/// `resources`. `self://models/hull.glb#Scene0` -> `models/hull.glb`.
fn self_ref_file(s: &str) -> Option<&str> {
    s.strip_prefix(SELF_SCHEME)
        .map(|rest| rest.split('#').next().unwrap_or(rest))
}

/// Every mod-relative (`self://`) resource FILE a content item references, each
/// stripped of the scheme and any asset label. Deduplicated is not guaranteed;
/// callers that validate do a membership test per entry.
pub fn collect_self_refs(content: &Content) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(value) = serde_json::to_value(content) {
        collect_value(&value, &mut out);
    }
    out
}

fn collect_value(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(file) = self_ref_file(s) {
                out.push(file.to_string());
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|i| collect_value(i, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| collect_value(v, out)),
        _ => {}
    }
}

/// The `self://` resource files a content item references that are NOT declared
/// in `resources` - the validation gate's finding set. Empty means every
/// mod-relative ref names a declared member.
pub fn undeclared_self_refs(content: &Content, resources: &[String]) -> Vec<String> {
    let mut undeclared: Vec<String> = collect_self_refs(content)
        .into_iter()
        .filter(|file| !resources.iter().any(|res| res == file))
        .collect();
    undeclared.sort();
    undeclared.dedup();
    undeclared
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::AssetRef;
    use nova_scenario::prelude::ScenarioConfig;

    use super::*;

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
        let rewritten = rewrite_self_refs(&content, "mods/example");
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
        let rewritten = rewrite_self_refs(&content, "mods://example");
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
        // `cubemap` is base-relative here (no self://): it must survive verbatim.
        let mut cfg = scenario_with_refs();
        cfg.cubemap = AssetRef::from("textures/cubemap.png".to_string());
        let content = Content::Scenario(cfg);
        let rewritten = rewrite_self_refs(&content, "mods/example");
        let Content::Scenario(cfg) = rewritten else {
            panic!("still a scenario");
        };
        assert_eq!(cfg.cubemap.path(), Some("textures/cubemap.png"));
    }

    #[test]
    fn a_handle_bearing_config_falls_back_gracefully() {
        // An `AssetRef::Handle` makes `serde_json::to_value` fail (the Handle
        // variant errors on serialize). The rewrite must NOT panic; it returns
        // the item unchanged (the Handle survives, still not a path). This pins
        // the "bad data never crashes the merge" fallback for the one input the
        // production RON path can never produce.
        let mut cfg = scenario_with_refs();
        cfg.cubemap = AssetRef::from(bevy::prelude::Handle::<bevy::prelude::Image>::default());
        assert_eq!(
            cfg.cubemap.path(),
            None,
            "precondition: a handle, not a path"
        );
        let content = Content::Scenario(cfg);
        let rewritten = rewrite_self_refs(&content, "mods/example");
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
    fn collects_both_refs_stripped_of_scheme() {
        let content = Content::Scenario(scenario_with_refs());
        let mut refs = collect_self_refs(&content);
        refs.sort();
        assert_eq!(refs, vec!["textures/nebula.png", "textures/rock.png"]);
    }

    #[test]
    fn undeclared_refs_are_the_ones_missing_from_resources() {
        let content = Content::Scenario(scenario_with_refs());
        // Only the skybox is declared; the rock texture is not.
        let declared = vec!["textures/nebula.png".to_string()];
        assert_eq!(
            undeclared_self_refs(&content, &declared),
            vec!["textures/rock.png".to_string()],
        );
        // Declare both: nothing undeclared.
        let declared = vec![
            "textures/nebula.png".to_string(),
            "textures/rock.png".to_string(),
        ];
        assert!(undeclared_self_refs(&content, &declared).is_empty());
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
        assert_eq!(
            undeclared_self_refs(&content, &["models/hull.glb".to_string()]),
            Vec::<String>::new(),
            "the bare file path satisfies the label-bearing ref"
        );
        let rewritten = rewrite_self_refs(&content, "mods/example");
        let ron_out = ron::to_string(&rewritten).expect("re-serializes");
        assert!(
            ron_out.contains("mods/example/models/hull.glb#Scene0"),
            "the rewrite keeps the #Scene0 label: {ron_out}"
        );
    }
}
