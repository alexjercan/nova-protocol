//! `log_clean`: panics, ERROR-level lines, and command errors at ANY level are
//! hard failures.
//!
//! The level token is matched as a whole word after ANSI stripping - a
//! substring match missed line-initial ERROR and false-positived on payloads
//! (a word merely containing "ERROR").
//!
//! Command errors need their own rule because the level alone does not catch
//! them: `remove`/`despawn` bake in the WARN handler at
//! queue time (bevy_ecs `commands/mod.rs`, `queue_handled(_, warn)`), so a
//! stale-entity command logs a WARN and sails past the ERROR gate. The matched
//! prefix is the one both flavors share (bevy_ecs `error/handler.rs`), so a
//! teardown-race regression fails the run instead of scrolling by.

use super::{Check, CheckStatus, RunArtifacts};

const THRESHOLD: &str = "no panics / ERROR lines / command errors";

/// The bevy_ecs command-error prefix, logged at the handler's level (WARN for
/// `remove`/`despawn`, ERROR for the rest).
const COMMAND_ERROR: &str = "Encountered an error in command";

/// The web pass's name in the manifest (`native/web.rs`).
const WEB_PASS: &str = "web";

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
    line.contains("panicked at")
        || line.contains(COMMAND_ERROR)
        || line.split_whitespace().any(|token| token == "ERROR")
}

fn clip(line: &str) -> String {
    line.chars().take(SAMPLE_CHARS).collect()
}

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let Some(log) = artifacts.log.as_ref() else {
        // A web pass ALWAYS writes web-run.log, so a web run with no log is a
        // lost capture, not an unknowable one. The distinction is what a
        // panicking wasm app used to hide behind: no run.log on a browser
        // platform meant SKIPPED, and the run verdicted OK at exit 0.
        let web_ran = artifacts
            .manifest
            .as_ref()
            .is_some_and(|manifest| manifest.passes.iter().any(|pass| pass.name == WEB_PASS));
        return Check {
            name: "log_clean",
            status: if web_ran {
                CheckStatus::Fail
            } else {
                CheckStatus::Skipped
            },
            value: "no log".into(),
            threshold: THRESHOLD.into(),
            detail: if web_ran {
                "the manifest records a web pass but no web-run.log was captured".into()
            } else {
                "log not captured alongside the run".into()
            },
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
            value: "0 offending lines".into(),
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
    use crate::evaluation::{
        checks::evaluate_checks,
        fixtures::*,
        manifest::{PassRecord, RunManifest},
    };

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
    fn a_warn_level_command_error_fails() {
        // The class the ERROR gate cannot see: `despawn` on a stale entity
        // logs at WARN, so only the command-error rule catches it.
        let c = scan(
            "INFO fine\n2026-08-06T10:00:00Z  WARN bevy_ecs::error::handler: \
             Encountered an error in command `bevy_ecs::system::commands::despawn`: \
             The entity 42v1 does not exist\n",
        );
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["offending"], 1, "{c:?}");
    }

    /// A web run has no run.log; its log is the browser's captured console,
    /// which carries the game's own output. Scanning it is the only thing
    /// standing between a panicking wasm app and a green exit 0.
    #[test]
    fn a_web_run_log_is_scanned_like_any_other() {
        let dir = scratch_run_dir();
        std::fs::remove_file(dir.join("run.log")).unwrap();
        std::fs::write(
            dir.join("web-run.log"),
            "INFO:CONSOLE thread 'main' panicked at src/x.rs:1:1:\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let c = check(&evaluate_checks(&artifacts), "log_clean").clone();
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A web run that captured no log is a lost capture, not an unknowable
    /// one: SKIPPING here is what let a panicking wasm app exit 0.
    #[test]
    fn a_web_run_with_no_log_fails_rather_than_skips() {
        let dir = scratch_run_dir();
        std::fs::remove_file(dir.join("run.log")).unwrap();
        let manifest = RunManifest {
            passes: vec![PassRecord {
                name: WEB_PASS.into(),
                success: true,
                timed_out: false,
            }],
            ..manifest_ok()
        };
        write_manifest(&dir, &manifest);
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let c = check(&evaluate_checks(&artifacts), "log_clean").clone();
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_quiet_log_passes() {
        let c = scan("INFO nova harness: reached Playing\nINFO cycle complete\n");
        assert_eq!(c.status, CheckStatus::Pass, "{c:?}");
    }
}
