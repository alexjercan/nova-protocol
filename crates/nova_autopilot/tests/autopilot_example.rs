//! Runs `examples/driven_app.rs` as a real process and asserts on what a
//! supervisor sees: the exit status and the log lines.
//!
//! This is the crate's only end-to-end proof. The lib tests drive the plugin
//! through a `MinimalPlugins` app; what they cannot cover is the real thing -
//! a windowed `DefaultPlugins` app whose state machine, input collection and
//! exit path are Bevy's own. In-process is not an option either: the things
//! under test are the process exit code and the stderr a supervisor reads.
//!
//! Both ends of the step contract run against that same app: a script whose
//! beats all resolve exits green through the completion protocol, and the same
//! script plus one unsatisfiable beat error-exits naming it.
//!
//! The example opens a real window, so a display is required. With neither
//! `DISPLAY` nor `WAYLAND_DISPLAY` the test skips loudly rather than failing,
//! so a plain `cargo test` on a headless box does not break. In CI it runs
//! under `xvfb-run` (see `.github/workflows/ci.yaml`).

use std::process::{Command, Output};

/// The step names `examples/driven_app.rs` declares, in script order. A run's
/// log has to name its beats; that is what turns a stall into a diagnosis.
const STEPS: [&str; 6] = [
    "spawn the cube",
    "thrust until the game moves the cube",
    "coast",
    "click the button",
    "release the button",
    "settle in Done",
];

/// The beat the example appends under `DRIVEN_APP_STALL`, which no world state
/// can ever satisfy.
const STALLED_STEP: &str = "wait for the cube to vanish";

#[test]
fn driven_app_logs_its_step_names_and_a_stalled_step_aborts() {
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!(
            "SKIP autopilot example: no DISPLAY or WAYLAND_DISPLAY set. The \
             example opens a window; run under a virtual display (e.g. \
             xvfb-run) to exercise it."
        );
        return;
    }

    let output = run_example(false);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "driven_app {}\n--- stderr tail ---\n{}",
        nova_autopilot::exit::describe(&output.status),
        tail(&stderr),
    );
    for step in STEPS {
        assert!(
            stderr.contains(&format!("autopilot: step `{step}` begins")),
            "the run never named its `{step}` beat\n--- stderr tail ---\n{}",
            tail(&stderr),
        );
    }
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
    // The app's OWN behavior, not the driver's bookkeeping: the synthesized
    // press reached a real `ButtonInput` read and moved something. The "thrust"
    // step waits on that fact, so reaching the exit at all already implies it -
    // asserting it here names the failure when it does not.
    assert!(
        stderr.contains("driven_app: thrust moved the cube"),
        "the autopilot never drove the example's gameplay\n\
         --- stderr tail ---\n{}",
        tail(&stderr),
    );

    let stalled = run_example(true);
    let stderr = String::from_utf8_lossy(&stalled.stderr);

    assert!(
        !stalled.status.success(),
        "a step whose predicate can never hold must FAIL the process, not pass \
         it\n--- stderr tail ---\n{}",
        tail(&stderr),
    );
    assert!(
        stderr.contains(&format!("autopilot: step `{STALLED_STEP}` stalled after")),
        "the abort did not name the beat that stalled\n--- stderr tail ---\n{}",
        tail(&stderr),
    );
    assert!(
        !stderr.contains("autopilot: cycle complete"),
        "a stalled run must never report its cycle complete\n\
         --- stderr tail ---\n{}",
        tail(&stderr),
    );
}

/// The pointer half of the input contract, against the real thing: `click_at`
/// puts the cursor over a `bevy_ui` node in a live `DefaultPlugins` app and the
/// PICKING BACKEND delivers a `Pointer<Press>` to that node.
///
/// The unit tests in `src/input.rs` only prove the three pieces of state a
/// click writes (window `cursor_position`, a `CursorMoved` message, a fresh
/// `just_pressed`); they cannot prove those pieces are what a widget actually
/// consumes. This can: the "click the button" beat waits on the game's own
/// press flag, so a click that lands where the widget is not stalls the run.
#[test]
fn click_at_position_reaches_the_widget_under_it() {
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!(
            "SKIP autopilot pointer example: no DISPLAY or WAYLAND_DISPLAY set. \
             The example opens a window; run under a virtual display (e.g. \
             xvfb-run) to exercise it."
        );
        return;
    }

    let output = run_example(false);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "driven_app {}\n--- stderr tail ---\n{}",
        nova_autopilot::exit::describe(&output.status),
        tail(&stderr),
    );
    assert!(
        stderr.contains("driven_app: the pointer pressed the button"),
        "the synthesized click never reached the widget under it\n\
         --- stderr tail ---\n{}",
        tail(&stderr),
    );
}

/// Launch the example, optionally with its unsatisfiable beat appended.
fn run_example(stall: bool) -> Output {
    let mut command = Command::new(env!("CARGO"));
    command
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
        // CI-level timeout. It also has to stay above the stalled beat's own
        // 1s deadline, or the run-level watcher wins the race and the
        // named-step diagnostic is lost.
        .env("NOVA_AUTOPILOT_DEADLINE", "30");
    if stall {
        command.env("DRIVEN_APP_STALL", "1");
    }
    command
        .output()
        .unwrap_or_else(|e| panic!("failed to launch the driven_app example: {e}"))
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
