//! Spec resolution: an example name or a category dir resolved against the
//! Cargo.toml example catalog.

/// A spec resolved against the example catalog.
#[derive(Debug, PartialEq)]
pub(crate) struct Resolved {
    pub examples: Vec<String>,
    /// True when the spec EXPANDS (list, category, --all) - the multi
    /// gates apply; a bare example name stays eligible for single-example
    /// matrix flags while still using the aggregate-shaped runner.
    pub multi: bool,
    /// The spec as given, for the aggregate manifest ("--all", "ui", ...).
    pub spec_display: String,
}

/// Resolve spec tokens against the catalog: an exact example name wins,
/// else a category dir name expands to its members, else an error that
/// lists the catalog. Pure - the catalog is injected (the probe-env
/// pattern), so every branch is unit-testable.
///
/// There is no exclusion axis: every cataloged example is a probe target.
/// What an example can be JUDGED on is its own runtime declaration
/// (`nova_probe::contract`) - an example that declares nothing is graded
/// UNPROBEABLE, which is an answer, not a reason to skip the spawn.
pub(crate) fn resolve_spec(
    tokens: &[String],
    all: bool,
    catalog: &[nova_probe::CatalogExample],
) -> Result<Resolved, String> {
    if all {
        return Ok(Resolved {
            examples: catalog.iter().map(|e| e.name.clone()).collect(),
            multi: true,
            spec_display: "--all".into(),
        });
    }
    if tokens.is_empty() {
        return Err(format!(
            "run needs a spec (or --all)\n{}",
            spec_help(catalog)
        ));
    }
    let categories = nova_probe::categories(catalog);
    let mut examples: Vec<String> = Vec::new();
    let mut multi = tokens.len() > 1;
    for token in tokens {
        if catalog.iter().any(|example| example.name == *token) {
            if !examples.contains(token) {
                examples.push(token.clone());
            }
        } else if categories.contains(&token.as_str()) {
            multi = true;
            for example in catalog.iter().filter(|e| e.category == *token) {
                if !examples.contains(&example.name) {
                    examples.push(example.name.clone());
                }
            }
        } else {
            return Err(format!(
                "unknown example or category `{token}`\n{}",
                spec_help(catalog)
            ));
        }
    }
    Ok(Resolved {
        examples,
        multi,
        spec_display: tokens.join(","),
    })
}

/// The catalog, listed by category, plus the spec forms - the body of
/// the bare-`probe run` error and every unknown-spec error.
fn spec_help(catalog: &[nova_probe::CatalogExample]) -> String {
    let mut help = String::from("examples by category:\n");
    for category in nova_probe::categories(catalog) {
        let members: Vec<&str> = catalog
            .iter()
            .filter(|example| example.category == category)
            .map(|example| example.name.as_str())
            .collect();
        help.push_str(&format!("  {category}: {}\n", members.join(", ")));
    }
    help.push_str(
        "forms: probe run <example>[,<example>...] | probe run <category> | probe run --all",
    );
    help
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::fixtures::{catalog, s};

    #[test]
    fn resolve_single_name_stays_single() {
        let resolved = resolve_spec(&s(&["scenario"]), false, &catalog()).unwrap();
        assert_eq!(resolved.examples, s(&["scenario"]));
        assert!(!resolved.multi, "a bare name keeps single-run semantics");
    }

    #[test]
    fn resolve_list_and_category_expand() {
        let resolved = resolve_spec(&s(&["playable", "scenario"]), false, &catalog()).unwrap();
        assert_eq!(resolved.examples, s(&["playable", "scenario"]));
        assert!(resolved.multi);

        let resolved = resolve_spec(&s(&["gameplay"]), false, &catalog()).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["scenario", "playable"]),
            "a category expands to ALL its members - there is no opt-out any more"
        );
        assert!(
            resolved.multi,
            "a category is a multi spec even with one member"
        );
    }

    /// Every category expands, `screenshots` included: capture producers are
    /// autopilot walks like any other, and what one is JUDGED on is its own
    /// runtime declaration, not a launch-side opinion.
    #[test]
    fn the_screenshots_category_expands_like_any_other() {
        let resolved = resolve_spec(&s(&["screenshots"]), false, &catalog()).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["screenshot_scene", "render_scale_shot"])
        );
        assert!(resolved.multi);
    }

    #[test]
    fn resolve_all_runs_the_whole_catalog() {
        let resolved = resolve_spec(&[], true, &catalog()).unwrap();
        assert_eq!(
            resolved.examples,
            s(&[
                "controller_section",
                "scenario",
                "playable",
                "screenshot_scene",
                "render_scale_shot",
                "scene_baseline",
                "many_bodies",
                "outcomes"
            ]),
            "--all is the catalog, nothing subtracted"
        );
        assert_eq!(resolved.spec_display, "--all");
    }

    #[test]
    fn resolve_errors_list_the_catalog() {
        let err = resolve_spec(&[], false, &catalog()).unwrap_err();
        assert!(err.contains("examples by category"), "{err}");
        assert!(err.contains("gameplay: scenario, playable"), "{err}");
        assert!(err.contains("--all"), "{err}");

        let err = resolve_spec(&s(&["typo"]), false, &catalog()).unwrap_err();
        assert!(err.contains("unknown example or category `typo`"), "{err}");
        assert!(err.contains("examples by category"), "{err}");
    }

    #[test]
    fn resolve_dedupes_overlapping_tokens() {
        let resolved = resolve_spec(&s(&["playable", "gameplay"]), false, &catalog()).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["playable", "scenario"]),
            "a name already included is not repeated by its category"
        );
    }
}
