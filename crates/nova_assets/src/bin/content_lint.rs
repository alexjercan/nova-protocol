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
//!
//! Mod developers can lint just their own mod (task 20260716-204618):
//!
//! ```text
//! cargo run -p nova_assets --bin content_lint -- --target the-ledger
//! cargo run -p nova_assets --bin content_lint -- --target path/to/my-mod
//! ```
//!
//! `--target` takes a mod directory (anywhere on disk - the dir name is the
//! mod id, portal-style) or an in-repo id (`webmods/<id>`,
//! `assets/mods/<id>`, or `base`). Base sections and every in-repo scenario
//! stay visible to the checks, so chains into base content resolve.

use nova_scenario::prelude::LintSeverity;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let issues = match args.as_slice() {
        [] => nova_assets::lint_walk::lint_content_tree(),
        [flag, target] if flag == "--target" => {
            let Some(dir) = nova_assets::lint_walk::resolve_target(target) else {
                eprintln!(
                    "content_lint: no mod named '{target}' (not a directory, not under webmods/ or assets/mods/, not 'base')"
                );
                return std::process::ExitCode::FAILURE;
            };
            println!("content_lint: target {}", dir.display());
            nova_assets::lint_walk::lint_target(&dir)
        }
        _ => {
            eprintln!("usage: content_lint [--target <mod-dir-or-id>]");
            return std::process::ExitCode::FAILURE;
        }
    };
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
