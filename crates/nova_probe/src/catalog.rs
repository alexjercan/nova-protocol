//! The example catalog, parsed from the root Cargo.toml's `[[example]]`
//! blocks.
//!
//! The examples live in category subdirectories with auto-discovery OFF
//! (`autoexamples = false`), so the catalog is the SINGLE source of truth
//! for what "an example" is - probe's multi-run
//! specs (`probe run ui`, `--all`) resolve against it, and the root
//! package's `catalog_matches_disk` drift test calls the same parser so
//! the two consumers can never disagree about the format.
//!
//! The catalog answers WHAT the examples are; [`CATEGORY_POLICIES`] answers
//! what probe DOES with each category.

use std::path::Path;

/// One `[[example]]` catalog entry: the cargo target name, its manifest
/// path, and the category (the `examples/<category>/...` path segment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogExample {
    /// The cargo target name of the example.
    pub name: String,
    /// The example's source path (`examples/<category>/<file>.rs`).
    pub path: String,
    /// The category segment of the example's path.
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

/// What probe does with a whole category - the run policy side of the
/// example-category contract (the prose side lives in the root Cargo.toml's
/// per-category comments and the dev wiki's category table).
///
/// Two axes, because two is all the contract has: whether probe runs the
/// category at all, and whether running it carries a frame-time pass.
/// "Runs correctness" and "is in `--all`" are the same column here - there
/// is no category that runs under an explicit spec but not under `--all`,
/// so they stay one boolean until a caller needs them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryPolicy {
    /// Probe runs this category at all: gates both `--all` and bare category
    /// expansion. False means the category leaves probe's scope entirely -
    /// `probe run <category>` is an error, not an empty run.
    pub probed: bool,
    /// Running it carries the `--fps` frame-time pass. False means `--fps`
    /// records a skip reason instead: the category makes no frame-time claim,
    /// so there is no window to measure and nothing to compare against.
    pub frame_time: bool,
}

/// The policy table: one explicit row per category on disk.
///
/// A table rather than scattered predicates so that a new category is a
/// compile-time edit next to [`CatalogExample`], and so the root package's
/// `every_category_has_a_probe_policy` can enumerate it and turn a missing
/// row into a red test rather than a silent [`category_policy`] default.
pub const CATEGORY_POLICIES: &[(&str, CategoryPolicy)] = &[
    // sections/ - one test range per ship section. Correctness curriculum:
    // it asserts behavior, never frame cost.
    (
        "sections",
        CategoryPolicy {
            probed: true,
            frame_time: false,
        },
    ),
    // systems/ - code-built fixtures for whole-system behavior. Correctness,
    // like sections/, at a coarser grain.
    (
        "systems",
        CategoryPolicy {
            probed: true,
            frame_time: false,
        },
    ),
    // stress/ - the ONLY home for frame-time claims: steady-state scenes
    // built to be measured and compared against a baseline.
    (
        "stress",
        CategoryPolicy {
            probed: true,
            frame_time: true,
        },
    ),
    // ui/ - staged UI flows. Correctness of layout and flow; a UI example's
    // frame cost is not a claim anyone reads.
    (
        "ui",
        CategoryPolicy {
            probed: true,
            frame_time: false,
        },
    ),
    // screenshots/ - capture producers, judged by their PNGs and human eyes.
    // Outside probe's scope: a probe verdict on one would assert nothing.
    (
        "screenshots",
        CategoryPolicy {
            probed: false,
            frame_time: false,
        },
    ),
    // TRANSITIONAL: remove with 20260804-094006, which absorbs perf_baseline
    // into stress/.
    (
        "perf",
        CategoryPolicy {
            probed: true,
            frame_time: true,
        },
    ),
];

/// The run policy for a category.
///
/// An unlisted category falls back to probed-without-frame-time: probe still
/// runs it (the conservative choice - a new category is more likely a
/// correctness one than a no-op), but claims no frame-time window for it.
/// That default is a backstop, never a design: the root package's
/// `every_category_has_a_probe_policy` fails if any category on disk reaches
/// it.
pub fn category_policy(category: &str) -> CategoryPolicy {
    CATEGORY_POLICIES
        .iter()
        .find(|(name, _)| *name == category)
        .map(|(_, policy)| *policy)
        .unwrap_or(CategoryPolicy {
            probed: true,
            frame_time: false,
        })
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

    #[test]
    fn the_policy_table_has_no_duplicate_rows() {
        // A duplicate would make the table's second row unreachable and the
        // contract ambiguous about which one is the contract.
        for (i, (name, _)) in CATEGORY_POLICIES.iter().enumerate() {
            assert!(
                !CATEGORY_POLICIES[..i]
                    .iter()
                    .any(|(prior, _)| prior == name),
                "duplicate policy row for category {name}"
            );
        }
    }

    #[test]
    fn an_unlisted_category_gets_the_conservative_default() {
        // The backstop, pinned so the fallback stays a KNOWN shape (probed,
        // no frame-time claim) - `every_category_has_a_probe_policy` is what
        // keeps any real category from reaching it.
        assert_eq!(
            category_policy("not_a_category"),
            CategoryPolicy {
                probed: true,
                frame_time: false,
            }
        );
    }
}
