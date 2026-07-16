//! The CI half of the content lint (task 20260716-191543): the same tree
//! walk the `content_lint` bin runs, asserted clean of Error-level issues.
//! Warns are printed but do not fail - they are authoring smells, not
//! broken references. See `nova_scenario::lint` for the check list and
//! `cargo run -p nova_assets --bin content_lint` for the author CLI.

use nova_scenario::prelude::LintSeverity;

#[test]
fn repo_content_tree_has_no_lint_errors() {
    let issues = nova_assets::lint_walk::lint_content_tree();
    let mut errors = Vec::new();
    for (bundle, issue) in &issues {
        match issue.severity {
            LintSeverity::Error => errors.push(format!(
                "[{bundle}] scenario '{}': {}",
                issue.scenario, issue.message
            )),
            LintSeverity::Warn => println!(
                "WARN [{bundle}] scenario '{}': {}",
                issue.scenario, issue.message
            ),
        }
    }
    assert!(
        errors.is_empty(),
        "content lint errors (fix the content or the lint):\n{}",
        errors.join("\n")
    );
}
