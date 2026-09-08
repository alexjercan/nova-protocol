//! `nova_bench`: the agent bench. A referee that lets an external agent play
//! a scenario over the process channel, and scores the run.
//!
//! Three processes, one referee:
//!
//! ```text
//!   agent  <-- referee protocol (unix socket) -->  nova_bench  <-- channel (stdio) -->  game
//! ```
//!
//! The agent never touches the game and the game never sees the agent. The
//! referee ([`referee::Referee`]) owns the clock, condenses what the agent
//! sees ([`observation`]), stamps the agent's gestures onto the wire
//! ([`gesture`]), logs every line in both directions ([`audit`]) and scores
//! the run from the game's own snapshots ([`score`]). An agent cannot fake a
//! result: the score never reads the agent's report.
//!
//! Agents are pluggable ([`agent`]): an in-process scripted baseline, an
//! external process on the socket, or `pi` in RPC mode with the relay
//! extension under `tools/nova_bench/pi/`.
//!
//! The front door is the game binary's `bench` subcommand (debug feature),
//! which spawns `current_exe()` as the game so the two can never drift:
//!
//! ```text
//! cargo run --features debug bench play <scenario> [--agent baseline|pi|cmd:<argv>] [--goal ...]
//! cargo run --features debug bench replay <run-dir>/audit.jsonl
//! ```
//!
//! The design record is `tasks/20260824-125933/ARCHITECTURE.md`.
#![warn(missing_docs)]

pub mod agent;
pub mod audit;
pub mod cli;
pub mod game;
pub mod gesture;
pub mod manual;
pub mod movie;
pub mod observation;
pub mod paths;
pub mod referee;
pub mod replay;
pub mod score;

use std::process::ExitCode;

/// Glob-import surface: the referee, its protocol pieces, and the agents.
pub mod prelude {
    pub use crate::{
        agent::{pi::PI_ENV, AgentSpec, SOCKET_ENV},
        audit::{BenchEvent, Bus, Ui},
        game::{GameChannel, GameConfig, GameProcess, ScenarioTarget},
        gesture::{expand, parse_gestures, Gesture, MAX_AIM_TICKS},
        movie::{stitch, Movie},
        observation::condense,
        referee::{Budget, Referee, TICKS_PER_SECOND},
        score::Score,
    };
}

/// Parse the forwarded command line (everything after `bench` on the game
/// binary's command line) and dispatch it; the exit code is the verdict.
pub fn main(args: &[String]) -> ExitCode {
    match cli::parse(args) {
        Err(message) => {
            eprintln!("bench: {message}");
            ExitCode::FAILURE
        }
        Ok(cli::Cmd::Help(text)) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Ok(cli::Cmd::Play(options)) => match agent::play(&options) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("bench: {message}");
                ExitCode::FAILURE
            }
        },
        Ok(cli::Cmd::Replay(options)) => match replay::replay(&options) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("bench: {message}");
                ExitCode::FAILURE
            }
        },
    }
}
