//! The `probe` command line: usage text, flags, and the parsed [`Cmd`].
//! Pure - resolution against the example catalog (and, for a scenario, against
//! the game's own registry) happens later.

use std::path::PathBuf;

pub(crate) const USAGE: &str = "\
usage: probe <subcommand>
  -h, --help
  print this help and exit successfully
  run <spec> [--all] [--out <dir>] [--correctness-only] [--samply]
  [--baseline <base-dir>] [--timeout <secs>] [--display <:N>]
  [--release] [--render gpu|sw] [--scenario <id>]... [--preset <p>]...
  [--platform native|web] [--repeat <n>]
  the post-feature check and the perf sweep. --correctness-only runs only
  the clean behavioral pass. <spec> is one example, a
  comma list (<example>,<example>), or a category dir
  (playable|systems|screenshots). --all runs the whole
  catalog - nothing is excluded; `probe run` alone lists it.
  Runs write to <out|probe-runs>/<short-commit>/<example>/ and
  write an aggregated index.html/index.json + probe-all.json above
  them, even for one example. Matrix flags (--scenario/--preset,
  repeatable) and --platform web (positional = scenario id)
  are single-example concerns. --baseline names a storage base; probe
  searches it for the nearest previous commit dir and compares each
  example against <base>/<commit>/<example>/ when it has a frametime.csv.
  Without --baseline, probe searches the --out base, or probe-runs.
  --repeat runs the frame-time pass n times instead of once. The report
  then gates the repeats on their mean and median and reads the worst
  frame only across the ones that pass - a single worst frame is ~30%
  noise on this host and cannot prove anything on its own.
  scenario <id|file.ron> [--out <dir>] [--baseline <base-dir>] [--samply]
  [--correctness-only] [--timeout <secs>] [--display <:N>] [--release]
  [--render gpu|sw] [--repeat <n>]
  measure a SCENARIO, with no example involved: the game binary boots into
  it and the same passes run (clean, frame time, profiled). A positional
  ending in .ron is a loose content file - registered for the run whether
  or not it ships in the catalog; anything else is an id from the merged
  registry.
  report <run-dir>... [--baseline <run-dir>]
  re-render the report (probe-run.json dirs) or the aggregate index
  (probe-all.json dirs); refuses dirs probe did not produce";

/// What `probe scenario` measures. Decided by SUFFIX so parsing stays pure: a
/// positional ending in `.ron` is a file, anything else an id.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ScenarioTarget {
    /// An id resolved against the merged registry inside the game binary.
    Id(String),
    /// A loose `*.content.ron`, registered for the run and never installed.
    File(PathBuf),
}

impl ScenarioTarget {
    /// Classify a positional. Pure - a path that does not exist is refused by
    /// the child, which is the half that can also say what the file contained.
    pub(crate) fn parse(token: &str) -> Self {
        if token.ends_with(".ron") {
            Self::File(PathBuf::from(token))
        } else {
            Self::Id(token.to_string())
        }
    }

    /// The run label: the id, or the file's stem with `.content` trimmed.
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Id(id) => id.clone(),
            Self::File(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .map(|name| {
                    name.trim_end_matches(".ron")
                        .trim_end_matches(".content")
                        .to_string()
                })
                .filter(|stem| !stem.is_empty())
                .unwrap_or_else(|| "scenario".into()),
        }
    }

    /// The arguments the game binary is launched with.
    pub(crate) fn args(&self) -> Vec<String> {
        match self {
            Self::Id(id) => vec!["--scenario".into(), id.clone()],
            Self::File(path) => vec!["--scenario-file".into(), path.display().to_string()],
        }
    }
}

/// Parsed `probe run` options.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunOptions {
    pub example: String,
    /// Set by `probe scenario`: the passes build and run the GAME BINARY
    /// pointed at this target instead of a cataloged example.
    pub scenario_target: Option<ScenarioTarget>,
    pub out: Option<PathBuf>,
    pub samply: bool,
    pub correctness_only: bool,
    pub baseline: Option<PathBuf>,
    pub timeout_secs: u64,
    pub display: Option<String>,
    pub release: bool,
    pub render: Render,
    pub scenarios: Vec<String>,
    pub presets: Vec<String>,
    pub platform: Platform,
    /// How many times the frame-time pass runs. One capture cannot prove a
    /// tail moved, so a claim about the worst frame is made over repeats and
    /// the report gates them; see [`crate::evaluation::frames`].
    pub repeat: u32,
}

/// Renderer for the capture: the real GPU, or the lavapipe software
/// floor (the worst-case CPU/fill bracket; NOT a web stand-in).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Render {
    Gpu,
    Sw,
}

/// Where the run executes. Web runs the perf_web wasm build under
/// headless Chromium and captures the frame line only (the recorder and
/// invariants are native-only by design).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Platform {
    Native,
    Web,
}

/// Parsed command line.
#[derive(Debug, PartialEq)]
pub(crate) enum Cmd {
    /// Print [`USAGE`] and exit successfully.
    Help,
    /// A `probe run` spec, resolved against the example catalog at
    /// dispatch (parse stays pure/fs-free): `tokens` is the comma-split
    /// positional (possibly empty - resolution errors with the catalog
    /// listing), `all` the --all flag.
    RunSpec {
        tokens: Vec<String>,
        all: bool,
        base: RunOptions,
    },
    /// A `probe scenario` target, measured through the game binary. No catalog
    /// is consulted: the id resolves inside the child, against the same merged
    /// registry the game itself reads.
    Scenario { base: RunOptions },
    Report {
        dirs: Vec<PathBuf>,
        baseline: Option<PathBuf>,
    },
}

fn default_run(example: String) -> RunOptions {
    RunOptions {
        example,
        scenario_target: None,
        out: None,
        samply: false,
        correctness_only: false,
        baseline: None,
        timeout_secs: 180,
        display: None,
        release: false,
        render: Render::Gpu,
        scenarios: Vec::new(),
        presets: Vec::new(),
        platform: Platform::Native,
        repeat: 1,
    }
}

/// Parse a `--repeat` value. Zero is not "no capture", it is a typo.
fn parse_repeat(value: Option<&String>) -> Result<u32, String> {
    match value.and_then(|v| v.trim().parse::<u32>().ok()) {
        Some(n) if n >= 1 => Ok(n),
        _ => Err("--repeat needs a count of 1 or more".into()),
    }
}

/// Parse the CLI: `run`, `scenario` and `report`. (The deprecated `sweep|web|profile`
/// aliases and the `trace` verb retired at the v0.8.0 cut. Native runs render
/// the top-N table in-report, and `probe report` re-renders it from the run
/// dir.)
pub(crate) fn parse(args: &[String]) -> Result<Cmd, String> {
    let mut iter = args.iter();
    match iter.next().map(String::as_str) {
        Some("-h" | "--help") => Ok(Cmd::Help),
        Some("run") => {
            let args = iter.cloned().collect::<Vec<_>>();
            if requests_help(&args) {
                Ok(Cmd::Help)
            } else {
                parse_run(args)
            }
        }
        Some("scenario") => {
            let args = iter.cloned().collect::<Vec<_>>();
            if requests_help(&args) {
                Ok(Cmd::Help)
            } else {
                parse_scenario(args)
            }
        }
        Some("report") => {
            let args = iter.cloned().collect::<Vec<_>>();
            if requests_help(&args) {
                return Ok(Cmd::Help);
            }
            let mut iter = args.iter();
            let mut dirs: Vec<PathBuf> = Vec::new();
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
                        dirs.push(PathBuf::from(other));
                    }
                }
            }
            if dirs.is_empty() {
                return Err("report needs at least one run dir".into());
            }
            Ok(Cmd::Report { dirs, baseline })
        }
        // Retired verbs get a pointed error, not a generic one: the
        // muscle-memory commands should say where they went.
        Some("trace") => Err(
            "`trace` retired (task 20260719-211500): native runs render the \
             top-N table into the run report, and `probe report <run-dir>` \
             re-renders it from the dir's trace.json"
                .into(),
        ),
        Some(alias @ ("sweep" | "web" | "profile")) => Err(format!(
            "`{alias}` retired (deprecated for one cycle, removed at v0.8.0): \
             use `probe run` - the sweep is `run <example> --release \
             --scenario ... --preset ...`, web is `run <scenario> --platform web`, \
             profiling is part of `run <example>`; add `--samply` for a flamegraph"
        )),
        Some(other) => Err(format!("unknown subcommand {other}")),
        None => Err("a subcommand is required".into()),
    }
}

fn requests_help(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
}

/// Parse `probe scenario <id|file.ron> [flags]`. The measurement flags are
/// `run`'s; the spec flags (--all, --scenario, --preset, --platform) are not -
/// the positional IS the scenario, and there is no catalog to expand.
fn parse_scenario(args: Vec<String>) -> Result<Cmd, String> {
    let mut target: Option<String> = None;
    let mut opts = default_run(String::new());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--samply" => opts.samply = true,
            "--correctness-only" => opts.correctness_only = true,
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
            "--repeat" => {
                opts.repeat = parse_repeat(iter.next())?;
            }
            "--render" => {
                opts.render = match iter.next().map(String::as_str) {
                    Some("gpu") => Render::Gpu,
                    Some("sw") => Render::Sw,
                    _ => return Err("--render needs gpu or sw".into()),
                };
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag {other}"));
            }
            other => {
                if target.replace(other.to_string()).is_some() {
                    return Err("scenario takes exactly one id or file".into());
                }
            }
        }
    }
    let target = target.ok_or("scenario needs an id or a .ron file")?;
    let target = ScenarioTarget::parse(&target);
    opts.example = target.label();
    opts.scenario_target = Some(target);
    if opts.correctness_only && (opts.samply || opts.baseline.is_some() || opts.repeat > 1) {
        return Err(
            "--correctness-only does not combine with measurement options \
             --samply/--baseline/--repeat"
                .into(),
        );
    }
    Ok(Cmd::Scenario { base: opts })
}

fn parse_run(args: Vec<String>) -> Result<Cmd, String> {
    let mut example: Option<String> = None;
    let mut all = false;
    let mut opts = default_run(String::new());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--all" => all = true,
            "--samply" => opts.samply = true,
            "--correctness-only" => opts.correctness_only = true,
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
            "--repeat" => {
                opts.repeat = parse_repeat(iter.next())?;
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
    // Honest-combination gates that need no catalog. Multi-spec gates live in
    // resolve because they need to know whether the spec expands.
    let matrix = !opts.scenarios.is_empty() || !opts.presets.is_empty();
    if opts.platform == Platform::Web
        && (opts.samply || opts.correctness_only || matrix || opts.repeat > 1)
    {
        return Err(
            "--platform web captures the web frame line only; it does not combine \
             with --correctness-only/--samply/--scenario/--preset/--repeat"
                .into(),
        );
    }
    if opts.correctness_only
        && (opts.samply || opts.baseline.is_some() || matrix || opts.repeat > 1)
    {
        return Err(
            "--correctness-only does not combine with measurement options \
             --samply/--baseline/--scenario/--preset/--repeat"
                .into(),
        );
    }
    // A sweep REPLACES the frame-time pass with its matrix cells, so there is
    // nothing for --repeat to repeat. Silently doing one capture would read as
    // a completed sweep of repeats.
    if matrix && opts.repeat > 1 {
        return Err(
            "--repeat repeats the frame-time pass, which a --scenario/--preset \
                    sweep replaces; sweep or repeat, not both"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::fixtures::s;

    #[test]
    fn parse_run_with_all_flags() {
        let cmd = parse(&s(&[
            "run",
            "playable",
            "--samply",
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
        assert!(base.samply);
        assert_eq!(base.out, Some(PathBuf::from("runs/x")));
        assert_eq!(base.baseline, Some(PathBuf::from("runs/old")));
        assert_eq!(base.timeout_secs, 60);
        assert_eq!(base.display.as_deref(), Some(":0"));
    }

    #[test]
    fn parse_run_specs() {
        // A comma list splits into tokens; resolution happens later.
        let Ok(Cmd::RunSpec { tokens, all, .. }) = parse(&s(&["run", "playable,scenario"])) else {
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
    fn help_is_available_at_every_command_level() {
        for args in [
            s(&["--help"]),
            s(&["-h"]),
            s(&["run", "--help"]),
            s(&["run", "playable", "-h"]),
            s(&["report", "--help"]),
            s(&["report", "runs/x", "-h"]),
            s(&["scenario", "--help"]),
            s(&["scenario", "some_id", "-h"]),
        ] {
            assert_eq!(parse(&args), Ok(Cmd::Help), "{args:?}");
        }
    }

    #[test]
    fn parse_rejects_bad_input() {
        assert!(parse(&s(&[])).is_err());
        assert!(parse(&s(&["run", "a", "b"])).is_err(), "one spec only");
        assert!(parse(&s(&["run", "a", "--nope"])).is_err());
        assert!(parse(&s(&["frobnicate"])).is_err());
    }

    #[test]
    fn removed_diagnostic_flags_are_rejected() {
        for flag in ["--fps", "--profile"] {
            let error = parse(&s(&["run", "playable", flag])).unwrap_err();
            assert!(error.contains("unknown flag"), "{flag}: {error}");
        }
        assert!(parse(&s(&["run", "playable", "--samply"])).is_ok());
    }

    #[test]
    fn new_verbs_and_flags_parse() {
        let Ok(Cmd::Report { dirs, baseline }) = parse(&s(&[
            "report",
            "runs/x",
            "runs/y",
            "--baseline",
            "runs/old",
        ])) else {
            panic!("report parses");
        };
        assert_eq!(dirs, vec![PathBuf::from("runs/x"), PathBuf::from("runs/y")]);
        assert_eq!(baseline, Some(PathBuf::from("runs/old")));

        let Ok(Cmd::RunSpec { tokens, base, .. }) = parse(&s(&[
            "run",
            "many_things",
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
        assert_eq!(tokens, s(&["many_things"]));
        assert!(base.release);
        assert_eq!(base.render, Render::Sw);
        assert_eq!(base.scenarios, s(&["a", "b"]));
        assert_eq!(base.presets, s(&["high"]));
    }

    #[test]
    fn honest_combination_gates() {
        let Ok(Cmd::RunSpec { base, .. }) = parse(&s(&["run", "x", "--correctness-only"])) else {
            panic!("correctness-only run parses");
        };
        assert!(base.correctness_only);
        for flag in ["--samply", "--baseline", "--scenario", "--preset"] {
            let mut args = s(&["run", "x", "--correctness-only", flag]);
            if flag != "--samply" {
                args.push("value".into());
            }
            assert!(parse(&args).is_err(), "{flag} combined with correctness");
        }
        // Matrix capture follows the example's runtime capability contract.
        assert!(parse(&s(&["run", "x", "--scenario", "a"])).is_ok());
        // Web does not combine with the native-only passes.
        assert!(parse(&s(&["run", "x", "--platform", "web", "--samply"])).is_err());
        // Web alone is fine (the positional is a scenario id, resolved
        // past the catalog at dispatch).
        let Ok(Cmd::RunSpec { tokens, base, .. }) =
            parse(&s(&["run", "some_scenario", "--platform", "web"]))
        else {
            panic!("web run parses");
        };
        assert_eq!(tokens, s(&["some_scenario"]));
        assert_eq!(base.platform, Platform::Web);
    }

    /// The positional is the subject, and its FORM says which kind. An id
    /// never touches the filesystem here; a `.ron` never touches the registry.
    #[test]
    fn a_scenario_positional_is_an_id_or_a_ron_file() {
        let Ok(Cmd::Scenario { base }) = parse(&s(&["scenario", "some_scenario"])) else {
            panic!("an id parses");
        };
        assert_eq!(
            base.scenario_target,
            Some(ScenarioTarget::Id("some_scenario".into()))
        );
        assert_eq!(base.example, "some_scenario", "the label is the id");
        assert_eq!(
            base.scenario_target.unwrap().args(),
            s(&["--scenario", "some_scenario"])
        );

        let Ok(Cmd::Scenario { base }) = parse(&s(&["scenario", "mods/x/thing.content.ron"]))
        else {
            panic!("a file parses");
        };
        let target = base.scenario_target.expect("a target");
        assert_eq!(
            target,
            ScenarioTarget::File(PathBuf::from("mods/x/thing.content.ron"))
        );
        assert_eq!(
            base.example, "thing",
            "the label drops the .content.ron suffix"
        );
        assert_eq!(
            target.args(),
            s(&["--scenario-file", "mods/x/thing.content.ron"])
        );
    }

    #[test]
    fn scenario_takes_one_subject_and_the_measurement_flags() {
        assert!(parse(&s(&["scenario"])).is_err(), "a subject is required");
        assert!(parse(&s(&["scenario", "a", "b"])).is_err(), "exactly one");
        // The spec axes belong to `run`: there is no catalog to expand here.
        for flag in ["--all", "--scenario", "--preset", "--platform"] {
            assert!(
                parse(&s(&["scenario", "a", flag, "value"])).is_err(),
                "{flag}"
            );
        }
        let Ok(Cmd::Scenario { base }) = parse(&s(&[
            "scenario",
            "a",
            "--release",
            "--render",
            "sw",
            "--samply",
            "--timeout",
            "900",
            "--out",
            "runs/x",
        ])) else {
            panic!("the measurement flags parse");
        };
        assert!(base.release && base.samply);
        assert_eq!(base.render, Render::Sw);
        assert_eq!(base.timeout_secs, 900);
        assert_eq!(base.out, Some(PathBuf::from("runs/x")));
    }

    #[test]
    fn repeat_parses_and_refuses_the_passes_it_cannot_repeat() {
        let Ok(Cmd::RunSpec { base, .. }) = parse(&s(&["run", "x", "--repeat", "5"])) else {
            panic!("--repeat parses on run");
        };
        assert_eq!(base.repeat, 5);
        let Ok(Cmd::Scenario { base }) = parse(&s(&["scenario", "a", "--repeat", "5"])) else {
            panic!("--repeat parses on scenario");
        };
        assert_eq!(base.repeat, 5);
        // One capture is the default, and the default is not a repeat set.
        let Ok(Cmd::RunSpec { base, .. }) = parse(&s(&["run", "x"])) else {
            panic!("bare run parses");
        };
        assert_eq!(base.repeat, 1);

        // Zero is a typo, not "skip the capture".
        for bad in ["0", "-1", "many", ""] {
            assert!(
                parse(&s(&["run", "x", "--repeat", bad])).is_err(),
                "--repeat {bad}"
            );
        }
        assert!(
            parse(&s(&["run", "x", "--repeat"])).is_err(),
            "needs a value"
        );

        // The three passes that have no frame-time pass to repeat.
        for tail in [
            vec!["--correctness-only"],
            vec!["--platform", "web"],
            vec!["--scenario", "a"],
        ] {
            let mut args = s(&["run", "x", "--repeat", "3"]);
            args.extend(tail.iter().map(|t| (*t).to_string()));
            assert!(parse(&args).is_err(), "{tail:?} combined with --repeat");
        }
        assert!(parse(&s(&[
            "scenario",
            "a",
            "--correctness-only",
            "--repeat",
            "3"
        ]))
        .is_err());
    }

    #[test]
    fn retired_verbs_error_with_pointers() {
        // The v0.8.0 cut removed the deprecated aliases and the trace verb:
        // muscle-memory commands must say where they went, not just
        // "unknown subcommand".
        for alias in ["sweep", "web", "profile"] {
            let err = parse(&s(&[alias])).unwrap_err();
            assert!(err.contains("retired"), "{alias}: {err}");
            assert!(err.contains("probe run"), "{alias}: {err}");
        }
        let err = parse(&s(&["trace", "t.json"])).unwrap_err();
        assert!(err.contains("retired"), "{err}");
        assert!(err.contains("native runs"), "{err}");
        // Genuinely unknown verbs keep the generic error.
        assert!(parse(&s(&["frobnicate"]))
            .unwrap_err()
            .contains("unknown subcommand"));
    }
}
