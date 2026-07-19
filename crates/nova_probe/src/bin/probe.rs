//! `probe` - the run-harness front door (task 20260719-112317; spike
//! tasks/20260719-112011/SPIKE.md; multi-run task 20260719-210438). One
//! command runs autopilot examples through the harness passes and hands
//! back the unified run report - or, for a multi spec, one report per
//! example plus an aggregated status index:
//!
//! ```text
//! cargo run -p nova_probe -- run playable            # clean pass + report
//! cargo run -p nova_probe -- run playable --profile  # + traced pass
//! cargo run -p nova_probe -- run playable,scenario   # comma list -> aggregate
//! cargo run -p nova_probe -- run ui                  # a category dir's examples
//! cargo run -p nova_probe -- run --all               # the whole catalog (minus NOT_PROBED)
//! cargo run -p nova_probe -- report <run-dir>           # re-render (manifest-gated)
//! ```
//!
//! `run` orchestrates natively: pass 1 CLEAN (timeline + invariants + log,
//! optionally the frame-time capture for wired examples), pass 2 PROFILED
//! (`--profile`: separate trace build, its overhead never touches pass 1's
//! numbers - the two-pass rule), optional `--samply` flamegraph run, then
//! the run report in-process. Multi specs resolve against the Cargo.toml
//! `[[example]]` catalog (the single source of truth - autoexamples is
//! off), run sequentially with continue-on-failure, and write
//! `index.html` + `index.json` + `probe-all.json` above the per-example
//! run dirs.

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
  run <spec> [--all] [--out <dir>] [--profile] [--samply] [--fps]
      [--baseline <run-dir>] [--timeout <secs>] [--display <:N>]
      [--release] [--render gpu|sw] [--scenario <id>]... [--preset <p>]...
      [--platform native|web]
      the post-feature check and the perf sweep. <spec> is one example, a
      comma list (playable,scenario), or a category dir (sections|gameplay|
      ui|screenshots|perf); --all runs the whole catalog minus NOT_PROBED.
      Multi specs run sequentially into <out|probe-runs>/<example>/ and
      write an aggregated index.html/index.json + probe-all.json above
      them. Matrix flags (--scenario/--preset, repeatable, with --fps),
      --platform web (positional = scenario id) and --baseline are
      single-example concerns. Artifacts + report land in the run dir.
  report <run-dir> [--baseline <run-dir>]
      re-render the report (probe-run.json dirs) or the aggregate index
      (probe-all.json dirs); refuses dirs probe did not produce";

    /// Parsed `probe run` options.
    #[derive(Debug, Clone, PartialEq)]
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
        /// A `probe run` spec, resolved against the example catalog at
        /// dispatch (parse stays pure/fs-free): `tokens` is the comma-split
        /// positional (possibly empty - resolution errors with the catalog
        /// listing), `all` the --all flag.
        RunSpec {
            tokens: Vec<String>,
            all: bool,
            base: RunOptions,
        },
        Report {
            dir: PathBuf,
            baseline: Option<PathBuf>,
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

    /// Parse the CLI: `run` and `report`. (The deprecated `sweep|web|profile`
    /// aliases and the `trace` verb retired at the v0.8.0 cut, task
    /// 20260719-211500 - `--profile` renders the top-N table in-report, and
    /// `probe report` re-renders it from the run dir.)
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
            // Retired verbs get a pointed error, not a generic one: the
            // muscle-memory commands should say where they went.
            Some("trace") => Err(
                "`trace` retired (task 20260719-211500): `--profile` renders the \
                 top-N table into the run report, and `probe report <run-dir>` \
                 re-renders it from the dir's trace.json"
                    .into(),
            ),
            Some(alias @ ("sweep" | "web" | "profile")) => Err(format!(
                "`{alias}` retired (deprecated for one cycle, removed at v0.8.0): \
                 use `probe run` - the sweep is `run perf_baseline --fps --release \
                 --scenario ... --preset ...`, web is `run <scenario> --platform web`, \
                 profiling is `run <example> --profile [--samply]`"
            )),
            Some(other) => Err(format!("unknown subcommand {other}")),
            None => Err("a subcommand is required".into()),
        }
    }

    fn parse_run(args: Vec<String>) -> Result<Cmd, String> {
        let mut example: Option<String> = None;
        let mut all = false;
        let mut opts = default_run(String::new());
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--all" => all = true,
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
                        return Err("only one spec may be given (commas form a list)".into());
                    }
                }
            }
        }
        if all && example.is_some() {
            return Err("give a spec or --all, not both".into());
        }
        // Honest-combination gates that need no catalog: the matrix is a
        // perf sweep (needs the capture armed), and the web pipeline has no
        // native passes. Multi-spec gates live in resolve (they need to
        // know whether the spec expands).
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
        let tokens: Vec<String> = example
            .map(|spec| {
                spec.split(',')
                    .filter(|token| !token.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Cmd::RunSpec {
            tokens,
            all,
            base: opts,
        })
    }

    /// Examples that `--all` and category expansion SKIP, each with its
    /// reason - listed in the aggregate report so the absence reads as a
    /// decision (no silent caps). An explicit `probe run <name>` still runs
    /// one (operator's choice, with a printed note).
    const NOT_PROBED: &[(&str, &str)] = &[(
        "render_scale_shot",
        "BCS_SHOT real-GPU pixel capture with no self-ending autopilot: under \
         probe's Xvfb it would time out, and its point (correct pixels) needs \
         a real GPU and human eyes",
    )];

    /// A spec resolved against the example catalog.
    #[derive(Debug, PartialEq)]
    pub struct Resolved {
        pub examples: Vec<String>,
        /// True when the spec EXPANDS (list, category, --all) - the multi
        /// gates and the aggregate apply; a bare example name stays the
        /// single-run path with today's semantics exactly.
        pub multi: bool,
        /// The spec as given, for the aggregate manifest ("--all", "ui", ...).
        pub spec_display: String,
        /// What expansion skipped, with reasons (empty for explicit names).
        pub excluded: Vec<(String, String)>,
    }

    /// Resolve spec tokens against the catalog: an exact example name wins,
    /// else a category dir name expands to its members (minus NOT_PROBED),
    /// else an error that lists the catalog. Pure - the catalog is injected
    /// (the probe-env pattern), so every branch is unit-testable.
    pub fn resolve_spec(
        tokens: &[String],
        all: bool,
        catalog: &[nova_probe::CatalogExample],
        not_probed: &[(&str, &str)],
    ) -> Result<Resolved, String> {
        let excluded_reason = |name: &str| {
            not_probed
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(n, r)| (n.to_string(), r.to_string()))
        };
        if all {
            let mut excluded = Vec::new();
            let examples = catalog
                .iter()
                .filter(|example| match excluded_reason(&example.name) {
                    Some(entry) => {
                        excluded.push(entry);
                        false
                    }
                    None => true,
                })
                .map(|example| example.name.clone())
                .collect();
            return Ok(Resolved {
                examples,
                multi: true,
                spec_display: "--all".into(),
                excluded,
            });
        }
        if tokens.is_empty() {
            return Err(format!(
                "run needs a spec (or --all)\n{}",
                spec_help(catalog)
            ));
        }
        let categories = nova_probe::categories(catalog);
        let mut examples: Vec<String> = Vec::new();
        let mut excluded = Vec::new();
        let mut multi = tokens.len() > 1;
        for token in tokens {
            if catalog.iter().any(|example| example.name == *token) {
                // Explicit names run even when NOT_PROBED lists them - the
                // operator asked; the driver prints the note.
                if !examples.contains(token) {
                    examples.push(token.clone());
                }
            } else if categories.contains(&token.as_str()) {
                multi = true;
                for example in catalog.iter().filter(|e| e.category == *token) {
                    match excluded_reason(&example.name) {
                        Some(entry) => {
                            if !excluded.contains(&entry) {
                                excluded.push(entry);
                            }
                        }
                        None => {
                            if !examples.contains(&example.name) {
                                examples.push(example.name.clone());
                            }
                        }
                    }
                }
            } else {
                return Err(format!(
                    "unknown example or category `{token}`\n{}",
                    spec_help(catalog)
                ));
            }
        }
        Ok(Resolved {
            examples,
            multi,
            spec_display: tokens.join(","),
            excluded,
        })
    }

    /// The catalog, listed by category, plus the spec forms - the body of
    /// the bare-`probe run` error and every unknown-spec error.
    fn spec_help(catalog: &[nova_probe::CatalogExample]) -> String {
        let mut help = String::from("examples by category:\n");
        for category in nova_probe::categories(catalog) {
            let members: Vec<&str> = catalog
                .iter()
                .filter(|example| example.category == category)
                .map(|example| example.name.as_str())
                .collect();
            help.push_str(&format!("  {category}: {}\n", members.join(", ")));
        }
        help.push_str(
            "forms: probe run <example>[,<example>...] | probe run <category> | probe run --all",
        );
        help
    }

    /// Dispatch a parsed run spec: resolve against the catalog, then either
    /// the single-run path (semantics unchanged) or the sequential multi
    /// driver with the aggregate. `--platform web` bypasses resolution -
    /// its positional is a SCENARIO id, not an example.
    fn run_spec(tokens: &[String], all: bool, mut base: RunOptions) -> Result<ExitCode, String> {
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
        if !resolved.multi {
            base.example = resolved.examples[0].clone();
            if let Some((_, reason)) = NOT_PROBED
                .iter()
                .find(|(name, _)| *name == base.example.as_str())
            {
                eprintln!(
                    "probe: note: {} is excluded from --all/category runs: {reason}",
                    base.example
                );
            }
            return run(&base);
        }
        // Multi gates: these flags are single-example concerns.
        if !base.scenarios.is_empty() || !base.presets.is_empty() {
            return Err(
                "the --scenario/--preset matrix is a single-example perf sweep; \
                 give one example"
                    .into(),
            );
        }
        if base.baseline.is_some() {
            return Err(
                "--baseline compares one run dir; it does not combine with a \
                 list/category/--all spec"
                    .into(),
            );
        }
        run_many(&resolved, &base, &catalog)
    }

    /// The sequential multi driver: each example through `run()` into
    /// `<base>/<example>/` (today's per-example artifacts unchanged), a row
    /// per example built from ITS checks.json (probe consumes its own agent
    /// surface), continue-on-failure, then the aggregate index. Per-run
    /// Xvfb spawn is kept (a ~1s cost per run that buys zero new lifecycle
    /// risk - recorded deviation from the spike's shared-Xvfb sketch).
    fn run_many(
        resolved: &Resolved,
        base: &RunOptions,
        catalog: &[nova_probe::CatalogExample],
    ) -> Result<ExitCode, String> {
        let root = repo_root();
        let out_base = base.out.clone().unwrap_or_else(|| root.join("probe-runs"));
        std::fs::create_dir_all(&out_base).map_err(|e| format!("could not create out dir: {e}"))?;
        let out_base = out_base
            .canonicalize()
            .map_err(|e| format!("could not resolve out dir: {e}"))?;
        let started_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (git_sha, host) = run_identity();
        let total = resolved.examples.len();
        let mut rows = Vec::new();
        for (i, example) in resolved.examples.iter().enumerate() {
            eprintln!("probe: ===== {example} [{}/{total}] =====", i + 1);
            let mut opts = base.clone();
            opts.example = example.clone();
            opts.out = Some(out_base.join(example));
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
                &out_base.join(example),
                run_error,
                started.elapsed().as_secs(),
            ));
        }
        let manifest = nova_probe::AllManifest {
            spec: resolved.spec_display.clone(),
            started_unix,
            git_sha,
            host,
            excluded: resolved.excluded.clone(),
            rows,
        };
        write_aggregate(&out_base, &manifest)?;
        print_aggregate(&out_base, &manifest);
        Ok(aggregate_exit(&manifest))
    }

    /// One aggregate row, read back from the run's own checks.json. A run
    /// that never produced one (build failure, probe error) becomes an
    /// ERROR row carrying the message - the sweep must show it, not skip it.
    fn build_row(
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

    fn write_aggregate(out_base: &Path, manifest: &nova_probe::AllManifest) -> Result<(), String> {
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

    fn print_aggregate(out_base: &Path, manifest: &nova_probe::AllManifest) {
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

    fn aggregate_exit(manifest: &nova_probe::AllManifest) -> ExitCode {
        match nova_probe::aggregate_verdict(&manifest.rows) {
            "OK" | "WARN" => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
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
    /// the wired examples (perf_baseline) read it - elsewhere it is a
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
                // The web capture is ALWAYS a trunk --release build (dev
                // wasm is unusably slow and trunk has no custom profiles).
                profile: "release".into(),
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
            Ok(Cmd::RunSpec { tokens, all, base }) => match run_spec(&tokens, all, base) {
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
        }
    }

    /// `probe report`: re-render an existing run dir - GATED on the
    /// manifest, so a report can only ever be built from a dir `probe run`
    /// itself produced (stale hand-assembled folders are refused, which is
    /// the whole point of the gate). An aggregate dir (probe-all.json)
    /// re-renders the index instead: each row is re-read fresh from its
    /// run dir's checks.json (re-render a single example's report via
    /// `probe report <base>/<example>`).
    fn report(dir: &Path, baseline: Option<&Path>) -> Result<ExitCode, String> {
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
        Ok(if verdict == "FAIL" || verdict == "NO_DATA" {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
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
        use super::*;

        fn s(args: &[&str]) -> Vec<String> {
            args.iter().map(|a| a.to_string()).collect()
        }

        #[test]
        fn parse_run_with_all_flags() {
            let cmd = parse(&s(&[
                "run",
                "playable",
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
            let Cmd::RunSpec { tokens, all, base } = cmd else {
                panic!("expected run spec");
            };
            assert_eq!(tokens, s(&["playable"]));
            assert!(!all);
            assert!(base.profile && base.samply && base.fps);
            assert_eq!(base.out, Some(PathBuf::from("runs/x")));
            assert_eq!(base.baseline, Some(PathBuf::from("runs/old")));
            assert_eq!(base.timeout_secs, 60);
            assert_eq!(base.display.as_deref(), Some(":0"));
        }

        #[test]
        fn parse_run_specs() {
            // A comma list splits into tokens; resolution happens later.
            let Ok(Cmd::RunSpec { tokens, all, .. }) = parse(&s(&["run", "playable,scenario"]))
            else {
                panic!("list parses");
            };
            assert_eq!(tokens, s(&["playable", "scenario"]));
            assert!(!all);

            // --all carries no tokens.
            let Ok(Cmd::RunSpec { tokens, all, .. }) = parse(&s(&["run", "--all"])) else {
                panic!("--all parses");
            };
            assert!(tokens.is_empty());
            assert!(all);

            // A bare run parses too - RESOLUTION errors with the catalog
            // listing (parse is pure and has no catalog to print).
            let Ok(Cmd::RunSpec { tokens, all, .. }) = parse(&s(&["run"])) else {
                panic!("bare run parses; resolution owns the error");
            };
            assert!(tokens.is_empty() && !all);

            // But a spec AND --all contradict.
            assert!(parse(&s(&["run", "playable", "--all"])).is_err());
        }

        #[test]
        fn parse_rejects_bad_input() {
            assert!(parse(&s(&[])).is_err());
            assert!(parse(&s(&["run", "a", "b"])).is_err(), "one spec only");
            assert!(parse(&s(&["run", "a", "--nope"])).is_err());
            assert!(parse(&s(&["frobnicate"])).is_err());
        }

        fn catalog() -> Vec<nova_probe::CatalogExample> {
            [
                ("controller_section", "sections"),
                ("scenario", "gameplay"),
                ("playable", "gameplay"),
                ("screenshot_reel", "screenshots"),
                ("render_scale_shot", "screenshots"),
                ("perf_baseline", "perf"),
            ]
            .into_iter()
            .map(|(name, category)| nova_probe::CatalogExample {
                name: name.into(),
                path: format!("examples/{category}/{name}.rs"),
                category: category.into(),
            })
            .collect()
        }

        const EXCLUDED: &[(&str, &str)] = &[("render_scale_shot", "needs a real GPU")];

        #[test]
        fn resolve_single_name_stays_single() {
            let resolved = resolve_spec(&s(&["playable"]), false, &catalog(), EXCLUDED).unwrap();
            assert_eq!(resolved.examples, s(&["playable"]));
            assert!(!resolved.multi, "a bare name keeps single-run semantics");
            assert!(resolved.excluded.is_empty());
        }

        #[test]
        fn resolve_list_and_category_expand() {
            let resolved =
                resolve_spec(&s(&["playable", "scenario"]), false, &catalog(), EXCLUDED).unwrap();
            assert_eq!(resolved.examples, s(&["playable", "scenario"]));
            assert!(resolved.multi);

            let resolved = resolve_spec(&s(&["screenshots"]), false, &catalog(), EXCLUDED).unwrap();
            assert_eq!(
                resolved.examples,
                s(&["screenshot_reel"]),
                "category expansion skips NOT_PROBED members"
            );
            assert!(
                resolved.multi,
                "a category is a multi spec even with one member"
            );
            assert_eq!(resolved.excluded.len(), 1, "and records what it skipped");
        }

        #[test]
        fn resolve_all_and_explicit_excluded() {
            let resolved = resolve_spec(&[], true, &catalog(), EXCLUDED).unwrap();
            assert!(
                !resolved.examples.contains(&"render_scale_shot".to_string()),
                "--all skips NOT_PROBED"
            );
            assert_eq!(resolved.examples.len(), 5);
            assert_eq!(resolved.spec_display, "--all");
            assert_eq!(
                resolved.excluded,
                vec![(
                    "render_scale_shot".to_string(),
                    "needs a real GPU".to_string()
                )]
            );

            // An explicit name overrides the exclusion (operator's choice).
            let resolved =
                resolve_spec(&s(&["render_scale_shot"]), false, &catalog(), EXCLUDED).unwrap();
            assert_eq!(resolved.examples, s(&["render_scale_shot"]));
            assert!(!resolved.multi);
        }

        #[test]
        fn resolve_errors_list_the_catalog() {
            let err = resolve_spec(&[], false, &catalog(), EXCLUDED).unwrap_err();
            assert!(err.contains("examples by category"), "{err}");
            assert!(err.contains("gameplay: scenario, playable"), "{err}");
            assert!(err.contains("--all"), "{err}");

            let err = resolve_spec(&s(&["typo"]), false, &catalog(), EXCLUDED).unwrap_err();
            assert!(err.contains("unknown example or category `typo`"), "{err}");
            assert!(err.contains("examples by category"), "{err}");
        }

        #[test]
        fn resolve_dedupes_overlapping_tokens() {
            let resolved =
                resolve_spec(&s(&["playable", "gameplay"]), false, &catalog(), EXCLUDED).unwrap();
            assert_eq!(
                resolved.examples,
                s(&["playable", "scenario"]),
                "a name already included is not repeated by its category"
            );
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

            let Ok(Cmd::RunSpec { tokens, base, .. }) = parse(&s(&[
                "run",
                "perf_baseline",
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
            assert_eq!(tokens, s(&["perf_baseline"]));
            assert!(base.release && base.fps);
            assert_eq!(base.render, Render::Sw);
            assert_eq!(base.scenarios, s(&["a", "b"]));
            assert_eq!(base.presets, s(&["high"]));
        }

        #[test]
        fn honest_combination_gates() {
            // A matrix without --fps is a sweep that measures nothing.
            assert!(parse(&s(&["run", "x", "--scenario", "a"])).is_err());
            // Web does not combine with the native-only passes.
            assert!(parse(&s(&["run", "x", "--platform", "web", "--profile"])).is_err());
            assert!(parse(&s(&["run", "x", "--platform", "web", "--fps"])).is_err());
            // Web alone is fine (the positional is a scenario id, resolved
            // past the catalog at dispatch).
            let Ok(Cmd::RunSpec { tokens, base, .. }) =
                parse(&s(&["run", "asteroid_field", "--platform", "web"]))
            else {
                panic!("web run parses");
            };
            assert_eq!(tokens, s(&["asteroid_field"]));
            assert_eq!(base.platform, Platform::Web);
        }

        #[test]
        fn retired_verbs_error_with_pointers() {
            // The v0.8.0 cut removed the deprecated aliases and the trace
            // verb (task 20260719-211500): muscle-memory commands must say
            // where they went, not just "unknown subcommand".
            for alias in ["sweep", "web", "profile"] {
                let err = parse(&s(&[alias])).unwrap_err();
                assert!(err.contains("retired"), "{alias}: {err}");
                assert!(err.contains("probe run"), "{alias}: {err}");
            }
            let err = parse(&s(&["trace", "t.json"])).unwrap_err();
            assert!(err.contains("retired"), "{err}");
            assert!(err.contains("--profile"), "{err}");
            // Genuinely unknown verbs keep the generic error.
            assert!(parse(&s(&["frobnicate"]))
                .unwrap_err()
                .contains("unknown subcommand"));
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
