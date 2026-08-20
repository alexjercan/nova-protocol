//! The `probe` command line: the clap definition, the honest-combination gates
//! and the parsed [`Cmd`]. Pure - resolution against the example catalog (and,
//! for a scenario, against the game's own registry) happens later.

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

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

/// Renderer for the capture: the real GPU, or the lavapipe software
/// floor (the worst-case CPU/fill bracket; NOT a web stand-in).
///
/// Which BACKEND draws, never whether anything draws - that is
/// [`RunOptions::norender`], and the two are mutually exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Render {
    Gpu,
    Sw,
}

/// Where the run executes. Web runs the perf_web wasm build under
/// headless Chromium and captures the frame line only (the recorder and
/// invariants are native-only by design).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Platform {
    Native,
    Web,
}

/// The measurement flags `run` and `scenario` share. Flattened into both so the
/// two verbs cannot drift apart, which is what happened while each parsed its
/// own copy.
#[derive(Debug, Clone, PartialEq, Args)]
pub(crate) struct MeasureArgs {
    /// Write the run directory here instead of `probe-runs/<commit>/`.
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
    /// Record a samply flamegraph in an extra pass.
    #[arg(long)]
    pub samply: bool,
    /// Run only the clean behavioral pass - no capture, no trace.
    #[arg(long)]
    pub correctness_only: bool,
    /// Storage base to compare frame times against; probe finds the nearest
    /// previous commit dir inside it.
    #[arg(long, value_name = "BASE-DIR")]
    pub baseline: Option<PathBuf>,
    /// Supervisor timeout for each child run.
    #[arg(long, value_name = "SECS", default_value_t = 180)]
    pub timeout: u64,
    /// Use this X display instead of starting an Xvfb.
    #[arg(long, value_name = ":N")]
    pub display: Option<String>,
    /// Build the children in release. Dev-profile frame numbers are not
    /// baselines.
    #[arg(long)]
    pub release: bool,
    /// Which BACKEND draws: the host GPU, or the lavapipe software floor.
    #[arg(
        long,
        value_name = "BACKEND",
        default_value = "gpu",
        conflicts_with = "norender"
    )]
    pub render: Render,
    /// Build the children headless: no device, no window, no X server (probe
    /// starts no Xvfb at all). A SPEED option - a headless run cannot see a
    /// render-side panic, so it runs beside a rendered one, never instead.
    #[arg(long)]
    pub norender: bool,
    /// Run the frame-time pass n times instead of once; the report gates the
    /// set rather than trusting one capture.
    #[arg(long, value_name = "N", default_value_t = 1,
          value_parser = clap::value_parser!(u32).range(1..))]
    pub repeat: u32,
}

/// Parsed `probe run` / `probe scenario` options, resolved out of the clap
/// structs so the rest of the crate keeps one shape for both verbs.
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
    /// Build the child headless (`nova_probe::NORENDER_ENV`): no device, no
    /// window, no X server, and no Xvfb started for it.
    ///
    /// A SPEED option, never a substitute. A headless run cannot see a
    /// render-side panic - a duplicate component, a broken material, a
    /// pipeline that will not compile - so a suite that runs only this way is
    /// blind to the failures that need a device to appear.
    ///
    /// Excludes `--render`: there is no backend to pick when nothing draws.
    pub norender: bool,
    pub scenarios: Vec<String>,
    pub presets: Vec<String>,
    pub platform: Platform,
    /// How many times the frame-time pass runs. One capture cannot prove a
    /// tail moved, so a claim about the worst frame is made over repeats and
    /// the report gates them; see [`crate::evaluation::frames`].
    pub repeat: u32,
}

impl RunOptions {
    fn from_measure(example: String, measure: MeasureArgs) -> Self {
        Self {
            example,
            scenario_target: None,
            out: measure.out,
            samply: measure.samply,
            correctness_only: measure.correctness_only,
            baseline: measure.baseline,
            timeout_secs: measure.timeout,
            display: measure.display,
            release: measure.release,
            render: measure.render,
            norender: measure.norender,
            scenarios: Vec::new(),
            presets: Vec::new(),
            platform: Platform::Native,
            repeat: measure.repeat,
        }
    }
}

/// The dispatch shape the rest of the crate consumes.
#[derive(Debug, PartialEq)]
pub(crate) enum Cmd {
    /// Print `help` and exit successfully. Carries clap's RENDERED text: the
    /// help a reader gets must describe the parser that produced it, so it is
    /// generated rather than kept as a second copy that can drift.
    Help(String),
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

#[derive(Debug, Parser)]
#[command(
    name = "probe",
    bin_name = "probe",
    about = "The run harness: take examples and scenarios through the passes, grade them, report.",
    long_about = "The run harness. `run` takes cataloged examples through the passes (clean, \
                  frame time, profiled) and writes a run report plus an aggregate index; \
                  `scenario` takes the same passes to a scenario through the game binary, with \
                  no example between the tool and the data; `report` re-renders a run directory \
                  probe already produced.",
    subcommand_required = true,
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Verb,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// Take one spec, a comma list, a category dir or the whole catalog
    /// through the harness passes.
    #[command(long_about = "\
Take examples through the harness passes and grade them.

<SPEC> is one example, a comma list (<example>,<example>), or a category dir \
(playable|systems|screenshots). --all runs the whole catalog - nothing is \
excluded; `probe run` with no spec lists it.

Runs write to <out|probe-runs>/<short-commit>/<example>/ and an aggregated \
index.html/index.json + probe-all.json above them, even for one example.

--baseline names a storage BASE: probe searches it for the nearest previous \
commit dir and compares each example against <base>/<commit>/<example>/ when \
that has a frametime.csv. Without it, probe searches the --out base, or \
probe-runs.

--repeat runs the frame-time pass n times. The report gates the repeats on \
their mean and median and reads the worst frame only across the ones that \
pass - a single worst frame is ~30% noise on this host and cannot prove \
anything on its own.

The matrix flags (--scenario/--preset, repeatable) and --platform web \
(positional = scenario id) are single-example concerns.")]
    Run {
        /// One example, a comma list, or a category dir. Omit it to list the
        /// catalog.
        #[arg(value_name = "SPEC")]
        spec: Option<String>,
        /// Run the whole catalog. Excludes a spec.
        #[arg(long, conflicts_with = "spec")]
        all: bool,
        /// Sweep this scenario id; repeatable, crossed with --preset.
        #[arg(long = "scenario", value_name = "ID", action = ArgAction::Append)]
        scenarios: Vec<String>,
        /// Sweep this graphics preset; repeatable, crossed with --scenario.
        #[arg(long = "preset", value_name = "low|medium|high", action = ArgAction::Append)]
        presets: Vec<String>,
        /// Native, or the perf_web wasm build under headless Chromium.
        #[arg(long, value_name = "TARGET", default_value = "native")]
        platform: Platform,
        #[command(flatten)]
        measure: MeasureArgs,
    },

    /// Measure a SCENARIO through the game binary, with no example involved.
    #[command(long_about = "\
Measure a scenario through the game binary: it boots into the scenario and the \
same passes run (clean, frame time, profiled). No catalog is consulted.

A positional ending in .ron is a loose content file, registered for the run \
whether or not it ships in the catalog; anything else is an id from the merged \
registry.

The spec axes (--all/--scenario/--preset/--platform) belong to `run`: the \
positional IS the scenario, and there is nothing to expand.")]
    Scenario {
        /// A scenario id, or a path to a loose `*.content.ron`.
        #[arg(value_name = "ID|FILE.RON")]
        target: String,
        #[command(flatten)]
        measure: MeasureArgs,
    },

    /// Re-render a run report or an aggregate index probe already produced.
    Report {
        /// One or more run dirs (probe-run.json) or aggregate dirs
        /// (probe-all.json). Dirs probe did not produce are refused.
        #[arg(value_name = "RUN-DIR", required = true)]
        dirs: Vec<PathBuf>,
        /// Compare against this run dir.
        #[arg(long, value_name = "RUN-DIR")]
        baseline: Option<PathBuf>,
    },

    /// Retired at the v0.8.0 cut.
    #[command(hide = true)]
    Trace {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
    /// Retired: `run <example> --release --scenario ... --preset ...`.
    #[command(hide = true)]
    Sweep {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
    /// Retired: `run <scenario> --platform web`.
    #[command(hide = true)]
    Web {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
    /// Retired: profiling is part of `run <example>`.
    #[command(hide = true)]
    Profile {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
}

/// Parse the CLI. `Err` is a message the caller prints before exiting non-zero;
/// [`Cmd::Help`] carries the text to print before exiting zero.
///
/// Clap owns the shape - flags, values, arity, and the `--render`/`--norender`
/// conflict. What it cannot own are the HONEST-COMBINATION gates below: they
/// are claims about which passes a run would actually take, and they need a
/// sentence saying why, not a generated one.
pub(crate) fn parse(args: &[String]) -> Result<Cmd, String> {
    // `probe` is invoked as `nova-protocol probe <args>`, so the forwarded
    // slice has no argv[0] to skip.
    let cli = match Cli::try_parse_from(std::iter::once("probe").chain(args.iter().map(|a| &**a))) {
        Ok(cli) => cli,
        Err(error) => {
            return match error.kind() {
                clap::error::ErrorKind::DisplayHelp => Ok(Cmd::Help(error.render().to_string())),
                // A bare `probe` has always been a REFUSAL that exits
                // non-zero. Clap's own answer to a missing subcommand is to
                // render help, which it would then be tempting to exit 0 on -
                // and that turns a scripted typo into a silent success.
                clap::error::ErrorKind::MissingSubcommand
                | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                    Err("a subcommand is required".into())
                }
                // Clap says "unrecognized subcommand"; this crate has always
                // said "unknown subcommand", and the retired verbs below need
                // to be distinguishable from a typo.
                clap::error::ErrorKind::InvalidSubcommand => Err(format!(
                    "unknown subcommand {}",
                    args.first().map_or("", |a| a.as_str())
                )),
                _ => Err(error.render().to_string()),
            };
        }
    };

    match cli.command {
        Verb::Run {
            spec,
            all,
            scenarios,
            presets,
            platform,
            measure,
        } => {
            let mut base = RunOptions::from_measure(String::new(), measure);
            base.scenarios = scenarios;
            base.presets = presets;
            base.platform = platform;
            gate_run(&base)?;
            let tokens = spec
                .map(|spec| {
                    spec.split(',')
                        .filter(|token| !token.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            Ok(Cmd::RunSpec { tokens, all, base })
        }
        Verb::Scenario { target, measure } => {
            let target = ScenarioTarget::parse(&target);
            let mut base = RunOptions::from_measure(target.label(), measure);
            base.scenario_target = Some(target);
            gate_scenario(&base)?;
            Ok(Cmd::Scenario { base })
        }
        Verb::Report { dirs, baseline } => Ok(Cmd::Report { dirs, baseline }),
        // Retired verbs get a pointed error, not a generic one: the
        // muscle-memory commands should say where they went.
        Verb::Trace { .. } => Err(
            "`trace` retired (task 20260719-211500): native runs render the \
                                   top-N table into the run report, and `probe report <run-dir>` \
                                   re-renders it from the dir's trace.json"
                .into(),
        ),
        Verb::Sweep { .. } => Err(retired_alias("sweep")),
        Verb::Web { .. } => Err(retired_alias("web")),
        Verb::Profile { .. } => Err(retired_alias("profile")),
    }
}

fn retired_alias(alias: &str) -> String {
    format!(
        "`{alias}` retired (deprecated for one cycle, removed at v0.8.0): \
         use `probe run` - the sweep is `run <example> --release \
         --scenario ... --preset ...`, web is `run <scenario> --platform web`, \
         profiling is part of `run <example>`; add `--samply` for a flamegraph"
    )
}

/// Combination gates common to both verbs.
fn gate_measure(base: &RunOptions, matrix: bool) -> Result<(), String> {
    if base.correctness_only
        && (base.samply || base.baseline.is_some() || matrix || base.repeat > 1)
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
    if matrix && base.repeat > 1 {
        return Err(
            "--repeat repeats the frame-time pass, which a --scenario/--preset \
             sweep replaces; sweep or repeat, not both"
                .into(),
        );
    }
    Ok(())
}

/// `run`'s gates. Multi-spec gates live in resolve, which knows whether the
/// spec expands; these need no catalog.
fn gate_run(base: &RunOptions) -> Result<(), String> {
    let matrix = !base.scenarios.is_empty() || !base.presets.is_empty();
    if base.platform == Platform::Web
        && (base.samply || base.correctness_only || matrix || base.repeat > 1)
    {
        return Err(
            "--platform web captures the web frame line only; it does not combine \
             with --correctness-only/--samply/--scenario/--preset/--repeat"
                .into(),
        );
    }
    // The headless switch is a process environment variable, and a browser has
    // no process environment. Refusing beats accepting the flag and rendering
    // anyway, which would put a rendered number under a headless label.
    if base.platform == Platform::Web && base.norender {
        return Err("--norender is native only: a wasm run has no process environment".into());
    }
    gate_measure(base, matrix)
}

/// `scenario`'s gates. It has no matrix axis at all.
fn gate_scenario(base: &RunOptions) -> Result<(), String> {
    gate_measure(base, false)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;
    use crate::native::fixtures::s;

    fn help(args: &[String]) -> bool {
        matches!(parse(args), Ok(Cmd::Help(_)))
    }

    /// The top-level help as a reader sees it.
    fn usage() -> String {
        Cli::command().render_long_help().to_string()
    }

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
            assert!(help(&args), "{args:?}");
        }
    }

    /// Help is GENERATED, so it cannot describe a flag the parser does not
    /// have. Spot-check the two the harness is discovered through.
    #[test]
    fn the_rendered_help_names_the_flags_it_parses() {
        let Ok(Cmd::Help(text)) = parse(&s(&["run", "--help"])) else {
            panic!("run help renders");
        };
        assert!(text.contains("--norender"), "{text}");
        assert!(text.contains("--correctness-only"), "{text}");
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
            assert!(error.contains(flag), "{flag}: {error}");
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
        assert!(
            parse(&s(&["report"])).is_err(),
            "report needs at least one run dir"
        );

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

    /// `--render` picks a backend and `--norender` says nothing draws, so
    /// asking for both is a contradiction the parser must refuse rather than
    /// silently honour one of.
    #[test]
    fn norender_excludes_a_backend_and_reaches_both_verbs() {
        for args in [
            s(&["run", "x", "--norender", "--render", "sw"]),
            s(&["run", "x", "--render", "gpu", "--norender"]),
            s(&["scenario", "a", "--norender", "--render", "sw"]),
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }

        let Ok(Cmd::RunSpec { base, .. }) = parse(&s(&["run", "x", "--norender"])) else {
            panic!("--norender parses on run");
        };
        assert!(base.norender && base.render == Render::Gpu);

        let Ok(Cmd::Scenario { base }) = parse(&s(&["scenario", "a", "--norender"])) else {
            panic!("--norender parses on scenario");
        };
        assert!(base.norender);

        // A browser has no process environment to set it in.
        assert!(parse(&s(&["run", "x", "--platform", "web", "--norender"])).is_err());
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

    /// The retired verbs are hidden, so they must not advertise themselves in
    /// the help a reader discovers the harness through.
    #[test]
    fn the_retired_verbs_stay_out_of_the_help() {
        let text = usage();
        for alias in ["sweep", "trace", "profile"] {
            assert!(!text.contains(&format!("  {alias}")), "{alias}: {text}");
        }
        for verb in ["run", "scenario", "report"] {
            assert!(text.contains(verb), "{verb}: {text}");
        }
    }
}
