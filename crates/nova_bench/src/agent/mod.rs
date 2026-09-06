//! Who plays: the agent seat and the `play` front door that sets the table
//! (run dir, bus, game, referee), seats the agent and writes the score.

pub mod baseline;
pub mod cmd;
pub mod pi;
pub mod socket;

use std::{path::PathBuf, process::ExitCode};

use crate::{
    audit::{BenchEvent, Bus},
    cli::PlayOptions,
    game::{game_exe, GameConfig, GameProcess},
    manual, movie, paths,
    referee::{Budget, Referee},
};

/// The agent seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSpec {
    /// The in-process scripted hunter. Zero tokens; what CI runs.
    Baseline,
    /// `pi` in RPC mode with the relay extension.
    Pi {
        /// `--model` for pi, when set.
        model: Option<String>,
        /// `--thinking` for pi, when set.
        thinking: Option<String>,
    },
    /// Any process that speaks the referee protocol on `NOVA_BENCH_SOCKET`.
    Cmd(Vec<String>),
}

impl AgentSpec {
    /// Parse `--agent` plus the pi-only flags.
    pub fn parse(
        name: &str,
        model: Option<String>,
        thinking: Option<String>,
    ) -> Result<Self, String> {
        if let Some(argv) = name.strip_prefix("cmd:") {
            let argv: Vec<String> = argv.split_whitespace().map(str::to_string).collect();
            if argv.is_empty() {
                return Err("`--agent cmd:<argv>` needs a command".into());
            }
            return Ok(Self::Cmd(argv));
        }
        match name {
            "baseline" => Ok(Self::Baseline),
            "pi" => Ok(Self::Pi { model, thinking }),
            other => Err(format!(
                "unknown agent `{other}`; use baseline, pi or cmd:<argv>"
            )),
        }
    }

    /// The label the run directory and the report use.
    pub fn label(&self) -> String {
        match self {
            Self::Baseline => "baseline".into(),
            Self::Pi { model, thinking } => {
                let model = model
                    .as_deref()
                    .map(|model| model.rsplit('/').next().unwrap_or(model))
                    .unwrap_or("default");
                match thinking {
                    Some(thinking) => format!("pi-{model}-{thinking}"),
                    None => format!("pi-{model}"),
                }
            }
            Self::Cmd(argv) => {
                let head = argv[0].rsplit('/').next().unwrap_or(&argv[0]);
                format!("cmd-{head}")
            }
        }
    }
}

/// One play, start to score.
pub fn play(options: &PlayOptions) -> Result<ExitCode, String> {
    let root = paths::repo_root();
    let run_dir = paths::run_dir(
        &root,
        options.out.as_deref(),
        &options.scenario.label(),
        &options.agent.label(),
    );
    let profile_dir = run_dir.join("profile");
    std::fs::create_dir_all(&profile_dir)
        .map_err(|error| format!("could not create {}: {error}", run_dir.display()))?;
    let bus = Bus::open(&run_dir.join("audit.jsonl"), options.ui)?;
    let goal = options
        .goal
        .clone()
        .unwrap_or_else(|| manual::DEFAULT_GOAL.to_string());
    let budget = Budget::from(&options.budget);
    bus.emit(BenchEvent::RunStart {
        scenario: options.scenario.token(),
        agent: options.agent.label(),
        goal: goal.clone(),
        seed: options.seed,
        budget: budget.to_json(),
        run_dir: run_dir.display().to_string(),
    });

    let config = GameConfig {
        exe: game_exe()?,
        root: root.clone(),
        scenario: options.scenario.clone(),
        seed: options.seed,
        record: options.record.clone(),
        profile_dir,
        log_path: run_dir.join("game.log"),
    };
    let game = GameProcess::spawn(&config)?;
    bus.emit(BenchEvent::Note {
        text: format!("game pid {} on {}", game.pid, config.exe.display()),
    });
    let mut referee = Referee::new(game, bus.clone(), budget, options.audit_raw);
    let first = referee.start()?;

    let socket = socket_path();
    let seated: Result<String, String> = match &options.agent {
        AgentSpec::Baseline => Ok(baseline::run(&mut referee)),
        AgentSpec::Cmd(argv) => cmd::run(&mut referee, argv, &socket, &run_dir.join("agent.log")),
        AgentSpec::Pi { model, thinking } => {
            let config = pi::PiConfig {
                model: model.clone(),
                thinking: thinking.clone(),
                extension: root.join("tools/nova_bench/pi/index.ts"),
                system_prompt: manual::MANUAL.to_string(),
                prompt: manual::opening(&options.scenario.label(), &goal, &first),
                log_path: run_dir.join("agent.log"),
            };
            pi::run(&mut referee, &config, &socket)
        }
    };
    let _ = std::fs::remove_file(&socket);
    let stop = match seated {
        Ok(stop) => stop,
        Err(error) => {
            referee.end(&format!("agent_error: {error}"));
            error
        }
    };
    if !referee.is_over() {
        referee.end(&stop);
    }

    let score = referee.score_json();
    let score_path = run_dir.join("score.json");
    std::fs::write(&score_path, format!("{:#}\n", score))
        .map_err(|error| format!("could not write {}: {error}", score_path.display()))?;
    println!("{}\n{}", run_dir.display(), referee.score_table());
    if let Some(frames) = &options.record {
        println!("{}", movie::report(&bus, frames));
    }

    let reason = referee.reason().unwrap_or_default();
    Ok(
        if reason.starts_with("game_error") || reason.starts_with("agent_error") {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        },
    )
}

/// A short path for the referee socket: unix socket paths are capped near
/// a hundred bytes, which a run directory under the repo can exceed.
fn socket_path() -> PathBuf {
    std::env::temp_dir().join(format!("nova-bench-{}.sock", std::process::id()))
}

/// The environment an external agent gets.
pub fn agent_env(socket: &std::path::Path) -> Vec<(String, String)> {
    vec![(
        "NOVA_BENCH_SOCKET".to_string(),
        socket.display().to_string(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_parse_and_label_themselves() {
        assert_eq!(
            AgentSpec::parse("baseline", None, None).unwrap(),
            AgentSpec::Baseline
        );
        let pi = AgentSpec::parse(
            "pi",
            Some("openai-codex/gpt-5.6-luna".into()),
            Some("low".into()),
        )
        .unwrap();
        assert_eq!(pi.label(), "pi-gpt-5.6-luna-low");
        assert_eq!(
            AgentSpec::parse("pi", None, None).unwrap().label(),
            "pi-default"
        );
        let cmd = AgentSpec::parse("cmd:python3 tools/agent.py", None, None).unwrap();
        assert_eq!(
            cmd,
            AgentSpec::Cmd(vec!["python3".into(), "tools/agent.py".into()])
        );
        assert_eq!(cmd.label(), "cmd-python3");
        assert!(AgentSpec::parse("cmd:", None, None).is_err());
        assert!(AgentSpec::parse("nobody", None, None).is_err());
    }

    #[test]
    fn the_agent_env_names_the_socket() {
        let env = agent_env(std::path::Path::new("/tmp/x.sock"));
        assert_eq!(
            env,
            vec![("NOVA_BENCH_SOCKET".to_string(), "/tmp/x.sock".to_string())]
        );
    }
}
