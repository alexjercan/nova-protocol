//! `probe` - the run-harness front door (task 20260719-112317; spike
//! tasks/20260719-112011/SPIKE.md). One command runs an autopilot example
//! through the harness passes and hands back the unified run report:
//!
//! ```text
//! cargo run -p nova_probe -- run 10_playable            # clean pass + report
//! cargo run -p nova_probe -- run 10_playable --profile  # + traced pass
//! cargo run -p nova_probe -- sweep gpu                  # perf-baseline.sh
//! cargo run -p nova_probe -- web asteroid_field         # perf-web.sh
//! cargo run -p nova_probe -- profile 08_scenario        # perf-profile.sh
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
  run <example> [--out <dir>] [--profile] [--samply] [--fps]
                [--baseline <run-dir>] [--timeout <secs>] [--display <:N>]
      the post-feature check: clean pass (timeline + invariants + log),
      optional profiled pass and samply flamegraph, then the run report
  sweep [args...]     scripts/perf-baseline.sh (frame-time sweep, gpu|sw)
  web [args...]       scripts/perf-web.sh (web/WebGPU frame-time capture)
  profile [args...]   scripts/perf-profile.sh (trace + top-N systems table)";

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
    }

    /// Parsed command line.
    #[derive(Debug, PartialEq)]
    pub enum Cmd {
        Run(RunOptions),
        Script {
            script: &'static str,
            args: Vec<String>,
        },
    }

    /// Parse the CLI. Script subcommands pass their args through verbatim.
    pub fn parse(args: &[String]) -> Result<Cmd, String> {
        let mut iter = args.iter();
        match iter.next().map(String::as_str) {
            Some("run") => {
                let mut example: Option<String> = None;
                let mut opts = RunOptions {
                    example: String::new(),
                    out: None,
                    profile: false,
                    samply: false,
                    fps: false,
                    baseline: None,
                    timeout_secs: 180,
                    display: None,
                };
                while let Some(arg) = iter.next() {
                    match arg.as_str() {
                        "--profile" => opts.profile = true,
                        "--samply" => opts.samply = true,
                        "--fps" => opts.fps = true,
                        "--out" => {
                            opts.out =
                                Some(PathBuf::from(iter.next().ok_or("--out needs a directory")?));
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
                            opts.display =
                                Some(iter.next().ok_or("--display needs e.g. :0")?.clone());
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
                Ok(Cmd::Run(opts))
            }
            Some("sweep") => Ok(Cmd::Script {
                script: "scripts/perf-baseline.sh",
                args: iter.cloned().collect(),
            }),
            Some("web") => Ok(Cmd::Script {
                script: "scripts/perf-web.sh",
                args: iter.cloned().collect(),
            }),
            Some("profile") => Ok(Cmd::Script {
                script: "scripts/perf-profile.sh",
                args: iter.cloned().collect(),
            }),
            Some(other) => Err(format!("unknown subcommand {other}")),
            None => Err("a subcommand is required".into()),
        }
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
    const RUN_ARTIFACTS: [&str; 9] = [
        "timeline.jsonl",
        "run.log",
        "trace.json",
        "trace-run.log",
        "frametime.csv",
        "samply-profile.json.gz",
        "samply-run.log",
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

        // Pass 1: CLEAN. Whatever happens here, the run continues to the
        // report - a timeout/crash is a described outcome, never an abort.
        eprintln!(
            "probe: [1/{}] clean pass: building {}",
            passes_total(opts),
            opts.example
        );
        build_example(&root, &opts.example, "debug", None)?;
        let bin = root.join("target/debug/examples").join(&opts.example);
        eprintln!("probe: clean run -> {}", out.join("run.log").display());
        let env = clean_pass_env(&root, &out, &display, opts.fps);
        let outcome = run_supervised(&bin, &[], &root, &env, &out.join("run.log"), timeout)?;
        if !outcome.success() {
            eprintln!("probe: clean run did not succeed; the report will say so");
        }
        passes.push(PassRecord {
            name: "clean".into(),
            success: outcome.success(),
            timed_out: outcome.timed_out(),
        });

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

        // The manifest: this run's identity + outcomes, written BEFORE the
        // report so the report reads it like any other artifact.
        let (git_sha, host) = run_identity();
        let manifest = RunManifest {
            example: opts.example.clone(),
            started_unix,
            git_sha,
            host,
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: opts.fps,
            passes,
        };
        std::fs::write(
            out.join("probe-run.json"),
            format!("{:#}\n", manifest.to_json()),
        )
        .map_err(|e| format!("could not write probe-run.json: {e}"))?;

        // The report, in-process.
        let artifacts = RunArtifacts::load(&out, opts.baseline.as_deref())?;
        let checks = evaluate_checks(&artifacts);
        let verdict = overall_verdict(&checks);
        std::fs::write(
            out.join("report.html"),
            render_run_report(&out, &artifacts, &checks),
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
            Ok(Cmd::Script { script, args }) => {
                let root = repo_root();
                match Command::new("bash")
                    .arg(root.join(script))
                    .args(&args)
                    .current_dir(&root)
                    .status()
                {
                    Ok(status) if status.success() => ExitCode::SUCCESS,
                    Ok(_) => ExitCode::FAILURE,
                    Err(e) => {
                        eprintln!("probe: could not run {script}: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
            Ok(Cmd::Run(opts)) => match run(&opts) {
                Ok(code) => code,
                Err(message) => {
                    eprintln!("probe: {message}");
                    ExitCode::FAILURE
                }
            },
        }
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
        fn script_subcommands_resolve_their_scripts() {
            for (sub, script) in [
                ("sweep", "scripts/perf-baseline.sh"),
                ("web", "scripts/perf-web.sh"),
                ("profile", "scripts/perf-profile.sh"),
            ] {
                let Ok(Cmd::Script { script: got, args }) = parse(&s(&[sub, "x", "y"])) else {
                    panic!("{sub} parses to a script");
                };
                assert_eq!(got, script);
                assert_eq!(args, s(&["x", "y"]));
            }
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
