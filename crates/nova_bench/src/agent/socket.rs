//! The referee's socket front: one JSON request per line, one reply per
//! line, on a unix socket. One client at a time; a client may open a
//! connection per request (the pi relay does) or hold one open.
//!
//! The server never blocks for long: it polls the agent process between
//! accepts and reads, so an agent that dies is noticed and the deadline is
//! kept.

use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::UnixListener,
    path::Path,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

use crate::{game::GameChannel, referee::Referee};

/// What the front's poll says between requests.
pub enum Poll {
    /// Keep serving.
    Continue,
    /// Stop serving, for this reason.
    Stop(String),
}

/// How long the server keeps answering after the run is over, so the agent
/// can learn it from a reply and say its last word.
const OVER_GRACE: Duration = Duration::from_secs(60);
const READ_TIMEOUT: Duration = Duration::from_millis(200);
const IDLE_SLEEP: Duration = Duration::from_millis(20);

/// Serve until `poll` says stop, or until the run has been over for the
/// grace period. Returns the stop reason.
pub fn serve<G: GameChannel>(
    referee: &mut Referee<G>,
    path: &Path,
    mut poll: impl FnMut(&mut Referee<G>) -> Poll,
) -> Result<String, String> {
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)
        .map_err(|error| format!("could not bind {}: {error}", path.display()))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("could not set the socket nonblocking: {error}"))?;
    let mut over_since: Option<Instant> = None;
    loop {
        if let Poll::Stop(reason) = poll(referee) {
            return Ok(reason);
        }
        if referee.check_deadline() {
            let since = over_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= OVER_GRACE {
                return Ok("agent_exit".into());
            }
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
                let mut writer = stream
                    .try_clone()
                    .map_err(|error| format!("could not clone the connection: {error}"))?;
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let reply = match serde_json::from_str::<Value>(line.trim()) {
                                Ok(request) => referee.handle(&request),
                                Err(error) => json!({ "error": format!("not JSON: {error}") }),
                            };
                            if writeln!(writer, "{reply}")
                                .and_then(|()| writer.flush())
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                ErrorKind::WouldBlock | ErrorKind::TimedOut
                            ) =>
                        {
                            if let Poll::Stop(reason) = poll(referee) {
                                return Ok(reason);
                            }
                            referee.check_deadline();
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(IDLE_SLEEP);
            }
            Err(error) => return Err(format!("the referee socket failed: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io::BufRead, os::unix::net::UnixStream};

    use super::*;
    use crate::{audit::Bus, game::Answer, referee::Budget};

    struct Scripted(Vec<Value>);

    impl GameChannel for Scripted {
        fn send(&mut self, _: &Value) -> Result<(), String> {
            Ok(())
        }
        fn read_answer(&mut self, _: Duration) -> Result<Answer, String> {
            Ok(Answer {
                snapshot: json!({ "ships": [], "mission": {} }),
                errors: vec![],
            })
        }
        fn close(&mut self) {
            self.0.clear();
        }
    }

    #[test]
    fn a_client_asks_over_the_socket_and_the_server_stops_when_polled_to() {
        let dir = std::env::temp_dir().join(format!("nova-bench-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("referee.sock");
        let mut referee = Referee::new(
            Scripted(vec![]),
            Bus::quiet(),
            Budget {
                ticks: 1000,
                turns: 10,
                deadline: Duration::from_secs(30),
            },
            false,
        );
        referee.start().unwrap();
        let client_path = path.clone();
        let client = std::thread::spawn(move || {
            let started = Instant::now();
            let stream = loop {
                if let Ok(stream) = UnixStream::connect(&client_path) {
                    break stream;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "server never came up"
                );
                std::thread::sleep(Duration::from_millis(10));
            };
            let mut writer = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            writeln!(
                writer,
                r#"{{"act": {{"gestures": [{{"tap": "x"}}], "ticks": 10}}}}"#
            )
            .unwrap();
            let mut reply = String::new();
            reader.read_line(&mut reply).unwrap();
            let reply: Value = serde_json::from_str(&reply).unwrap();
            assert_eq!(reply["ok"]["tick"], 11);
            writeln!(writer, "garbage").unwrap();
            let mut text = String::new();
            reader.read_line(&mut text).unwrap();
            assert!(text.contains("not JSON"));
        });
        let mut polls = 0;
        let reason = serve(&mut referee, &path, |referee| {
            polls += 1;
            if referee.tick() >= 11 && polls > 5 {
                Poll::Stop("agent_exit".into())
            } else {
                Poll::Continue
            }
        })
        .unwrap();
        client.join().unwrap();
        assert_eq!(reason, "agent_exit");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
