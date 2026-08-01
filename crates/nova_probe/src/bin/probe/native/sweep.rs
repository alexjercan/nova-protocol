//! The sequential multi-example driver and the aggregate index it writes.

use std::{path::Path, process::ExitCode, time::Instant};

use nova_probe::run_report::run_identity;

use super::{
    cli::{Platform, RunOptions},
    paths::{
        baseline_for, default_output_base, default_output_root, git_history_short, repo_root,
        resolve_baseline_root, resolve_full_git_sha,
    },
    run::run,
    spec::{resolve_spec, Resolved, NOT_PROBED},
    supervise::ensure_display,
};

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
    let catalog = nova_probe::load_example_catalog(&root)?;
    let resolved = resolve_spec(tokens, all, &catalog, NOT_PROBED)?;
    // Multi gates: these flags are single-example concerns.
    if resolved.multi && (!base.scenarios.is_empty() || !base.presets.is_empty()) {
        return Err(
            "the --scenario/--preset matrix is a single-example perf sweep; \
             give one example"
                .into(),
        );
    }
    for example in &resolved.examples {
        if let Some((_, reason)) = NOT_PROBED.iter().find(|(name, _)| *name == example) {
            eprintln!("probe: note: {example} is excluded from --all/category runs: {reason}");
        }
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
    catalog: &[nova_probe::CatalogExample],
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
    let started_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
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
    let manifest = nova_probe::AllManifest {
        spec: resolved.spec_display.clone(),
        started_unix,
        git_sha,
        full_git_sha,
        host,
        excluded: resolved.excluded.clone(),
        rows,
    };
    write_aggregate(&out_root, &manifest)?;
    print_aggregate(&out_root, &manifest);
    Ok(aggregate_exit(&manifest))
}

/// One aggregate row, read back from the run's own checks.json. A run
/// that never produced one (build failure, probe error) becomes an
/// ERROR row carrying the message - the sweep must show it, not skip it.
pub(crate) fn build_row(
    example: &str,
    category: &str,
    dir: &Path,
    run_error: Option<String>,
    duration_secs: u64,
) -> nova_probe::AllRow {
    let checks = std::fs::read_to_string(dir.join("checks.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok());
    match checks {
        Some(value) => nova_probe::AllRow {
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
            error: run_error,
        },
        None => nova_probe::AllRow {
            example: example.into(),
            category: category.into(),
            verdict: "ERROR".into(),
            measured: "-".into(),
            checks: Vec::new(),
            duration_secs,
            error: Some(run_error.unwrap_or_else(|| "the run produced no checks.json".into())),
        },
    }
}

pub(crate) fn write_aggregate(
    out_base: &Path,
    manifest: &nova_probe::AllManifest,
) -> Result<(), String> {
    std::fs::write(
        out_base.join("probe-all.json"),
        format!("{:#}\n", manifest.to_json()),
    )
    .map_err(|e| format!("could not write probe-all.json: {e}"))?;
    std::fs::write(
        out_base.join("index.json"),
        format!("{:#}\n", nova_probe::index_json(manifest)),
    )
    .map_err(|e| format!("could not write index.json: {e}"))?;
    std::fs::write(
        out_base.join("index.html"),
        nova_probe::render_index(manifest),
    )
    .map_err(|e| format!("could not write index.html: {e}"))
}

pub(crate) fn print_aggregate(out_base: &Path, manifest: &nova_probe::AllManifest) {
    let overall = nova_probe::aggregate_verdict(&manifest.rows);
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
    for (example, reason) in &manifest.excluded {
        println!("  {example:<24} NOT PROBED - {reason}");
    }
}

pub(crate) fn aggregate_exit(manifest: &nova_probe::AllManifest) -> ExitCode {
    match nova_probe::aggregate_verdict(&manifest.rows) {
        "OK" | "WARN" => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
