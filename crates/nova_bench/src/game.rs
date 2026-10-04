//! The game as a child process on the channel: spawn `current_exe()` with
//! `--norender --channel step`, write wire lines to its stdin, read snapshot
//! and error lines off its stdout.
//!
//! The reader thread owns the blocking `read_line`; the referee only ever
//! receives from its mpsc with a timeout, so a game that hangs is a reported
//! failure rather than a hung bench. The child's stderr (the game's log) goes
//! to `game.log` in the run dir.
//!
//! [`GameChannel`] is the seam a test drives with a scripted fake: the
//! referee is generic over it and never names the process.

use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What `bench play` plays: an id resolved by the game against its merged
/// registry, or a loose `*.content.ron` registered for the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioTarget {
    /// A scenario id.
    Id(String),
    /// A loose content file.
    File(PathBuf),
}

impl ScenarioTarget {
    /// Classify a positional by suffix, so parsing stays pure. A file that
    /// does not exist is refused by the game, which can also say why.
    pub fn parse(token: &str) -> Self {
        if token.ends_with(".ron") {
            Self::File(PathBuf::from(token))
        } else {
            Self::Id(token.to_string())
        }
    }

    /// The run label: the id, or the file's stem with `.content` trimmed.
    pub fn label(&self) -> String {
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
    pub fn args(&self) -> Vec<String> {
        match self {
            Self::Id(id) => vec!["--scenario".into(), id.clone()],
            Self::File(path) => vec!["--scenario-file".into(), path.display().to_string()],
        }
    }
}

/// The play identity: an authored scenario or the menu-created open world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayTarget {
    /// An authored scenario launched directly.
    Scenario(ScenarioTarget),
    /// New Game entered through the main menu with this requested world seed.
    NewGame {
        /// The seed the player enters in the modal.
        world_seed: u32,
    },
}

impl PlayTarget {
    /// A stable directory and display label.
    pub fn label(&self) -> String {
        match self {
            Self::Scenario(scenario) => scenario.label(),
            Self::NewGame { world_seed } => format!("new-game-{world_seed}"),
        }
    }
}

/// How to launch the game.
#[derive(Debug, Clone)]
pub struct GameConfig {
    /// The game binary - normally `current_exe()`.
    pub exe: PathBuf,
    /// The repo root the game runs in; `BEVY_ASSET_ROOT` when unset.
    pub root: PathBuf,
    /// What to play.
    pub target: PlayTarget,
    /// `NOVA_SEED`; unset lets the OS seed the run.
    pub seed: Option<u64>,
    /// The channel's `--record <DIR>`.
    pub record: Option<PathBuf>,
    /// The empty profile the child is pointed at (`probe`'s sandbox rule):
    /// `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `NOVA_MODDING_CACHE_ROOT` land
    /// under it unless the operator already exports them.
    pub profile_dir: PathBuf,
    /// Where the child's stderr goes.
    pub log_path: PathBuf,
}

impl GameConfig {
    /// The child's command line after the binary.
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![
            "--norender".to_string(),
            "--mute".to_string(),
            "--channel".to_string(),
            "step".to_string(),
        ];
        if let PlayTarget::Scenario(scenario) = &self.target {
            args.extend(scenario.args());
        }
        if let Some(record) = &self.record {
            args.push("--record".into());
            args.push(record.display().to_string());
        }
        args
    }

    fn close_timeout(&self) -> Duration {
        if self.record.is_some() {
            RECORDED_CLOSE_TIMEOUT
        } else {
            CLOSE_TIMEOUT
        }
    }

    /// The environment pushed onto the child, on top of the inherited one.
    /// A variable the operator already exports is left alone, so a deliberate
    /// "bench my real profile" run stays possible.
    pub fn env(&self, already_set: impl Fn(&str) -> bool) -> Vec<(String, String)> {
        let mut env = Vec::new();
        let mut push = |key: &str, value: String| {
            if !already_set(key) {
                env.push((key.to_string(), value));
            }
        };
        push(
            "XDG_CONFIG_HOME",
            self.profile_dir.join("config").display().to_string(),
        );
        push(
            "XDG_DATA_HOME",
            self.profile_dir.join("data").display().to_string(),
        );
        push(
            "NOVA_MODDING_CACHE_ROOT",
            self.profile_dir.join("mods").display().to_string(),
        );
        push("BEVY_ASSET_ROOT", self.root.display().to_string());
        if let Some(seed) = self.seed {
            env.push(("NOVA_SEED".to_string(), seed.to_string()));
        }
        env
    }
}

/// What one read off the game's stdout returned.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    /// The snapshot that ended the read.
    pub snapshot: Value,
    /// The error lines the game emitted before it, in order.
    pub errors: Vec<Value>,
}

/// The seam between the referee and the game: write a line, read until a
/// snapshot. The process implements it; a test's scripted fake does too.
pub trait GameChannel {
    /// Write one wire line.
    fn send(&mut self, line: &Value) -> Result<(), String>;
    /// Read lines until a snapshot arrives, collecting error lines on the way.
    fn read_answer(&mut self, timeout: Duration) -> Result<Answer, String>;
    /// Close the wire (EOF is the channel's clean exit), reap the child, and
    /// report an unsuccessful exit or a timeout.
    fn close(&mut self) -> Result<(), String>;
}

/// The live game.
pub struct GameProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    close_timeout: Duration,
    /// The child's pid, for the log and for a kill by recorded pid.
    pub pid: u32,
}

/// How long the game may take to boot and answer the first step: asset IO
/// on a cold cache, matching the channel's own boot deadline.
pub const BOOT_TIMEOUT: Duration = Duration::from_secs(300);
/// How long one stepped answer may take once the world is up.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(120);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);
const RECORDED_CLOSE_TIMEOUT: Duration = Duration::from_secs(300);

impl GameProcess {
    /// Spawn the game and its stdout reader. Nothing is read until the first
    /// [`GameChannel::send`]: the channel prints nothing before its first
    /// step instruction.
    pub fn spawn(config: &GameConfig) -> Result<Self, String> {
        let log = std::fs::File::create(&config.log_path)
            .map_err(|error| format!("could not create {}: {error}", config.log_path.display()))?;
        let env = config.env(|key| std::env::var_os(key).is_some());
        let mut child = Command::new(&config.exe)
            .args(config.args())
            .current_dir(&config.root)
            .envs(
                env.iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|error| format!("could not run {}: {error}", config.exe.display()))?;
        let pid = child.id();
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        let (sender, lines) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("nova-bench-game-stdout".to_string())
            .spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    if sender.send(line).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| format!("could not spawn the stdout reader: {error}"))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            lines,
            close_timeout: config.close_timeout(),
            pid,
        })
    }

    /// The child's exit status if it has exited.
    pub fn exited(&mut self) -> Option<std::process::ExitStatus> {
        self.child.try_wait().ok().flatten()
    }
}

/// Sort one stdout line: a snapshot, an error line, or noise.
fn classify(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(trimmed).ok()
}

impl GameChannel for GameProcess {
    fn send(&mut self, line: &Value) -> Result<(), String> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err("the game's stdin is closed".into());
        };
        writeln!(stdin, "{line}")
            .and_then(|()| stdin.flush())
            .map_err(|error| format!("the game stopped reading its stdin: {error}"))
    }

    fn read_answer(&mut self, timeout: Duration) -> Result<Answer, String> {
        let deadline = Instant::now() + timeout;
        let mut errors = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "the game did not answer within {}s",
                    timeout.as_secs()
                ));
            }
            match self
                .lines
                .recv_timeout(remaining.min(Duration::from_millis(500)))
            {
                Ok(line) => {
                    let Some(value) = classify(&line) else {
                        continue;
                    };
                    if value.get("ships").is_some() {
                        return Ok(Answer {
                            snapshot: value,
                            errors,
                        });
                    }
                    if value.get("error").is_some() {
                        errors.push(value);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(status) = self.exited() {
                        return Err(format!("the game exited ({status}) before answering"));
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    let status = self
                        .child
                        .wait()
                        .map(|status| status.to_string())
                        .unwrap_or_else(|_| "unknown".into());
                    return Err(format!(
                        "the game closed its stdout ({status}); its log says why"
                    ));
                }
            }
        }
    }

    fn close(&mut self) -> Result<(), String> {
        // EOF on stdin is the channel's clean exit in step mode.
        self.stdin.take();
        let started = Instant::now();
        while started.elapsed() < self.close_timeout {
            match self.child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => return Err(format!("the game exited ({status}) on close")),
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(error) => return Err(format!("could not wait for the game on close: {error}")),
            }
        }
        let seconds = self.close_timeout.as_secs();
        self.child.kill().map_err(|error| {
            format!("the game did not exit within {seconds}s; could not kill it: {error}")
        })?;
        self.child.wait().map_err(|error| {
            format!("the game did not exit within {seconds}s; could not reap it: {error}")
        })?;
        Err(format!(
            "the game did not exit within {seconds}s of closing its stdin"
        ))
    }
}

/// The path of the binary to spawn as the game: this very executable.
pub fn game_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|error| format!("could not locate the game binary: {error}"))
}

/// `true` when a path names something on disk.
pub fn exists(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launch_line_is_a_stepped_headless_channel_on_the_scenario() {
        let config = GameConfig {
            exe: PathBuf::from("/bin/game"),
            root: PathBuf::from("/repo"),
            target: PlayTarget::Scenario(ScenarioTarget::parse("tutorial")),
            seed: Some(7),
            record: Some(PathBuf::from("/frames")),
            profile_dir: PathBuf::from("/run/profile"),
            log_path: PathBuf::from("/run/game.log"),
        };
        assert_eq!(config.close_timeout(), RECORDED_CLOSE_TIMEOUT);
        let mut unrecorded = config.clone();
        unrecorded.record = None;
        assert_eq!(unrecorded.close_timeout(), CLOSE_TIMEOUT);
        assert_eq!(
            config.args(),
            [
                "--norender",
                "--mute",
                "--channel",
                "step",
                "--scenario",
                "tutorial",
                "--record",
                "/frames"
            ]
        );
        let env = config.env(|key| key == "XDG_DATA_HOME");
        assert!(env.contains(&("NOVA_SEED".into(), "7".into())));
        assert!(env.contains(&("XDG_CONFIG_HOME".into(), "/run/profile/config".into())));
        assert!(env.contains(&("BEVY_ASSET_ROOT".into(), "/repo".into())));
        assert!(!env.iter().any(|(key, _)| key == "XDG_DATA_HOME"));
    }

    #[test]
    fn a_loose_file_labels_itself_by_stem() {
        let target = ScenarioTarget::parse("tasks/x/poc/acceptance.content.ron");
        assert_eq!(target.label(), "acceptance");
        assert_eq!(target.args()[0], "--scenario-file");
        assert_eq!(ScenarioTarget::parse("first_shift").label(), "first_shift");
    }

    #[test]
    fn stdout_lines_sort_into_snapshots_errors_and_noise() {
        assert!(classify("").is_none());
        assert!(classify("not json").is_none());
        let snapshot = classify(r#"{"schema":1,"ships":[]}"#).unwrap();
        assert!(snapshot.get("ships").is_some());
        let error = classify(r#"{"schema":1,"error":"x","line":3}"#).unwrap();
        assert_eq!(error["line"], 3);
    }
}
