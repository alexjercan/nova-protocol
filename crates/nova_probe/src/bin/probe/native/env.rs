//! The child-run environments: the fps capture window, the per-pass env
//! blocks, and the sweep matrix cells.

use std::path::Path;

use nova_probe::profile_sandbox;

use super::cli::Render;

/// The non-`perf/` fps window: a shorter warm-up +
/// capture than the perf baseline so a bare `probe run <example> --fps`
/// finishes quickly. `perf/` examples (dedicated steady-state scenes) and
/// the sweep matrix keep the capture crate's full 180/900 baseline window
/// so baselines stay like-for-like.
const NON_PERF_WARMUP: u32 = 60;
const NON_PERF_FRAMES: u32 = 240;

/// Conservative software-render frame-rate FLOOR (frames/sec) used to size
/// the completion deadline to the capture window.
/// The heavy perf scene measured ~2.3 fps in a dev build under lavapipe, so
/// the floor is the slowest we budget for: the sized deadline is a ceiling
/// a legitimately-slow-but-progressing capture fits under, while a genuine
/// hang still fails (just at a window-appropriate bound, not a flat 120s).
const FPS_FLOOR: f64 = 2.0;
/// Extra seconds added to the sized deadline for scene load + asset warm.
const FPS_LOAD_MARGIN_SECS: u64 = 45;

/// Resolve an example's fps policy from the catalog + probe metadata: its
/// category (drives the window default) and, when configured fps-exempt,
/// the reason to record. Fail-OPEN (unknown example -> empty category, not
/// exempt) so a catalog hiccup never silently suppresses a real capture.
pub(crate) fn example_fps_policy(root: &Path, example: &str) -> (String, Option<String>) {
    let category = nova_probe::load_example_catalog(root)
        .ok()
        .and_then(|catalog| {
            catalog
                .iter()
                .find(|entry| entry.name == example)
                .map(|entry| entry.category.clone())
        })
        .unwrap_or_default();
    let reason = nova_probe::load_fps_exempt(root)
        .iter()
        .any(|name| name == example)
        .then(|| {
            "narrative scenario (configured in Cargo.toml \
             [package.metadata.nova_probe] fps_exempt); no stable frame-time window"
                .to_string()
        });
    (category, reason)
}

/// Read an env var as u32 (empty/unparseable -> None). Reads probe's own
/// environment, which the child inherits, so operator overrides are honored.
fn env_u32(key: &str) -> Option<u32> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

/// Resolve the fps capture window (warmup, frames) probe will use for a
/// category: the operator's `NOVA_PERF_WARMUP`/`NOVA_PERF_FRAMES` win, else
/// the `perf/` full baseline window (the capture crate's defaults) or the
/// short non-perf window.
fn resolve_fps_window(category: &str) -> (u32, u32) {
    let (default_warmup, default_frames) = if category == "perf" {
        (
            nova_probe::DEFAULT_WARMUP_FRAMES,
            nova_probe::DEFAULT_CAPTURE_FRAMES,
        )
    } else {
        (NON_PERF_WARMUP, NON_PERF_FRAMES)
    };
    (
        env_u32("NOVA_PERF_WARMUP").unwrap_or(default_warmup),
        env_u32("NOVA_PERF_FRAMES").unwrap_or(default_frames),
    )
}

/// The completion deadline (seconds) a capture window needs at the
/// pessimistic [`FPS_FLOOR`], plus the load margin. Sized so a
/// slow-but-progressing capture completes instead of tripping the hang
/// detector.
fn fps_deadline_secs(warmup: u32, frames: u32) -> u64 {
    (f64::from(warmup + frames) / FPS_FLOOR).ceil() as u64 + FPS_LOAD_MARGIN_SECS
}

/// Env for the fps pass: the resolved capture window set EXPLICITLY (even
/// for `perf/`, so the deadline matches the exact window the child
/// measures) plus the window-sized `BCS_HARNESS_DEADLINE`. Returns the
/// deadline seconds too, so the caller can raise the supervisor timeout
/// above it. The operator's `NOVA_PERF_WARMUP`/`FRAMES` are already folded
/// in by [`resolve_fps_window`]; their `BCS_HARNESS_DEADLINE` wins here
/// (pushed only when unset).
pub(crate) fn fps_window_and_deadline_env(category: &str) -> (Vec<(String, String)>, u64) {
    let (warmup, frames) = resolve_fps_window(category);
    let deadline = fps_deadline_secs(warmup, frames);
    let mut env = vec![
        ("NOVA_PERF_WARMUP".into(), warmup.to_string()),
        ("NOVA_PERF_FRAMES".into(), frames.to_string()),
    ];
    if std::env::var_os("BCS_HARNESS_DEADLINE").is_none() {
        env.push(("BCS_HARNESS_DEADLINE".into(), deadline.to_string()));
    }
    (env, deadline)
}

/// Environment for the CLEAN pass: autopilot + recorder + invariants
/// always; the frame-time capture only on request (`--fps`) since only
/// the wired examples (perf_baseline) read it - elsewhere it is a
/// harmless no-op env. Plus the profile sandbox, so the run cannot read
/// the operator's mod cache, enabled mods or settings
/// ([`nova_probe::profile_sandbox`]).
pub(crate) fn clean_pass_env(
    root: &Path,
    out: &Path,
    display: &str,
    fps: bool,
) -> Vec<(String, String)> {
    let mut env = profile_sandbox::env(out);
    env.extend(vec![
        ("BCS_AUTOPILOT".into(), "1".into()),
        ("BEVY_ASSET_ROOT".into(), root.display().to_string()),
        ("DISPLAY".into(), display.into()),
        (
            "NOVA_PERF_TIMELINE".into(),
            out.join("timeline.jsonl").display().to_string(),
        ),
        ("NOVA_PERF_INVARIANTS".into(), "1".into()),
    ]);
    if fps {
        env.push(("NOVA_PERF".into(), "1".into()));
        env.push(("NOVA_PERF_OUT".into(), out.display().to_string()));
        // Label rows by the example so probe-vs-probe baselines match
        // (the capture's default label "scene" matches nothing).
        env.push((
            "NOVA_PERF_LABEL".into(),
            out.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "scene".into()),
        ));
    }
    env
}

/// Per-cell additions for a sweep matrix run: scenario + preset +
/// the sweep's label convention, plus the software-raster ICD floor
/// when --render sw (exactly perf-baseline.sh's env: forced lavapipe
/// via VK_ICD_FILENAMES/VK_DRIVER_FILES + vulkan backend, and the sw
/// warmup/frames defaults unless the caller pinned their own).
pub(crate) fn sweep_cell_env(
    scenario: Option<&str>,
    preset: Option<&str>,
    render: Render,
) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if let Some(scenario) = scenario {
        env.push(("NOVA_PERF_SCENARIO".into(), scenario.into()));
    }
    if let Some(preset) = preset {
        env.push(("NOVA_PERF_QUALITY".into(), preset.into()));
    }
    match (scenario, preset) {
        (Some(s), Some(p)) => env.push(("NOVA_PERF_LABEL".into(), format!("{s}-{p}"))),
        (Some(s), None) => env.push(("NOVA_PERF_LABEL".into(), s.into())),
        _ => {}
    }
    if render == Render::Sw {
        let icd = std::env::var("LVP_ICD")
            .unwrap_or_else(|_| "/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json".into());
        env.push(("VK_ICD_FILENAMES".into(), icd.clone()));
        env.push(("VK_DRIVER_FILES".into(), icd));
        env.push(("WGPU_BACKEND".into(), "vulkan".into()));
        if std::env::var("NOVA_PERF_WARMUP").is_err() {
            env.push(("NOVA_PERF_WARMUP".into(), "20".into()));
        }
        if std::env::var("NOVA_PERF_FRAMES").is_err() {
            env.push(("NOVA_PERF_FRAMES".into(), "120".into()));
        }
    }
    env
}

/// Environment for the PROFILED pass: the chrome-trace writer plus the
/// RUST_LOG override that un-hides the per-system spans (the game's own
/// log filter sets bevy_ecs=warn, which silently kills them -
/// env-filter-governs-spans). No recorder/invariants here: this pass
/// exists for the trace only, and its numbers never feed the report's
/// correctness or FPS sections. Profile-sandboxed like the clean pass:
/// every native child run gets the same empty, probe-owned profile.
pub(crate) fn trace_pass_env(root: &Path, out: &Path, display: &str) -> Vec<(String, String)> {
    let rust_log = match std::env::var("RUST_LOG") {
        Ok(existing) if !existing.is_empty() => format!("{existing},bevy_ecs=info"),
        _ => "bevy_ecs=info".into(),
    };
    let mut env = profile_sandbox::env(out);
    env.extend(vec![
        ("BCS_AUTOPILOT".into(), "1".into()),
        ("BEVY_ASSET_ROOT".into(), root.display().to_string()),
        ("DISPLAY".into(), display.into()),
        (
            "TRACE_CHROME".into(),
            out.join("trace.json").display().to_string(),
        ),
        ("RUST_LOG".into(), rust_log),
    ]);
    env
}

/// Environment for the SAMPLY pass: the sampled run drives the same
/// autopilot scene, so it carries the autopilot + asset root + display -
/// and the profile sandbox, like every other native child run (a pass
/// that spawns the example is a pass that must not read the operator's
/// mod cache). No recorder/capture: samply's own sampler is
/// the instrument here.
pub(crate) fn samply_pass_env(root: &Path, out: &Path, display: &str) -> Vec<(String, String)> {
    let mut env = profile_sandbox::env(out);
    env.extend(vec![
        ("BCS_AUTOPILOT".to_string(), "1".to_string()),
        ("BEVY_ASSET_ROOT".to_string(), root.display().to_string()),
        ("DISPLAY".to_string(), display.to_string()),
    ]);
    env
}

/// Cells of the sweep matrix (scenarios x presets); a missing dimension
/// contributes a single None cell, so no flags = one default cell.
pub(crate) fn matrix_cells(
    scenarios: &[String],
    presets: &[String],
) -> Vec<(Option<String>, Option<String>)> {
    let ss: Vec<Option<String>> = if scenarios.is_empty() {
        vec![None]
    } else {
        scenarios.iter().cloned().map(Some).collect()
    };
    let ps: Vec<Option<String>> = if presets.is_empty() {
        vec![None]
    } else {
        presets.iter().cloned().map(Some).collect()
    };
    let mut cells = Vec::new();
    for s in &ss {
        for p in &ps {
            cells.push((s.clone(), p.clone()));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::fixtures::s;

    #[test]
    fn resolve_fps_window_defaults_per_category() {
        // Deterministic only when the operator has not pinned a window
        // (the suite runs without NOVA_PERF_* set; guard against a stray).
        if std::env::var_os("NOVA_PERF_WARMUP").is_none()
            && std::env::var_os("NOVA_PERF_FRAMES").is_none()
        {
            // perf/ keeps the full baseline window; other categories get
            // the short window.
            assert_eq!(
                resolve_fps_window("perf"),
                (
                    nova_probe::DEFAULT_WARMUP_FRAMES,
                    nova_probe::DEFAULT_CAPTURE_FRAMES
                )
            );
            assert_eq!(
                resolve_fps_window("gameplay"),
                (NON_PERF_WARMUP, NON_PERF_FRAMES)
            );
        }
    }

    #[test]
    fn fps_deadline_scales_with_the_window_and_clears_the_flat_default() {
        // The flat bcs default is 120s; the perf window (180+900) must get a
        // much larger deadline, and the short window a smaller one - the
        // whole point of sizing.
        let perf = fps_deadline_secs(
            nova_probe::DEFAULT_WARMUP_FRAMES,
            nova_probe::DEFAULT_CAPTURE_FRAMES,
        );
        let short = fps_deadline_secs(NON_PERF_WARMUP, NON_PERF_FRAMES);
        // 1080 / 2.0 + 45 = 585; 300 / 2.0 + 45 = 195.
        assert_eq!(perf, 585);
        assert_eq!(short, 195);
        assert!(
            perf > 120 && short > 120,
            "both clear the flat 120s default"
        );
        assert!(perf > short, "a bigger window gets a bigger deadline");
    }

    #[test]
    fn fps_window_and_deadline_env_sets_window_and_deadline() {
        if std::env::var_os("NOVA_PERF_WARMUP").is_none()
            && std::env::var_os("NOVA_PERF_FRAMES").is_none()
            && std::env::var_os("BCS_HARNESS_DEADLINE").is_none()
        {
            let (env, deadline) = fps_window_and_deadline_env("perf");
            assert_eq!(deadline, 585);
            assert!(env
                .iter()
                .any(|(k, v)| k == "NOVA_PERF_FRAMES" && v == "900"));
            assert!(env
                .iter()
                .any(|(k, v)| k == "BCS_HARNESS_DEADLINE" && v == "585"));
        }
    }

    #[test]
    fn matrix_cells_cross_scenarios_and_presets() {
        let cells = matrix_cells(&s(&["a", "b"]), &s(&["high", "low"]));
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0], (Some("a".to_string()), Some("high".to_string())));
        assert_eq!(cells[3], (Some("b".to_string()), Some("low".to_string())));
        assert_eq!(matrix_cells(&[], &[]), vec![(None, None)]);
    }

    #[test]
    fn sweep_cell_env_sets_label_and_sw_floor() {
        let env = sweep_cell_env(Some("asteroid_field"), Some("low"), Render::Sw);
        let get = |k: &str| env.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        assert_eq!(get("NOVA_PERF_SCENARIO").as_deref(), Some("asteroid_field"));
        assert_eq!(get("NOVA_PERF_QUALITY").as_deref(), Some("low"));
        assert_eq!(
            get("NOVA_PERF_LABEL").as_deref(),
            Some("asteroid_field-low"),
            "the sweep's label convention"
        );
        assert_eq!(get("WGPU_BACKEND").as_deref(), Some("vulkan"));
        assert!(get("VK_ICD_FILENAMES").unwrap().contains("lvp_icd"));

        let env = sweep_cell_env(None, None, Render::Gpu);
        assert!(env.is_empty(), "default cell adds nothing: {env:?}");
    }

    #[test]
    fn clean_env_always_arms_recorder_and_invariants_fps_only_on_request() {
        let root = Path::new("/repo");
        let out = Path::new("/repo/probe-runs/x");
        let env = clean_pass_env(root, out, ":97", false);
        let get = |k: &str, e: &[(String, String)]| {
            e.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone())
        };
        assert_eq!(get("BCS_AUTOPILOT", &env).as_deref(), Some("1"));
        assert_eq!(
            get("NOVA_PERF_TIMELINE", &env).as_deref(),
            Some("/repo/probe-runs/x/timeline.jsonl")
        );
        assert_eq!(get("NOVA_PERF_INVARIANTS", &env).as_deref(), Some("1"));
        assert_eq!(get("NOVA_PERF", &env), None, "fps off by default");

        let env = clean_pass_env(root, out, ":97", true);
        assert_eq!(get("NOVA_PERF", &env).as_deref(), Some("1"));
        assert_eq!(
            get("NOVA_PERF_OUT", &env).as_deref(),
            Some("/repo/probe-runs/x")
        );
        // Rows label by the run-dir name so probe-vs-probe baselines
        // match (the capture's default "scene" matches nothing).
        assert_eq!(get("NOVA_PERF_LABEL", &env).as_deref(), Some("x"));
        let env = clean_pass_env(root, out, ":97", false);
        assert_eq!(
            get("NOVA_PERF_LABEL", &env),
            None,
            "label rides with --fps only"
        );
    }

    #[test]
    fn every_child_run_env_carries_the_profile_sandbox() {
        let root = Path::new("/repo");
        let out = Path::new("/repo/probe-runs/x");
        // Expectations are COMPUTED from the sandbox itself: a variable
        // the test host exports is one probe deliberately leaves to the
        // operator (that policy is pinned in profile_sandbox's own
        // tests). The guard keeps this from degrading into a test that
        // asserts nothing on a host that exports all three.
        let expected = profile_sandbox::env(out);
        assert!(
            !expected.is_empty(),
            "this test needs a host that does not export all of {:?}; \
             it cannot prove the wiring when probe pushes nothing",
            profile_sandbox::SANDBOXED_VARS
        );
        // EVERY builder that feeds run_supervised with a native example:
        // clean (also the sweep + fps passes), profiled, samply.
        for env in [
            clean_pass_env(root, out, ":97", false),
            clean_pass_env(root, out, ":97", true),
            trace_pass_env(root, out, ":97"),
            samply_pass_env(root, out, ":97"),
        ] {
            for (var, path) in &expected {
                let value = env
                    .iter()
                    .find(|(k, _)| k == var)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| panic!("{var} missing: every child run is sandboxed"));
                assert_eq!(&value, path, "{var} must point at this run's own sandbox");
                assert!(
                    value.starts_with("/repo/probe-runs/x/profile/"),
                    "{var} must point inside the run dir, got {value}"
                );
            }
        }
    }

    #[test]
    fn trace_env_overrides_the_span_killing_filter_and_skips_the_recorder() {
        let root = Path::new("/repo");
        let out = Path::new("/repo/probe-runs/x");
        let env = trace_pass_env(root, out, ":97");
        let get = |k: &str| env.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        assert_eq!(
            get("TRACE_CHROME").as_deref(),
            Some("/repo/probe-runs/x/trace.json")
        );
        assert!(
            get("RUST_LOG").unwrap().contains("bevy_ecs=info"),
            "the game filter's bevy_ecs=warn kills system spans"
        );
        assert_eq!(
            get("NOVA_PERF_TIMELINE"),
            None,
            "the profiled pass never overwrites the clean pass's timeline"
        );
    }
}
