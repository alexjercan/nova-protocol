//! Two display-free source gates over the example catalog, kept where the
//! catalog parser lives (`nova_probe_cli::load_example_catalog`) now that
//! `tests/examples_smoke.rs` is gone and probe is the only verdict on a run.
//!
//! Neither spawns anything: they read `Cargo.toml` and `examples/` off disk, so
//! they run everywhere, including a bare `cargo test` on a headless box.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// The repo root: this crate is `<root>/crates/nova_probe_cli`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repo root must resolve from the crate manifest dir")
}

/// Disk and the `Cargo.toml` `[[example]]` catalog must agree exactly.
///
/// One direction is load-bearing and the rest is belt: with
/// `autoexamples = false`, an example file that has NO catalog block does not
/// build at all, and nothing else in the toolchain says so - it is silently
/// dead code. The other direction (a block with no file)
/// already fails the build, and the `examples/<category>/<file>` path shape is
/// pinned by `catalog::tests::refuses_an_uncategorized_path`; both are asserted
/// here anyway because the equality that catches the real case catches them
/// for free.
#[test]
fn catalog_matches_disk() {
    let root = repo_root();

    // Example roots on disk: every .rs file DIRECTLY under a category dir.
    // Deeper files (e.g. systems/turret_gunnery/slider.rs, screenshots/shared/)
    // are modules of a sibling root, and data/ holds no code.
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
                    .strip_prefix(&root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                on_disk.insert((name, rel));
            }
        }
    }

    // The catalog, via THE parser probe's multi-run specs resolve against -
    // one parser, two consumers, no drift between them. It refuses a manifest
    // without `autoexamples = false` itself, so this unwrap also pins that
    // discovery stays off.
    let catalog = nova_probe_cli::load_example_catalog(&root)
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
}

/// The systems/ invariant ROSTER: every invariant each range asserts, named.
///
/// Each entry is `(example, &[invariant slug, ...])`, and each slug must appear
/// in that example's source as a `nova_probe::probe_marker` literal
/// `"outcome: <slug>"` beside the `assert!` it belongs to.
///
/// EVERY `systems/` range is on it, not only the former section curriculum
/// (task 20260817-013618). That is the doctrine made executable: a bug becomes
/// a range here, the fix turns it green, and the roster is what makes a later
/// deletion of the assertion fail a test rather than quietly pass.
///
/// Some slugs are ROUND-COMPLETION invariants - "the whole set held again on a
/// reloaded rig / in a second scene" - and have no assert of their own to sit
/// beside, because the fact they claim is that the round's OTHER asserts all
/// passed. Each rides the last assertion of its round (guarded on the round
/// label) rather than a step of its own, so it is still emitted only from a
/// line that a failing invariant would never reach:
///
/// - `damage invariants hold after reload` (hull_damage)
/// - `turret invariants hold after reload` (turret_gunnery)
/// - `launch chain holds in the crossing scene` (torpedo_launch)
///
/// The same reading covers a claim a STEP PREDICATE already held when the
/// reporting hook ran (`the live hull defends itself`, `the defeat overlay
/// comes up on death`): the hook is unreachable unless the predicate held, so
/// the marker still cannot be emitted by a run that failed the claim.
///
/// What this test bounds is that every invariant is NAMED. That the invariants
/// HOLD is what the runs themselves prove, by panicking.
const SYSTEMS_ROSTER: &[(&str, &[&str])] = &[
    (
        "attitude_hold",
        &[
            "attitude command swept",
            "attitude tracks",
            "attitude reconverges after reload",
            "attitude response is inertia invariant",
        ],
    ),
    (
        "thrust_and_plume",
        &[
            "burn accelerates",
            "plume material exists",
            "plume follows throttle",
            "partial throttle is proportional",
            "plume returns to idle",
        ],
    ),
    (
        "hull_damage",
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
        "turret_gunnery",
        &[
            "turret fired",
            "range target hit",
            "turret tracks the mover",
            "turret invariants hold after reload",
        ],
    ),
    (
        "torpedo_launch",
        &[
            "the scene switch took the ordnance",
            "torpedo fired",
            "torpedo armed",
            "torpedo detonated",
            "gate damaged",
            "launch chain holds in the crossing scene",
            "torpedo leads the crosser",
        ],
    ),
    (
        "blast_penetration",
        &[
            "destroyed section attenuates pressure",
            "surviving section stops pressure",
            "simultaneous blasts share one health snapshot",
            "fixtures do not consume penetration",
            "sections shield fixtures behind them",
        ],
    ),
    (
        "scenario_grammar",
        &[
            "onstart seeds variables and objectives",
            "the round arithmetic closes the round",
            "the trigger volume fires on enter and exit",
            "the disarmed escort fires the neutralized handler",
            "every kill reaches the tally",
            "both objectives complete",
        ],
    ),
    (
        "player_path",
        &[
            "the combat lock is on the prey",
            "the scenario saw the kill",
            "the scenario saw the travel lock",
            "the entity speed watch publishes a number",
        ],
    ),
    (
        "outcomes",
        &[
            "the defeat overlay comes up on death",
            "the retry reload clears the outcome",
            "the kill completes the objective",
            "the checkpoint queues the chain target",
            "the queued switch lingers",
            "continue loads the chained scenario",
        ],
    ),
    (
        "neutralized_quiet",
        &[
            "the live hull defends itself",
            "the wreck holds no defence target",
            "the wreck still carries a working gun",
            "no mount is firing on a wreck",
            "a torpedo is inside the envelope",
            "the wreck still takes damage",
            "the wreck stays defeated once",
        ],
    ),
    (
        "borrowed_battery",
        &[
            "the computer claims an idle mount",
            "the cold hull fires inside the bearing gate",
            "the player lock steals every mount",
            "the mount returns only after the regrasp grace",
        ],
    ),
    (
        "ship_editor",
        &[
            "new ship carries its controller",
            "two clicks place two sections",
            "select mode places nothing",
            "delete removes the section clicked",
            "the gallery lists the catalog",
            "the filter narrows the gallery",
            "the focus card names the part",
            "the gallery pick builds",
            "the build derives one connected graph",
            "hover and Q arm the part",
            "the skin clads the build",
            "the skin reflows around the held part",
            "the shared mount fits a hull face",
            "an occupied socket refuses",
            "a blocked drive lane refuses",
            "the flown ship re-derives the graph",
            "the flown ship wears the skin",
        ],
    ),
    (
        "hud_indicators",
        &[
            "the lock is live under the sweep",
            "the focus meter fills during the dwell",
            "the dwell ring rides the dwell",
            "the reticle sits on the locked target",
            "the readout carries distance and health",
            "the lead pip sits on the aim point",
            "one component marker per section",
            "the target inset films the lock",
            "the safety is hot while combat-locked",
            "the destination marker tracks the goto",
            "the velocity sphere tracks the burn",
            "the pinned component marker is highlighted",
            "every indicator hides when its anchor dies",
        ],
    ),
    (
        "menu_boot",
        &["new game reaches gameplay", "the menu tore down"],
    ),
    (
        "menu_picker",
        &[
            "the row click selects the row",
            "two or more rows measured",
            "the pane split holds across selections",
        ],
    ),
    (
        "nova_os",
        &[
            "tab opens the computer",
            "the ship app owns the screen",
            "the pointer reaches the offscreen subtree",
            "the press lands on the widget behind the glass",
            "the click through the glass closes the app",
            "the app switch leaves one screen",
        ],
    ),
    (
        "stress_bullets",
        &[
            "the battery assembled every mount",
            "a thousand rounds in the sky at once",
            "the volley drained to nothing",
            "the teardown left nothing behind",
        ],
    ),
    (
        "stress_torpedoes",
        &[
            "the rack assembled every bay",
            "a thousand torpedoes under guidance at once",
            "the ordnance drained to nothing",
            "the teardown left nothing behind",
        ],
    ),
    (
        "stress_one_structure",
        &[
            "the hull assembled every section",
            "the structure aggregates to one body",
            "every section carries a health node",
            "the skin clad the whole hull",
            "the teardown left nothing behind",
        ],
    ),
    (
        "stress_many_structures",
        &[
            "the fleet assembled every ship",
            "every ship acquired a hostile",
            "the fleet comes back after a churn cycle",
            "the teardown left nothing behind",
        ],
    ),
];

/// How many invariants the `systems/` ranges assert between them.
const SYSTEMS_INVARIANTS: usize = 118;

/// Every `systems/` range names EXACTLY the invariants on its roster.
///
/// The stopping rule made executable: "deepen" is bounded by a named invariant
/// list per run, so the list has to live somewhere
/// a deletion fails. Dropping an invariant leaves the run green - probe's
/// `invariants_held` counts VIOLATIONS, and a range that asserts less simply
/// violates nothing - and this test is what turns that into a red one. The runs
/// themselves are what prove the invariants HOLD; this only pins WHICH.
///
/// Matched both ways on purpose: a roster slug with no marker is an invariant
/// that was removed, and a marker with no roster slug is one added without
/// saying so. The example set is read from the catalog rather than a second
/// hand-kept list, so a new `systems/` range needs a roster to pass.
#[test]
fn systems_ranges_assert_their_invariant_roster() {
    const PREFIX: &str = "\"outcome: ";

    let root = repo_root();
    let listed: usize = SYSTEMS_ROSTER.iter().map(|(_, names)| names.len()).sum();
    assert_eq!(
        listed, SYSTEMS_INVARIANTS,
        "the systems/ roster lists {listed} invariants, not {SYSTEMS_INVARIANTS}"
    );

    let catalog = nova_probe_cli::load_example_catalog(&root).expect("the catalog must parse");
    let ranges: BTreeSet<&str> = catalog
        .iter()
        .filter(|example| example.category == "systems")
        .map(|example| example.name.as_str())
        .collect();
    assert_eq!(
        SYSTEMS_ROSTER
            .iter()
            .map(|(example, _)| *example)
            .collect::<BTreeSet<_>>(),
        ranges,
        "every systems/ example needs a roster, and only those"
    );

    for (example, names) in SYSTEMS_ROSTER {
        let path = root.join("examples/systems").join(format!("{example}.rs"));
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
