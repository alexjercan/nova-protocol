//! The `bench` command line: the clap definition and the parsed [`Cmd`].
//! Pure - nothing here touches a process or a file.

use std::{path::PathBuf, time::Duration};

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::{agent::AgentSpec, audit::Ui, game::ScenarioTarget};

/// The parsed command.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    /// Rendered help text, printed and exited zero.
    Help(String),
    /// One play: an agent on one scenario, scored.
    Play(PlayOptions),
    /// Re-drive a recorded audit's wire lines against a fresh game.
    Replay(ReplayOptions),
}

/// Everything `bench play` needs.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayOptions {
    /// What to play.
    pub scenario: ScenarioTarget,
    /// Who plays.
    pub agent: AgentSpec,
    /// The goal the agent is prompted with. `None` is the built-in default:
    /// follow the objectives to Victory.
    pub goal: Option<String>,
    /// `NOVA_SEED` for the game; unset means the OS seeds it.
    pub seed: Option<u64>,
    /// The run's budgets.
    pub budget: BudgetArgs,
    /// The run directory; `None` picks `bench-runs/<sha>/<scenario>/<agent>-<n>`.
    pub out: Option<PathBuf>,
    /// Draw every tick offscreen into this directory (the channel's `--record`).
    pub record: Option<PathBuf>,
    /// The renderer attached to the event bus.
    pub ui: Ui,
    /// Keep the full snapshot in the audit beside the condensed observation.
    pub audit_raw: bool,
}

/// Everything `bench replay` needs.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayOptions {
    /// The audit to re-drive.
    pub audit: PathBuf,
    /// Where the replay's own audit goes; `None` is `<audit dir>/replay`.
    pub out: Option<PathBuf>,
    /// Draw every tick offscreen into this directory.
    pub record: Option<PathBuf>,
    /// The renderer attached to the event bus.
    pub ui: Ui,
}

/// The three budgets a run ends on, whichever comes first, plus the wall
/// clock a single game step may take.
#[derive(Debug, Clone, PartialEq, Args)]
pub struct BudgetArgs {
    /// Game ticks the run may use (60 ticks is one simulated second).
    #[arg(long, default_value_t = 18_000)]
    pub ticks: u64,
    /// `act` calls the agent may make.
    #[arg(long, default_value_t = 300)]
    pub turns: u64,
    /// Wall-clock seconds the whole run may take.
    #[arg(long, default_value_t = 1800)]
    pub deadline: u64,
}

impl BudgetArgs {
    /// The deadline as a duration.
    pub fn deadline(&self) -> Duration {
        Duration::from_secs(self.deadline)
    }
}

/// `--ui` as clap sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum UiArg {
    /// One line per event on stderr.
    Log,
    /// Nothing on stderr; the audit still records everything.
    Quiet,
}

impl From<UiArg> for Ui {
    fn from(ui: UiArg) -> Self {
        match ui {
            UiArg::Log => Ui::Log,
            UiArg::Quiet => Ui::Quiet,
        }
    }
}

#[derive(Parser)]
#[command(name = "bench", about = "The agent bench: an agent plays a scenario, the referee scores it", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// One play: spawn the game in step mode, seat an agent, score the run.
    Play {
        /// A scenario id from the merged registry, or a loose `*.content.ron`.
        scenario: String,
        /// Who plays: `baseline` (in-process hunter), `pi` (pi in RPC mode with
        /// the relay extension), or `cmd:<argv>` (any process that speaks the
        /// referee protocol; it gets `NOVA_BENCH_SOCKET`).
        #[arg(long, default_value = "baseline")]
        agent: String,
        /// The goal the agent is prompted with; default: follow the objectives.
        #[arg(long)]
        goal: Option<String>,
        /// pi only: the model pattern (`gpt-5.6-luna`, `openai-codex/gpt-5.6-sol`).
        #[arg(long)]
        model: Option<String>,
        /// pi only: the thinking level (off, minimal, low, medium, high, xhigh, max).
        #[arg(long)]
        thinking: Option<String>,
        /// Seed the gameplay RNG so the run replays byte for byte.
        #[arg(long)]
        seed: Option<u64>,
        #[command(flatten)]
        budget: BudgetArgs,
        /// The run directory (default `bench-runs/<sha>/<scenario>/<agent>-<n>`).
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// Draw every tick offscreen and save `DIR/frame_%06d.png` (needs a GPU).
        #[arg(long, value_name = "DIR")]
        record: Option<PathBuf>,
        /// What to show while the run plays.
        #[arg(long, value_enum, default_value_t = UiArg::Log)]
        ui: UiArg,
        /// Keep the full snapshot in the audit beside the condensed observation.
        #[arg(long)]
        audit_raw: bool,
    },
    /// Feed a recorded audit's wire lines to a fresh game and compare the end.
    Replay {
        /// The `audit.jsonl` of a play.
        audit: PathBuf,
        /// Where the replay's audit goes (default `<audit dir>/replay`).
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// Draw every tick offscreen and save `DIR/frame_%06d.png` (needs a GPU).
        #[arg(long, value_name = "DIR")]
        record: Option<PathBuf>,
        /// What to show while the replay runs.
        #[arg(long, value_enum, default_value_t = UiArg::Log)]
        ui: UiArg,
    },
}

/// Parse the forwarded slice (no argv[0]: `bench` is a subcommand of the game
/// binary). A bare `bench` is a refusal, not a help dump that exits zero.
pub fn parse(args: &[String]) -> Result<Cmd, String> {
    let cli = match Cli::try_parse_from(std::iter::once("bench").chain(args.iter().map(|a| &**a))) {
        Ok(cli) => cli,
        Err(error) => {
            return match error.kind() {
                clap::error::ErrorKind::DisplayHelp => Ok(Cmd::Help(error.render().to_string())),
                clap::error::ErrorKind::MissingSubcommand
                | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                    Err("a subcommand is required (play, replay)".into())
                }
                _ => Err(error.render().to_string().trim_end().to_string()),
            };
        }
    };
    match cli.command {
        Sub::Play {
            scenario,
            agent,
            goal,
            model,
            thinking,
            seed,
            budget,
            out,
            record,
            ui,
            audit_raw,
        } => {
            let agent = AgentSpec::parse(&agent, model, thinking)?;
            Ok(Cmd::Play(PlayOptions {
                scenario: ScenarioTarget::parse(&scenario),
                agent,
                goal,
                seed,
                budget,
                out,
                record,
                ui: ui.into(),
                audit_raw,
            }))
        }
        Sub::Replay {
            audit,
            out,
            record,
            ui,
        } => Ok(Cmd::Replay(ReplayOptions {
            audit,
            out,
            record,
            ui: ui.into(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_play_parses_its_scenario_agent_and_budgets() {
        let Cmd::Play(play) = parse(&args(&[
            "play",
            "tutorial",
            "--agent",
            "pi",
            "--model",
            "gpt-5.6-luna",
            "--thinking",
            "low",
            "--ticks",
            "600",
            "--seed",
            "7",
        ]))
        .unwrap() else {
            panic!("play parses")
        };
        assert_eq!(play.scenario, ScenarioTarget::Id("tutorial".into()));
        assert_eq!(
            play.agent,
            AgentSpec::Pi {
                model: Some("gpt-5.6-luna".into()),
                thinking: Some("low".into()),
            }
        );
        assert_eq!(play.budget.ticks, 600);
        assert_eq!(play.budget.turns, 300);
        assert_eq!(play.seed, Some(7));
        assert_eq!(play.ui, Ui::Log);
    }

    #[test]
    fn a_loose_file_is_a_scenario_file_and_the_default_agent_is_the_baseline() {
        let Cmd::Play(play) = parse(&args(&["play", "world.content.ron"])).unwrap() else {
            panic!("play parses")
        };
        assert_eq!(
            play.scenario,
            ScenarioTarget::File(PathBuf::from("world.content.ron"))
        );
        assert_eq!(play.agent, AgentSpec::Baseline);
    }

    #[test]
    fn a_bare_bench_is_refused_and_help_is_rendered() {
        assert!(parse(&[]).is_err());
        assert!(matches!(parse(&args(&["--help"])), Ok(Cmd::Help(_))));
        assert!(parse(&args(&["play", "tutorial", "--agent", "nobody"])).is_err());
    }
}
