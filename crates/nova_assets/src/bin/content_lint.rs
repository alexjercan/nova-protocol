//! Lint the repo's content tree (task 20260716-191543):
//!
//! ```text
//! cargo run -p nova_assets --bin content_lint
//! ```
//!
//! Walks `assets/base`, `assets/mods/*` and `webmods/*`, runs the
//! identifier-level checks the load/publish gates cannot make (unknown
//! section prototypes, dangling NextScenario targets, unspawnable filter
//! targets, duplicate ids; warns on unset variables and unmatched
//! ObjectiveComplete). Exits non-zero on any Error - CI runs the same walk
//! via the `content_lint_gate` test.

use nova_scenario::prelude::LintSeverity;

fn main() -> std::process::ExitCode {
    let issues = nova_assets::lint_walk::lint_content_tree();
    let mut errors = 0;
    for (bundle, issue) in &issues {
        let tag = match issue.severity {
            LintSeverity::Error => {
                errors += 1;
                "ERROR"
            }
            LintSeverity::Warn => "WARN ",
        };
        println!(
            "{tag} [{bundle}] scenario '{}': {}",
            issue.scenario, issue.message
        );
    }
    if errors > 0 {
        println!(
            "content_lint: {errors} error(s), {} finding(s) total",
            issues.len()
        );
        std::process::ExitCode::FAILURE
    } else {
        println!("content_lint: clean ({} warning(s))", issues.len());
        std::process::ExitCode::SUCCESS
    }
}
