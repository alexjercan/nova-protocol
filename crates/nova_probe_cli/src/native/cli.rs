//! The `probe` command line: usage text, flags, and the parsed [`Cmd`].
//! Pure - resolution against the example catalog happens later.

use std::path::PathBuf;

pub(crate) const USAGE: &str = "\
usage: probe <subcommand>
  run <spec> [--all] [--out <dir>] [--correctness-only] [--samply]
  [--baseline <base-dir>] [--timeout <secs>] [--display <:N>]
  [--release] [--render gpu|sw] [--scenario <id>]... [--preset <p>]...
  [--platform native|web]
  the post-feature check and the perf sweep. --correctness-only runs only
  the clean behavioral pass. <spec> is one example, a
  comma list (player_path,scenario_grammar), or a category dir
  (sections|systems|stress|ui|screenshots). --all runs the whole
  catalog - nothing is excluded.
  Runs write to <out|probe-runs>/<short-commit>/<example>/ and
  write an aggregated index.html/index.json + probe-all.json above
  them, even for one example. Matrix flags (--scenario/--preset,
  repeatable) and --platform web (positional = scenario id)
  are single-example concerns. --baseline names a storage base; probe
  searches it for the nearest previous commit dir and compares each
  example against <base>/<commit>/<example>/ when it has a frametime.csv.
  Without --baseline, probe searches the --out base, or probe-runs.
  report <run-dir>... [--baseline <run-dir>]
  re-render the report (probe-run.json dirs) or the aggregate index
  (probe-all.json dirs); refuses dirs probe did not produce";

/// Parsed `probe run` options.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunOptions {
    pub example: String,
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
        dirs: Vec<PathBuf>,
        baseline: Option<PathBuf>,
    },
}

fn default_run(example: String) -> RunOptions {
    RunOptions {
        example,
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
    }
}

/// Parse the CLI: `run` and `report`. (The deprecated `sweep|web|profile`
/// aliases and the `trace` verb retired at the v0.8.0 cut. Native runs render
/// the top-N table in-report, and `probe report` re-renders it from the run
/// dir.)
pub(crate) fn parse(args: &[String]) -> Result<Cmd, String> {
    let mut iter = args.iter();
    match iter.next().map(String::as_str) {
        Some("run") => parse_run(iter.cloned().collect::<Vec<_>>()),
        Some("report") => {
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
             use `probe run` - the sweep is `run scene_baseline --release \
             --scenario ... --preset ...`, web is `run <scenario> --platform web`, \
             profiling is part of `run <example>`; add `--samply` for a flamegraph"
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
    if opts.platform == Platform::Web && (opts.samply || opts.correctness_only || matrix) {
        return Err(
            "--platform web captures the web frame line only; it does not combine \
             with --correctness-only/--samply/--scenario/--preset"
                .into(),
        );
    }
    if opts.correctness_only && (opts.samply || opts.baseline.is_some() || matrix) {
        return Err(
            "--correctness-only does not combine with measurement options \
             --samply/--baseline/--scenario/--preset"
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
            "scene_baseline",
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
        assert_eq!(tokens, s(&["scene_baseline"]));
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
            parse(&s(&["run", "asteroid_field", "--platform", "web"]))
        else {
            panic!("web run parses");
        };
        assert_eq!(tokens, s(&["asteroid_field"]));
        assert_eq!(base.platform, Platform::Web);
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
