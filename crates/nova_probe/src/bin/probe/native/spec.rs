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
    /// A per-EXAMPLE entry is named bare (`render_scale_shot`); a
    /// per-CATEGORY entry carries the contract's trailing slash
    /// (`screenshots/`), so the two axes stay distinguishable in the
    /// aggregate report and a category can never collide with a name.
    pub excluded: Vec<(String, String)>,
}

/// Why an unprobed category is absent from an expansion - the category
/// policy's side of `excluded`, phrased for the aggregate report.
fn unprobed_reason(category: &str) -> String {
    format!(
        "category `{category}/` is not a probe target - it is judged by its \
         own artifacts, not by a probe verdict"
    )
}

/// Resolve spec tokens against the catalog: an exact example name wins,
/// else a category dir name expands to its members (minus NOT_PROBED),
/// else an error that lists the catalog. Pure - the catalog is injected
/// (the probe-env pattern), so every branch is unit-testable.
///
/// Two orthogonal exclusion axes, both recorded in `excluded` so the
/// aggregate report shows an absence as a decision: the per-CATEGORY
/// `probed` policy (`nova_probe::category_policy`), and the per-EXAMPLE
/// `not_probed` list. An explicit example name overrides both - the
/// operator asked for it by name.
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
    let unprobed_entry = |category: &str| (format!("{category}/"), unprobed_reason(category));
    if all {
        let mut excluded = Vec::new();
        let mut examples = Vec::new();
        for example in catalog {
            // The category policy comes first: an unprobed category is out
            // wholesale, recorded once by CATEGORY rather than once per
            // member (the decision was made about the category).
            if !nova_probe::category_policy(&example.category).probed {
                let entry = unprobed_entry(&example.category);
                if !excluded.contains(&entry) {
                    excluded.push(entry);
                }
                continue;
            }
            match excluded_reason(&example.name) {
                Some(entry) => excluded.push(entry),
                None => examples.push(example.name.clone()),
            }
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
            // A bare unprobed category ERRORS rather than expanding to an
            // empty run: "not a probe target" is the honest answer, and a
            // silent no-op would read as a pass.
            if !nova_probe::category_policy(token).probed {
                return Err(format!(
                    "{}\n{}",
                    unprobed_reason(token),
                    spec_help(catalog)
                ));
            }
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

    /// A per-EXAMPLE exclusion inside a PROBED category, so the two
    /// exclusion axes stay separable in these tests.
    const EXCLUDED: &[(&str, &str)] = &[("playable", "needs a real GPU")];

    #[test]
    fn resolve_single_name_stays_single() {
        let resolved = resolve_spec(&s(&["scenario"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["scenario"]));
        assert!(!resolved.multi, "a bare name keeps single-run semantics");
        assert!(resolved.excluded.is_empty());
    }

    #[test]
    fn resolve_list_and_category_expand() {
        let resolved =
            resolve_spec(&s(&["playable", "scenario"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["playable", "scenario"]));
        assert!(resolved.multi);

        let resolved = resolve_spec(&s(&["gameplay"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(
            resolved.examples,
            s(&["scenario"]),
            "category expansion skips NOT_PROBED members"
        );
        assert!(
            resolved.multi,
            "a category is a multi spec even with one member"
        );
        assert_eq!(resolved.excluded.len(), 1, "and records what it skipped");
    }

    /// The category policy chooses which passes a category gets, and probe
    /// honors both halves: `stress` carries the frame-time pass,
    /// `sections`/`systems`/`ui` run correctness only, and `screenshots`
    /// leaves probe's scope entirely - `--all` records the absence as a
    /// decision instead of quietly shrinking.
    #[test]
    fn category_run_policy_selects_passes_per_category() {
        assert!(
            nova_probe::category_policy("stress").frame_time,
            "stress/ is where frame-time claims live"
        );
        for correctness_only in ["sections", "systems", "ui"] {
            let policy = nova_probe::category_policy(correctness_only);
            assert!(policy.probed, "{correctness_only} runs under probe");
            assert!(
                !policy.frame_time,
                "{correctness_only} makes no frame-time claim"
            );
        }

        let resolved = resolve_spec(&[], true, &catalog(), EXCLUDED).unwrap();
        assert!(
            !resolved
                .examples
                .iter()
                .any(|e| e.starts_with("screenshot")),
            "--all skips an unprobed category: {:?}",
            resolved.examples
        );
        // The fixture has TWO screenshots members, so this also pins the
        // dedupe: recorded ONCE, by category, named with the contract's
        // trailing slash so it cannot be read as an example.
        let recorded: Vec<&(String, String)> = resolved
            .excluded
            .iter()
            .filter(|(name, _)| name == "screenshots/")
            .collect();
        assert_eq!(
            recorded.len(),
            1,
            "--all records the skipped category once, not once per member: {:?}",
            resolved.excluded
        );
        assert!(recorded[0].1.contains("not a probe target"), "{recorded:?}");

        // Bare expansion is gated by the same boolean: an error, never an
        // empty run that reads as a pass.
        let err = resolve_spec(&s(&["screenshots"]), false, &catalog(), EXCLUDED).unwrap_err();
        assert!(
            err.contains("`screenshots/` is not a probe target"),
            "{err}"
        );
        assert!(err.contains("examples by category"), "{err}");

        // An explicit member still runs - the operator asked by name.
        let resolved =
            resolve_spec(&s(&["render_scale_shot"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["render_scale_shot"]));
    }

    #[test]
    fn resolve_all_and_explicit_excluded() {
        let resolved = resolve_spec(&[], true, &catalog(), EXCLUDED).unwrap();
        assert!(
            !resolved.examples.contains(&"playable".to_string()),
            "--all skips NOT_PROBED"
        );
        assert_eq!(
            resolved.examples,
            s(&[
                "controller_section",
                "scenario",
                "scene_baseline",
                "many_bodies",
                "outcomes"
            ])
        );
        assert_eq!(resolved.spec_display, "--all");
        // Exact, not `contains`: this pins the per-category dedupe too - the
        // fixture has two `screenshots` members, so a broken dedupe would
        // record the category twice and only an exact vector catches it.
        assert_eq!(
            resolved.excluded,
            vec![
                ("playable".to_string(), "needs a real GPU".to_string()),
                ("screenshots/".to_string(), unprobed_reason("screenshots")),
            ]
        );

        // An explicit name overrides the exclusion (operator's choice).
        let resolved = resolve_spec(&s(&["playable"]), false, &catalog(), EXCLUDED).unwrap();
        assert_eq!(resolved.examples, s(&["playable"]));
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
