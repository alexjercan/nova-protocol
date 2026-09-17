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
        "system_turn_limit",
        &[
            "turn rate holds at the structural limit",
            "a shortened hull turns harder",
            "the intact hull is unmoved by its neighbour",
        ],
    ),
    (
        "system_hull_scaling",
        &[
            "the reference hulls span the size the sweep assumes",
            "both hulls publish a live attitude envelope",
            "the computer is pinned to the largest shipped hull",
            "a bigger hull is seen from further away",
            "the hull inputs are recorded",
            "a hull burns at the size of the hull",
        ],
    ),
    (
        "system_collision_damage",
        &[
            "two capitals may come alongside for free",
            "a touch under the safe speed is free at either hull size",
            "a ram spends hit points on both bodies",
            "the bite grows with the closing speed",
            "a ram on a rock is paid out of both durabilities",
            "a destructive ram leaves a wreck",
            "the contact census is recorded",
        ],
    ),
    (
        "system_ai_combat",
        &[
            "an engaged hull is flown by its flight computer",
            "an engaged hull holds its standoff instead of closing",
            "the engagement geometry is recorded",
        ],
    ),
    (
        "system_ai_evade",
        &[
            "a gun held on a hull breaks it into a weave",
            "a jink leg carries the whole hull off the line",
            "the weave is recorded",
        ],
    ),
    (
        "system_ai_patrol",
        &[
            "a hostile takes a flying patrol off its leg",
            "the patrol resumes on the route it kept",
            "the leash walks a dragged-out picket home",
            "the beat is recorded",
        ],
    ),
    (
        "system_gravity_wells",
        &[
            "a hull inside two wells is owned by one that reaches it",
            "the incumbent well holds until a challenger clears the margin",
            "the well crossing is recorded",
        ],
    ),
    (
        "system_flight_legs",
        &[
            "the composed leg chain completes on both hulls",
            "an unobstructed goto coasts before it brakes",
            "a replacement action owns the helm on the next flight tick",
            "the flown lanes are recorded",
        ],
    ),
    (
        "system_helm_orders",
        &[
            "a scenario order takes the helm and flies the hull",
            "a hostile contact takes the helm and leaves the order alone",
            "the cleared sky hands the same leg back",
            "the order log is recorded",
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
            "the fire gate is the size of what is shot at",
            "a folded mount has sunk behind shut lids",
            "the fold is lazier than the raise",
            "no round leaves a housed mount",
        ],
    ),
    (
        "system_wreck_lock",
        &[
            "an unrelated sever leaves the lock and the pin standing",
            "severing the pinned section clears the pin and keeps the lock",
            "the severed fragment is its own close contact",
            "a fragment is never acquired on its own",
            "the reticle and the inset follow the acquired fragment",
            "the same gun hits the fragment it acquired",
        ],
    ),
    (
        "system_mission_hud",
        &[
            "a posted objective reaches the stack in its own chip",
            "the marker chip stands off its target's projection",
            "the marker chip carries the live range to its target",
            "an off-screen mark pins to the edge and points home",
            "a comms cue reaches the panel in the speaker's own voice",
            "a bound readout renders its variable in the authored format",
            "a moved variable moves the readout with it",
            "the marker chip leaves with its target and the stack stands",
        ],
    ),
    (
        "system_ship_audio",
        &[
            "a burst raking a hull collapses to one report per hull",
            "an authored cue is heard through the hull it belongs to",
            "an exterior cue is parked on the listener's bearing to it",
            "a dead trigger clicks once a pull and not once a frame",
            "the drive's hum tracks the throttle and leaves at rest",
        ],
    ),
    (
        "system_hud_scales",
        &[
            "every visible indicator lands on the live window",
            "the markers count one per section until the budget caps them",
            "the inset frames the capital's whole hull",
            "the lead pip holds the projected intercept at capital scale",
        ],
    ),
    (
        "system_torpedo_capital",
        &[
            "the fuze stands off the skin of both hull sizes",
            "the warhead stops on the face, not on the aim point",
            "the capital pays on the face the warhead reached",
            "a one-section hull is fuzed, not overflown",
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
            "a warhead arms clear of the hull that fired it",
            "the ordnance makes its authored cruise",
        ],
    ),
    (
        "system_railgun_hulls",
        &[
            "one lance sits at the same station on both hulls",
            "recoil moves each hull by its own measured mass",
            "the off-axis shot spins the hull inside its own limits",
            "the computer takes the recoil back out of the heading",
            "the bore sight and the slug agree where the shot went",
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
            "a fragment leaves at the speed its own size asks for",
        ],
    ),
    (
        "system_docking_ports",
        &[
            "one command builds one connection and one root joint",
            "both sleeves reach out once the joint holds",
            "the sleeve never changes what the hull collides with",
            "the joint carries the pair without zeroing its drift",
            "the joint holds the pose the two hulls met in",
            "a docked hull ignores the throttle",
            "the dock verb takes the dock away from either hull",
            "the throttle bites the moment the dock lets go",
            "a destroyed port frees its partner",
            "the dock geometry is recorded",
            "the scenario hears every dock and every release",
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
        "system_cinematic",
        &[
            "a live scene offers the skip on the real hud",
            "a paused scene refuses the skip",
            "the skip is announced before the finish",
            "an unskippable scene offers no way out",
            "a scene that runs out reports its finish",
            "the skip cancels the beats it did not play",
            "the title card comes down on its own",
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
            "the kind is picked from a list, not typed",
            "the picked kind is the rock on the stage",
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
            "a tuned row keeps the part it names",
            "reset drops the field instead of pinning it",
            "the tree can be read as the ids an event names",
            "section ids survive exit and re-entry",
            "the scenario node reports the document",
            "the scenario root is authored like any other node",
            "the range's sky is picked from what the bundles ship",
            "a seeded hull is entered and inspected as a ship",
            "a named document saves without asking again",
            "a saved range is switched on for the way out",
            "a destructive verb asks first",
            "the document survives a save and an open",
            "ids minted after a load do not collide",
            "the flown ship re-derives the graph",
            "the flown ship wears the skin",
            "a generated hull comes out bound",
            "a named save writes a bundle of its own",
        ],
    ),
    (
        "system_ui_scale",
        &[
            "a world-anchored label keeps its logical place",
            "the bar under-widgets measure ends where it did",
            "the stage's names stand apart",
            "the top bar keeps its controls apart",
        ],
    ),
    (
        "system_pause_settings",
        &[
            "the Settings panel fits a narrow window",
            "the Settings panel fits the smallest window the game allows",
            "the Settings panel stops at its own maximum width",
            "the window keeps a floor under its own layout",
            "a fresh install opens borderless",
            "an explicitly saved window mode survives a restart",
            "the menu's Settings panel fits a narrow window",
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
        "system_hud_shell",
        &[
            "both shells enclose the live hull",
            "the flight chips clear the outer shell",
            "severing the hull shrinks its shells",
            "the shells stay nested through the shrink",
            "every camera mode clears the live hull",
            "the world-anchored chips clear their target",
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
            "the stacked layouts measure the widget above them",
            "every indicator hides when its anchor dies",
        ],
    ),
    (
        "system_lock_line_of_sight",
        &[
            "a clear line takes the lock",
            "cover breaks a held lock",
            "cover keeps a lock from being taken",
            "a cleared line gives the lock back",
            "cover drops the travel designation too",
            "an engaged trip flies through the drop",
            "an idle lock does not time out",
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
        "system_session_loop",
        &[
            "the launch scenario opens once and the menu is reachable",
            "leaving gameplay restarts the content exactly once",
            "the restart builds no throwaway menu or backdrop",
            "the restart leaves one status bar with its FPS and version",
            "a returned-to menu is a first-boot menu",
            "New Game after a restart starts the bundle's own start",
            "the session loop repeats at the same cost",
        ],
    ),
    (
        "bug_failed_assets",
        &[
            "a broken optional mod is disabled instead of blocking the boot",
            "the disabled set is persisted",
            "the mods that still work are untouched",
            "the acknowledgement re-enables nothing",
            "the recovered game still starts",
            "the native fatal report offers Quit",
            "the web fatal report explains instead of offering Quit",
        ],
    ),
    (
        "bug_menu_fallback",
        &[
            "the menu has no clean backdrop to draw",
            "the fallback camera is the menu's only 3D camera",
            "a bare menu ends the scenario it came from",
            "the fallback camera never stands over a live scene",
        ],
    ),
    (
        "system_scenario_picker",
        &[
            "the row click selects the row",
            "two or more rows measured",
            "the pane split holds across selections",
            "the played row is not the picker's default",
            "the picker starts the row the player clicked",
        ],
    ),
    (
        "bug_outcome_pause",
        &[
            "a scripted run is exempt from the focus pause",
            "losing the window pauses interactive play",
            "regaining the window never resumes by itself",
            "an outcome landing over the pause menu takes the screen",
            "a timed advance leaves no unpaused frame",
            "a timed advance hands its pause to the pause menu",
        ],
    ),
    (
        "bug_refused_scenario",
        &[
            "a refused scenario ends the one it replaces",
            "the refusal clears the scenario mirrors",
            "the refusal report names the issue",
            "the refusal leaves no loading panel over the report",
            "the refusal frees the cursor",
            "the refusal report reaches the main menu",
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
        "stress_hull_collapse",
        &[
            "one siege slug opens exactly its rake corridor",
            "every corridor cell left the hull",
            "every wreck piece went physical",
            "every piece is thrown clear of what buried it",
            "the chain of fires is the size of the collapse",
            "one hull coming apart is one kick",
            "the collapse frame cost is recorded",
            "the debris the collapse threw is recorded",
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
            "the shell over flight keeps the game frozen",
            "a typed command answers from the live world",
            "Tab completes an id only the live world knows",
            "Escape gives back the surface the shell covered",
            "Tab opens NOVA OS after the command shell has been used",
            "`:` opens the command shell from the pause menu",
            "the open computer sits above the modals it covers",
            "the shell gives back the pause menu it covered",
            "`:` opens the command shell on the main menu",
            "the shell over the menu leaves the ambience running",
            "the open computer blocks the menu behind it",
            "the menu takes the click again once the computer closes",
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
            "the rebind run's settings store is inert",
            "the registry takes a wire rebind",
        ],
    ),
    (
        "system_headless_drag",
        &[
            "a wire drag moves the volume",
            "the slider widget agrees with the resource",
            "a wire drag moves the look sensitivity",
            "the flight rig reads the new mouse gain",
        ],
    ),
    (
        "system_settings_persist",
        &[
            "the run writes an isolated settings store",
            "the settings UI writes the store",
            "a new app comes up on the saved settings",
            "the rebound key drives the verb in gameplay",
            "the relaunched settings panel shows the saved values",
            "a reset clears the persisted keybind and nothing else",
            "an older partial store loads its omitted fields on defaults",
        ],
    ),
    (
        "system_training_journey",
        &[
            "the run writes an isolated training profile",
            "a fresh profile is offered Basic Training",
            "the corner opens the handbook on the first lesson",
            "opening a lesson reads it and proves nothing",
            "the handbook launches the lesson's own practice range",
            "a won range proves only the lessons that name it",
            "the proven record reaches the file",
            "a relaunch loads what the last run proved",
            "the loaded record reaches the screen",
            "reading the handbook does not answer the offer",
            "the dismissal is a setting and the record is not",
            "the answered offer is gone on the next launch",
            "the handbook is reachable without the corner",
            "a proven lesson can be practised again",
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
const SYSTEMS_INVARIANTS: usize = 412;

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
