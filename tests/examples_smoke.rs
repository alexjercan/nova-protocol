//! Smoke-tests the autopilot-harnessed examples as a `cargo test` target,
//! one test per example category so a single category runs alone via the
//! test-name filter (`cargo test --test examples_smoke sections`).
//!
//! Each of these examples drives itself under `NOVA_AUTOPILOT` - via the
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
];

/// examples/systems/ - code-built fixtures for the cross-cutting gameplay
/// systems: the scenario grammar, the player path, and the outcome arc.
const SYSTEMS: &[&str] = &["scenario_grammar", "player_path", "outcomes"];

/// examples/ui/ - staged UI flows, each driving real widgets with synthesized
/// pointer input (editor via the preset; the rest drive their own
/// `AutopilotPlugin` timelines).
///
/// `widget_zoo` is a widget-library showcase, not a game: it carries
/// `GameStates` and steps into `Playing` in `Startup` solely so the shared
/// harness can drive it, and "reached Playing" means "the widget library is up"
/// there - the run's real claims are its own in-example assertions (hover and
/// pressed faces, reskin, segmented select, check flips, slider drag, and the
/// live tree after each rebuild), which panic and fail this test.
const UI: &[&str] = &[
    "editor",
    "hud_range",
    "menu_newgame",
    "menu_scenarios",
    "widget_zoo",
];

/// examples/stress/ - the scale sweeps. Each spawns a swarm at
/// `NOVA_STRESS_COUNT` (default per example), holds it, tears it down and
/// asserts the world came back to baseline, so the smoke run gates the
/// correctness half of a category whose other half is a frame-time number.
/// `scene_baseline` is deliberately absent - see `NOT_SMOKED`.
const STRESS: &[&str] = &["many_bodies", "many_projectiles", "many_sections"];

/// examples/screenshots/ - the capture producers still run a full harnessed
/// cycle headless (capture is inert without `NOVA_SHOT`), so they smoke too.
const SCREENSHOTS: &[&str] = &[
    "screenshot_reel",
    "screenshot_ui",
    "screenshot_combat",
    "screenshot_sections",
    "screenshot_juice",
    "screenshot_orbit",
    "screenshot_nova_os",
];

/// Cataloged examples deliberately NOT in any smoke list - each entry is a
/// decision, not an omission. `catalog_matches_disk` fails if an example is
/// neither smoked nor listed here.
/// - render_scale_shot: NOVA_SHOT-driven single capture on a real GPU (its
///   point is pixels, which Xvfb + a warmed-up autopilot cycle cannot judge);
///   verified by eyeballing the PNGs (task 20260718-004723).
/// - scene_baseline: not harnessed - probe owns it (`probe run scene_baseline
///   --fps`), and a smoke pass would only measure noise.
const NOT_SMOKED: &[&str] = &["render_scale_shot", "scene_baseline"];

#[test]
fn sections_reach_playing_without_panic() {
    smoke(SECTIONS);
}

#[test]
fn systems_reach_playing_without_panic() {
    smoke(SYSTEMS);
}

#[test]
fn ui_reach_playing_without_panic() {
    smoke(UI);
}

#[test]
fn stress_reach_playing_without_panic() {
    smoke(STRESS);
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
    for &example in [SECTIONS, SYSTEMS, UI, STRESS, SCREENSHOTS, NOT_SMOKED]
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

/// Every category on disk has an EXPLICIT row in probe's policy table, so
/// the unknown-category default in `nova_probe::category_policy` can never
/// quietly apply to a real category.
///
/// The default is a backstop (probed, no frame-time claim), not a design: a
/// new category dir that skipped the contract would otherwise be probed on a
/// guess. It lives here, beside `catalog_matches_disk`, because
/// `CARGO_MANIFEST_DIR` is the repo root in this target - probe's own tests
/// cannot see the real catalog. Display-free, like its neighbor.
#[test]
fn every_category_has_a_probe_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog = nova_probe::load_example_catalog(root)
        .expect("the [[example]] catalog must parse (and autoexamples must stay off)");
    for category in nova_probe::categories(&catalog) {
        assert!(
            nova_probe::CATEGORY_POLICIES
                .iter()
                .any(|(name, _)| *name == category),
            "category `{category}/` has no row in nova_probe::CATEGORY_POLICIES - \
             a new category states its contract (probed? frame-time?) there \
             rather than inheriting the unknown-category default"
        );
    }
}

/// Every example that names a harness driver must reach it through
/// `nova_debug::harness::` - the Nova adapter over `nova_autopilot`.
///
/// This is the gate for a failure `cargo check` CANNOT see. The
/// `bevy_common_systems` prelude still exports its own `AutopilotPlugin` /
/// `ScreenshotPlugin`, and it comes into every example through
/// `nova_protocol::prelude::*`. A bare `AutopilotPlugin::new()` therefore
/// compiles either way, but the bcs one arms on the retired bcs activation env
/// (never set any more) and writes a bcs `AutopilotLoop` - so the example boots
/// INERT while a `MessageReader<nova_autopilot::AutopilotLoop>` next to it
/// aborts the run on an uninitialised message. That is how `playable` broke
/// during the migration (task 20260802-183403); this test is why it cannot
/// come back.
///
/// Display-free, like `catalog_matches_disk`: it is a source grep, not a run.
#[test]
fn examples_name_drivers_through_the_nova_harness() {
    // Every harness name the bcs prelude ALSO exports and `nova_debug`'s
    // prelude does not, so a bare use resolves silently to the inert twin.
    // Names both preludes export are safe by construction: a glob-vs-glob
    // clash is a compile error at the use site.
    const DRIVERS: &[&str] = &[
        "AutopilotPlugin",
        "AutopilotLoop",
        "ScreenshotPlugin",
        "ScreenshotReelPlugin",
        "HarnessCompletion",
    ];
    const QUALIFIED: &str = "nova_protocol::nova_debug::harness::";

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let category = entry.unwrap().path();
        if !category.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&category).unwrap() {
            let path = entry.unwrap().path();
            if !path.extension().is_some_and(|e| e == "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for (number, line) in source.lines().enumerate() {
                // Prose in `//!` and `///` docs may name a driver freely; only
                // CODE has to be qualified.
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for driver in DRIVERS {
                    if line.contains(driver) && !line.contains(QUALIFIED) {
                        offenders.push(format!(
                            "{}:{}: {}",
                            path.strip_prefix(root).unwrap().display(),
                            number + 1,
                            code.trim_end()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "examples must name harness drivers as `{QUALIFIED}<Driver>`; a bare \
         name silently resolves to the bevy_common_systems prelude's inert \
         twin:\n{}",
        offenders.join("\n")
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
            .env("NOVA_AUTOPILOT", "1")
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
            stderr.contains(nova_debug::harness::REACHED_PLAYING),
            "example {example} never reached Playing\n--- stderr tail ---\n{}",
            tail(&stderr),
        );
        // Two completion contracts: examples that run out the autopilot's
        // lifetime print its "cycle complete"; SELF-ENDING examples
        // (menu_scenarios walks a scripted arc and exits on its final stage)
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

/// The `sections/` invariant ROSTER: every invariant each range asserts, named.
///
/// Each entry is `(example, &[invariant slug, ...])`, and each slug must appear
/// in that example's source as a `nova_probe::probe_marker` literal
/// `"outcome: <slug>"` beside the `assert!` it belongs to.
///
/// Three slugs are ROUND-COMPLETION invariants - "the whole set held again on a
/// reloaded rig / in a second scene" - and have no assert of their own to sit
/// beside, because the fact they claim is that the round's OTHER asserts all
/// passed. Each rides the last assertion of its round (guarded on the round
/// label) rather than a step of its own, so it is still emitted only from a
/// line that a failing invariant would never reach:
///
/// - `damage invariants hold after reload` (hull_section)
/// - `turret invariants hold after reload` (turret_section)
/// - `launch chain holds in the crossing scene` (torpedo_section)
///
/// What this test bounds is that every invariant is NAMED. That the invariants
/// HOLD is what the runs themselves prove, by panicking.
const SECTION_ROSTER: &[(&str, &[&str])] = &[
    (
        "controller_section",
        &[
            "attitude command swept",
            "attitude tracks",
            "attitude reconverges after reload",
            "attitude tracks on rig b",
        ],
    ),
    (
        "thruster_section",
        &[
            "burn accelerates",
            "plume material exists",
            "plume follows throttle",
            "partial throttle is proportional",
            "plume returns to idle",
        ],
    ),
    (
        "hull_section",
        &[
            "partial hit exact",
            "section destroyed",
            "root and controller survive",
            "com follows surviving sections",
            "com moved aft",
            "root interpolates",
            "camera anchor tracks com",
            "damage invariants hold after reload",
        ],
    ),
    (
        "turret_section",
        &[
            "turret fired",
            "gate damaged",
            "turret tracks the mover",
            "turret invariants hold after reload",
        ],
    ),
    (
        "torpedo_section",
        &[
            "torpedo fired",
            "torpedo armed",
            "torpedo detonated",
            "gate damaged",
            "launch chain holds in the crossing scene",
            "torpedo leads the crosser",
        ],
    ),
];

/// How many invariants the five `sections/` ranges assert between them.
const SECTION_INVARIANTS: usize = 27;

/// Every `sections/` range names EXACTLY the invariants on its roster.
///
/// The stopping rule of task 20260804-093950 made executable: "deepen" is
/// bounded by a named invariant list per run, so the list has to live somewhere
/// a deletion fails. Dropping an invariant leaves the run green - it just
/// asserts less - and this test is what turns that into a red one. The runs
/// themselves are what prove the invariants HOLD; this only pins WHICH.
///
/// Matched both ways on purpose: a roster slug with no marker is an invariant
/// that was removed, and a marker with no roster slug is one added without
/// saying so.
///
/// Display-free, like `catalog_matches_disk`: it is a source grep, not a run.
#[test]
fn sections_assert_their_invariant_roster() {
    const PREFIX: &str = "\"outcome: ";

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let listed: usize = SECTION_ROSTER.iter().map(|(_, names)| names.len()).sum();
    assert_eq!(
        listed, SECTION_INVARIANTS,
        "the sections/ roster lists {listed} invariants, not {SECTION_INVARIANTS}"
    );
    assert_eq!(
        SECTION_ROSTER
            .iter()
            .map(|(example, _)| *example)
            .collect::<BTreeSet<_>>(),
        SECTIONS.iter().copied().collect::<BTreeSet<_>>(),
        "every sections/ example needs a roster, and only those"
    );

    for (example, names) in SECTION_ROSTER {
        let path = root.join("examples/sections").join(format!("{example}.rs"));
        let source = std::fs::read_to_string(&path).unwrap();
        let marked: BTreeSet<&str> = source
            .match_indices(PREFIX)
            .filter_map(|(at, _)| {
                let rest = &source[at + PREFIX.len()..];
                rest.find('"').map(|end| &rest[..end])
            })
            .collect();
        assert_eq!(
            marked,
            names.iter().copied().collect::<BTreeSet<_>>(),
            "{example} does not emit exactly its roster of invariant markers; \
             each invariant carries one `nova_probe::probe_marker` named \
             `outcome: <slug>` beside its assert"
        );
    }
}
