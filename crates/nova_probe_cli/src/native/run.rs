//! One example through the harness passes: clean, fps, profiled, samply,
//! then the run report.

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use nova_probe::{
    profile_sandbox,
    run_report::{
        checks_json, evaluate_checks, overall_verdict, print_checks, render_run_report,
        run_identity, PassRecord, RunArtifacts, RunManifest,
    },
};

use super::{
    cli::{Platform, RunOptions},
    env::{
        clean_pass_env, fps_window_and_deadline_env, matrix_cells, samply_pass_env, sweep_cell_env,
        trace_pass_env,
    },
    paths::{default_output_root, repo_root, resolve_full_git_sha},
    supervise::{build_example, ensure_display, run_supervised},
    web::web_capture,
};

/// probe's own artifact filenames: surgically removed from the out dir
/// at the start of a run so nothing stale (an old trace, a previous
/// checks.json) can present as this run's evidence. NOTE: never a recursive
/// wipe - the dir may be user-supplied.
const RUN_ARTIFACTS: [&str; 12] = [
    "timeline.jsonl",
    "probe-contract.json",
    "run.log",
    "fps-run.log",
    "trace.json",
    "trace-run.log",
    "frametime.csv",
    "samply-profile.json.gz",
    "samply-run.log",
    "web-run.log",
    "report.html",
    "checks.json",
];

/// The sweep-cell logs already in `out`. They are NUMBERED, so they cannot be
/// listed in [`RUN_ARTIFACTS`] - and `RunArtifacts::load` globs and
/// concatenates exactly this set, so a previous sweep's cell logs present as
/// this run's evidence unless `clean_out_dir` removes them too.
fn stale_cell_logs(out: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(out) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("run-") && name.ends_with(".log"))
        })
        .collect()
}

fn clean_out_dir(out: &Path) -> Result<(), String> {
    let named = RUN_ARTIFACTS
        .iter()
        .map(|name| out.join(name))
        .chain(std::iter::once(out.join("probe-run.json")));
    for path in named.chain(stale_cell_logs(out)) {
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("could not clear stale {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn run(opts: &RunOptions) -> Result<ExitCode, String> {
    let root = repo_root();
    let out = opts.out.clone().unwrap_or_else(|| {
        let (git_sha, _) = run_identity();
        default_output_root(&root, None, &git_sha).join(&opts.example)
    });
    std::fs::create_dir_all(&out).map_err(|e| format!("could not create out dir: {e}"))?;
    let out = out
        .canonicalize()
        .map_err(|e| format!("could not resolve out dir: {e}"))?;
    // Nothing stale may survive into this run's report.
    clean_out_dir(&out)?;
    // NOTE: the child runs read their mod cache / enabled mods / settings
    // from this run's own empty profile, never the operator's. The env
    // itself rides in clean_pass_env and trace_pass_env; this creates the
    // dirs and reports any variable the operator kept for themselves.
    profile_sandbox::prepare(&out);
    // NOTE: a bad baseline path must fail BEFORE minutes of build+run, and
    // it must actually parse.
    if let Some(baseline) = &opts.baseline {
        let csv = baseline.join("frametime.csv");
        let contents = std::fs::read_to_string(&csv)
            .map_err(|e| format!("--baseline invalid before running: {}: {e}", csv.display()))?;
        nova_probe::parse_frametime_csv(&contents)
            .map_err(|e| format!("--baseline invalid before running: {e}"))?;
    }
    let (display, _xvfb) = ensure_display(opts.display.as_deref())?;
    let timeout = Duration::from_secs(opts.timeout_secs);
    let started_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut passes: Vec<PassRecord> = Vec::new();

    if opts.platform == Platform::Web {
        let outcome = web_capture(&root, &out, &display, &opts.example, timeout)?;
        passes.push(outcome);
        return finish_report(
            opts,
            &out,
            started_unix,
            passes,
            /*armed_native*/ false,
        );
    }

    // Pass 1: CLEAN (native) - correctness only. NOTE: the frame capture
    // NEVER rides this pass; the recorder flushes JSONL per entry on the
    // frame path, so fps-on-clean numbers are contaminated - measurement
    // passes (fps, trace, samply) are always separate. With matrix flags the
    // cells REPLACE the clean pass entirely (a sweep is a measurement
    // invocation).
    let cells = matrix_cells(&opts.scenarios, &opts.presets);
    let sweeping = !opts.scenarios.is_empty() || !opts.presets.is_empty();
    eprintln!(
        "probe: [1/{}] clean pass: building {}{}",
        passes_total(opts),
        opts.example,
        if opts.release { " (release)" } else { "" }
    );
    let build_features = "debug";
    let (profile_dir, cargo_profile) = if opts.release {
        ("release", Some("release"))
    } else {
        ("debug", None)
    };
    build_example(&root, &opts.example, build_features, cargo_profile)?;
    let bin = root
        .join("target")
        .join(profile_dir)
        .join("examples")
        .join(&opts.example);
    for (i, (scenario, preset)) in cells.iter().enumerate() {
        let cell_name = match (scenario, preset) {
            (Some(s), Some(p)) => format!("clean {s}-{p}"),
            (Some(s), None) => format!("clean {s}"),
            _ => "clean".to_string(),
        };
        let log_name = if cells.len() > 1 {
            format!("run-{i}.log")
        } else {
            "run.log".to_string()
        };
        eprintln!("probe: {cell_name} -> {}", out.join(&log_name).display());
        let mut env = clean_pass_env(&root, &out, &display, sweeping && opts.fps);
        if sweeping {
            // Sweep cells measure frames, not the recorder surfaces.
            env.retain(|(k, _)| k != "NOVA_PERF_TIMELINE" && k != "NOVA_PERF_INVARIANTS");
            // The per-example label yields to the sweep convention.
            env.retain(|(k, _)| k != "NOVA_PERF_LABEL");
        }
        env.extend(sweep_cell_env(
            scenario.as_deref(),
            preset.as_deref(),
            opts.render,
        ));
        let outcome = run_supervised(&bin, &[], &root, &env, &out.join(&log_name), timeout)?;
        if !outcome.success() {
            eprintln!("probe: {cell_name} did not succeed; the report will say so");
        }
        passes.push(PassRecord {
            name: cell_name,
            success: outcome.success(),
            timed_out: outcome.timed_out(),
        });
    }

    // FPS pass (optional, non-sweep): the dedicated capture-only run -
    // same binary as the clean pass, recorder/invariants OFF the frame
    // path, its own log. The completion protocol keeps the app alive
    // until the capture window closes. Whether there is anything to
    // capture is the EXAMPLE's answer (it wired nova_frametime(), or it
    // did not), and its contract tells the report which.
    if opts.fps && !sweeping {
        {
            eprintln!(
                "probe: fps pass: capture-only -> {}",
                out.join("fps-run.log").display()
            );
            // NOVA_PERF_CONTRACT too: the CLEAN pass owns probe-contract.json,
            // and letting the fps pass rewrite it makes the report's "what the
            // example wired" half describe the wrong run the moment the two
            // passes diverge.
            let mut env = clean_pass_env(&root, &out, &display, true);
            env.retain(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "NOVA_PERF_TIMELINE" | "NOVA_PERF_INVARIANTS" | "NOVA_PERF_CONTRACT"
                )
            });
            // The baseline capture window + a completion deadline SIZED to
            // it, so a slow-but-progressing capture (a heavy dev scene under
            // software rendering) completes instead of tripping the flat
            // 120s hang detector.
            let (window_env, deadline_secs) = fps_window_and_deadline_env();
            env.extend(window_env);
            // The supervisor timeout MUST exceed the in-process deadline,
            // or probe kills the child before the deadline can complete or
            // report; keep the operator's --timeout if it is larger.
            let fps_timeout = Duration::from_secs((deadline_secs + 30).max(opts.timeout_secs));
            eprintln!("probe: fps pass deadline {deadline_secs}s (window-sized)");
            let outcome = run_supervised(
                &bin,
                &[],
                &root,
                &env,
                &out.join("fps-run.log"),
                fps_timeout,
            )?;
            if !outcome.success() {
                eprintln!("probe: fps pass did not succeed; the report will say so");
            }
            passes.push(PassRecord {
                name: "fps".into(),
                success: outcome.success(),
                timed_out: outcome.timed_out(),
            });
        }
    }

    // Pass 2: PROFILED (optional; separate build so tracing overhead
    // never touches pass 1's numbers). Failures degrade to "no trace" -
    // a successful clean pass is never discarded.
    if opts.profile {
        eprintln!(
            "probe: [2/{}] profiled pass: building with tracing",
            passes_total(opts)
        );
        match build_example(&root, &opts.example, "debug,trace", None) {
            Err(e) => {
                eprintln!("probe: profiled build failed ({e}); continuing without a trace");
                passes.push(PassRecord {
                    name: "profiled".into(),
                    success: false,
                    timed_out: false,
                });
            }
            Ok(()) => {
                let env = trace_pass_env(&root, &out, &display);
                eprintln!("probe: traced run -> {}", out.join("trace.json").display());
                // Tracing throttles the run hard; give it double time.
                let outcome = run_supervised(
                    &bin,
                    &[],
                    &root,
                    &env,
                    &out.join("trace-run.log"),
                    timeout * 2,
                )?;
                if !outcome.success() {
                    eprintln!("probe: traced run did not succeed; the trace may be partial");
                }
                passes.push(PassRecord {
                    name: "profiled".into(),
                    success: outcome.success(),
                    timed_out: outcome.timed_out(),
                });
            }
        }
    }

    // Pass 3: samply flamegraph (optional, tolerant - a missing/blocked
    // profiler never fails the check; supervised so a hung sampled run
    // cannot wedge probe either).
    if opts.samply {
        eprintln!("probe: [{n}/{n}] samply pass", n = passes_total(opts));
        match build_example(&root, &opts.example, "debug", Some("profiling")) {
            Err(e) => eprintln!("probe: samply build failed ({e}); flamegraph skipped"),
            Ok(()) => {
                let sbin = root.join("target/profiling/examples").join(&opts.example);
                let samply_env = samply_pass_env(&root, &out, &display);
                let samply = Path::new("samply");
                let profile_out = out.join("samply-profile.json.gz");
                let outcome = run_supervised(
                    samply,
                    &[
                        "record",
                        "--save-only",
                        "-o",
                        &profile_out.display().to_string(),
                        &sbin.display().to_string(),
                    ],
                    &root,
                    &samply_env,
                    &out.join("samply-run.log"),
                    timeout * 2,
                );
                match outcome {
                    Ok(o) if o.success() => eprintln!(
                        "probe: flamegraph saved; open with: samply load {}",
                        profile_out.display()
                    ),
                    Ok(_) => eprintln!(
                        "probe: samply run failed or timed out (perms? \
                         perf_event_paranoid/mlock_kb; see samply-run.log); skipped"
                    ),
                    Err(e) => eprintln!("probe: samply not runnable ({e}); skipped"),
                }
            }
        }
    }

    finish_report(
        opts,
        &out,
        started_unix,
        passes,
        /*armed_native*/ !sweeping,
    )
}

/// Write the manifest, assemble the report in-process, print + exit.
fn finish_report(
    opts: &RunOptions,
    out: &Path,
    started_unix: u64,
    passes: Vec<PassRecord>,
    armed_native: bool,
) -> Result<ExitCode, String> {
    let (git_sha, host) = run_identity();
    let full_git_sha = resolve_full_git_sha(&repo_root());
    let manifest = RunManifest {
        example: opts.example.clone(),
        started_unix,
        git_sha,
        full_git_sha,
        host,
        armed_timeline: armed_native,
        armed_invariants: armed_native,
        armed_fps: opts.fps || opts.platform == Platform::Web,
        passes,
    };
    std::fs::write(
        out.join("probe-run.json"),
        format!("{:#}\n", manifest.to_json()),
    )
    .map_err(|e| format!("could not write probe-run.json: {e}"))?;

    let artifacts = RunArtifacts::load(out, opts.baseline.as_deref())?;
    let checks = evaluate_checks(&artifacts);
    let verdict = overall_verdict(&checks);
    std::fs::write(
        out.join("report.html"),
        render_run_report(out, &artifacts, &checks),
    )
    .map_err(|e| format!("could not write report.html: {e}"))?;
    std::fs::write(
        out.join("checks.json"),
        format!("{:#}\n", checks_json(&checks, artifacts.manifest.as_ref())),
    )
    .map_err(|e| format!("could not write checks.json: {e}"))?;

    println!("probe: {verdict} - {}", out.join("report.html").display());
    print_checks(&checks);
    // Fail-closed, the same rule the aggregate uses: only a graded run that
    // came out OK or WARN exits zero. FAIL, NO_DATA and UNPROBEABLE do not.
    Ok(match verdict {
        "OK" | "WARN" => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    })
}

fn passes_total(opts: &RunOptions) -> usize {
    let sweeping = !opts.scenarios.is_empty() || !opts.presets.is_empty();
    1 + usize::from(opts.fps && !sweeping) + usize::from(opts.profile) + usize::from(opts.samply)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sweep's numbered cell logs are read back as this run's log, so
    /// leaving them behind lets a previous sweep's output be scanned as the
    /// current run's - and the surgical removal must still not touch anything
    /// probe did not write.
    #[test]
    fn cleaning_removes_stale_cell_logs_and_nothing_else() {
        let out = std::env::temp_dir().join(format!("nova_probe_clean_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        for name in ["run-1.log", "run-12.log", "checks.json", "run.log"] {
            std::fs::write(out.join(name), "stale\n").unwrap();
        }
        std::fs::write(out.join("notes.txt"), "the operator's own file\n").unwrap();

        clean_out_dir(&out).unwrap();

        for name in ["run-1.log", "run-12.log", "checks.json", "run.log"] {
            assert!(!out.join(name).exists(), "{name} survived the clean");
        }
        assert!(
            out.join("notes.txt").is_file(),
            "the clean is surgical, never a recursive wipe of a user-supplied dir"
        );
        let _ = std::fs::remove_dir_all(&out);
    }
}
