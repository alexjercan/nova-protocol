//! Smoke-tests the autopilot-harnessed examples as a `cargo test` target.
//!
//! Each of these examples wires `nova_debug::harness::nova_autopilot`, so under
//! `BCS_AUTOPILOT` it drives itself Loading -> Playing, exercises a few seconds of
//! gameplay, and exits cleanly with `AppExit::Success`, logging
//! `nova harness: reached Playing` and `autopilot: cycle complete, no panic`. This
//! test runs each one headless and asserts on exactly that - turning the examples'
//! built-in harness into an automated regression check.
//!
//! The examples open a real window (they use `DefaultPlugins`), so a display is
//! required. In CI set up a virtual one, e.g. `Xvfb :99 & export DISPLAY=:99`. With
//! no `DISPLAY` the test skips loudly rather than failing, so a plain `cargo test`
//! on a headless box does not break.

use std::process::Command;

/// The examples that wire `nova_autopilot` (grep `nova_autopilot` in `examples/`).
const HARNESSED_EXAMPLES: &[&str] = &[
    "03_scenario",
    "06_torpedo_range",
    "07_torpedo_guidance",
    "08_turret_range",
];

/// Run each harnessed example headless and assert it reaches gameplay and exits
/// without panic. Sequential on purpose: each spawns a `cargo run`, and running
/// them one at a time avoids piling up concurrent builds/windows.
#[test]
fn harnessed_examples_reach_playing_without_panic() {
    let Some(display) = std::env::var_os("DISPLAY") else {
        eprintln!(
            "SKIP harnessed_examples_reach_playing_without_panic: no DISPLAY set. \
             The examples open a window; run under a virtual display (e.g. Xvfb) to \
             smoke-test them."
        );
        return;
    };
    eprintln!("running example smoke tests on DISPLAY={display:?}");

    for &example in HARNESSED_EXAMPLES {
        eprintln!("smoke: {example}");
        let output = Command::new(env!("CARGO"))
            .args([
                "run",
                "--quiet",
                "--example",
                example,
                "--features",
                "debug",
            ])
            .env("BCS_AUTOPILOT", "1")
            .output()
            .unwrap_or_else(|e| panic!("failed to launch example {example}: {e}"));

        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "example {example} exited with {:?}\n--- stderr tail ---\n{}",
            output.status.code(),
            tail(&stderr),
        );
        assert!(
            stderr.contains("nova harness: reached Playing"),
            "example {example} never reached Playing\n--- stderr tail ---\n{}",
            tail(&stderr),
        );
        assert!(
            stderr.contains("autopilot: cycle complete, no panic"),
            "example {example} did not complete the autopilot cycle\n--- stderr tail ---\n{}",
            tail(&stderr),
        );
    }
}

/// The last few KB of output, so a failure message is useful without dumping the
/// whole (very chatty) debug log.
fn tail(s: &str) -> String {
    let start = s.len().saturating_sub(2000);
    s[start..].to_string()
}
