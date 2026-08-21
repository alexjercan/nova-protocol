//! The child-run environments: the fps capture window, the per-pass env
//! blocks, and the sweep matrix cells.

use std::path::Path;

use nova_autopilot::{autopilot::AUTOPILOT_ENV, completion::DEADLINE_ENV};

use super::cli::Render;
use crate::native::profile_sandbox;

/// Conservative software-render frame-rate FLOOR (frames/sec) used to size
/// the completion deadline to the capture window.
/// The heavy perf scene measured ~2.3 fps in a dev build under lavapipe, so
/// the floor is the slowest we budget for: the sized deadline is a ceiling
/// a legitimately-slow-but-progressing capture fits under, while a genuine
/// hang still fails (just at a window-appropriate bound, not a flat 120s).
const FPS_FLOOR: f64 = 2.0;
/// Extra seconds added to the sized deadline for scene load + asset warm.
const FPS_LOAD_MARGIN_SECS: u64 = 45;

/// `DISPLAY` for a child pass, or nothing at all when the run has no X server.
///
/// A `--norender` run never calls `ensure_display`, so it has no display
/// string to pass on. Pushing an empty `DISPLAY` would be worse than pushing
/// none: winit is not in a headless build, but anything else that reads the
/// variable would see a set-but-invalid one.
fn display_env(display: &str) -> Option<(String, String)> {
    (!display.is_empty()).then(|| ("DISPLAY".to_string(), display.to_string()))
}

/// Read an env var as u32 (empty/unparseable -> None). Reads probe's own
/// environment, which the child inherits, so operator overrides are honored.
fn env_u32(key: &str) -> Option<u32> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

/// Resolve the fps capture window (warmup, frames) probe SIZES ITS DEADLINE
/// against: the operator's `NOVA_PROBE_WARMUP`/`NOVA_PROBE_FRAMES` win, else the
/// capture crate's full 180/900 baseline window.
///
/// ONE window, not a per-category default: under the category policy the fps
/// pass runs only for a frame-time category, and those exist to be compared
/// against a baseline - a shorter window would make their numbers
/// incomparable with the sweep's. (This replaced a short non-`perf/` window
/// that the policy made unreachable: the categories it served no longer run
/// an fps pass at all.)
///
/// It is a DEADLINE size, not an instruction: an example may declare a shorter
/// window of its own (`FrameTimePlugin::window`) because its scene cannot run
/// for 900 frames, and probe must not overwrite that from here - which is why
/// the two env vars are only passed on when the operator actually set them.
fn resolve_fps_window() -> (u32, u32) {
    (
        env_u32("NOVA_PROBE_WARMUP").unwrap_or(nova_probe::DEFAULT_WARMUP_FRAMES),
        env_u32("NOVA_PROBE_FRAMES").unwrap_or(nova_probe::DEFAULT_CAPTURE_FRAMES),
    )
}

/// The completion deadline (seconds) a capture window needs at the
/// pessimistic [`FPS_FLOOR`], plus the load margin. Sized so a
/// slow-but-progressing capture completes instead of tripping the hang
/// detector.
fn fps_deadline_secs(warmup: u32, frames: u32) -> u64 {
    (f64::from(warmup + frames) / FPS_FLOOR).ceil() as u64 + FPS_LOAD_MARGIN_SECS
}

/// Env for the fps pass: the operator's capture window when they pinned one,
/// plus the window-sized [`DEADLINE_ENV`]. Returns the deadline seconds too, so
/// the caller can raise the supervisor timeout above it. Their [`DEADLINE_ENV`]
/// wins here (pushed only when unset).
///
/// The window vars are forwarded ONLY when the operator set them. Probe used to
/// push the baseline 180/900 unconditionally, which silently overwrote an
/// example's own declared window - and a window is a property of the SCENE (a
/// fight that ends, a chapter that completes) that the host cannot know. The
/// deadline is still sized on the full baseline window, so it stays a ceiling
/// for any shorter one.
pub(crate) fn fps_window_and_deadline_env() -> (Vec<(String, String)>, u64) {
    let (warmup, frames) = resolve_fps_window();
    let deadline = fps_deadline_secs(warmup, frames);
    let mut env = Vec::new();
    for (key, value) in [("NOVA_PROBE_WARMUP", warmup), ("NOVA_PROBE_FRAMES", frames)] {
        if std::env::var_os(key).is_some() {
            env.push((key.into(), value.to_string()));
        }
    }
    if std::env::var_os(DEADLINE_ENV).is_none() {
        env.push((DEADLINE_ENV.into(), deadline.to_string()));
    }
    (env, deadline)
}

/// Environment for the CLEAN pass: autopilot + recorder + invariants always,
/// with frame-time capture when the caller is a measurement pass. Arming is
/// free for a program that wires no such plugin: the plugin is inert and its
/// contract does not declare the capability, so the report says so.
/// The clean pass OWNS the contract: the trace and samply passes must not
/// name a path, or a later pass would rewrite the claim.
/// Plus the profile sandbox, so the run cannot read
/// the operator's mod cache, enabled mods or settings
/// ([`crate::profile_sandbox`]).
pub(crate) fn clean_pass_env(
    root: &Path,
    out: &Path,
    display: &str,
    fps: bool,
) -> Vec<(String, String)> {
    let mut env = profile_sandbox::env(out);
    env.extend(display_env(display));
    env.extend(vec![
        (AUTOPILOT_ENV.into(), "1".into()),
        ("BEVY_ASSET_ROOT".into(), root.display().to_string()),
        (
            "NOVA_PROBE_TIMELINE".into(),
            out.join("timeline.jsonl").display().to_string(),
        ),
        ("NOVA_PROBE_INVARIANTS".into(), "1".into()),
        (
            "NOVA_PROBE_CONTRACT".into(),
            out.join("probe-contract.json").display().to_string(),
        ),
    ]);
    if fps {
        env.push(("NOVA_PROBE".into(), "1".into()));
        env.push(("NOVA_PROBE_OUT".into(), out.display().to_string()));
        // Label rows by the example so probe-vs-probe baselines match
        // (the capture's default label "scene" matches nothing).
        env.push((
            "NOVA_PROBE_LABEL".into(),
            out.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "scene".into()),
        ));
    }
    env
}

/// Per-cell additions for a sweep matrix run: scenario + preset + the sweep's
/// label convention. The renderer is NOT here - it belongs to every pass, so it
/// is [`render_env`]'s.
pub(crate) fn sweep_cell_env(
    scenario: Option<&str>,
    preset: Option<&str>,
) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if let Some(scenario) = scenario {
        env.push(("NOVA_PROBE_SCENARIO".into(), scenario.into()));
    }
    if let Some(preset) = preset {
        env.push(("NOVA_PROBE_QUALITY".into(), preset.into()));
    }
    match (scenario, preset) {
        (Some(s), Some(p)) => env.push(("NOVA_PROBE_LABEL".into(), format!("{s}-{p}"))),
        (Some(s), None) => env.push(("NOVA_PROBE_LABEL".into(), s.into())),
        _ => {}
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
    env.extend(display_env(display));
    env.extend(vec![
        (AUTOPILOT_ENV.into(), "1".into()),
        ("BEVY_ASSET_ROOT".into(), root.display().to_string()),
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
    env.extend(display_env(display));
    env.extend(vec![
        (AUTOPILOT_ENV.to_string(), "1".to_string()),
        ("BEVY_ASSET_ROOT".to_string(), root.display().to_string()),
    ]);
    env
}

/// The renderer selection, applied to EVERY native pass rather than to the
/// clean pass alone: the number a `--render` flag exists to produce is written
/// by the fps pass, so a selection the fps pass did not get is a selection that
/// did not happen.
///
/// `norender` wins outright and the parser refuses the combination, so the
/// software floor's short window CANNOT leak into a headless run. That matters:
/// the 20/120 window is a concession to how slow lavapipe is, and a headless
/// run has no fill cost to shorten around. It keeps the 180/900 baseline, which
/// is what makes its rows readable against the rendered ones.
///
/// - `norender`: the headless switch, and NOTHING else.
/// - [`Render::Gpu`]: nothing. The host's own adapter, as inherited.
/// - [`Render::Sw`]: lavapipe forced through the Vulkan ICD vars, exactly
///   `perf-baseline.sh`'s env, plus the short window.
pub(crate) fn render_env(render: Render, norender: bool) -> Vec<(String, String)> {
    if norender {
        return vec![(nova_probe::NORENDER_ENV.into(), "1".into())];
    }
    let mut env = Vec::new();
    if render == Render::Sw {
        let icd = std::env::var("LVP_ICD")
            .unwrap_or_else(|_| "/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json".into());
        env.push(("VK_ICD_FILENAMES".into(), icd.clone()));
        env.push(("VK_DRIVER_FILES".into(), icd));
        env.push(("WGPU_BACKEND".into(), "vulkan".into()));
        if std::env::var("NOVA_PROBE_WARMUP").is_err() {
            env.push(("NOVA_PROBE_WARMUP".into(), "20".into()));
        }
        if std::env::var("NOVA_PROBE_FRAMES").is_err() {
            env.push(("NOVA_PROBE_FRAMES".into(), "120".into()));
        }
    }
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
    fn the_fps_window_is_the_baseline_window() {
        // Deterministic only when the operator has not pinned a window
        // (the suite runs without NOVA_PROBE_* set; guard against a stray).
        if std::env::var_os("NOVA_PROBE_WARMUP").is_none()
            && std::env::var_os("NOVA_PROBE_FRAMES").is_none()
        {
            // One window for every category that captures at all, so probe
            // numbers stay comparable with the sweep's baselines.
            assert_eq!(
                resolve_fps_window(),
                (
                    nova_probe::DEFAULT_WARMUP_FRAMES,
                    nova_probe::DEFAULT_CAPTURE_FRAMES
                )
            );
        }
    }

    #[test]
    fn fps_deadline_scales_with_the_window_and_clears_the_flat_default() {
        // The flat bcs default is 120s; the baseline window (180+900) must
        // get a much larger deadline - the whole point of sizing.
        let baseline = fps_deadline_secs(
            nova_probe::DEFAULT_WARMUP_FRAMES,
            nova_probe::DEFAULT_CAPTURE_FRAMES,
        );
        let short = fps_deadline_secs(60, 240);
        // 1080 / 2.0 + 45 = 585; 300 / 2.0 + 45 = 195.
        assert_eq!(baseline, 585);
        assert_eq!(short, 195);
        assert!(
            baseline > 120 && short > 120,
            "both clear the flat 120s default"
        );
        assert!(baseline > short, "a bigger window gets a bigger deadline");
    }

    /// The deadline is sized on the baseline window, and the window itself is
    /// LEFT ALONE - an example that declared a shorter one keeps it.
    #[test]
    fn fps_env_sizes_the_deadline_without_pinning_the_window() {
        if std::env::var_os("NOVA_PROBE_WARMUP").is_none()
            && std::env::var_os("NOVA_PROBE_FRAMES").is_none()
            && std::env::var_os(DEADLINE_ENV).is_none()
        {
            let (env, deadline) = fps_window_and_deadline_env();
            assert_eq!(deadline, 585);
            assert!(
                !env.iter()
                    .any(|(k, _)| k == "NOVA_PROBE_FRAMES" || k == "NOVA_PROBE_WARMUP"),
                "an unpinned window must not be forced on the child: {env:?}"
            );
            assert!(env
                .iter()
                .any(|(k, v)| k == "NOVA_AUTOPILOT_DEADLINE" && v == "585"));
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
    fn sweep_cell_env_sets_only_the_matrix_coordinates_and_the_label() {
        let env = sweep_cell_env(Some("some_scenario"), Some("low"));
        let get = |k: &str| env.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        assert_eq!(get("NOVA_PROBE_SCENARIO").as_deref(), Some("some_scenario"));
        assert_eq!(get("NOVA_PROBE_QUALITY").as_deref(), Some("low"));
        assert_eq!(
            get("NOVA_PROBE_LABEL").as_deref(),
            Some("some_scenario-low"),
            "the sweep's label convention"
        );

        assert!(
            sweep_cell_env(None, None).is_empty(),
            "default cell adds nothing"
        );
    }

    #[test]
    fn norender_pushes_the_headless_switch_and_leaves_the_window_alone() {
        let env = render_env(Render::Gpu, true);
        let get = |k: &str| env.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        assert_eq!(get(nova_probe::NORENDER_ENV).as_deref(), Some("1"));
        assert_eq!(get("WGPU_BACKEND"), None, "no backend is selected, not one");
        // The sw window is a concession to lavapipe's speed. Headless has no
        // fill cost, so its rows stay comparable with the rendered baseline -
        // and the parser refuses the combination that could bring it here.
        assert!(
            !env.iter()
                .any(|(k, _)| k == "NOVA_PROBE_WARMUP" || k == "NOVA_PROBE_FRAMES"),
            "headless keeps the baseline window: {env:?}"
        );
        assert_eq!(
            render_env(Render::Sw, true),
            render_env(Render::Gpu, true),
            "headless ignores the backend either way"
        );
    }

    #[test]
    fn render_sw_forces_the_lavapipe_icd() {
        let env = render_env(Render::Sw, false);
        let get = |k: &str| env.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        assert_eq!(get("WGPU_BACKEND").as_deref(), Some("vulkan"));
        assert!(get("VK_ICD_FILENAMES").unwrap().contains("lvp_icd"));
        assert_eq!(get(nova_probe::NORENDER_ENV), None);
    }

    #[test]
    fn render_gpu_adds_nothing() {
        assert!(render_env(Render::Gpu, false).is_empty());
    }

    /// A headless pass has no display to name, and a set-but-empty `DISPLAY`
    /// is worse than an absent one.
    #[test]
    fn a_pass_with_no_display_pushes_no_display_variable() {
        let root = Path::new("/repo");
        let out = Path::new("/repo/probe-runs/x");
        for env in [
            clean_pass_env(root, out, "", true),
            trace_pass_env(root, out, ""),
            samply_pass_env(root, out, ""),
        ] {
            assert!(
                !env.iter().any(|(k, _)| k == "DISPLAY"),
                "no DISPLAY without one: {env:?}"
            );
        }
        assert!(clean_pass_env(root, out, ":97", true)
            .iter()
            .any(|(k, v)| k == "DISPLAY" && v == ":97"));
    }

    #[test]
    fn clean_env_arms_only_the_requested_capture_surfaces() {
        let root = Path::new("/repo");
        let out = Path::new("/repo/probe-runs/x");
        let env = clean_pass_env(root, out, ":97", false);
        let get = |k: &str, e: &[(String, String)]| {
            e.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone())
        };
        assert_eq!(get("NOVA_AUTOPILOT", &env).as_deref(), Some("1"));
        assert_eq!(
            get("NOVA_PROBE_TIMELINE", &env).as_deref(),
            Some("/repo/probe-runs/x/timeline.jsonl")
        );
        assert_eq!(get("NOVA_PROBE_INVARIANTS", &env).as_deref(), Some("1"));
        assert_eq!(get("NOVA_PROBE", &env), None, "clean pass excludes fps");

        let env = clean_pass_env(root, out, ":97", true);
        assert_eq!(get("NOVA_PROBE", &env).as_deref(), Some("1"));
        assert_eq!(
            get("NOVA_PROBE_OUT", &env).as_deref(),
            Some("/repo/probe-runs/x")
        );
        // Rows label by the run-dir name so probe-vs-probe baselines
        // match (the capture's default "scene" matches nothing).
        assert_eq!(get("NOVA_PROBE_LABEL", &env).as_deref(), Some("x"));
        let env = clean_pass_env(root, out, ":97", false);
        assert_eq!(
            get("NOVA_PROBE_LABEL", &env),
            None,
            "label rides with frame-time capture only"
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
            get("NOVA_PROBE_TIMELINE"),
            None,
            "the profiled pass never overwrites the clean pass's timeline"
        );
    }
}
