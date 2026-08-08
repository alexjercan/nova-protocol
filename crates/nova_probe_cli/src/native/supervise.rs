//! Spawning child runs: the throwaway Xvfb, the example build, and the
//! supervised, timeout-bounded run itself.

use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// Kill-by-recorded-PID guard for the throwaway Xvfb (never pkill).
pub(crate) struct XvfbGuard(Child);

impl Drop for XvfbGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The throwaway-Xvfb candidate walk for this process: pid-anchored
/// start, then the whole :80-:89 band in rotation (clear of the retired
/// perf scripts' :94/:95). Pure, so the band and its full coverage are
/// pinned by a test without spawning servers.
fn display_candidates() -> Vec<String> {
    let base = std::process::id() % 10;
    (0..10)
        .map(|offset| format!(":{}", 80 + (base + offset) % 10))
        .collect()
}

/// Use the explicit display, or spawn a throwaway Xvfb on a free one:
/// WALK the candidates until a server holds. NOTE: two concurrent probes
/// can land on the same pid%10 display, and a multi-run's kill/respawn can
/// race its own server's teardown; picking a number and hoping is not
/// allocation.
pub(crate) fn ensure_display(
    explicit: Option<&str>,
) -> Result<(String, Option<XvfbGuard>), String> {
    if let Some(display) = explicit {
        return Ok((display.to_string(), None));
    }
    let mut last_status = String::new();
    for display in display_candidates() {
        let mut child = Command::new("Xvfb")
            .args([display.as_str(), "-screen", "0", "1280x720x24"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not start Xvfb (is it installed?): {e}"))?;
        std::thread::sleep(Duration::from_secs(2));
        // A dead child means the display is taken (another probe, a
        // stale lock); try the next one rather than failing the run.
        match child.try_wait() {
            Ok(Some(status)) => {
                last_status = status.to_string();
            }
            _ => return Ok((display, Some(XvfbGuard(child)))),
        }
    }
    Err(format!(
        "no free display in :80-:89 (every Xvfb attempt died; last: \
         {last_status}) - pass --display to pin one"
    ))
}

/// Build the example with the given feature set, streaming cargo output.
pub(crate) fn build_example(
    root: &Path,
    example: &str,
    features: &str,
    profile: Option<&str>,
) -> Result<(), String> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root)
        .args(["build", "--example", example, "--features", features]);
    if let Some(profile) = profile {
        cmd.args(["--profile", profile]);
        // Frame pointers for honest sampled stacks; only ever combined
        // with the profiling profile so its cache stays consistent.
        let flags = std::env::var("RUSTFLAGS").unwrap_or_default();
        cmd.env(
            "RUSTFLAGS",
            format!("{flags} -C force-frame-pointers=yes").trim(),
        );
    }
    let status = cmd
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build --example {example} failed"));
    }
    Ok(())
}

/// How a supervised child run ended. A timeout is an OUTCOME, not an
/// error: the hung-run case is exactly what the report must describe
/// (an Err path here would abort before any report existed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RunOutcome {
    Completed { success: bool },
    TimedOut,
}

impl RunOutcome {
    pub(crate) fn success(self) -> bool {
        matches!(self, RunOutcome::Completed { success: true })
    }
    pub(crate) fn timed_out(self) -> bool {
        matches!(self, RunOutcome::TimedOut)
    }
}

/// Run a supervised child with `env`, capturing stdout+stderr to
/// `log_path`, killing it after `timeout` (a hung run must not wedge
/// the check - the autopilot's own backstop normally exits far
/// earlier). Errors only for infrastructure failures (spawn/log IO).
pub(crate) fn run_supervised(
    bin: &Path,
    extra_args: &[&str],
    root: &Path,
    env: &[(String, String)],
    log_path: &Path,
    timeout: Duration,
) -> Result<RunOutcome, String> {
    let log = std::fs::File::create(log_path)
        .map_err(|e| format!("could not create {}: {e}", log_path.display()))?;
    let err_log = log
        .try_clone()
        .map_err(|e| format!("could not clone log handle: {e}"))?;
    let mut child = Command::new(bin)
        .args(extra_args)
        .current_dir(root)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log))
        .spawn()
        .map_err(|e| format!("could not run {}: {e}", bin.display()))?;
    let start = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                return Ok(RunOutcome::Completed {
                    success: status.success(),
                })
            }
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!(
                    "probe: run exceeded {}s and was killed (log: {})",
                    timeout.as_secs(),
                    log_path.display()
                );
                return Ok(RunOutcome::TimedOut);
            }
            None => std::thread::sleep(Duration::from_millis(250)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_candidates_cover_the_whole_band_once() {
        let candidates = display_candidates();
        assert_eq!(candidates.len(), 10, "the walk tries every display");
        let mut numbers: Vec<u32> = candidates
            .iter()
            .map(|d| d.strip_prefix(':').unwrap().parse().unwrap())
            .collect();
        numbers.sort_unstable();
        // The full :80-:89 band, each exactly once - clear of the
        // retired perf scripts' :94/:95, no candidate repeated.
        assert_eq!(numbers, (80..=89).collect::<Vec<u32>>());
        assert_eq!(
            candidates,
            display_candidates(),
            "stable within one process (pid-anchored start)"
        );
    }
}
