//! Smoke-tests the autopilot-harnessed examples as a `cargo test` target,
//! one test per example category so a single category runs alone via the
//! test-name filter (`cargo test --test examples_smoke sections`).
//!
//! Each of these examples drives itself under `BCS_AUTOPILOT` - via the
//! `nova_debug::harness::nova_autopilot` preset or its own staged
//! `AutopilotPlugin` timeline (hud_range/menu_newgame) - Loading -> Playing,
//! exercises a few seconds of gameplay (many with in-example behavior
//! assertions that panic on failure), and exits cleanly with
//! `AppExit::Success`, logging `nova harness: reached Playing` and
//! `autopilot: cycle complete, no panic`. This test runs each one headless and
//! asserts on exactly that - turning the examples' built-in harness into an
//! automated regression check. It also FAILS any run whose stderr contains
//! "Encountered an error in command": the fallback-to-panic handler swap only
//! escalates unhandled commands, while `remove`/`despawn` bake in the WARN
//! handler at queue time - the grep is what makes handled command warns
//! (stale-entity teardown races) gate CI (task 20260713-203709).
//!
//! The examples open a real window (they use `DefaultPlugins`), so a display is
//! required. In CI set up a virtual one, e.g. `Xvfb :99 & export DISPLAY=:99`. With
//! no `DISPLAY` the test skips loudly rather than failing, so a plain `cargo test`
//! on a headless box does not break.
//!
//! `catalog_matches_disk` needs no display: it pins disk == Cargo.toml catalog
//! == these smoke lists, so a new example that skips the catalog (which, with
//! `autoexamples = false`, means it does not build AT ALL) or skips its
//! category's smoke list fails a bare `cargo test` (task 20260719-193728).

use std::{collections::BTreeSet, path::Path, process::Command};

/// examples/sections/ - one harnessed range per ship section.
const SECTIONS: &[&str] = &[
    "controller_section",
    "thruster_section",
    "hull_section",
    "turret_section",
    "torpedo_section",
    "torpedo_guidance",
    "com_range",
];

/// examples/gameplay/ - full autopilot scenario runs.
const GAMEPLAY: &[&str] = &["scenario", "playable", "broadside", "lifeline"];

/// examples/ui/ - staged UI flows (editor via the preset; hud_range and
/// menu_newgame drive their own `AutopilotPlugin` timelines).
const UI: &[&str] = &["editor", "hud_range", "menu_newgame"];

/// examples/screenshots/ - the capture producers still run a full harnessed
/// cycle headless (capture is inert without `BCS_SHOT`), so they smoke too.
const SCREENSHOTS: &[&str] = &[
    "screenshot_reel",
    "screenshot_ui",
    "screenshot_combat",
    "screenshot_sections",
    "screenshot_juice",
    "screenshot_orbit",
];

/// Cataloged examples deliberately NOT in any smoke list - each entry is a
/// decision, not an omission. `catalog_matches_disk` fails if an example is
/// neither smoked nor listed here.
/// - render_scale_shot: BCS_SHOT-driven single capture on a real GPU (its
///   point is pixels, which Xvfb + a warmed-up autopilot cycle cannot judge);
///   verified by eyeballing the PNGs (task 20260718-004723).
/// - perf_baseline: not harnessed - probe owns it (`probe run perf_baseline
///   --fps`), and a smoke pass would only measure noise.
const NOT_SMOKED: &[&str] = &["render_scale_shot", "perf_baseline"];

#[test]
fn sections_reach_playing_without_panic() {
    smoke(SECTIONS);
}

#[test]
fn gameplay_reach_playing_without_panic() {
    smoke(GAMEPLAY);
}

#[test]
fn ui_reach_playing_without_panic() {
    smoke(UI);
}

#[test]
fn screenshots_reach_playing_without_panic() {
    smoke(SCREENSHOTS);
}

/// Disk, the Cargo.toml `[[example]]` catalog, and the smoke lists above must
/// agree exactly. Display-free on purpose: this is the drift gate that runs
/// everywhere, including a bare `cargo test` on a headless box.
#[test]
fn catalog_matches_disk() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Example roots on disk: every .rs file DIRECTLY under a category dir.
    // Deeper files (e.g. sections/turret_section/slider.rs) are modules of
    // their sibling root, and data/ holds no code.
    let mut on_disk = BTreeSet::new();
    for category in std::fs::read_dir(root.join("examples")).unwrap() {
        let category = category.unwrap().path();
        if !category.is_dir() {
            panic!(
                "stray file directly under examples/ (examples live in \
                 category dirs): {}",
                category.display()
            );
        }
        for entry in std::fs::read_dir(&category).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "rs") {
                let name = path.file_stem().unwrap().to_str().unwrap().to_string();
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                on_disk.insert((name, rel));
            }
        }
    }

    // The catalog, via THE parser probe's multi-run specs resolve against
    // (nova_probe::catalog) - one parser, two consumers, no drift between
    // them. It refuses a manifest without `autoexamples = false` itself,
    // so this unwrap also pins that discovery stays off.
    let catalog = nova_probe::load_example_catalog(root)
        .expect("the [[example]] catalog must parse (and autoexamples must stay off)");
    let cataloged: BTreeSet<(String, String)> = catalog
        .iter()
        .map(|example| (example.name.clone(), example.path.clone()))
        .collect();
    assert_eq!(
        cataloged, on_disk,
        "Cargo.toml [[example]] catalog and examples/ disagree - every \
         example file needs exactly one catalog block (and vice versa)"
    );

    // Every cataloged example is either smoked or deliberately not.
    let mut accounted = BTreeSet::new();
    for &example in [SECTIONS, GAMEPLAY, UI, SCREENSHOTS, NOT_SMOKED]
        .iter()
        .flat_map(|list| list.iter())
    {
        assert!(
            accounted.insert(example),
            "example {example} appears in more than one smoke list"
        );
    }
    let catalog_names: BTreeSet<&str> = cataloged.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        accounted, catalog_names,
        "smoke lists (+ NOT_SMOKED) and the catalog disagree - a new example \
         joins its category's list or NOT_SMOKED with a reason"
    );
}

/// Run each harnessed example headless and assert it reaches gameplay and exits
/// without panic. Sequential on purpose: each spawns a `cargo run`, and running
/// them one at a time avoids piling up concurrent builds/windows.
fn smoke(examples: &[&str]) {
    let Some(display) = std::env::var_os("DISPLAY") else {
        eprintln!(
            "SKIP examples smoke: no DISPLAY set. The examples open a window; \
             run under a virtual display (e.g. Xvfb) to smoke-test them."
        );
        return;
    };
    eprintln!("running example smoke tests on DISPLAY={display:?}");

    for &example in examples {
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
        // Two completion contracts: examples that run out the autopilot's
        // lifetime print its "cycle complete"; SELF-ENDING examples
        // (broadside walks a scripted arc and exits on its final stage)
        // print their own sentinel instead - idling out a long lifetime to
        // hear the autopilot say it would waste ~30s per CI run. A stalled
        // self-ending script cannot slip through the OR: its in-example
        // completion guard panics on an unfinished exit (non-zero status,
        // caught above).
        assert!(
            stderr.contains("autopilot: cycle complete, no panic")
                || stderr.contains("probe: script complete, exiting"),
            "example {example} did not complete its cycle\n--- stderr tail ---\n{}",
            tail(&stderr),
        );
        // Command errors gate the run too (task 20260713-203709): the
        // fallback-to-panic handler swap (menu_newgame) only escalates
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
