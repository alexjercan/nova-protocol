//! An external agent process on the socket: anything that reads
//! `NOVA_BENCH_SOCKET` and speaks the referee protocol. Its stdout and
//! stderr go to `agent.log` in the run dir.

use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::{
    agent::{
        agent_env,
        socket::{serve, Poll},
    },
    audit::BenchEvent,
    game::GameChannel,
    referee::Referee,
};

/// Spawn `argv`, serve it until it exits or the run ends, then reap it.
pub fn run<G: GameChannel>(
    referee: &mut Referee<G>,
    argv: &[String],
    socket: &Path,
    log_path: &Path,
) -> Result<String, String> {
    let log = std::fs::File::create(log_path)
        .map_err(|error| format!("could not create {}: {error}", log_path.display()))?;
    let log_err = log
        .try_clone()
        .map_err(|error| format!("could not clone the agent log: {error}"))?;
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .envs(agent_env(socket))
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|error| format!("could not run {}: {error}", argv[0]))?;
    referee.bus().emit(BenchEvent::Note {
        text: format!("agent pid {} ({})", child.id(), argv.join(" ")),
    });
    let reason = serve(referee, socket, |_| match child.try_wait() {
        Ok(Some(status)) => Poll::Stop(format!("agent_exit ({status})")),
        _ => Poll::Continue,
    });
    if child.try_wait().ok().flatten().is_none() {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5) {
            if child.try_wait().ok().flatten().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    reason
}
