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
    // Deeper files (e.g. systems/system_turret_gunnery/slider.rs,
    // screenshots/shared/) are modules of a sibling root, and data/ holds no
    // code.
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
/// - `damage invariants hold after reload` (system_hull_damage)
/// - `turret invariants hold after reload` (system_turret_gunnery)
/// - `launch chain holds in the crossing scene` (system_torpedo_launch)
///
/// The same reading covers a claim a STEP PREDICATE already held when the
/// reporting hook ran (`the live hull defends itself`, `the defeat overlay
/// comes up on death`): the hook is unreachable unless the predicate held, so
/// the marker still cannot be emitted by a run that failed the claim.
///
/// Three of `system_ship_editor`'s slugs are read the same way - `a floating
/// picker edits the colour of a scenario object`, `the Key row arms the rebind
/// and takes the key` and `the tree can be read as the ids an event names`.
/// Each sits in an `on_enter` hook that reports and logs, one beat after the
/// `until` that established it.
///
/// Five slugs are RECORDED OBSERVATIONS rather than claims - `the idle contact
/// cost is recorded` and `the settled step cost is recorded` (bug_sandbox_soak)
/// and `the swap cost is recorded` (bug_carve_apply) carry milliseconds, which
/// are a fact about the host that ran them and can never be asserted on a
/// shared runner. They are on the roster so the evidence cannot be deleted
/// quietly; each range's asserted claim beside them is structural.
///
/// `the replay digest is recorded` and `the entropy draw is recorded`
/// (system_headless_replay) read the same way for a different reason: the
/// claim that range carries is that TWO runs on one seed print one digest,
/// which a single process cannot see. The range prints; the runner diffs. The
/// slugs keep the printing on the roster.
///
/// The six `system_headless_*` ranges are `nova_channel` spikes (task
/// 20260820-174148) rather than probe ranges, and they landed without markers.
/// They are held to the same rule as everything else in `systems/`: whatever a
/// range claims, it names.
///
/// What this test bounds is that every invariant is NAMED. That the invariants
/// HOLD is what the runs themselves prove, by panicking.
const SYSTEMS_ROSTER: &[(&str, &[&str])] = &[
    (
        "system_attitude_hold",
        &[
            "attitude command swept",
            "attitude tracks",
            "attitude reconverges after reload",
            "attitude ceiling is the hull structural limit",
            "attitude ignores mass at fixed geometry",
        ],
    ),
    (
        "system_thrust_and_plume",
        &[
            "burn accelerates",
            "plume material exists",
            "plume follows throttle",
            "partial throttle is proportional",
            "plume returns to idle",
        ],
    ),
    (
        "system_hull_damage",
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
        "system_destruction_finale",
        &[
            "the turret breaks into its own art",
            "the thruster breaks into its own art",
            "the hull breaks into its own art",
            "ordinary carving leaves a viable asteroid",
            "the asteroid exhausts its own geometry",
            "one death leaves one body",
            "no death came apart into nothing",
        ],
    ),
    (
        "bug_carve_apply",
        &[
            "the cut severed bodies off the rock",
            "the swap takes one grid per rock",
            "the swap cost is recorded",
        ],
    ),
    (
        "system_turret_gunnery",
        &[
            "turret fired",
            "range target hit",
            "turret tracks the mover",
            "turret invariants hold after reload",
        ],
    ),
    (
        "system_torpedo_launch",
        &[
            "the scene switch took the ordnance",
            "torpedo fired",
            "torpedo armed",
            "torpedo detonated",
            "torpedoes detonate before contact",
            "gate damaged",
            "launch chain holds in the crossing scene",
            "torpedo leads the crosser",
        ],
    ),
    (
        "system_railgun_lance",
        &[
            "the commit outlives the trigger",
            "the charge bolt tracks the charge",
            "one slug rakes every layer",
            "recoil shoves the ship that fired",
            "the lance holds one shell",
            "the authored rake rides the shot",
            "the rake widens the corridor",
            "the rake spends one budget",
            "a wide rake craters instead of boring",
        ],
    ),
    (
        "system_blast_penetration",
        &[
            "destroyed section attenuates pressure",
            "surviving section stops pressure",
            "simultaneous blasts share one health snapshot",
            "fixtures do not consume penetration",
            "sections shield fixtures behind them",
        ],
    ),
    (
        "system_section_severing",
        &[
            "interior section becomes a hole",
            "detached component gets a rigid body",
            "command component keeps ship identity",
            "wreck remains inert and damageable",
        ],
    ),
    (
        "system_scenario_grammar",
        &[
            "onstart seeds variables and objectives",
            "the round arithmetic closes the round",
            "the trigger volume fires on enter and exit",
            "the disarmed escort fires the neutralized handler",
            "every kill reaches the tally",
            "a once handler retires after one pass",
            "both objectives complete",
        ],
    ),
    (
        "system_player_path",
        &[
            "the combat lock is on the prey",
            "the scenario saw the kill",
            "the scenario saw the travel lock",
            "the entity speed watch publishes a number",
        ],
    ),
    (
        "system_outcomes",
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
        "bug_neutralized_quiet",
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
        "system_borrowed_battery",
        &[
            "the computer claims an idle mount",
            "the cold hull fires inside the bearing gate",
            "the player lock steals every mount",
            "the mount returns only after the regrasp grace",
        ],
    ),
    (
        "system_ship_editor",
        &[
            "a blank ship is founded at its origin",
            "two clicks place two sections",
            "select mode selects, and places nothing",
            "Del removes the marked section",
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
            "Add obeys the context",
            "Ctrl+S answers under the parts gallery",
            "an Add row opens the gallery on its kind",
            "a second ship stands beside the first",
            "an offset ship builds in its own space",
            "the scenario node lists both ships",
            "the palette places a world object",
            "the inspector writes a placed object's config",
            "a number under its floor is refused where it is typed",
            "one axis box writes one number",
            "Delete removes a world object",
            "a floating picker edits the colour of a scenario object",
            "pointing at a ship lights its row in the tree",
            "a world click selects the ship",
            "a drag slides the ship on the ground plane",
            "the Y handle moves the ship off the ground plane",
            "Frame Selection puts the camera on the marked node",
            "entering a node isolates it in the tree",
            "a tree row reveals its kind and its whole id on hover",
            "the Key row arms the rebind and takes the key",
            "the inspector opens on the fields the kind is authored through",
            "the tree can be read as the ids an event names",
            "section ids survive exit and re-entry",
            "the scenario node reports the document",
            "the scenario root is authored like any other node",
            "the range's sky is picked from what the bundles ship",
            "a seeded hull is entered and inspected as a ship",
            "a saved range is switched on for the way out",
            "a destructive verb asks first",
            "the document survives a save and an open",
            "ids minted after a load do not collide",
            "the flown ship re-derives the graph",
            "the flown ship wears the skin",
        ],
    ),
    (
        "system_ui_scale",
        &[
            "a world-anchored label keeps its logical place",
            "the stage's names stand apart",
            "the top bar keeps its controls apart",
        ],
    ),
    (
        "system_field_controls",
        &[
            "a declared field wears its own unit",
            "a number is scrubbed by its own name",
            "a scrub arrives at the floor",
            "a vector axis is scrubbed by its row's step",
        ],
    ),
    (
        "system_input_modes",
        &[
            "insert mode keeps delete off the tree",
            "the keyboard comes back to normal",
            "browse mode keeps escape off the back-out",
            "bind mode keeps delete off the tree",
        ],
    ),
    (
        "bug_sandbox_soak",
        &[
            "every unshot rock collides as a hull",
            "the idle contact cost is recorded",
            "the settled step cost is recorded",
        ],
    ),
    (
        "system_hud_indicators",
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
        "system_menu_boot",
        &[
            "F5 restarts the game onto the content on disk",
            "new game reaches gameplay",
            "the menu tore down",
        ],
    ),
    (
        "bug_menu_picker",
        &[
            "the row click selects the row",
            "two or more rows measured",
            "the pane split holds across selections",
        ],
    ),
    (
        "system_nova_os",
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
        "stress_point_defense",
        &[
            "both hulls stood up whole",
            "the computer took every mount",
            "the envelope filled with inbound ordnance",
            "the battery was working the stream",
            "the battery shot torpedoes down",
            "the sky filled with point-defense rounds",
            "the sky drained to nothing",
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
    (
        "system_headless_pointer",
        &[
            "the pause overlay lays out with no renderer",
            "a wire click resumes the game",
        ],
    ),
    (
        "system_command_shell",
        &[
            "`:` opens the command shell",
            "a typed command answers from the live world",
            "Tab completes an id only the live world knows",
            "Escape gives back the surface the shell covered",
            "Tab opens NOVA OS after the command shell has been used",
        ],
    ),
    (
        "system_headless_novaos",
        &[
            "the NOVA OS verb registers headless",
            "the whole action table registers headless",
            "the terminal takes typing with no renderer",
        ],
    ),
    (
        "system_headless_replay",
        &[
            "the replay digest is recorded",
            "the entropy draw is recorded",
        ],
    ),
    (
        "system_headless_rebind",
        &[
            "the rebind store starts isolated",
            "the registry takes a wire rebind",
        ],
    ),
    (
        "system_headless_drag",
        &[
            "a wire drag moves the volume",
            "the slider widget agrees with the resource",
        ],
    ),
    (
        "system_headless_crt",
        &[
            "the forwarded pointer reaches the blip",
            "the window mouse cannot reach behind the glass",
            "the press lands through the glass",
            "the clicked blip engages GOTO on its contact",
        ],
    ),
];

/// How many invariants the `systems/` ranges assert between them.
const SYSTEMS_INVARIANTS: usize = 218;

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
