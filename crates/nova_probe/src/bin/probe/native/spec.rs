//! Spec resolution: an example name or a category dir resolved against the
//! Cargo.toml example catalog.

/// Categories `--all` and bare category expansion SKIP, each with its
/// reason. This is the ONE launch-side opinion left: what an example can be
/// judged on is now its own runtime declaration
/// (`nova_probe::contract`), but whether to SPAWN it at all cannot be - that
/// answer is needed before the process exists. There is no per-EXAMPLE
/// opt-out: an example that cannot survive a probe run FAILS, loudly.
pub(crate) const NOT_PROBED_CATEGORIES: &[(&str, &str)] = &[(
    "screenshots",
    "capture producers, judged by their PNGs and human eyes: a probe \
     verdict on one would assert nothing",
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
    /// Entries carry the contract's trailing slash (`screenshots/`) so a
    /// skipped category can never be read as an example name.
    pub excluded: Vec<(String, String)>,
}

/// Why an unprobed category is absent from an expansion - the category
/// side of `excluded`, phrased for the aggregate report.
fn unprobed_reason(category: &str, reason: &str) -> String {
    format!("category `{category}/` is not a probe target - {reason}")
}

/// Resolve spec tokens against the catalog: an exact example name wins,
/// else a category dir name expands to its members, else an error that
/// lists the catalog. Pure - the catalog is injected (the probe-env
/// pattern), so every branch is unit-testable.
///
/// One exclusion axis, injected and recorded in `excluded` so the aggregate
/// report shows an absence as a decision: [`NOT_PROBED_CATEGORIES`]. An
/// explicit example name overrides it - the operator asked for it by name.
pub(crate) fn resolve_spec(
    tokens: &[String],
    all: bool,
    catalog: &[nova_probe::CatalogExample],
    not_probed_categories: &[(&str, &str)],
) -> Result<Resolved, String> {
    let category_reason = |category: &str| {
        not_probed_categories
            .iter()
            .find(|(n, _)| *n == category)
            .map(|(_, reason)| unprobed_reason(category, reason))
    };
    if all {
        let mut excluded = Vec::new();
        let mut examples = Vec::new();
        for example in catalog {
            // The category comes first: an unprobed one is out wholesale,
            // recorded once by CATEGORY rather than once per member (the
            // decision was made about the category).
            if let Some(reason) = category_reason(&example.category) {
                let entry = (format!("{}/", example.category), reason);
                if !excluded.contains(&entry) {
                    excluded.push(entry);
                }
                continue;
            }
            examples.push(example.name.clone());
        }
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
    let mut multi = tokens.len() > 1;
    for token in tokens {
        if catalog.iter().any(|example| example.name == *token) {
            // An explicit name runs even from an unprobed category - the
            // operator asked for it.
            if !examples.contains(token) {
                examples.push(token.clone());
            }
        } else if categories.contains(&token.as_str()) {
            // A bare unprobed category ERRORS rather than expanding to an
            // empty run: "not a probe target" is the honest answer, and a
            // silent no-op would read as a pass.
            if let Some(reason) = category_reason(token) {
                return Err(format!("{reason}\n{}", spec_help(catalog)));
            }
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
        // A named spec excludes nothing: an unprobed category errors above
        // rather than shrinking, and a name always runs.
        excluded: Vec::new(),
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

    /// The one exclusion axis, injected the way production injects it.
    const EXCLUDED_CATEGORIES: &[(&str, &str)] = &[("screenshots", "judged by their PNGs")];

    #[test]
    fn resolve_single_name_stays_single() {
        let resolved =
            resolve_spec(&s(&["scenario"]), false, &catalog(), EXCLUDED_CATEGORIES).unwrap();
        assert_eq!(resolved.examples, s(&["scenario"]));
        assert!(!resolved.multi, "a bare name keeps single-run semantics");
        assert!(resolved.excluded.is_empty());
    }

    #[test]
    fn resolve_list_and_category_expand() {
        let resolved = resolve_spec(
            &s(&["playable", "scenario"]),
            false,
            &catalog(),
            EXCLUDED_CATEGORIES,
        )
        .unwrap();
        assert_eq!(resolved.examples, s(&["playable", "scenario"]));
        assert!(resolved.multi);

        let resolved =
            resolve_spec(&s(&["gameplay"]), false, &catalog(), EXCLUDED_CATEGORIES).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["scenario", "playable"]),
            "a probed category expands to ALL its members - there is no \
             per-example opt-out any more"
        );
        assert!(
            resolved.multi,
            "a category is a multi spec even with one member"
        );
        assert!(resolved.excluded.is_empty());
    }

    /// An unprobed category leaves probe's scope entirely: `--all` records
    /// the absence as a decision instead of quietly shrinking, and a bare
    /// expansion errors instead of running nothing. What a probed example is
    /// JUDGED on is no longer decided here - the example declares it.
    #[test]
    fn an_unprobed_category_is_recorded_never_silently_dropped() {
        let resolved = resolve_spec(&[], true, &catalog(), EXCLUDED_CATEGORIES).unwrap();
        assert!(
            !resolved
                .examples
                .iter()
                .any(|e| e.starts_with("screenshot")),
            "--all skips an unprobed category: {:?}",
            resolved.examples
        );
        // Exact, not `contains`: the fixture has TWO screenshots members, so
        // this pins the dedupe - recorded ONCE, by category, named with the
        // contract's trailing slash so it cannot be read as an example.
        assert_eq!(
            resolved.excluded,
            vec![(
                "screenshots/".to_string(),
                unprobed_reason("screenshots", EXCLUDED_CATEGORIES[0].1),
            )]
        );

        // Bare expansion is gated by the same boolean: an error, never an
        // empty run that reads as a pass.
        let err =
            resolve_spec(&s(&["screenshots"]), false, &catalog(), EXCLUDED_CATEGORIES).unwrap_err();
        assert!(
            err.contains("`screenshots/` is not a probe target"),
            "{err}"
        );
        assert!(err.contains("examples by category"), "{err}");

        // An explicit member still runs - the operator asked by name.
        let resolved = resolve_spec(
            &s(&["render_scale_shot"]),
            false,
            &catalog(),
            EXCLUDED_CATEGORIES,
        )
        .unwrap();
        assert_eq!(resolved.examples, s(&["render_scale_shot"]));
    }

    #[test]
    fn resolve_all_runs_every_probed_example() {
        let resolved = resolve_spec(&[], true, &catalog(), EXCLUDED_CATEGORIES).unwrap();
        assert_eq!(
            resolved.examples,
            s(&[
                "controller_section",
                "scenario",
                "playable",
                "scene_baseline",
                "many_bodies",
                "outcomes"
            ]),
            "--all is the catalog minus unprobed CATEGORIES, nothing else"
        );
        assert_eq!(resolved.spec_display, "--all");
    }

    #[test]
    fn resolve_errors_list_the_catalog() {
        let err = resolve_spec(&[], false, &catalog(), EXCLUDED_CATEGORIES).unwrap_err();
        assert!(err.contains("examples by category"), "{err}");
        assert!(err.contains("gameplay: scenario, playable"), "{err}");
        assert!(err.contains("--all"), "{err}");

        let err = resolve_spec(&s(&["typo"]), false, &catalog(), EXCLUDED_CATEGORIES).unwrap_err();
        assert!(err.contains("unknown example or category `typo`"), "{err}");
        assert!(err.contains("examples by category"), "{err}");
    }

    #[test]
    fn resolve_dedupes_overlapping_tokens() {
        let resolved = resolve_spec(
            &s(&["playable", "gameplay"]),
            false,
            &catalog(),
            EXCLUDED_CATEGORIES,
        )
        .unwrap();
        assert_eq!(
            resolved.examples,
            s(&["playable", "scenario"]),
            "a name already included is not repeated by its category"
        );
    }
}
