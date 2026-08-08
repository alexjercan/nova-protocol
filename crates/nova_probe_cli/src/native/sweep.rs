//! The sequential multi-example driver and the aggregate index it writes.

use std::{path::Path, process::ExitCode, time::Instant};

use super::{
    cli::{Platform, RunOptions},
    paths::{
        baseline_for, default_output_base, default_output_root, git_history_short, repo_root,
        resolve_baseline_root, resolve_full_git_sha,
    },
    run::run,
    spec::{resolve_spec, Resolved},
    supervise::ensure_display,
};
use crate::evaluation::run_identity;

/// Dispatch a parsed run spec: resolve against the catalog, then run the
/// resolved examples through the aggregate-shaped driver. `--platform web`
/// bypasses resolution - its positional is a SCENARIO id, not an example.
pub(crate) fn run_spec(
    tokens: &[String],
    all: bool,
    mut base: RunOptions,
) -> Result<ExitCode, String> {
    if base.platform == Platform::Web {
        if all || tokens.len() != 1 {
            return Err(
                "--platform web takes exactly one scenario id; it does not combine \
                 with a list/category/--all spec"
                    .into(),
            );
        }
        base.example = tokens[0].clone();
        return run(&base);
    }
    let root = repo_root();
    let catalog = crate::load_example_catalog(&root)?;
    let resolved = resolve_spec(tokens, all, &catalog)?;
    // Multi gates: these flags are single-example concerns.
    if resolved.multi && (!base.scenarios.is_empty() || !base.presets.is_empty()) {
        return Err(
            "the --scenario/--preset matrix is a single-example perf sweep; \
             give one example"
                .into(),
        );
    }
    run_many(&resolved, &base, &catalog)
}

/// The sequential run driver: each spec item goes through `run()` into
/// `<out-base>/<short-sha>/<example>/`, then one row per example is built
/// from ITS checks.json (probe consumes its own agent surface),
/// continue-on-failure, then the aggregate index. ONE Xvfb is shared
/// across the whole sweep. NOTE: every run in this process derives the SAME
/// pid-based display, so per-run spawn/kill raced the old server's lock
/// teardown and Xvfb died "immediately - display in use" mid-fleet. One
/// spawn, one kill; run()'s explicit-display path skips its own spawn.
fn run_many(
    resolved: &Resolved,
    base: &RunOptions,
    catalog: &[crate::CatalogExample],
) -> Result<ExitCode, String> {
    let root = repo_root();
    let (git_sha, host) = run_identity();
    let full_git_sha = resolve_full_git_sha(&root);
    let out_base = default_output_base(&root, base.out.clone());
    std::fs::create_dir_all(&out_base).map_err(|e| format!("could not create out base: {e}"))?;
    let out_base = out_base
        .canonicalize()
        .map_err(|e| format!("could not resolve out base: {e}"))?;
    let out_root = default_output_root(&root, Some(out_base.clone()), &git_sha);
    std::fs::create_dir_all(&out_root).map_err(|e| format!("could not create out root: {e}"))?;
    let out_root = out_root
        .canonicalize()
        .map_err(|e| format!("could not resolve out root: {e}"))?;
    let baseline_base = base.baseline.clone().unwrap_or_else(|| out_base.clone());
    let baseline_root = resolve_baseline_root(
        &baseline_base,
        &git_sha,
        &git_history_short(&root),
        base.baseline.is_some(),
    );
    match (&base.baseline, &baseline_root) {
        (Some(base), Some(root)) => {
            eprintln!("probe: baseline {} -> {}", base.display(), root.display())
        }
        (None, Some(root)) => eprintln!(
            "probe: auto baseline in {} -> {}",
            out_base.display(),
            root.display()
        ),
        (Some(base), None) => eprintln!(
            "probe: no baseline commit dir found in {}; skipping fps comparison",
            base.display()
        ),
        (None, None) => eprintln!(
            "probe: no previous baseline commit dir found in {}; skipping fps comparison",
            out_base.display()
        ),
    }
    let (display, _xvfb) = ensure_display(base.display.as_deref())?;
    let started_unix = unix_now();
    let total = resolved.examples.len();
    let mut rows = Vec::new();
    let mut baseline_matches = 0usize;
    for (i, example) in resolved.examples.iter().enumerate() {
        eprintln!("probe: ===== {example} [{}/{total}] =====", i + 1);
        let mut opts = base.clone();
        opts.example = example.clone();
        opts.out = Some(out_root.join(example));
        opts.display = Some(display.clone());
        opts.baseline = baseline_root.as_ref().and_then(|root| {
            baseline_for(root, example).map_or_else(
                || {
                    eprintln!(
                        "probe: {example}: no baseline in {}, skipping fps comparison",
                        root.display()
                    );
                    None
                },
                |dir| {
                    baseline_matches += 1;
                    Some(dir)
                },
            )
        });
        let started = Instant::now();
        // Sampled BEFORE the run, so a checks.json stamped earlier than this
        // cannot be this example's.
        let cell_started_unix = unix_now();
        let run_error = match run(&opts) {
            Ok(_) => None,
            Err(message) => {
                eprintln!("probe: {example}: {message}; continuing with the next example");
                Some(message)
            }
        };
        let category = catalog
            .iter()
            .find(|entry| entry.name == *example)
            .map(|entry| entry.category.clone())
            .unwrap_or_default();
        rows.push(build_row(
            example,
            &category,
            &out_root.join(example),
            run_error,
            started.elapsed().as_secs(),
            &RunStamp {
                git_sha: &git_sha,
                started_not_before: cell_started_unix,
            },
        ));
    }
    if let Some(root) = &baseline_root {
        if baseline_matches == 0 {
            eprintln!(
                "probe: warning: baseline {} matched none of the {total} example(s) - \
                 expected <commit-root>/<example>/frametime.csv",
                root.display()
            );
        }
    }
    let manifest = crate::AllManifest {
        spec: resolved.spec_display.clone(),
        started_unix,
        git_sha,
        full_git_sha,
        host,
        rows,
    };
    write_aggregate(&out_root, &manifest)?;
    print_aggregate(&out_root, &manifest);
    Ok(aggregate_exit(&manifest))
}

/// Wall-clock unix seconds, the same stamp `run` writes into the manifest.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Who a row's checks.json must belong to. A run that fails BEFORE
/// `clean_out_dir` leaves the PREVIOUS run's checks.json on disk, and
/// nothing in the file itself says which run wrote it.
pub(crate) struct RunStamp<'a> {
    /// The revision under test. Every row of one sweep shares it.
    pub git_sha: &'a str,
    /// Earliest start the checks.json may record. `0` accepts any, for
    /// re-rendering a dir whose runs are already history.
    pub started_not_before: u64,
}

impl RunStamp<'_> {
    /// Whether `checks` was written by the run this row is about. The
    /// identity lives in the manifest probe embeds under `run`; a checks.json
    /// without one cannot vouch for itself.
    fn matches(&self, checks: &serde_json::Value) -> bool {
        let run = &checks["run"];
        run.get("git_sha").and_then(|v| v.as_str()) == Some(self.git_sha)
            && run
                .get("started_unix")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= self.started_not_before
    }
}

/// One aggregate row, read back from the run's own checks.json. A run
/// that never produced one (build failure, probe error) becomes an
/// ERROR row carrying the message - the sweep must show it, not skip it.
///
/// A run that ERRORED cannot verdict better than ERROR whatever is on disk,
/// and a checks.json that fails `stamp` is a previous run's: both used to
/// present a stale OK, and `aggregate_exit` returned SUCCESS on a commit
/// that was never probed.
pub(crate) fn build_row(
    example: &str,
    category: &str,
    dir: &Path,
    run_error: Option<String>,
    duration_secs: u64,
    stamp: &RunStamp<'_>,
) -> crate::AllRow {
    let on_disk = std::fs::read_to_string(dir.join("checks.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok());
    let error_row = |reason: String| crate::AllRow {
        example: example.into(),
        category: category.into(),
        verdict: "ERROR".into(),
        measured: "-".into(),
        checks: Vec::new(),
        duration_secs,
        error: Some(reason),
    };
    match (on_disk, run_error) {
        (_, Some(error)) => error_row(error),
        (None, None) => error_row("the run produced no checks.json".into()),
        (Some(value), None) if !stamp.matches(&value) => error_row(format!(
            "{} holds an earlier run's checks.json; this run wrote none",
            dir.display()
        )),
        (Some(value), None) => crate::AllRow {
            example: example.into(),
            category: category.into(),
            verdict: value
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("ERROR")
                .into(),
            measured: value
                .get("measured")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .into(),
            checks: value
                .get("checks")
                .and_then(|c| c.as_array())
                .map(|checks| {
                    checks
                        .iter()
                        .filter_map(|check| {
                            Some((
                                check.get("name")?.as_str()?.to_string(),
                                check.get("status")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            duration_secs,
            error: None,
        },
    }
}

pub(crate) fn write_aggregate(
    out_base: &Path,
    manifest: &crate::AllManifest,
) -> Result<(), String> {
    std::fs::write(
        out_base.join("probe-all.json"),
        format!("{:#}\n", manifest.to_json()),
    )
    .map_err(|e| format!("could not write probe-all.json: {e}"))?;
    std::fs::write(
        out_base.join("index.json"),
        format!("{:#}\n", crate::index_json(manifest)),
    )
    .map_err(|e| format!("could not write index.json: {e}"))?;
    std::fs::write(out_base.join("index.html"), crate::render_index(manifest))
        .map_err(|e| format!("could not write index.html: {e}"))
}

pub(crate) fn print_aggregate(out_base: &Path, manifest: &crate::AllManifest) {
    let overall = crate::aggregate_verdict(&manifest.rows);
    println!(
        "probe: aggregate {overall} - {}",
        out_base.join("index.html").display()
    );
    for row in &manifest.rows {
        println!(
            "  {:<24} {:<8} measured {:>4}  {}s",
            row.example, row.verdict, row.measured, row.duration_secs
        );
    }
}

pub(crate) fn aggregate_exit(manifest: &crate::AllManifest) -> ExitCode {
    match crate::aggregate_verdict(&manifest.rows) {
        "OK" | "WARN" => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "61675034";

    /// A cell dir holding the PREVIOUS run's green checks.json, which is what
    /// is on disk whenever `run` fails before `clean_out_dir` gets to it.
    fn stale_cell(name: &str, started_unix: u64) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nova_probe_row_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("checks.json"),
            serde_json::json!({
                "verdict": "OK",
                "measured": "5/7",
                "checks": [{ "name": "process_exit", "status": "PASS" }],
                "run": { "git_sha": SHA, "started_unix": started_unix },
            })
            .to_string(),
        )
        .unwrap();
        dir
    }

    fn stamp(started_not_before: u64) -> RunStamp<'static> {
        RunStamp {
            git_sha: SHA,
            started_not_before,
        }
    }

    /// The green-and-wrong row: the run failed, but the previous run's OK is
    /// still on disk. Reading it verbatim is how the sweep exited SUCCESS on a
    /// commit that was never probed.
    #[test]
    fn an_errored_run_verdicts_error_over_a_leftover_green_checks_json() {
        let dir = stale_cell("errored", 100);
        let row = build_row(
            "playable",
            "gameplay",
            &dir,
            Some("the build failed".into()),
            3,
            &stamp(200),
        );
        assert_eq!(row.verdict, "ERROR");
        assert_eq!(row.error.as_deref(), Some("the build failed"));
        assert!(
            row.checks.is_empty(),
            "a stale run's checks are not evidence"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The other half: no error was reported, so only the stamp can tell that
    /// the checks.json predates this run.
    #[test]
    fn a_checks_json_older_than_the_run_cannot_stand_in_for_it() {
        let dir = stale_cell("stale", 100);
        let row = build_row("playable", "gameplay", &dir, None, 3, &stamp(200));
        assert_eq!(row.verdict, "ERROR");
        let reason = row.error.unwrap_or_default();
        assert!(reason.contains("earlier run's checks.json"), "{reason}");

        // Written by this run, and the same row reads normally.
        let fresh = stale_cell("fresh", 300);
        let row = build_row("playable", "gameplay", &fresh, None, 3, &stamp(200));
        assert_eq!(row.verdict, "OK");
        assert_eq!(row.error, None);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fresh);
    }

    /// A different revision's checks.json is never this run's, however recent.
    #[test]
    fn a_checks_json_from_another_revision_is_rejected() {
        let dir = stale_cell("other_sha", 300);
        let row = build_row(
            "playable",
            "gameplay",
            &dir,
            None,
            3,
            &RunStamp {
                git_sha: "deadbeef",
                started_not_before: 0,
            },
        );
        assert_eq!(row.verdict, "ERROR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
