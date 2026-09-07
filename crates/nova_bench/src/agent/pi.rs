//! `pi` in the agent seat: spawned in RPC mode with the relay extension
//! under `tools/nova_bench/pi/`, prompted once with the manual, the
//! scenario and the goal, and read as an event stream. Assistant text,
//! thinking, tool calls and usage land on the bus; the tools themselves
//! reach the referee over the socket like any other client.

use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

use serde_json::{json, Value};

use crate::{
    agent::{
        agent_env,
        socket::{serve, Poll},
    },
    audit::BenchEvent,
    game::GameChannel,
    manual,
    referee::Referee,
};

/// How pi is launched.
#[derive(Debug, Clone)]
pub struct PiConfig {
    /// `--model`, when set.
    pub model: Option<String>,
    /// `--thinking`, when set.
    pub thinking: Option<String>,
    /// The relay extension file.
    pub extension: PathBuf,
    /// The system prompt: the manual.
    pub system_prompt: String,
    /// The first user message: scenario, goal, first observation.
    pub prompt: String,
    /// Where pi's stderr goes.
    pub log_path: PathBuf,
}

/// The pi binary to spawn, when the one on `PATH` is not the one to run.
pub const PI_ENV: &str = "NOVA_BENCH_PI";

/// The pi binary; [`PI_ENV`] overrides the one on `PATH`.
fn pi_binary() -> String {
    std::env::var(PI_ENV).unwrap_or_else(|_| "pi".to_string())
}

impl PiConfig {
    /// The command line after the binary.
    pub fn args(&self) -> Vec<String> {
        let mut args: Vec<String> = [
            "--mode",
            "rpc",
            "--no-session",
            "--no-extensions",
            "--no-skills",
            "--no-prompt-templates",
            "--no-context-files",
            "--no-builtin-tools",
            "--tools",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        // The nix `pi` wrapper appends `--extension` flags of its own, so this
        // allowlist is what keeps them out. It is built from the manual's tool
        // table rather than spelled here, because a tool missing from it is
        // filtered out at run time with no error anywhere.
        args.push(crate::manual::tool_list());
        args.push("--extension".into());
        args.push(self.extension.display().to_string());
        args.push("--system-prompt".into());
        args.push(self.system_prompt.clone());
        if let Some(model) = &self.model {
            args.push("--model".into());
            args.push(model.clone());
        }
        if let Some(thinking) = &self.thinking {
            args.push("--thinking".into());
            args.push(thinking.clone());
        }
        args
    }
}

struct PiProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    events: Receiver<Value>,
    nudges: u32,
    prompts: u32,
}

impl PiProcess {
    fn spawn(config: &PiConfig, socket: &Path) -> Result<Self, String> {
        let log = std::fs::File::create(&config.log_path)
            .map_err(|error| format!("could not create {}: {error}", config.log_path.display()))?;
        let binary = pi_binary();
        let mut child = Command::new(&binary)
            .args(config.args())
            .envs(agent_env(socket))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|error| format!("could not run {binary}: {error}"))?;
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        let (sender, events) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("nova-bench-pi-stdout".to_string())
            .spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let value = serde_json::from_str::<Value>(line.trim())
                        .unwrap_or_else(|_| json!({ "type": "stdout", "text": line }));
                    if sender.send(value).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| format!("could not spawn the pi reader: {error}"))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            events,
            nudges: 0,
            prompts: 0,
        })
    }

    fn prompt(&mut self, message: &str) -> Result<(), String> {
        self.prompts += 1;
        let line = json!({ "id": format!("prompt-{}", self.prompts), "type": "prompt", "message": message });
        self.write(&line)
    }

    fn write(&mut self, line: &Value) -> Result<(), String> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err("pi's stdin is closed".into());
        };
        writeln!(stdin, "{line}")
            .and_then(|()| stdin.flush())
            .map_err(|error| format!("pi stopped reading its stdin: {error}"))
    }

    fn exited(&mut self) -> Option<std::process::ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    fn shutdown(&mut self) {
        if self.exited().is_none() {
            let _ = self.write(&json!({ "type": "abort" }));
        }
        self.stdin.take();
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5) {
            if self.exited().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Spawn pi, prompt it, serve it until it settles for good or the run
/// ends, then shut it down.
pub fn run<G: GameChannel>(
    referee: &mut Referee<G>,
    config: &PiConfig,
    socket: &Path,
) -> Result<String, String> {
    let mut pi = PiProcess::spawn(config, socket)?;
    referee.bus().emit(BenchEvent::Note {
        text: format!(
            "pi pid {} model {} thinking {}",
            pi.child.id(),
            config.model.as_deref().unwrap_or("default"),
            config.thinking.as_deref().unwrap_or("default")
        ),
    });
    pi.prompt(&config.prompt)?;
    let reason = serve(referee, socket, |referee| poll(referee, &mut pi));
    pi.shutdown();
    reason
}

/// Drain pi's events onto the bus and decide whether to keep serving.
fn poll<G: GameChannel>(referee: &mut Referee<G>, pi: &mut PiProcess) -> Poll {
    loop {
        let event = match pi.events.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                let status = pi
                    .exited()
                    .map_or("closed its stdout".to_string(), |status| status.to_string());
                return Poll::Stop(format!("agent_exit (pi {status})"));
            }
        };
        if let Some(stop) = absorb(referee, pi, &event) {
            return Poll::Stop(stop);
        }
    }
    if let Some(status) = pi.exited() {
        return Poll::Stop(format!("agent_exit (pi {status})"));
    }
    Poll::Continue
}

/// One RPC event. Returns a stop reason when pi is done for good.
fn absorb<G: GameChannel>(
    referee: &mut Referee<G>,
    pi: &mut PiProcess,
    event: &Value,
) -> Option<String> {
    let bus = referee.bus().clone();
    match event["type"].as_str().unwrap_or_default() {
        "message_end" => {
            let message = &event["message"];
            if message["role"] != "assistant" {
                return None;
            }
            for block in message["content"].as_array().into_iter().flatten() {
                match block["type"].as_str() {
                    Some("text") => {
                        let text = block["text"]
                            .as_str()
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        if !text.is_empty() {
                            bus.emit(BenchEvent::AgentText { text });
                        }
                    }
                    Some("thinking") => {
                        let text = block["thinking"]
                            .as_str()
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        if !text.is_empty() {
                            bus.emit(BenchEvent::AgentThinking { text });
                        }
                    }
                    _ => {}
                }
            }
            if message["usage"].is_object() {
                referee.scorer_mut().usage(&message["usage"]);
                bus.emit(BenchEvent::AgentUsage {
                    usage: message["usage"].clone(),
                });
            }
            if message["stopReason"] == "error" {
                bus.emit(BenchEvent::Note {
                    text: format!("pi reported a model error: {}", message["errorMessage"]),
                });
            }
        }
        "tool_execution_start" => bus.emit(BenchEvent::AgentTool {
            name: event["toolName"].as_str().unwrap_or("?").to_string(),
            args: event["args"].clone(),
            result: None,
            is_error: false,
        }),
        "tool_execution_end" => {
            let result = event["result"]["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            bus.emit(BenchEvent::AgentTool {
                name: event["toolName"].as_str().unwrap_or("?").to_string(),
                args: event["args"].clone(),
                result: Some(result),
                is_error: event["isError"] == true,
            });
        }
        "agent_settled" => {
            if referee.is_over() {
                return Some("agent_exit".into());
            }
            if pi.nudges >= 1 {
                return Some("agent_exit (pi settled without finishing)".into());
            }
            pi.nudges += 1;
            bus.emit(BenchEvent::Note {
                text: "pi settled with the run still open; nudged once".into(),
            });
            if let Err(error) = pi.prompt(manual::NUDGE) {
                return Some(format!("agent_exit ({error})"));
            }
        }
        "response" => {
            if event["success"] == false {
                bus.emit(BenchEvent::Note {
                    text: format!("pi refused a command: {}", event["error"]),
                });
                return Some(format!(
                    "agent_error: pi refused the prompt: {}",
                    event["error"]
                ));
            }
        }
        "stdout" => bus.emit(BenchEvent::Note {
            text: format!("pi: {}", event["text"]),
        }),
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_is_launched_headless_with_only_the_relay_tools() {
        let config = PiConfig {
            model: Some("gpt-5.6-luna".into()),
            thinking: Some("low".into()),
            extension: PathBuf::from("/repo/tools/nova_bench/pi/index.ts"),
            system_prompt: "manual".into(),
            prompt: "go".into(),
            log_path: PathBuf::from("/run/agent.log"),
        };
        let args = config.args();
        assert_eq!(&args[..2], ["--mode", "rpc"]);
        assert!(args.contains(&"--no-builtin-tools".to_string()));
        let tools = crate::manual::tool_list();
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--tools".to_string(), tools.clone()]));
        assert!(
            tools.contains("page"),
            "the allowlist must carry every tool"
        );
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--model", "gpt-5.6-luna"]));
        assert!(args.windows(2).any(|pair| pair == ["--thinking", "low"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--system-prompt", "manual"]));
    }
}
