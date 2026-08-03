//! Spec resolution: an example name or a category dir resolved against the
//! Cargo.toml example catalog.

/// Examples that `--all` and category expansion SKIP, each with its
/// reason - listed in the aggregate report so the absence reads as a
/// decision (no silent caps). An explicit `probe run <name>` still runs
/// one (operator's choice, with a printed note).
pub(crate) const NOT_PROBED: &[(&str, &str)] = &[(
    "render_scale_shot",
    "NOVA_SHOT real-GPU pixel capture with no self-ending autopilot: under \
     probe's Xvfb it would time out, and its point (correct pixels) needs \
     a real GPU and human eyes",
)];

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
    /// What expansion skipped, with reasons (empty for explicit names).
    pub excluded: Vec<(String, String)>,
}

/// Resolve spec tokens against the catalog: an exact example name wins,
/// else a category dir name expands to its members (minus NOT_PROBED),
/// else an error that lists the catalog. Pure - the catalog is injected
/// (the probe-env pattern), so every branch is unit-testable.
pub(crate) fn resolve_spec(
    tokens: &[String],
    all: bool,
    catalog: &[nova_probe::CatalogExample],
    not_probed: &[(&str, &str)],
) -> Result<Resolved, String> {
    let excluded_reason = |name: &str| {
        not_probed
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(n, r)| (n.to_string(), r.to_string()))
    };
    if all {
        let mut excluded = Vec::new();
        let examples = catalog
            .iter()
            .filter(|example| match excluded_reason(&example.name) {
                Some(entry) => {
                    excluded.push(entry);
                    false
                }
                None => true,
            })
            .map(|example| example.name.clone())
            .collect();
        return Ok(Resolved {
            examples,
            multi: true,
            spec_display: "--all".into(),
            excluded,
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
    let mut excluded = Vec::new();
    let mut multi = tokens.len() > 1;
    for token in tokens {
        if catalog.iter().any(|example| example.name == *token) {
            // Explicit names run even when NOT_PROBED lists them - the
            // operator asked; the driver prints the note.
            if !examples.contains(token) {
                examples.push(token.clone());
            }
        } else if categories.contains(&token.as_str()) {
            multi = true;
            for example in catalog.iter().filter(|e| e.category == *token) {
                match excluded_reason(&example.name) {
                    Some(entry) => {
                        if !excluded.contains(&entry) {
                            excluded.push(entry);
                        }
                    }
                    None => {
                        if !examples.contains(&example.name) {
                            examples.push(example.name.clone());
                        }
                    }
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
        excluded,
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

    const EXCLUDED: &[(&str, &str)] = &[("render_scale_shot", "needs a real GPU")];

    #[test]
    fn resolve_single_name_stays_single() {
        let resolved = resolve_spec(&s(&["playable"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["playable"]));
        assert!(!resolved.multi, "a bare name keeps single-run semantics");
        assert!(resolved.excluded.is_empty());
    }

    #[test]
    fn resolve_list_and_category_expand() {
        let resolved =
            resolve_spec(&s(&["playable", "scenario"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["playable", "scenario"]));
        assert!(resolved.multi);

        let resolved = resolve_spec(&s(&["screenshots"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["screenshot_reel"]),
            "category expansion skips NOT_PROBED members"
        );
        assert!(
            resolved.multi,
            "a category is a multi spec even with one member"
        );
        assert_eq!(resolved.excluded.len(), 1, "and records what it skipped");
    }

    #[test]
    fn resolve_all_and_explicit_excluded() {
        let resolved = resolve_spec(&[], true, &catalog(), EXCLUDED).unwrap();
        assert!(
            !resolved.examples.contains(&"render_scale_shot".to_string()),
            "--all skips NOT_PROBED"
        );
        assert_eq!(resolved.examples.len(), 5);
        assert_eq!(resolved.spec_display, "--all");
        assert_eq!(
            resolved.excluded,
            vec![(
                "render_scale_shot".to_string(),
                "needs a real GPU".to_string()
            )]
        );

        // An explicit name overrides the exclusion (operator's choice).
        let resolved =
            resolve_spec(&s(&["render_scale_shot"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["render_scale_shot"]));
        assert!(!resolved.multi);
    }

    #[test]
    fn resolve_errors_list_the_catalog() {
        let err = resolve_spec(&[], false, &catalog(), EXCLUDED).unwrap_err();
        assert!(err.contains("examples by category"), "{err}");
        assert!(err.contains("gameplay: scenario, playable"), "{err}");
        assert!(err.contains("--all"), "{err}");

        let err = resolve_spec(&s(&["typo"]), false, &catalog(), EXCLUDED).unwrap_err();
        assert!(err.contains("unknown example or category `typo`"), "{err}");
        assert!(err.contains("examples by category"), "{err}");
    }

    #[test]
    fn resolve_dedupes_overlapping_tokens() {
        let resolved =
            resolve_spec(&s(&["playable", "gameplay"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["playable", "scenario"]),
            "a name already included is not repeated by its category"
        );
    }
}
