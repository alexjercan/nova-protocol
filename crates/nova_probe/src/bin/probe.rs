//! `probe` - the run-harness front door (task 20260719-112317; spike
//! tasks/20260719-112011/SPIKE.md). One command runs an autopilot example
//! through the harness passes and hands back the unified run report:
//!
//! ```text
//! cargo run -p nova_probe -- run 10_playable            # clean pass + report
//! cargo run -p nova_probe -- run 10_playable --profile  # + traced pass
//! cargo run -p nova_probe -- report <run-dir>           # re-render (manifest-gated)
//! cargo run -p nova_probe -- trace <trace.json>          # top-N systems table
//! ```
//!
//! `run` orchestrates natively: pass 1 CLEAN (timeline + invariants + log,
//! optionally the frame-time capture for wired examples), pass 2 PROFILED
//! (`--profile`: separate trace build, its overhead never touches pass 1's
//! numbers - the two-pass rule), optional `--samply` flamegraph run, then
//! the run report in-process. `sweep`/`web`/`profile` are thin wrappers
//! over the battle-tested scripts - one front door, scripts as the engine.

// Native-only like the recorder/report it wraps; the wasm build gets a stub
// main so `cargo check --target wasm32` over the package stays green.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    native::main()
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        path::{Path, PathBuf},
        process::{Child, Command, ExitCode, Stdio},
        time::{Duration, Instant},
    };

    use nova_probe::run_report::{
        checks_json, evaluate_checks, overall_verdict, print_checks, render_run_report,
        run_identity, PassRecord, RunArtifacts, RunManifest,
    };

    pub const USAGE: &str = "\
usage: probe <subcommand>
  run <example|scenario> [--out <dir>] [--profile] [--samply] [--fps]
      [--baseline <run-dir>] [--timeout <secs>] [--display <:N>]
      [--release] [--render gpu|sw] [--scenario <id>]... [--preset <p>]...
      [--platform native|web]
      the post-feature check and the perf sweep. Matrix flags (--scenario/
      --preset, repeatable) sweep the frame-time capture (with --fps);
      --platform web captures the web/WebGPU frame line (positional =
      scenario id). Artifacts + report land in the run dir.
  report <run-dir> [--baseline <run-dir>]
      re-render the report; refuses dirs without probe-run.json
  trace <trace.json> [--top N] [-o <table.md>]
      top-N costliest-systems table from a chrome trace
  sweep|web|profile   DEPRECATED aliases that map onto `run` flags";

    /// Parsed `probe run` options.
    #[derive(Debug, PartialEq)]
    pub struct RunOptions {
        pub example: String,
        pub out: Option<PathBuf>,
        pub profile: bool,
        pub samply: bool,
        pub fps: bool,
        pub baseline: Option<PathBuf>,
        pub timeout_secs: u64,
        pub display: Option<String>,
        pub release: bool,
        pub render: Render,
        pub scenarios: Vec<String>,
        pub presets: Vec<String>,
        pub platform: Platform,
    }

    /// Renderer for the capture: the real GPU, or the lavapipe software
    /// floor (the worst-case CPU/fill bracket; NOT a web stand-in).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Render {
        Gpu,
        Sw,
    }

    /// Where the run executes. Web runs the perf_web wasm build under
    /// headless Chromium and captures the frame line only (the recorder and
    /// invariants are native-only by design).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Platform {
        Native,
        Web,
    }

    /// Parsed command line.
    #[derive(Debug, PartialEq)]
    pub enum Cmd {
        Run(RunOptions),
        Report {
            dir: PathBuf,
            baseline: Option<PathBuf>,
        },
        Trace {
            file: PathBuf,
            top: usize,
            out: Option<PathBuf>,
        },
    }

    fn default_run(example: String) -> RunOptions {
        RunOptions {
            example,
            out: None,
            profile: false,
            samply: false,
            fps: false,
            baseline: None,
            timeout_secs: 180,
            display: None,
            release: false,
            render: Render::Gpu,
            scenarios: Vec::new(),
            presets: Vec::new(),
            platform: Platform::Native,
        }
    }

    /// Parse the CLI. The deprecated aliases map onto `run` invocations and
    /// say so on stderr.
    pub fn parse(args: &[String]) -> Result<Cmd, String> {
        let mut iter = args.iter();
        match iter.next().map(String::as_str) {
            Some("run") => parse_run(iter.cloned().collect::<Vec<_>>()),
            Some("report") => {
                let mut dir: Option<PathBuf> = None;
                let mut baseline: Option<PathBuf> = None;
                while let Some(arg) = iter.next() {
                    match arg.as_str() {
                        "--baseline" => {
                            baseline = Some(PathBuf::from(
                                iter.next().ok_or("--baseline needs a run dir")?,
                            ));
                        }
                        other if other.starts_with('-') => {
                            return Err(format!("unknown flag {other}"));
                        }
                        other => {
                            if dir.replace(PathBuf::from(other)).is_some() {
                                return Err("only one run dir may be given".into());
                            }
                        }
                    }
                }
                Ok(Cmd::Report {
                    dir: dir.ok_or("report needs a run dir")?,
                    baseline,
                })
            }
            Some("trace") => {
                let mut file: Option<PathBuf> = None;
                let mut top = 20usize;
                let mut out: Option<PathBuf> = None;
                while let Some(arg) = iter.next() {
                    match arg.as_str() {
                        "--top" => {
                            top = iter
                                .next()
                                .and_then(|v| v.parse().ok())
                                .ok_or("--top needs a number")?;
                        }
                        "-o" | "--output" => {
                            out = Some(PathBuf::from(iter.next().ok_or("-o needs a path")?));
                        }
                        other if other.starts_with('-') => {
                            return Err(format!("unknown flag {other}"));
                        }
                        other => {
                            if file.replace(PathBuf::from(other)).is_some() {
                                return Err("only one trace file may be given".into());
                            }
                        }
                    }
                }
                Ok(Cmd::Trace {
                    file: file.ok_or("trace needs a chrome-trace file")?,
                    top,
                    out,
                })
            }
            // Deprecated aliases: map onto run flags (perf-baseline.sh /
            // perf-web.sh / perf-profile.sh are gone; the mapping preserves
            // their defaults).
            Some("sweep") => {
                let rest: Vec<String> = iter.cloned().collect();
                let render = match rest.first().map(String::as_str) {
                    Some("sw") => Render::Sw,
                    _ => Render::Gpu,
                };
                eprintln!(
                    "probe: `sweep` is deprecated; use: probe run 20_perf_baseline --fps \
                     --release --render {} --scenario asteroid_field --scenario broadside \
                     --scenario shakedown_run --preset high --preset low",
                    if render == Render::Sw { "sw" } else { "gpu" }
                );
                let mut opts = default_run("20_perf_baseline".into());
                opts.fps = true;
                opts.release = true;
                opts.render = render;
                opts.scenarios = ["asteroid_field", "broadside", "shakedown_run"]
                    .map(String::from)
                    .to_vec();
                opts.presets = ["high", "low"].map(String::from).to_vec();
                if let Some(out) = rest.get(1) {
                    opts.out = Some(PathBuf::from(out));
                }
                Ok(Cmd::Run(opts))
            }
            Some("web") => {
                let scenario = iter
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "asteroid_field".into());
                eprintln!("probe: `web` is deprecated; use: probe run {scenario} --platform web");
                let mut opts = default_run(scenario);
                opts.platform = Platform::Web;
                Ok(Cmd::Run(opts))
            }
            Some("profile") => {
                let example = iter.next().cloned().unwrap_or_else(|| "08_scenario".into());
                eprintln!(
                    "probe: `profile` is deprecated; use: probe run {example} --profile \
                     (add --samply for the flamegraph)"
                );
                let mut opts = default_run(example);
                opts.profile = true;
                Ok(Cmd::Run(opts))
            }
            Some(other) => Err(format!("unknown subcommand {other}")),
            None => Err("a subcommand is required".into()),
        }
    }

    fn parse_run(args: Vec<String>) -> Result<Cmd, String> {
        let mut example: Option<String> = None;
        let mut opts = default_run(String::new());
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--profile" => opts.profile = true,
                "--samply" => opts.samply = true,
                "--fps" => opts.fps = true,
                "--release" => opts.release = true,
                "--out" => {
                    opts.out = Some(PathBuf::from(iter.next().ok_or("--out needs a directory")?));
                }
                "--baseline" => {
                    opts.baseline = Some(PathBuf::from(
                        iter.next().ok_or("--baseline needs a run dir")?,
                    ));
                }
                "--timeout" => {
                    opts.timeout_secs = iter
                        .next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("--timeout needs seconds")?;
                }
                "--display" => {
                    opts.display = Some(iter.next().ok_or("--display needs e.g. :0")?.clone());
                }
                "--render" => {
                    opts.render = match iter.next().map(String::as_str) {
                        Some("gpu") => Render::Gpu,
                        Some("sw") => Render::Sw,
                        _ => return Err("--render needs gpu or sw".into()),
                    };
                }
                "--scenario" => {
                    opts.scenarios
                        .push(iter.next().ok_or("--scenario needs an id")?.clone());
                }
                "--preset" => {
                    opts.presets
                        .push(iter.next().ok_or("--preset needs low|medium|high")?.clone());
                }
                "--platform" => {
                    opts.platform = match iter.next().map(String::as_str) {
                        Some("native") => Platform::Native,
                        Some("web") => Platform::Web,
                        _ => return Err("--platform needs native or web".into()),
                    };
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown flag {other}"));
                }
                other => {
                    if example.replace(other.to_string()).is_some() {
                        return Err("only one example may be given".into());
                    }
                }
            }
        }
        opts.example = example.ok_or("run needs an example name")?;
        // Honest-combination gates: the matrix is a perf sweep (needs the
        // capture armed), and the web pipeline has no native passes.
        let matrix = !opts.scenarios.is_empty() || !opts.presets.is_empty();
        if matrix && !opts.fps {
            return Err("--scenario/--preset form a perf sweep: add --fps".into());
        }
        if opts.platform == Platform::Web && (opts.profile || opts.samply || opts.fps || matrix) {
            return Err(
                "--platform web captures the web frame line only; it does not combine \
                 with --profile/--samply/--fps/--scenario/--preset"
                    .into(),
            );
        }
        Ok(Cmd::Run(opts))
    }

    /// The repo root, derived from this crate's manifest dir at compile time
    /// (crates/nova_probe -> ../..). A dev tool run via cargo from the repo.
    pub fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    }

    /// Environment for the CLEAN pass: autopilot + recorder + invariants
    /// always; the frame-time capture only on request (`--fps`) since only
    /// the wired examples (20_perf_baseline) read it - elsewhere it is a
    /// harmless no-op env.
    pub fn clean_pass_env(
        root: &Path,
        out: &Path,
        display: &str,
        fps: bool,
    ) -> Vec<(String, String)> {
        let mut env = vec![
            ("BCS_AUTOPILOT".into(), "1".into()),
            ("BEVY_ASSET_ROOT".into(), root.display().to_string()),
            ("DISPLAY".into(), display.into()),
            (
                "NOVA_PERF_TIMELINE".into(),
                out.join("timeline.jsonl").display().to_string(),
            ),
            ("NOVA_PERF_INVARIANTS".into(), "1".into()),
        ];
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
    pub fn sweep_cell_env(
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
            let icd = std::env::var("LVP_ICD").unwrap_or_else(|_| {
                "/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json".into()
            });
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
    /// correctness or FPS sections.
    pub fn trace_pass_env(root: &Path, out: &Path, display: &str) -> Vec<(String, String)> {
        let rust_log = match std::env::var("RUST_LOG") {
            Ok(existing) if !existing.is_empty() => format!("{existing},bevy_ecs=info"),
            _ => "bevy_ecs=info".into(),
        };
        vec![
            ("BCS_AUTOPILOT".into(), "1".into()),
            ("BEVY_ASSET_ROOT".into(), root.display().to_string()),
            ("DISPLAY".into(), display.into()),
            (
                "TRACE_CHROME".into(),
                out.join("trace.json").display().to_string(),
            ),
            ("RUST_LOG".into(), rust_log),
        ]
    }

    /// Kill-by-recorded-PID guard for the throwaway Xvfb (never pkill).
    struct XvfbGuard(Child);

    impl Drop for XvfbGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// The throwaway-Xvfb display for this process: pid-derived so two
    /// concurrent `probe run`s get distinct servers. The :80-:89 band stays
    /// clear of the perf scripts' hardcoded :94/:95 (a ten-way pid
    /// collision is possible but vanishingly unlikely for a dev tool -
    /// pass --display to pin one explicitly).
    pub fn default_display() -> String {
        format!(":{}", 80 + std::process::id() % 10)
    }

    /// Use the explicit display, or spawn a throwaway Xvfb on a private one.
    fn ensure_display(explicit: Option<&str>) -> Result<(String, Option<XvfbGuard>), String> {
        if let Some(display) = explicit {
            return Ok((display.to_string(), None));
        }
        let display = default_display();
        let mut child = Command::new("Xvfb")
            .args([display.as_str(), "-screen", "0", "1280x720x24"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not start Xvfb (is it installed?): {e}"))?;
        std::thread::sleep(Duration::from_secs(2));
        // A dead child here means the display is taken (or Xvfb refused);
        // running the example against it would fail confusingly later.
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!(
                "Xvfb on {display} exited immediately ({status}) - display in use? \
                 pass --display to pin a free one"
            ));
        }
        Ok((display, Some(XvfbGuard(child))))
    }

    /// Build the example with the given feature set, streaming cargo output.
    fn build_example(
        root: &Path,
        example: &str,
        features: &str,
        profile: Option<&str>,
    ) -> Result<(), String> {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root)
            .args(["build", "--example", example, "--features", features]);
        if let Some(profile) = profile {
            cmd.args(["--profile", profile]);
            // Frame pointers for honest sampled stacks; only ever combined
            // with the profiling profile so its cache stays consistent.
            let flags = std::env::var("RUSTFLAGS").unwrap_or_default();
            cmd.env(
                "RUSTFLAGS",
                format!("{} -C force-frame-pointers=yes", flags)
                    .trim()
                    .to_string(),
            );
        }
        let status = cmd
            .status()
            .map_err(|e| format!("could not run cargo: {e}"))?;
        if !status.success() {
            return Err(format!("cargo build --example {example} failed"));
        }
        Ok(())
    }

    /// How a supervised child run ended. A timeout is an OUTCOME, not an
    /// error: the hung-run case is exactly what the report must describe
    /// (finding 2 - the old Err path aborted before any report existed).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum RunOutcome {
        Completed { success: bool },
        TimedOut,
    }

    impl RunOutcome {
        fn success(self) -> bool {
            matches!(self, RunOutcome::Completed { success: true })
        }
        fn timed_out(self) -> bool {
            matches!(self, RunOutcome::TimedOut)
        }
    }

    /// Run a supervised child with `env`, capturing stdout+stderr to
    /// `log_path`, killing it after `timeout` (a hung run must not wedge
    /// the check - the autopilot's own backstop normally exits far
    /// earlier). Errors only for infrastructure failures (spawn/log IO).
    fn run_supervised(
        bin: &Path,
        extra_args: &[&str],
        root: &Path,
        env: &[(String, String)],
        log_path: &Path,
        timeout: Duration,
    ) -> Result<RunOutcome, String> {
        let log = std::fs::File::create(log_path)
            .map_err(|e| format!("could not create {}: {e}", log_path.display()))?;
        let err_log = log
            .try_clone()
            .map_err(|e| format!("could not clone log handle: {e}"))?;
        let mut child = Command::new(bin)
            .args(extra_args)
            .current_dir(root)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(err_log))
            .spawn()
            .map_err(|e| format!("could not run {}: {e}", bin.display()))?;
        let start = Instant::now();
        loop {
            match child.try_wait().map_err(|e| e.to_string())? {
                Some(status) => {
                    return Ok(RunOutcome::Completed {
                        success: status.success(),
                    })
                }
                None if start.elapsed() > timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    eprintln!(
                        "probe: run exceeded {}s and was killed (log: {})",
                        timeout.as_secs(),
                        log_path.display()
                    );
                    return Ok(RunOutcome::TimedOut);
                }
                None => std::thread::sleep(Duration::from_millis(250)),
            }
        }
    }

    /// probe's own artifact filenames: surgically removed from the out dir
    /// at the start of a run so nothing stale (an old trace, a previous
    /// checks.json) can present as this run's evidence (finding 1). Never
    /// a recursive wipe - the dir may be user-supplied.
    const RUN_ARTIFACTS: [&str; 10] = [
        "timeline.jsonl",
        "run.log",
        "trace.json",
        "trace-run.log",
        "frametime.csv",
        "samply-profile.json.gz",
        "samply-run.log",
        "web-run.log",
        "report.html",
        "checks.json",
    ];

    fn clean_out_dir(out: &Path) -> Result<(), String> {
        for name in RUN_ARTIFACTS {
            let path = out.join(name);
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("could not clear stale {}: {e}", path.display()))?;
            }
        }
        let manifest = out.join("probe-run.json");
        if manifest.exists() {
            std::fs::remove_file(&manifest)
                .map_err(|e| format!("could not clear stale {}: {e}", manifest.display()))?;
        }
        Ok(())
    }

    fn run(opts: &RunOptions) -> Result<ExitCode, String> {
        let root = repo_root();
        let out = opts
            .out
            .clone()
            .unwrap_or_else(|| root.join("probe-runs").join(&opts.example));
        std::fs::create_dir_all(&out).map_err(|e| format!("could not create out dir: {e}"))?;
        let out = out
            .canonicalize()
            .map_err(|e| format!("could not resolve out dir: {e}"))?;
        // Nothing stale may survive into this run's report.
        clean_out_dir(&out)?;
        // A bad baseline path must fail BEFORE minutes of build+run
        // (finding 2c), and it must actually parse.
        if let Some(baseline) = &opts.baseline {
            let csv = baseline.join("frametime.csv");
            let contents = std::fs::read_to_string(&csv).map_err(|e| {
                format!("--baseline invalid before running: {}: {e}", csv.display())
            })?;
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

        // Web platform: a single scraped capture pass, then the report.
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

        // Pass 1: CLEAN (native). With matrix flags this is the perf sweep:
        // one supervised run per scenario x preset cell, all appending into
        // THIS run's fresh frametime.csv; the recorder/invariants only arm
        // on the single-cell shape (a sweep measures frames, and each cell
        // would clobber the previous cell's timeline).
        let cells = matrix_cells(&opts.scenarios, &opts.presets);
        let sweeping = cells.len() > 1 || opts.scenarios.first().is_some();
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
            let mut env = clean_pass_env(&root, &out, &display, opts.fps);
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

        // Pass 2: PROFILED (optional; separate build so tracing overhead
        // never touches pass 1's numbers). Failures degrade to "no trace" -
        // a successful clean pass is never discarded (finding 2b).
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
        // cannot wedge probe either, finding 11).
        if opts.samply {
            eprintln!("probe: [{n}/{n}] samply pass", n = passes_total(opts));
            match build_example(&root, &opts.example, "debug", Some("profiling")) {
                Err(e) => eprintln!("probe: samply build failed ({e}); flamegraph skipped"),
                Ok(()) => {
                    let sbin = root.join("target/profiling/examples").join(&opts.example);
                    let samply_env = vec![
                        ("BCS_AUTOPILOT".to_string(), "1".to_string()),
                        ("BEVY_ASSET_ROOT".to_string(), root.display().to_string()),
                        ("DISPLAY".to_string(), display.clone()),
                    ];
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

    /// The web/WebGPU capture (ported from perf-web.sh): trunk-build the
    /// perf_web wasm app, serve it from an embedded static server, drive a
    /// headless-display Chromium at the perf URL with the EXACT flag set
    /// the v0.7.0 baseline work calibrated (WebGPU needs the GPU process +
    /// Vulkan + a non-blocklisted adapter), scrape the `nova perf:` summary
    /// line from the console log, and write it as a frametime.csv row. The
    /// positional argument is the SCENARIO id on this platform.
    fn web_capture(
        root: &Path,
        out: &Path,
        display: &str,
        scenario: &str,
        timeout: Duration,
    ) -> Result<PassRecord, String> {
        let quality = std::env::var("QUALITY").unwrap_or_else(|_| "high".into());
        let frames = std::env::var("FRAMES").unwrap_or_else(|_| "600".into());
        let warmup = std::env::var("WARMUP").unwrap_or_else(|_| "180".into());
        let label = format!("{scenario}-{quality}-web");

        // Build the wasm bundle (trunk only supports the release profile).
        let dist = root.join("target/probe-dist");
        eprintln!(
            "probe: trunk build (release) perf.html -> {}",
            dist.display()
        );
        let status = Command::new("trunk")
            .current_dir(root)
            .args(["build", "--release", "-d"])
            .arg(&dist)
            .arg("perf.html")
            .status()
            .map_err(|e| format!("could not run trunk (is it installed?): {e}"))?;
        if !status.success() {
            return Err("trunk build failed".into());
        }

        // Serve the dist dir on an ephemeral local port.
        let port = serve_dir(dist.clone())?;
        let url = format!(
            "http://127.0.0.1:{port}/?perf=1&scenario={scenario}&quality={quality}\
             &frames={frames}&warmup={warmup}&label={label}"
        )
        .replace(' ', "");
        eprintln!("probe: chromium -> {url}");

        // Chromium with the calibrated WebGPU flags (verbatim from
        // perf-web.sh; probed "ADAPTER OK 21 features" on this rig).
        let log_path = out.join("web-run.log");
        let log = std::fs::File::create(&log_path)
            .map_err(|e| format!("could not create {}: {e}", log_path.display()))?;
        let err_log = log.try_clone().map_err(|e| e.to_string())?;
        let mut chromium = Command::new("chromium")
            .args([
                "--no-sandbox",
                "--disable-gpu-sandbox",
                "--ignore-gpu-blocklist",
                "--enable-unsafe-webgpu",
                "--enable-features=Vulkan,WebGPU",
                "--use-angle=vulkan",
                "--enable-logging=stderr",
                "--v=1",
                "--window-size=1280,720",
            ])
            .arg(&url)
            .env("DISPLAY", display)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(err_log))
            .spawn()
            .map_err(|e| format!("could not run chromium (is it installed?): {e}"))?;

        // Poll the console log for the summary line; kill by recorded PID
        // either way.
        let needle = format!("nova perf: label={label} frames");
        let start = Instant::now();
        let mut found = false;
        while start.elapsed() < timeout {
            if std::fs::read_to_string(&log_path)
                .map(|log| log.contains(&needle))
                .unwrap_or(false)
            {
                found = true;
                break;
            }
            if let Ok(Some(_)) = chromium.try_wait() {
                break; // chromium exited early; the log has the story
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        let timed_out = !found && start.elapsed() >= timeout;
        let _ = chromium.kill();
        let _ = chromium.wait();

        let mut scraped = false;
        if found {
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            let parsed = log
                .lines()
                .find(|l| l.contains(&needle))
                .and_then(nova_probe::parse_summary_line);
            let Some((parsed_label, stats)) = parsed else {
                // Degrade, never abort: the chromium log holds the line for
                // forensics and the report will show the failed pass.
                eprintln!(
                    "probe: summary line found but not parseable (see {}); \
                     the report will show the failed capture",
                    log_path.display()
                );
                return Ok(PassRecord {
                    name: "web".into(),
                    success: false,
                    timed_out: false,
                });
            };
            scraped = true;
            // Adapter identity when chromium logged it.
            let adapter = log
                .lines()
                .find(|l| l.contains("AdapterInfo {"))
                .and_then(|l| l.split("name: \"").nth(1))
                .and_then(|l| l.split('\"').next())
                .unwrap_or("unknown")
                .to_string();
            let (git_sha, host) = run_identity();
            let meta = nova_probe::RunMeta {
                backend: "webgpu".into(),
                adapter,
                resolution: "1280x720".into(),
                quality: quality.clone(),
                git_sha,
                host,
            };
            nova_probe::append_frametime_row(
                &out.join("frametime.csv"),
                &parsed_label,
                &stats,
                &meta,
            )?;
            eprintln!(
                "probe: web capture scraped -> {}",
                out.join("frametime.csv").display()
            );
        } else {
            eprintln!(
                "probe: no summary line captured (see {}); the report will show it",
                log_path.display()
            );
        }
        Ok(PassRecord {
            name: "web".into(),
            success: scraped,
            timed_out,
        })
    }

    /// Serve `dir` statically on an ephemeral 127.0.0.1 port from a daemon
    /// thread (dies with the process). Minimal GET-only server - enough for
    /// trunk's dist output; `.wasm` gets its real content type so streaming
    /// instantiation works.
    fn serve_dir(dir: PathBuf) -> Result<u16, String> {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("could not bind the static server: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let dir = dir.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(match stream.try_clone() {
                        Ok(s) => s,
                        Err(_) => return,
                    });
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        return;
                    }
                    // Drain headers.
                    let mut line = String::new();
                    while reader.read_line(&mut line).is_ok() && line.trim() != "" {
                        line.clear();
                    }
                    let path = request_line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("/")
                        .split('?')
                        .next()
                        .unwrap_or("/");
                    let rel = path.trim_start_matches('/');
                    let file = if rel.is_empty() {
                        dir.join("index.html")
                    } else {
                        dir.join(rel)
                    };
                    // No traversal outside the dist dir.
                    let safe = file
                        .canonicalize()
                        .ok()
                        .filter(|f| f.starts_with(&dir))
                        .filter(|f| f.is_file());
                    match safe.and_then(|f| std::fs::read(&f).ok().map(|b| (f, b))) {
                        Some((f, body)) => {
                            let ctype = match f.extension().and_then(|e| e.to_str()) {
                                Some("html") => "text/html",
                                Some("js") => "application/javascript",
                                Some("wasm") => "application/wasm",
                                Some("css") => "text/css",
                                Some("png") => "image/png",
                                Some("wav") => "audio/wav",
                                Some("ron") | Some("json") => "text/plain",
                                _ => "application/octet-stream",
                            };
                            let _ = write!(
                                stream,
                                "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\n\
                                 Content-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(&body);
                        }
                        None => {
                            let _ = write!(
                                stream,
                                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\
                                 Connection: close\r\n\r\n"
                            );
                        }
                    }
                });
            }
        });
        Ok(port)
    }

    /// Cells of the sweep matrix (scenarios x presets); a missing dimension
    /// contributes a single None cell, so no flags = one default cell.
    fn matrix_cells(
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

    /// Write the manifest, assemble the report in-process, print + exit.
    fn finish_report(
        opts: &RunOptions,
        out: &Path,
        started_unix: u64,
        passes: Vec<PassRecord>,
        armed_native: bool,
    ) -> Result<ExitCode, String> {
        let (git_sha, host) = run_identity();
        let manifest = RunManifest {
            example: opts.example.clone(),
            started_unix,
            git_sha,
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

        // The report, in-process.
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
        Ok(if verdict == "FAIL" || verdict == "NO_DATA" {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        })
    }

    fn passes_total(opts: &RunOptions) -> usize {
        1 + usize::from(opts.profile) + usize::from(opts.samply)
    }

    pub fn main() -> ExitCode {
        let args: Vec<String> = std::env::args().skip(1).collect();
        match parse(&args) {
            Err(message) => {
                eprintln!("probe: {message}\n\n{USAGE}");
                ExitCode::FAILURE
            }
            Ok(Cmd::Run(opts)) => match run(&opts) {
                Ok(code) => code,
                Err(message) => {
                    eprintln!("probe: {message}");
                    ExitCode::FAILURE
                }
            },
            Ok(Cmd::Report { dir, baseline }) => match report(&dir, baseline.as_deref()) {
                Ok(code) => code,
                Err(message) => {
                    eprintln!("probe: {message}");
                    ExitCode::FAILURE
                }
            },
            Ok(Cmd::Trace { file, top, out }) => match trace_table(&file, top, out.as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("probe: {message}");
                    ExitCode::FAILURE
                }
            },
        }
    }

    /// `probe report`: re-render an existing run dir - GATED on the
    /// manifest, so a report can only ever be built from a dir `probe run`
    /// itself produced (stale hand-assembled folders are refused, which is
    /// the whole point of the gate).
    fn report(dir: &Path, baseline: Option<&Path>) -> Result<ExitCode, String> {
        if !dir.join("probe-run.json").exists() {
            return Err(format!(
                "{} has no probe-run.json - probe only reports over dirs it produced; \
                 run `probe run <example> --out {}` first",
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
        Ok(if verdict == "FAIL" || verdict == "NO_DATA" {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        })
    }

    /// `probe trace`: the standalone top-N table (absorbed perf_trace).
    fn trace_table(file: &Path, top: usize, out: Option<&Path>) -> Result<(), String> {
        let contents = std::fs::read_to_string(file)
            .map_err(|e| format!("could not read {}: {e}", file.display()))?;
        let costs = nova_probe::profile::aggregate_system_costs(&contents)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let table = nova_probe::profile::render_top_table(&costs, top);
        match out {
            Some(path) => {
                std::fs::write(path, &table)
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                println!(
                    "probe: wrote {} ({} systems aggregated)",
                    path.display(),
                    costs.len()
                );
            }
            None => print!("{table}"),
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn s(args: &[&str]) -> Vec<String> {
            args.iter().map(|a| a.to_string()).collect()
        }

        #[test]
        fn parse_run_with_all_flags() {
            let cmd = parse(&s(&[
                "run",
                "10_playable",
                "--profile",
                "--samply",
                "--fps",
                "--out",
                "runs/x",
                "--baseline",
                "runs/old",
                "--timeout",
                "60",
                "--display",
                ":0",
            ]))
            .expect("parses");
            let Cmd::Run(opts) = cmd else {
                panic!("expected run");
            };
            assert_eq!(opts.example, "10_playable");
            assert!(opts.profile && opts.samply && opts.fps);
            assert_eq!(opts.out, Some(PathBuf::from("runs/x")));
            assert_eq!(opts.baseline, Some(PathBuf::from("runs/old")));
            assert_eq!(opts.timeout_secs, 60);
            assert_eq!(opts.display.as_deref(), Some(":0"));
        }

        #[test]
        fn parse_rejects_bad_input() {
            assert!(parse(&s(&[])).is_err());
            assert!(parse(&s(&["run"])).is_err(), "example required");
            assert!(parse(&s(&["run", "a", "b"])).is_err(), "one example only");
            assert!(parse(&s(&["run", "a", "--nope"])).is_err());
            assert!(parse(&s(&["frobnicate"])).is_err());
        }

        #[test]
        fn new_verbs_and_flags_parse() {
            let Ok(Cmd::Report { dir, baseline }) =
                parse(&s(&["report", "runs/x", "--baseline", "runs/old"]))
            else {
                panic!("report parses");
            };
            assert_eq!(dir, PathBuf::from("runs/x"));
            assert_eq!(baseline, Some(PathBuf::from("runs/old")));

            let Ok(Cmd::Trace { file, top, out }) =
                parse(&s(&["trace", "t.json", "--top", "7", "-o", "t.md"]))
            else {
                panic!("trace parses");
            };
            assert_eq!(file, PathBuf::from("t.json"));
            assert_eq!(top, 7);
            assert_eq!(out, Some(PathBuf::from("t.md")));

            let Ok(Cmd::Run(opts)) = parse(&s(&[
                "run",
                "20_perf_baseline",
                "--fps",
                "--release",
                "--render",
                "sw",
                "--scenario",
                "a",
                "--scenario",
                "b",
                "--preset",
                "high",
            ])) else {
                panic!("sweep-shaped run parses");
            };
            assert!(opts.release && opts.fps);
            assert_eq!(opts.render, Render::Sw);
            assert_eq!(opts.scenarios, s(&["a", "b"]));
            assert_eq!(opts.presets, s(&["high"]));
        }

        #[test]
        fn honest_combination_gates() {
            // A matrix without --fps is a sweep that measures nothing.
            assert!(parse(&s(&["run", "x", "--scenario", "a"])).is_err());
            // Web does not combine with the native-only passes.
            assert!(parse(&s(&["run", "x", "--platform", "web", "--profile"])).is_err());
            assert!(parse(&s(&["run", "x", "--platform", "web", "--fps"])).is_err());
            // Web alone is fine.
            let Ok(Cmd::Run(opts)) = parse(&s(&["run", "asteroid_field", "--platform", "web"]))
            else {
                panic!("web run parses");
            };
            assert_eq!(opts.platform, Platform::Web);
        }

        #[test]
        fn deprecated_aliases_map_onto_run_flags() {
            let Ok(Cmd::Run(opts)) = parse(&s(&["sweep", "sw", "out/dir"])) else {
                panic!("sweep alias maps");
            };
            assert_eq!(opts.example, "20_perf_baseline");
            assert!(opts.fps && opts.release);
            assert_eq!(opts.render, Render::Sw);
            assert_eq!(opts.scenarios.len(), 3);
            assert_eq!(opts.presets, s(&["high", "low"]));
            assert_eq!(opts.out, Some(PathBuf::from("out/dir")));

            let Ok(Cmd::Run(opts)) = parse(&s(&["web", "broadside"])) else {
                panic!("web alias maps");
            };
            assert_eq!(opts.example, "broadside");
            assert_eq!(opts.platform, Platform::Web);

            let Ok(Cmd::Run(opts)) = parse(&s(&["profile"])) else {
                panic!("profile alias maps");
            };
            assert_eq!(opts.example, "08_scenario");
            assert!(opts.profile);
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
        fn default_display_is_pid_derived_within_the_reserved_band() {
            let display = default_display();
            let n: u32 = display.strip_prefix(':').unwrap().parse().unwrap();
            // :80-:89 - clear of the perf scripts' hardcoded :94/:95.
            assert!((80..=89).contains(&n), "{display}");
            assert_eq!(display, default_display(), "stable within one process");
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
}
