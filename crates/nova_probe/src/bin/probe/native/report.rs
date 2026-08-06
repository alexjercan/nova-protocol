//! `probe report`: re-render a run dir or an aggregate index that probe
//! itself produced.

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use nova_probe::run_report::{
    checks_json, evaluate_checks, overall_verdict, print_checks, render_run_report, RunArtifacts,
};

use super::sweep::{aggregate_exit, build_row, print_aggregate, write_aggregate};

/// `probe report`: re-render an existing run dir - GATED on the
/// manifest, so a report can only ever be built from a dir `probe run`
/// itself produced (stale hand-assembled folders are refused, which is
/// the whole point of the gate). An aggregate dir (probe-all.json)
/// re-renders the index instead: each row is re-read fresh from its
/// run dir's checks.json (re-render a single example's report via
/// `probe report <base>/<example>`).
pub(crate) fn report_many(dirs: &[PathBuf], baseline: Option<&Path>) -> Result<ExitCode, String> {
    let mut worst = ExitCode::SUCCESS;
    for dir in dirs {
        let code = report(dir, baseline)?;
        if code == ExitCode::FAILURE {
            worst = ExitCode::FAILURE;
        }
    }
    Ok(worst)
}

pub(crate) fn report(dir: &Path, baseline: Option<&Path>) -> Result<ExitCode, String> {
    if dir.join("probe-all.json").exists() {
        if baseline.is_some() {
            return Err("--baseline compares one run dir; it does not apply to an \
                 aggregate index"
                .into());
        }
        return report_aggregate(dir);
    }
    if !dir.join("probe-run.json").exists() {
        return Err(format!(
            "{} has neither probe-run.json nor probe-all.json - probe only \
             reports over dirs it produced; run `probe run <example> --out {}` first",
            dir.display(),
            dir.display()
        ));
    }
    let artifacts = RunArtifacts::load(dir, baseline)?;
    let checks = evaluate_checks(&artifacts);
    let verdict = overall_verdict(&checks);
    std::fs::write(
        dir.join("report.html"),
        render_run_report(dir, &artifacts, &checks),
    )
    .map_err(|e| format!("could not write report.html: {e}"))?;
    std::fs::write(
        dir.join("checks.json"),
        format!("{:#}\n", checks_json(&checks, artifacts.manifest.as_ref())),
    )
    .map_err(|e| format!("could not write checks.json: {e}"))?;
    println!("probe: {verdict} - {}", dir.join("report.html").display());
    print_checks(&checks);
    // Fail-closed, the same rule the aggregate uses: only a graded run that
    // came out OK or WARN exits zero. FAIL, NO_DATA and UNPROBEABLE do not.
    Ok(match verdict {
        "OK" | "WARN" => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    })
}

/// Re-render an aggregate dir's index: identity/durations/exclusions
/// come from probe-all.json; every row's verdict/measured/checks are
/// re-read FRESH from its run dir's checks.json (a row whose dir lost
/// its checks.json keeps the manifest's recorded row - deleting
/// evidence does not upgrade a verdict).
fn report_aggregate(dir: &Path) -> Result<ExitCode, String> {
    let contents = std::fs::read_to_string(dir.join("probe-all.json"))
        .map_err(|e| format!("could not read probe-all.json: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| format!("probe-all.json is not valid JSON: {e}"))?;
    let mut manifest = nova_probe::AllManifest::from_json(&value)?;
    manifest.rows = manifest
        .rows
        .iter()
        .map(|row| {
            let refreshed = build_row(
                &row.example,
                &row.category,
                &dir.join(&row.example),
                row.error.clone(),
                row.duration_secs,
            );
            if refreshed.checks.is_empty() && !row.checks.is_empty() {
                row.clone()
            } else {
                refreshed
            }
        })
        .collect();
    write_aggregate(dir, &manifest)?;
    print_aggregate(dir, &manifest);
    Ok(aggregate_exit(&manifest))
}

#[cfg(test)]
mod tests {
    use nova_probe::run_report::{PassRecord, RunManifest};

    use super::*;

    #[test]
    fn report_many_rerenders_multiple_run_dirs() {
        let base =
            std::env::temp_dir().join(format!("nova_probe_report_many_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for example in ["playable", "scenario"] {
            let dir = base.join(example);
            std::fs::create_dir_all(&dir).unwrap();
            let manifest = RunManifest {
                example: example.to_string(),
                started_unix: 1,
                git_sha: "61675034".into(),
                full_git_sha: "61675034abcdef".into(),
                host: "host".into(),
                armed_timeline: false,
                armed_invariants: false,
                armed_fps: false,
                passes: vec![PassRecord {
                    name: "clean".into(),
                    success: true,
                    timed_out: false,
                }],
            };
            std::fs::write(
                dir.join("probe-run.json"),
                format!("{:#}\n", manifest.to_json()),
            )
            .unwrap();
        }

        let dirs = vec![base.join("playable"), base.join("scenario")];
        // FAILURE, and correctly so: these dirs hold a manifest that armed
        // nothing and no artifacts, so every capability check is N/A and the
        // runs graded no claim. Re-rendering them still produces both files -
        // which is what this test is about.
        assert_eq!(report_many(&dirs, None), Ok(ExitCode::FAILURE));
        assert!(base.join("playable").join("report.html").is_file());
        assert!(base.join("scenario").join("checks.json").is_file());
        let _ = std::fs::remove_dir_all(&base);
    }
}
