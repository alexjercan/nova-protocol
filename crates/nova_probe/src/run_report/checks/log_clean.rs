//! `log_clean`: panics and ERROR-level lines are hard failures.
//!
//! The level token is matched as a whole word after ANSI stripping - a
//! substring match missed line-initial ERROR and false-positived on payloads
//! (a word merely containing "ERROR").

use super::{Check, CheckStatus, RunArtifacts};

const THRESHOLD: &str = "no panics / ERROR lines";

/// How much of an offending line the report quotes.
const SAMPLE_CHARS: usize = 160;

/// Drop ANSI escape sequences (color codes) from a log line, so the log
/// scan sees the same text regardless of whether the run's output went to a
/// TTY or a file.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI sequence: ESC [ ... final byte in @-~
            if chars.peek() == Some(&'[') {
                chars.next();
                for f in chars.by_ref() {
                    if ('@'..='~').contains(&f) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Whether a stripped line is a failure.
fn is_offending(line: &str) -> bool {
    line.contains("panicked at") || line.split_whitespace().any(|token| token == "ERROR")
}

fn clip(line: &str) -> String {
    line.chars().take(SAMPLE_CHARS).collect()
}

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let Some(log) = artifacts.log.as_ref() else {
        return Check {
            name: "log_clean",
            status: CheckStatus::Skipped,
            value: "no run.log".into(),
            threshold: THRESHOLD.into(),
            detail: "log not captured alongside the run".into(),
            data: serde_json::Value::Null,
        };
    };

    let offending: Vec<String> = log
        .lines()
        .map(strip_ansi)
        .filter(|line| is_offending(line))
        .collect();

    if offending.is_empty() {
        Check {
            name: "log_clean",
            status: CheckStatus::Pass,
            value: "0 panic/ERROR lines".into(),
            threshold: THRESHOLD.into(),
            detail: "log scanned clean".into(),
            data: serde_json::json!({ "offending": 0 }),
        }
    } else {
        Check {
            name: "log_clean",
            status: CheckStatus::Fail,
            value: format!("{} offending line(s)", offending.len()),
            threshold: THRESHOLD.into(),
            detail: format!("first: {}", clip(&offending[0])),
            data: serde_json::json!({
                "offending": offending.len(),
                "sample": offending.iter().take(5).map(|s| clip(s)).collect::<Vec<_>>(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_report::{checks::evaluate_checks, fixtures::*};

    /// Scan `log` as if it were the run's captured output.
    fn scan(log: &str) -> Check {
        let dir = scratch_run_dir();
        std::fs::write(dir.join("run.log"), log).unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "log_clean").clone();
        let _ = std::fs::remove_dir_all(&dir);
        c
    }

    #[test]
    fn ansi_colored_error_line_still_fails() {
        // A TTY-captured bevy log wraps the level in color codes; the scan
        // must see through them.
        let c = scan("ok line\n2026-07-19T10:00:00Z \u{1b}[31m ERROR \u{1b}[0m something broke\n");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
    }

    #[test]
    fn planted_panic_fails() {
        let c = scan("INFO fine\nthread 'main' panicked at src/x.rs:1:1:\nboom\n");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
    }

    #[test]
    fn line_initial_error_is_caught_and_a_payload_word_is_not() {
        // The old substring match needed surrounding spaces to fire, and
        // matched any word containing ERROR.
        let c = scan("ERROR boot diagnostics failed\nnoting TERRORD is fine\n");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["offending"], 1, "TERRORD must not count: {c:?}");
    }

    #[test]
    fn a_quiet_log_passes() {
        let c = scan("INFO nova harness: reached Playing\nINFO cycle complete\n");
        assert_eq!(c.status, CheckStatus::Pass, "{c:?}");
    }
}
