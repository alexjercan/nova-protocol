//! Smoke-tests the autopilot-harnessed examples as a `cargo test` target.
//!
//! Each of these examples drives itself under `BCS_AUTOPILOT` - via the
//! `nova_debug::harness::nova_autopilot` preset or its own staged
//! `AutopilotPlugin` timeline (11/12) - Loading -> Playing, exercises a few
//! seconds of gameplay (many with in-example behavior assertions that panic on
//! failure), and exits cleanly with `AppExit::Success`, logging
//! `nova harness: reached Playing` and `autopilot: cycle complete, no panic`. This
//! test runs each one headless and asserts on exactly that - turning the examples'
//! built-in harness into an automated regression check. It also FAILS any run
//! whose stderr contains "Encountered an error in command": the
//! fallback-to-panic handler swap only escalates unhandled commands, while
//! `remove`/`despawn` bake in the WARN handler at queue time - the grep is
//! what makes handled command warns (stale-entity teardown races) gate CI
//! (task 20260713-203709).
//!
//! The examples open a real window (they use `DefaultPlugins`), so a display is
//! required. In CI set up a virtual one, e.g. `Xvfb :99 & export DISPLAY=:99`. With
//! no `DISPLAY` the test skips loudly rather than failing, so a plain `cargo test`
//! on a headless box does not break.

use std::process::Command;

/// The examples that drive themselves under `BCS_AUTOPILOT` - the
/// `nova_autopilot()` preset or a custom staged `AutopilotPlugin` (11/12).
/// Every example that gains a harness joins this list (task 20260712-211352).
const HARNESSED_EXAMPLES: &[&str] = &[
    "01_controller_section",
    "02_thruster_section",
    "03_hull_section",
    "04_turret_section",
    "05_torpedo_section",
    "06_torpedo_guidance",
    "07_com_range",
    "08_scenario",
    "09_editor",
    "10_playable",
    "11_hud_range",
    "12_menu_newgame",
    "13_screenshot_reel",
    "14_screenshot_ui",
    "15_screenshot_combat",
    "16_screenshot_sections",
    "17_screenshot_juice",
    "18_screenshot_orbit",
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
        // Command errors gate the run too (task 20260713-203709): the
        // fallback-to-panic handler swap (12_menu_newgame) only escalates
        // UNHANDLED commands - `remove`/`despawn` bake in the WARN handler
        // at queue time (bevy_ecs commands/mod.rs `queue_handled(_, warn)`),
        // so their "Entity despawned" errors log a warn and sail past the
        // panic gate. This grep is the stable prefix both flavors share
        // (bevy_ecs error/handler.rs), so a teardown-race regression fails
        // the suite instead of scrolling by (the 2026-07-12 playtest warn
        // class, fixed in 20260712-115902, would NOT have failed CI).
        // Print the matching lines themselves above the tail: this error
        // class fires early (load/teardown), and a chatty run can push it
        // out of the 48 KB tail (review R1.1).
        let command_errors: Vec<&str> = stderr
            .lines()
            .filter(|line| line.contains("Encountered an error in command"))
            .collect();
        assert!(
            command_errors.is_empty(),
            "example {example} logged a command error (stale entity command?)\n--- matching lines ---\n{}\n--- stderr tail ---\n{}",
            command_errors.join("\n"),
            tail(&stderr),
        );
    }
}

/// The last chunk of output, so a failure message is useful without dumping the
/// whole (very chatty) debug log. Sized to keep a full RUST_BACKTRACE=full
/// panic backtrace (CI runs the smoke step with it, ~30 KB) plus the lines
/// leading up to it; 2 KB proved too small and cut exactly the frames that
/// mattered.
fn tail(s: &str) -> String {
    let start = s.len().saturating_sub(48_000);
    // Don't split a UTF-8 code point (log output can contain non-ASCII).
    let start = (start..s.len())
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0);
    s[start..].to_string()
}
