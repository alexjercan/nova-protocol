//! Runs `examples/driven_app.rs` as a real process and asserts on what a
//! supervisor sees: the exit status and the log lines.
//!
//! This is the crate's only end-to-end proof. The lib tests drive the plugin
//! through a `MinimalPlugins` app; what they cannot cover is the real thing -
//! a windowed `DefaultPlugins` app whose state machine, input collection and
//! exit path are Bevy's own. In-process is not an option either: the things
//! under test are the process exit code and the stderr a supervisor reads.
//!
//! The example opens a real window, so a display is required. With neither
//! `DISPLAY` nor `WAYLAND_DISPLAY` the test skips loudly rather than failing,
//! so a plain `cargo test` on a headless box does not break. In CI it runs
//! under `xvfb-run` (see `.github/workflows/ci.yaml`).

use std::process::Command;

#[test]
fn autopilot_example_completes_a_cycle() {
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!(
            "SKIP autopilot example: no DISPLAY or WAYLAND_DISPLAY set. The \
             example opens a window; run under a virtual display (e.g. \
             xvfb-run) to exercise it."
        );
        return;
    }

    let output = Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet",
            "-p",
            "nova_autopilot",
            "--example",
            "driven_app",
        ])
        .env("NOVA_AUTOPILOT", "1")
        // Well under the 120s default: this cycle is ~3s, so a hang should
        // surface as a named-laggard error exit inside the test, not as a
        // CI-level timeout.
        .env("NOVA_AUTOPILOT_DEADLINE", "30")
        .output()
        .unwrap_or_else(|e| panic!("failed to launch the driven_app example: {e}"));

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "driven_app exited with {:?}\n--- stderr tail ---\n{}",
        output.status.code(),
        tail(&stderr),
    );
    assert!(
        stderr.contains("autopilot: cycle complete, no panic"),
        "driven_app did not complete its cycle\n--- stderr tail ---\n{}",
        tail(&stderr),
    );
    assert!(
        stderr.contains("harness completion: all collectors done, exiting"),
        "driven_app did not exit through the completion protocol\n\
         --- stderr tail ---\n{}",
        tail(&stderr),
    );
    // The app's OWN behavior, not the driver's bookkeeping: the closure's
    // press reached a real `ButtonInput` read and moved something.
    assert!(
        stderr.contains("driven_app: thrust moved the cube"),
        "the autopilot never drove the example's gameplay\n\
         --- stderr tail ---\n{}",
        tail(&stderr),
    );
}

/// The last chunk of output, so a failure message is useful without dumping
/// the whole (chatty) log. Same shape as `tests/examples_smoke.rs`.
fn tail(s: &str) -> String {
    let start = s.len().saturating_sub(48_000);
    // Don't split a UTF-8 code point (log output can contain non-ASCII).
    let start = (start..s.len())
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0);
    s[start..].to_string()
}
