//! The example catalog, parsed from the root Cargo.toml's `[[example]]`
//! blocks (task 20260719-210438).
//!
//! The examples live in category subdirectories with auto-discovery OFF
//! (`autoexamples = false`, task 20260719-193728), so the catalog is the
//! SINGLE source of truth for what "an example" is - probe's multi-run
//! specs (`probe run gameplay`, `--all`) resolve against it, and the root
//! package's `catalog_matches_disk` drift test calls the same parser so
//! the two consumers can never disagree about the format.

use std::path::Path;

/// One `[[example]]` catalog entry: the cargo target name, its manifest
/// path, and the category (the `examples/<category>/...` path segment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogExample {
    pub name: String,
    pub path: String,
    pub category: String,
}

/// Parse the `[[example]]` blocks out of a root Cargo.toml.
///
/// Deliberately a line-level parse of the repo's own manifest style (the
/// same shape `catalog_matches_disk` pins): a `[[example]]` header
/// followed by `name = "..."` and `path = "examples/<cat>/<file>.rs"`
/// lines. Fail-closed: a block missing either key, a path outside a
/// category dir, a duplicate name, a name colliding with a category, or
/// a manifest without `autoexamples = false` is an ERROR, not a guess -
/// with discovery off, a malformed catalog means examples silently do
/// not build, so the parser refuses to paper over it.
pub fn parse_example_catalog(manifest: &str) -> Result<Vec<CatalogExample>, String> {
    if !manifest.contains("autoexamples = false") {
        return Err(
            "the manifest does not set `autoexamples = false` - the [[example]] catalog \
             is only authoritative with auto-discovery off"
                .into(),
        );
    }
    let mut examples = Vec::new();
    let mut in_example = false;
    let (mut name, mut path): (Option<String>, Option<String>) = (None, None);
    let flush = |name: &mut Option<String>,
                 path: &mut Option<String>,
                 examples: &mut Vec<CatalogExample>|
     -> Result<(), String> {
        match (name.take(), path.take()) {
            (Some(name), Some(path)) => {
                let category = path
                    .strip_prefix("examples/")
                    .and_then(|rest| rest.split_once('/'))
                    .map(|(category, _)| category.to_string())
                    .ok_or_else(|| {
                        format!("example {name}: path {path} is not examples/<category>/<file>")
                    })?;
                examples.push(CatalogExample {
                    name,
                    path,
                    category,
                });
                Ok(())
            }
            (None, None) => Ok(()),
            (Some(name), None) => Err(format!("[[example]] {name} has no path")),
            (None, Some(path)) => Err(format!("[[example]] at {path} has no name")),
        }
    };
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            flush(&mut name, &mut path, &mut examples)?;
            in_example = line == "[[example]]";
            continue;
        }
        if !in_example {
            continue;
        }
        if let Some(value) = line.strip_prefix("name = ") {
            name = Some(value.trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("path = ") {
            path = Some(value.trim_matches('"').to_string());
        }
    }
    flush(&mut name, &mut path, &mut examples)?;
    if examples.is_empty() {
        return Err("the manifest has no [[example]] blocks".into());
    }
    for (i, example) in examples.iter().enumerate() {
        if examples[..i].iter().any(|prior| prior.name == example.name) {
            return Err(format!("duplicate [[example]] name {}", example.name));
        }
        if examples.iter().any(|any| any.category == example.name) {
            return Err(format!(
                "example name {} collides with a category name - spec resolution \
                 (name-or-category) needs them disjoint",
                example.name
            ));
        }
    }
    Ok(examples)
}

/// Load and parse the catalog from `<root>/Cargo.toml`.
pub fn load_example_catalog(root: &Path) -> Result<Vec<CatalogExample>, String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("could not read {}: {e}", manifest_path.display()))?;
    parse_example_catalog(&manifest)
}

/// The distinct categories, in catalog order.
pub fn categories(catalog: &[CatalogExample]) -> Vec<&str> {
    let mut seen = Vec::new();
    for example in catalog {
        if !seen.contains(&example.category.as_str()) {
            seen.push(example.category.as_str());
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
[package]
name = "game"
autoexamples = false

[[example]]
name = "controller_section"
path = "examples/sections/controller_section.rs"

[[example]]
name = "scenario"
path = "examples/gameplay/scenario.rs"

[lib]
path = "src/lib.rs"
"#;

    #[test]
    fn parses_blocks_with_categories_in_order() {
        let catalog = parse_example_catalog(GOOD).unwrap();
        assert_eq!(
            catalog,
            vec![
                CatalogExample {
                    name: "controller_section".into(),
                    path: "examples/sections/controller_section.rs".into(),
                    category: "sections".into(),
                },
                CatalogExample {
                    name: "scenario".into(),
                    path: "examples/gameplay/scenario.rs".into(),
                    category: "gameplay".into(),
                },
            ]
        );
        assert_eq!(categories(&catalog), vec!["sections", "gameplay"]);
    }

    #[test]
    fn refuses_a_manifest_with_discovery_on() {
        let manifest = GOOD.replace("autoexamples = false", "");
        let err = parse_example_catalog(&manifest).unwrap_err();
        assert!(err.contains("autoexamples"), "{err}");
    }

    #[test]
    fn refuses_a_block_missing_its_path() {
        let manifest = format!("{GOOD}\n[[example]]\nname = \"orphan\"\n");
        let err = parse_example_catalog(&manifest).unwrap_err();
        assert!(err.contains("orphan has no path"), "{err}");
    }

    #[test]
    fn refuses_an_uncategorized_path() {
        let manifest = GOOD.replace("examples/gameplay/scenario.rs", "examples/scenario.rs");
        let err = parse_example_catalog(&manifest).unwrap_err();
        assert!(err.contains("not examples/<category>/<file>"), "{err}");
    }

    #[test]
    fn refuses_duplicates_and_name_category_collisions() {
        let dup = format!(
            "{GOOD}\n[[example]]\nname = \"scenario\"\npath = \"examples/ui/scenario.rs\"\n"
        );
        assert!(parse_example_catalog(&dup)
            .unwrap_err()
            .contains("duplicate"));

        let collide = format!(
            "{GOOD}\n[[example]]\nname = \"sections\"\npath = \"examples/ui/sections.rs\"\n"
        );
        assert!(parse_example_catalog(&collide)
            .unwrap_err()
            .contains("collides with a category"));
    }
}
