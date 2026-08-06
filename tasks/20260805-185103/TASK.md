# Cleanup and maintenance: close the engine gaps the screenshot pipeline routed around

- PRIORITY: 72
- TAGS: v0.10.0, chore, refactor, tooling, testing
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-115955

## Context

A session-long read-only investigation (three rounds, two out-of-context
verification agents) reviewed the screenshot/autopilot tooling, the game crates
and the `bevy_common_systems` boundary against KISS/YAGNI. `NOTES.md` holds the
full finding set with `file:line` evidence and the corrections each round made
to the last; this task is the parent that sequences the work.

The thesis: the screenshot examples are not hacky from laziness. Each capture
need hit a missing engine capability and routed around it. The flicker the owner
reported in `screenshot_combat`'s first `pose` is the proof - `pose()` is
correct, and the engine's camera ordering contract is what is unfinished.

Nova does NOT meaningfully hack bcs - no `#[allow]`, newtype or shim exists to
make a bcs type fit. The real boundary problems are inverted from the
hypothesis: bcs holds nova's code (`integrity`'s three combat constants,
`ui/health_display.rs` in ship-section vocabulary nova never uses), nova
re-implemented bcs `persist` twice, and nova's own prelude leaks bcs's retired
harness twins into every example.

This is a CLEANUP AND MAINTENANCE parent, deliberately slotted above the
in-flight PNG refresh (priority 70). The investigation's own scope finding is
that v0.10.0's two open engine tasks sit at priorities 50 and 66 while an image
task sits at 70 - the ordering encodes the problem it is trying to fix.

## Inputs

| Input | Where | Note |
| --- | --- | --- |
| Full finding set | `NOTES.md` in this task | Round-3 corrected; supersedes rounds 1-2 |
| The flicker | `crates/nova_scenario/src/loader/mod.rs:376-402` | one ordering edge, against the wrong writer |
| The fix pattern, already in-tree | `crates/nova_gameplay/src/camera_controller/mod.rs:106-114` | diagnoses this exact class eight lines from the bug |
| bcs pin | `Cargo.lock:817-819` (`v0.19.5`) | the bcs WORKING TREE is at `v0.19.6-6-g127f311`; verify bcs claims with `git show v0.19.5:<path>`, not the checkout |
| Owner decisions | `NOTES.md` "Owner rulings" | two round-3 conclusions were reversed by the owner on stated conditions |

## Steps

Ordering rationale for the two non-obvious constraints: step 4 wires
`nova_timeline` into six screenshot examples, two of which step 3 rewrites -
reverse them and both files get edited twice. And
`examples_name_drivers_through_the_nova_harness` cannot be deleted while it
still has a subject, so step 2 precedes step 6.

Each step below becomes its own child task at planning time.

- [x] 1. **Fix the torn `timeline.jsonl`.** `crates/nova_probe/src/recorder.rs:201`
      uses truncating `File::create` with no singleton guard while an earlier
      instance's `BufWriter` holds its own offset. Two recorders, one path.
      Small, and load-bearing for steps 4-6, which make the timeline the sole
      verdict.
      commits: `faa4011d`
- [x] 2. **Stop leaking the bcs prelude.**
      `crates/nova_gameplay/src/lib.rs:70` is the ONLY
      `pub use bevy_common_systems::prelude::*` in the workspace (the other 35
      sites are private `use` and stay). Delete it; compile; add back only what
      breaks as explicit named re-exports. Explicitly do NOT re-export
      `AutopilotPlugin`, `AutopilotLoop`, `ScreenshotPlugin`,
      `ScreenshotReelPlugin`, `HarnessCompletion` - the inert twins that boot an
      example dead, and exactly the `DRIVERS` list at
      `tests/examples_smoke.rs:233-239`. Then delete
      `examples_name_drivers_through_the_nova_harness` (`:227`). Front half of
      the `20260731-205553` warning cleanup: the prelude is why that task is
      stalled.
      commits: `26bc29e0`
- [x] 3. **One capture idiom: promote `shoot`, delete the reel.** Promote
      `shoot` to `nova_debug::harness::shoot` (collapses three copies:
      `screenshot_flight.rs:646`, `screenshot_nova_os.rs:282`,
      `screenshot_combat.rs:856`); fold `examples/ui/widget_zoo.rs:605` and
      `examples/ui/menu_scenarios.rs:267` onto it, deleting the duplicated
      `NOVA_SHOT_DIR` resolution (also duplicated in a SHIPPING crate at
      `crates/nova_scenario/src/actions/view.rs:70`); **convert
      `examples/screenshots/screenshot_sections.rs:40` and
      `screenshot_scene.rs:77` from `nova_reel(beats)` to autopilot steps** -
      this is the real cost, two files rewritten; then delete
      `crates/nova_autopilot/src/reel.rs`, `ReelBeat`, `ScreenshotReelPlugin`,
      `crates/nova_autopilot/tests/reel.rs` and `nova_reel()`
      (`crates/nova_debug/src/harness.rs:372-392`). **Keep `capture_window`** -
      it is the primitive `shoot` wraps.
      commits: `63cc7bd2`
- [x] 4. **Capture ack + one uniform scene settle.** `capture_window` becomes a
      completion collector / emits a per-shot ack; steps use
      `until(shot_written(name))`. That deletes the save-latency settle outright
      - it was never a duration, it was a missing await
      (`screenshot_combat.rs:161-165` says so). Then ONE scene-settle value on
      both paths, replacing the 90/6, 40/6 and 20/2 splits. Also makes the
      `FIGURES` manifest at `scripts/gen-web-screenshots.py:74-105` checkable.
      Done as the ACK alone, not a collector: once every shot step awaits its
      own write, a run cannot exit with a capture pending, and a collector whose
      pending set starts empty would be a second, ambiguous mechanism.
      `SETTLE_FRAMES` = 30, verified by capture runs of all six examples (the
      90 and 40 were carrying write latency on top of stillness). Shot steps
      carry `SHOT_DEADLINE_SECS`, so a lost capture aborts NAMING the step
      instead of hanging. `menu_scenarios` and `widget_zoo` had the same guessed
      hold in hand-rolled rigs and wait on the ack too. FIGURES left implicit
      (owner's call): the ack already turns a missing declared shot into a hard
      run failure, and the manifest stays advisory per the asset-coverage rule.
      commits: `20fcb406`
- [x] 4.1. **Runtime coverage: the example declares by WIRING.** Replace
      probe's hand-maintained coverage tables with a runtime declaration - the
      example declares what it can be judged on by the probe plugins it wires
      ("if it adds the frametime plugin it does frametime, simple as that").
      Design and reviewer rounds in `PROTOTYPE-runtime-coverage.md`.
      `contract::declare` runs at the TOP of each plugin's `build()`, above the
      arming guard, and the first call registers the Startup system that writes
      `probe-contract.json`. Checks resolve on a 2x2 - contract (what the
      example WIRED) x `RunManifest::armed_*` (what probe ARMED): undeclared or
      unarmed is N/A carrying its reason as a value, declared + armed + silent
      is a FAIL. `CategoryPolicy`/`CATEGORY_POLICIES`, `fps_skip_reason`,
      `RunManifest::fps_skipped` and `NOT_PROBED` are all deleted;
      `NOT_PROBED_CATEGORIES` stays as the one launch-side opinion (whether to
      SPAWN cannot be runtime - the answer is needed before the process
      exists). The fold gains UNPROBEABLE: `process_exit` and `log_clean` need
      no plugin, so an unwired example used to pass those two and print OK at
      `measured 2/6`. Two owner questions settled in the prototype: `NOT_PROBED`
      goes (option 3, let it fail - an example that cannot survive a probe run
      FAILS instead of being listed away), and `armed_*` survives as the second
      axis (arming happens before the child exists, so the contract cannot
      drive it; `--fps` stays an operator flag). Questions 3 (which examples
      keep a real frame-time claim - the 12 `nova_frametime()` lines are now
      LIVE under `--fps`) and 4 (`probe_marker` as a `Capability`) are left
      open. NOTE for step 5: its first sentence is obsolete - there is no
      `catalog.rs` policy to flip any more, and its `CheckStatus::Skipped`
      requirement is now met.
      commits: `6b84fef7`, `b63e3735`, `08acead5`, `af4e2c16`, `c62436a8`,
      `a7cb6fbf`, `cd8bcfea`, `6f88e1d2`
- [x] 5. **Make probe cover the `screenshots/` category.** DONE - the plan
      changed on contact and the record below is what shipped, not what was
      written. The `catalog.rs` policy table it opened against was deleted in
      step 4.1 (an example DECLARES what it can be judged on), and the
      `Skipped`/`Warn` fold it asked for landed in `cd8bcfea`, so what was
      left was: make the producers declarable, and delete the last
      launch-side opinion.
      - `e81a466f` - the six `screenshot_*` walks each add
        `nova_timeline()` + `nova_invariants()`. Both are inert without
        their `NOVA_PERF_*` env, so a capture run is unchanged. NO
        `nova_frametime()`: a posed walk has no steady-state window, so a
        captured fps would measure the script, not the engine -
        `scene_baseline --fps` owns that claim.
      - `1f4db201` - `render_scale_shot` converted to the one capture
        idiom. It was the last example driven by `nova_screenshot()`, which
        forces `Playing` on frame one, races the asset load, and never
        self-ends under probe - a probe run hit the 180s deadline and was
        KILLED (FAIL, measured 2/6, 188s). It is now an autopilot script
        waiting on `player_ship_present()` and ending on
        `shoot`/`shot_written`, which also cuts the settle from 240 frames
        to 120. The live preset switch became a STEP whose `on_enter`
        no-ops when `NOVA_SWITCH_QUALITY` is unset, so both modes walk the
        same beats and the two shots stay comparable. It joins the smoke
        suite; `scene_baseline` is now the only deliberate `NOT_SMOKED`
        entry, and it is one because probe owns it.
      - `de288622` - `log_clean` fails on `"Encountered an error in
        command"` at ANY level, the smoke suite's one unique assertion
        (task `20260713-203709`). No recorded run under `probe-runs/`
        contains the line, so the wider gate fails nothing that passed
        before. This is the precondition for step 6.
      - `cf28c543` - `NOT_PROBED_CATEGORIES` deleted, and with it the whole
        exclusion axis (`Resolved.excluded`, `AllManifest.excluded`, its
        `probe-all.json` field and round-trip, the sweep print, the index's
        "Not probed (deliberately)" section) - nothing could populate it
        any more. `--all` is the catalog with nothing subtracted and
        `probe run screenshots` expands like any other category. An example
        that declares nothing grades UNPROBEABLE, which is an answer, not a
        reason to skip the spawn.

      Verification: `probe run screenshots` resolves and runs all 7
      members. A full sweep graded 6/7 OK (`screenshot_combat` FAIL, see
      below); a solo re-run of that example came back OK, and a second
      sweep was OK through its first five rows - `screenshot_combat`
      included - before it was terminated, so the 7-row green sweep is NOT
      yet on record. Every graded row read `measured 5/6` with
      `fps_within_baseline` N/A ("the example wires no
      `nova_probe::nova_frametime()`"), which is the contract handshake
      reading correctly. `cargo check --workspace --all-targets
      --features debug` clean; `nova_probe` lib 99/99, bins 27/27, fmt
      clean.

      One finding, NOT caused by this step: `screenshot_combat` is an
      INTERMITTENT - once in three graded runs its `track the torpedoes
      in` step
      stalled at its 12s deadline and the run error-exited (`process_exit`,
      `run_completed` and `log_clean` all caught it). The example completes
      with the evidence plugins inert AND armed, and the passing probe runs
      clear that step in 4.2s, so the failure is a missed intercept rather
      than slow frames or the new wiring. Probe EXPOSED a pre-existing
      flake; filed as `20260806-140928` rather than papered over with a
      wider deadline.
- [x] 6. **Delete the smoke suite; converge the verdicts.** `tests/` is gone.
      The two display-free source gates that lived in it moved to
      `crates/nova_probe/tests/catalog_drift.rs` (repo root =
      `env!("CARGO_MANIFEST_DIR")/../..`), beside the catalog parser they call:
      `catalog_matches_disk`, now disk-vs-catalog alone, and
      `sections_assert_their_invariant_roster`, which reads its example set
      from the catalog instead of the deleted `SECTIONS` list. CI's "Examples
      smoke test" step is now "Probe correctness sweep" -
      `cargo run -p nova_probe --features nova-protocol/debug -- run --all`
      under `xvfb-run`, with the run dirs uploaded as an artifact.
      commits: `b3e82bb5`

      Two items in the step text were written against a tree that has since
      moved, and are recorded here rather than silently dropped:

      - `every_category_has_a_probe_policy` was already gone - deleted with
        `NOT_PROBED_CATEGORIES` in step 5 (`cf28c543`). Nothing to move.
      - The step called `sections_assert_their_invariant_roster` and
        `crates/nova_autopilot/tests/autopilot_example.rs:37-90` deletions.
        Neither was deleted, on the owner's ruling. The roster test is the only
        thing that fails when an invariant is dropped from a `sections/` range
        (probe's `invariants_held` counts VIOLATIONS, and a range that asserts
        less violates nothing), so it moved rather than died. `autopilot_example`
        grades `driven_app`, which lives in nova_autopilot's OWN `examples/` and
        is therefore not in the root catalog probe resolves against - probe
        cannot reach it, and its unique claims (a stalled beat error-exits
        naming itself; a synthesized click reaches the widget under it) have no
        probe equivalent. It is a second verdict over a DIFFERENT subject, not a
        duplicate. Its `fn tail` stopped being a copy the moment
        `examples_smoke.rs` was deleted; the comment pointing at the dead file
        was removed.
- [x] 7. **Camera authority sets.** Independent of 1-6; touches no automation
      code. One nova-owned set chain in `PostUpdate`:
      `CameraShakeSystems::Restore -> CameraAuthority::Solve` (chase + WASD
      sync) `-> ::Override` (`enforce_scripted_camera_pose`) `-> ::Additive`
      (`CameraShakeSystems::Apply`) `-> TransformSystems::Propagate`. **Zero bcs
      changes** - every writer already exports its set. Kills the flicker, the
      two missing `Propagate` edges, and the duplicate edge registration at
      `camera_controller/mod.rs:112-114` / `framing.rs:475`. Highest
      value-to-effort in the whole investigation.
      commits: `c944b4d1`, `cd1bff21`
- [x] 8. **Then, independently** (each its own child, no ordering between them):
      move bcs `integrity` + `ui/health_display.rs` to nova and scrub the nova
      task IDs from `bcs/src/physics/pd_controller.rs:535`; use bcs `persist`
      and delete nova's two copies plus the shadowed `feedback::flash`
      (`juice.rs:275`) and hand-rolled `time::Cooldown`; drop the physics pair
      from `base_scenario_object` (`crates/nova_scenario/src/actions/spawn.rs:97-113`)
      and delete the misaligned test at `:772-786`, moving its rationale comment
      onto the ship/asteroid bundles; the env-var pass; the `hud/mod.rs`
      registry refactor; the M9 dead-code sweep.

      DONE. Three owner rulings reshaped the step before implementation, and
      four of the investigation's claims did not survive contact with the tree;
      both are recorded below. Three
      more items landed in a second pass after the first report.

      **Owner rulings (asked because the step text was falsified by the code):**

      - bcs `integrity` + `ui/health_display.rs` do NOT move as-is: both have a
        live bcs consumer (`examples/15_integrity.rs` wires `IntegrityPlugin`
        AND `HealthDisplayPlugin`) and both modules are generic. The ruling is
        to RE-IMPLEMENT integrity inside nova, so nova owns its damage types
        and health, and to scrub the nova vocabulary out of bcs. NOT DONE - see
        the remainder below.
      - bcs gets commits only; no tag, no push, no pin bump. Nova stays on
        `v0.19.5`, so nothing in this run can consume a bcs change.
      - Persistence is nova's OWN layer modelled on bcs `persist`, exposing
        load/write and not only a plugin - not a swap onto `PersistPlugin`.
        This also keeps the RON format and the config-dir location, so no
        player loses a saved setting or mod set.

      **Landed:**

      - `e5da687` (bcs master) - the nova scrub, wider than the step named.
        Nine sites across five files, not one: `pd_controller.rs` (two task
        IDs plus "nova's flight tests" / "nova's test-rig"), `camera/shake.rs`
        (a `tasks/*/RETRO.md` path), `debug/inspector.rs` (three
        "nova-protocol task ..." citations) and `modding/events.rs` (a
        benchmarking task ID). Every comment kept its reasoning; only the
        pointer a bcs reader cannot resolve went. Also documents integrity's
        three damage constants as feel defaults and generalises
        `health_display`'s ship/section vocabulary. fmt + `cargo check
        --all-targets` clean.
      - `c4c58d06` - the physics pair off `base_scenario_object` onto the ship
        and asteroid bundles. The misaligned test went with it: its docstring
        said "every dynamic scenario body" but it asserted on the BASE bundle
        that the static kinds also use, so it pinned the defect. Four stale
        "overrides the base bundle's Dynamic" comments in `beacon.rs`,
        `light.rs` and `asteroid.rs` were corrected too. Verified by
        `probe run hull_section` (OK; the example asserts the live player root
        still carries `TransformInterpolation` after section loss - the better
        behavioural pin NOTES named) and a green `probe run systems` 3/3.
      - `4526c050` - the env-var pass. The concrete defect was three capture
        gates where the docs promise one: `widget_zoo` invented
        `NOVA_ZOO_CAPTURE` and `hud_range` invented `NOVA_INSET_SHOT` while
        `menu_scenarios` used the documented `NOVA_CAPTURE`, and
        `automation-harness.md:247` already states the rule ("`shoot` is its
        own gate: it captures only when `NOVA_CAPTURE` is set"). Both now read
        `capturing()`. Probe never sets `NOVA_CAPTURE`, so probe runs are
        byte-identical - confirmed by a green `probe run ui` 5/5. Also drops
        the deleted reel from `development.md`'s driver list.
      - `4de50263` - the dead-code sweep, which mostly DISCONFIRMED the
        investigation. Two real deletions: `AppBuilder::with_main_menu` (zero
        callers; its `Option<bool>` collapses to `use_default_plugins`, which
        is what every caller already got) and `nova_ui`'s `debug = []` feature
        (empty, named by no `Cargo.toml`, no `cfg` in the crate).
      - `79ccd26f` - one persistence store, two callers. `mod_prefs` and
        `settings_store` carried byte-identical backends (~180 duplicated
        lines). `nova_assets::persist` now owns them, keyed by name and generic
        over the value, with `load`/`save`/`load_from`/`save_to` as the
        surface. Both keys resolve to the exact paths and localStorage keys
        used before. A store and NOT a plugin, deliberately: both callers
        project resources through a policy the value type cannot know (the
        settings menu debounces a slider drag into one write and folds four
        resources into one blob; the mod set sorts a `HashSet` for a
        diff-friendly file), so a load-on-build/save-on-change plugin would be
        bypassed by both. `nova_menu` drops `ron`, `dirs` and `web-sys`. Tests:
        `nova_assets` persist 3/3, `nova_menu` settings_store 5/5 - now
        value-shaped only (field round-trips, and the serde-default path an
        older store takes).
      - The `feedback::flash` shadow is ALREADY GONE, closed by step 2. bcs's
        `Flash`/`FlashPlugin` has zero references anywhere in nova now, and
        `juice.rs`'s `Flash` is a PRIVATE struct for an expanding world-space
        ring (position, start time, kind, distance strength) - a different
        thing from bcs's material hit-flash, which clones a `StandardMaterial`
        and eases its emissive back. It shadowed a preluded name while the bcs
        prelude leaked; it shadows nothing now. No change needed.

      **Claims that did not reproduce**, checked against the tree before acting:

      - `--samply` is not dead: eight probe files reference it.
      - `nova_info`'s `debug` feature is not dead: `build.rs` reads
        `CARGO_FEATURE_DEBUG` to pick the version string.
      - `nova_probe/src/bin/probe/native/perf_web.rs` does not exist.
      - The god-mode residual is a non-defect. `capture.rs`'s unconditional
        force-heal is inside `combat_burst_driver`, an OPT-IN driver an example
        passes to `FrameTimePlugin::drive` - which IS the per-example flag the
        residual asked for. A scenario measuring death cost simply does not
        wire that driver.
      - The `fps_within_baseline` deletion the owner folded in is moot: step
        4.1's contract handshake already fixed the stated defect ("armed and
        always skips while the run prints OK"). It now reads N/A with its
        reason and is counted out of `measured`, so the check no longer lies.
      - `AIBehaviorState::Retreat` is speculative but is left alone: it is a
        `Reflect`-registered public variant with a written rationale and an
        `engages()` stub, so deleting it is a breaking API change for a
        cosmetic gain. Flagged, not swept.

      **Landed in the second pass:**

      - `5b26c501` - the hand-rolled `time::Cooldown`. Six components spelled a
        cooldown as `Timer::from_seconds(.., Once)` plus a workaround for the
        one thing a `Once` timer gets backwards: a fresh one is NOT finished, so
        a weapon built on it starts unable to fire. Three called
        `tick(duration())` at construction and two called `finish()`.
        `TorpedoSectionSpawnerFireState`, `AITorpedoBay::cooldown`,
        `AIThreat::damage_memory`, `AIEvade::duration` + `::cooldown`,
        `AIEngageGrace::timer` and `AIFireCadence::timer` are now `Cooldown`.
        `AIFireCadence` stops rebuilding its timer per phase flip and uses
        `trigger_for`; the engage grace's "pin to finished" hack (tick by
        `remaining`, because only `tick` updates Bevy's finished flag) becomes
        `trigger_for(0.0)`.

        Three timers deliberately stay: `AIEvade::jink` is `Repeating`; the
        `flow`/`outcome`/`objective_feedback` timers are one-shot DELAYS, not
        gates; and `TurretSectionBarrelFireState` needs `elapsed`-before-tick
        (the sub-tick lead that keeps the bullet stream uniformly spaced under
        ship motion) plus `set_duration` (live fire-rate retune), neither of
        which `Cooldown` has. Its docstring now says so.

      - `29cba4f7` - the `hud/mod.rs` registry. Eleven Add/Remove observer
        pairs, of which seven remove bodies were byte-identical modulo one
        marker type, four setup bodies identical modulo a tier and a bundle fn,
        and `remove_hud_flight_status` open-coded the same despawn loop eight
        more times. Three primitives replace them: `despawn_player_hud::<M>`
        (used fifteen times), `add_player_hud::<M, _>(app, tier, build)` for a
        widget with no spawn-time dependency, and `is_player_ship_root` for the
        guard every setup shares. The exceptions each say why they are one - a
        resource at spawn, a back-pointer that must filter by target, or three
        entity kinds no single marker covers. 1463 -> 1337 lines.

      - `5f67c75a` - nova owns its health and destruction pipeline. NOT a move:
        `integrity::health`, `integrity::components` and `integrity::core` are
        new, and bcs keeps its own copies for its own example.

        Both workarounds this deletes were already written down in the tree.
        `damage.rs` said *"bcs carries NO damage type, and Bevy 0.19 gives no
        ordering between observers of one event"*, so nova pre-scaled at every
        call site to avoid racing a subtractor it did not own; the subtractor is
        nova's now. And the ram formula's three constants were hand-copied with
        the note *"nova must not modify bcs, so the constants are duplicated
        here with this citation"* - there is one definition now, and
        `representative_kinetic_damage` IS `impact_damage`, so the turret's
        authored damage cannot drift from the ram it was authored against.

        Impact damage also became TYPED: a ram is `DamageType::Kinetic` and now
        meets the same per-section resistance table a slug does. It used to
        bypass it entirely.

        Two regressions from `c4c58d06` surfaced here and are fixed:
        `scenario_bodies_move_between_fixed_ticks` and three asteroid test rigs
        asserted on, or duplicated, the `RigidBody` that commit moved onto the
        per-kind bundles. Both were invisible to `cargo check` - the asteroid
        one was a duplicate-component PANIC - and neither crate's tests were run
        for that commit. The interpolation test now spawns an asteroid, which
        owns the pair.

      Verification for the pass: `cargo check --workspace --all-targets
      --features debug` and `cargo fmt --check` clean. Tests: nova_gameplay
      798/798, nova_scenario 151/151, nova_assets 97/97, nova_menu 76/76,
      nova_ui 21/21, nova_debug 12/12, nova_core 2/2. Probe: all of `sections`
      (5), `systems` (3), `stress` (4) and `ui` (5) graded OK.

## Definition of Done

- [ ] Every step above has a child task, each with its own DoD. This task closes
      when the children do. (manual: owner confirms the child set is complete)
- [ ] The flicker is gone: `screenshot_combat`'s first `pose` holds a stable
      frame across repeated runs. (manual: owner watches the capture run)
- [x] `tests/examples_smoke.rs` no longer exists, and CI runs a probe
      correctness pass covering the `screenshots/` category.
      (cmd: `test ! -f tests/examples_smoke.rs && rg -q "nova_probe" .github/workflows/ci.yaml`)
- [x] Nova's prelude no longer re-exports the bcs prelude.
      (cmd: `! rg -n '^\s*pub use bevy_common_systems::prelude::\*' crates`)
- [x] The screenshot reel is gone and `shoot` is the single capture idiom.
      (cmd: `test ! -f crates/nova_autopilot/src/reel.rs && ! rg -n '^[^/]*ScreenshotReelPlugin|^[^/]*ReelBeat' crates examples`)
- [x] No example branches its step timing on whether it is capturing.
      (cmd: `! rg -n "if capturing" examples/screenshots`)

## Notes

**Owner rulings that reverse the investigation's round-3 conclusions.** Both are
conditional, and the conditions are steps above, not assumptions:

- R3 said keep `tests/examples_smoke.rs`, because probe refuses the
  `screenshots/` category BY DESIGN (`catalog.rs:181-188`: *"Outside probe's
  scope: a probe verdict on one would assert nothing"*). The owner's answer is
  to change that design - valid, but the three coverage losses R3 documented are
  real and each gets an explicit fix in step 5.
- R3 said withdraw the reel delete, because `ScreenshotReelPlugin` has two live
  example users through the `nova_reel()` wrapper. Correct while the users
  exist. Converting them is budgeted in step 3.

**Things the investigation got wrong and later corrected - do not re-derive
them.** Each was verified against source:

- **Keep `log_clean`.** It is whole-token matching after ANSI stripping, not a
  substring grep (`crates/nova_probe/src/run_report/checks.rs:472-491`, with a
  comment saying substring matching was the previous BROKEN version). And
  conflating a panic with a wgpu teardown race is a false POSITIVE, not "green
  and wrong".
- **Do not change bcs's camera shake.** `CameraShakeSystems` exists
  (`bcs/src/camera/shake.rs:166`), is preluded, and `:205-209` already orders
  `Restore.before(Chase::Sync)` / `Apply.after(Chase::Sync)`. The real gap is
  that neither `Apply` nor `enforce_scripted_camera_pose` has an edge to
  `TransformSystems::Propagate`.
- **Do not add a deadline-panic to the autopilot.** Expiry ALREADY aborts with
  `AppExit::error()` naming the step (`crates/nova_autopilot/src/autopilot.rs:467-484`,
  with a NOTE comment saying exactly that). The real gap is a step with NO
  deadline and an unsatisfiable `until`, which hangs forever - and both
  subprocess harnesses block on `Command::output()` with no timeout, so that is
  a silent 60-minute CI hang. Make `deadline` mandatory or defaulted.
- **Do not build a flat `HashMap<String, Entity>` scenario index.** `EntityId`s
  are NOT unique - `crates/nova_scenario/src/actions/spawn.rs:36-39` states that
  spaceship SECTIONS carry their own `EntityId`s and an unscoped match would rip
  a section out of every ship in the scene. Five insertion sites, not one. Key
  it scoped (`(root_id, local_id)`) or skip it.

**Counts to distrust.** The investigation's numbers drifted; the shapes held.
Not reproducible: "253 of 477 prelude exports" (actual ~222 of ~387), "8
zero-caller items in the automation crates" (did not reproduce under three
passes; what exists is ~79 items with no CROSS-crate reference, i.e. internal
API), "27 env vars" (24 unique `NOVA_*`), "11 setup/remove pairs in
`hud/mod.rs`" (never enumerated). Wrong: `JuiceSettings` has 5 fields, not 14,
and is NOT dead - read every frame and `#[reflect(Resource)]`-registered, so it
is editor-tunable; `AIBehaviorState::Retreat` appears in 2 test functions, not
3, and none asserts the stub; `StepBuilder` has 5 setters, not 11; THREE of four
scenario object kinds override `RigidBody::Dynamic` (`asteroid.rs:280` too), not
two. Line references throughout the investigation run 2-3 low - it was written
against a slightly earlier tree.

**What the investigation did NOT find**, which constrains what is worth doing:
no `*_legacy`/`*_old`/`*_v2` anywhere; one damage path, one spawn path, one
camera path; all declared code-map boundaries hold except an undocumented
`nova_debug -> nova_autopilot` edge; no bcs version drift; the modding scaffold
is clean; no test asserts on doc prose. The suspected "testing documentation"
category is real but small - one WGSL source-text test worth KEEPING, three
tests asserting Bevy's log wording, and the invariant roster. Going-forward
rule: no test whose subject is source text or log text.

**The bcs boundary rule**, twice-corrected. "Safe iff it exports a `SystemSet`"
is wrong - only `meth` and the modding scaffold are opinion-free; orbit, PD and
persist are all plugins with schedule opinions, and every bcs module that writes
shared state already exports a set. The set is table stakes, not a
differentiator. What holds: **(a) order, don't disable** - ordering needs only
the exported set, costs one redundant write per frame, and survives bcs adding a
new writer, whereas a gate breaks silently; **(b) import behavior, not
presentation, and never a renderer you will not use.** The closest thing to a
real bcs hack in the tree supports (b): nova adds bcs's objectives plugin purely
for its Resource, discards its renderer (`crates/nova_gameplay/src/hud/mod.rs:293-298`),
then hand-diffs `GameObjectives` (`crates/nova_scenario/src/world.rs:52-72`) to
dodge a per-frame despawn/respawn in `bcs/src/ui/objectives.rs:104-107`. That
conflict is change-detection and renderer ownership - no SystemSet fixes it.

**Deliberately NOT in scope**, decided with the owner:

- Storing perf baselines in the repo. `git checkout <tag>` + re-run is the
  policy; `probe-runs/` stays gitignored. The residual is only that
  `fps_within_baseline` is armed and always skips while the run prints OK -
  delete the check, and let perf be `nova probe --compare <run-dir>` run by
  hand. Folded into step 8's dead-code sweep.
- Removing god mode from perf capture. A mix of god-mode and non-god-mode
  examples is wanted. The residual defect is that
  `crates/nova_probe/src/capture.rs:574-584` force-heals every `Health`
  UNCONDITIONALLY, so a scenario measuring death/despawn cost is
  unconstructible. Make it a per-EXAMPLE flag - note it cannot live in
  `CATEGORY_POLICIES`, which is keyed by category, and the driver is selected by
  `NOVA_PERF_COMBAT` (`scene_baseline.rs:95`).
- Promoting `nova_autopilot` out of nova. Per-game autopilot until the design is
  understood well enough to promote. Nova's is already the better design
  (predicate-driven vs bcs's frame-driven).

## Close-out - step 1: the torn `timeline.jsonl`

**What.** `ProbeTimeline::create` no longer truncates first and asks questions
later. It opens the path non-truncating, takes an EXCLUSIVE advisory file lock
(`File::try_lock`, std since 1.89), and only then `set_len(0)`. A second
recorder aiming at a live path gets `WouldBlock` and is refused with a message
naming the path. The plugin's arm-failure log moves `warn!` -> `error!`.

**Why.** The old `File::create` truncated to offset 0 while an earlier
recorder's `BufWriter` kept writing at its own offset, splicing two streams
into one line - the `malformed timeline line 143` failure recorded on
`20260804-174231` and in `20260804-094021`'s review. Steps 4-6 make the
timeline the SOLE verdict, so a silently torn artifact is the worst possible
foundation.

The log-level change is the second half of the fix: a refused recorder writes
nothing, and probe's `reached_playing` / `run_completed` return `Skipped` on a
missing timeline, which today folds into a passing verdict. `log_clean` greps
ANSI-stripped whole-word `ERROR`, so `error!` turns "probe asked for a timeline
and did not get one" into a run FAILURE instead of a thinner OK. That is the
same "green and wrong" class this parent task exists to close.

**Alternatives.** (a) A process-global registry of held paths - the fix
suggested on `20260804-174231`. Rejected: bevy already rejects a duplicate
unique plugin, so the only in-process double-arm is two `App`s, and a registry
does nothing for the two-process case that the run directory
(`probe-runs/<commit>/<example>/`, reused across invocations at one commit)
makes reachable. The file lock covers both with one mechanism and no new state.
(b) Open in append mode. Rejected: a stale timeline from a previous invocation
would silently prefix the new one; probe wants a fresh artifact per run, and
truncation under the lock gives exactly that.

**Difficulties.** The historical splice was never reproduced from first
principles - `t_real` 5.08 and 4.79 on the same `frame 168` says the two
writers had near-identical uptimes, which two sequential probe passes do not
explain. The fix is written against the MECHANISM (truncate-vs-offset), which
is proven regardless of which pair of writers raced, rather than against a
reproduction that six recorded runs could not produce.

**Evidence.**

- `recorder::tests::a_second_recorder_on_one_path_is_refused_not_torn` - the
  real plugin over two real `App`s on one path. Written first, failed on
  "the second recorder is refused, not armed on a shared path", passes now.
  It also pins re-arming after the holder drops, which probe's
  directory-reuse depends on.
- `cargo test -p nova_probe --lib` - 74 passed, 0 failed.
- `cargo check -p nova_probe --tests --bins` clean; `cargo fmt --all -- --check`
  clean.
- End to end: `cargo run -p nova_probe -- run menu_scenarios` - the example
  whose timeline tore. **OK, measured 5/6**, `run_completed` PASS (`run_end` at
  frame 178), `reached_playing` PASS, `invariants_held` PASS (0 violations /
  178 frames), `log_clean` PASS. The recorder still arms and still closes its
  bracket under the lock.

**Reflection.** One run is not proof of a race fixed, and this close-out does
not claim it is - what is proven is that the tearing MECHANISM is gone by
construction and that the guard does not cost the normal path. The `warn!` ->
`error!` move was not in the step text; it is in scope because a guard that
disables the artifact silently would trade a torn timeline for a missing one,
and steps 5-6 are explicitly about `Skipped` not folding into green.

## Close-out - step 2: the leaked bcs prelude

**What.** `crates/nova_gameplay/src/lib.rs`'s prelude no longer globs
`bevy_common_systems::prelude::*`. In its place is an explicit 28-name list of
the bcs vocabulary nova's own gameplay code is written in, with a comment naming
the five harness twins that are never on it. Two consumers that were reaching
bcs THROUGH that glob now import bcs directly, because their own crate already
depends on it: `crates/nova_scenario/src/objects/area.rs` (swapped a
now-unused `use nova_gameplay::prelude::*` for
`bevy_common_systems::prelude::CommandsGameEventExt`) and
`crates/nova_assets/src/scenario/shakedown/tests/walk.rs`
(`CommandsGameEventExt, EventHandler, GameEventsPlugin`).
`examples_name_drivers_through_the_nova_harness` is deleted from
`tests/examples_smoke.rs` - its subject is gone.

**Why.** The glob dragged bcs's retired `AutopilotPlugin`, `AutopilotLoop`,
`ScreenshotPlugin`, `ScreenshotReelPlugin` and `HarnessCompletion` into every
`use nova_protocol::prelude::*`, where a bare name silently resolved to the
inert twin and booted the example dead (`20260802-183403`). A source-grep test
policed that; deleting the glob turns the same failure into a compile error, so
the grep test is dead weight rather than a safety net.

**Alternatives.** (a) Keep the glob and widen the grep test - rejected: it
polices a symptom the type system can police. (b) Push every re-exported name
down to a direct bcs import in each consuming crate - rejected for
`nova_editor`, `nova_menu` and `nova_core`, which do NOT depend on bcs; adding
the dependency to three crates to avoid 28 re-export names is a bigger boundary
change than the step asks for. The rule actually applied: a crate that already
depends on bcs imports directly; a crate that does not gets the name through
nova_gameplay's prelude.

**Difficulties.** The blast radius could not be predicted, only measured, and it
came in five compile rounds because each fix unblocked the next crate in the
dependency order: nova_gameplay (42 errors) -> nova_scenario (`fire`) ->
nova_assets + nova_editor + nova_menu -> nova_core (the bcs status-bar
helpers) -> examples (`ChaseCamera`). Trait-method errors (`fire`,
`play_sfx_volume`) do not name the trait, so each needed a lookup in the pinned
bcs tree (`v0.19.5` = `30d1bef`) to find `CommandsGameEventExt` /
`SfxCommandsExt`.

**Evidence.**
- `cargo check --workspace --all-targets` clean; `cargo fmt --all -- --check`
  clean.
- Warning set is IDENTICAL to master's (11 dead-code/unused-import warnings in
  examples, verified by running the same check in the main checkout). This step
  neither adds nor removes a warning - it unblocks `20260731-205553`, it does
  not do it.
- The deleted test's guarantee, re-proven at the type level: a throwaway
  `tests/` file doing `use nova_protocol::prelude::*` then naming
  `AutopilotPlugin` / `ScreenshotPlugin` / `ScreenshotReelPlugin` fails to
  compile with E0433/E0425 (`--features debug`). The twins are unreachable, not
  merely unused. File removed after the check.
- `cargo test -p nova_assets --lib shakedown` 16/16;
  `cargo test --test examples_smoke -- catalog every_category sections_assert`
  3/3 (the display-free survivors of the file the deleted test lived in).
- RUN, not just checked: `cargo run -p nova_probe -- run
  hull_section,menu_scenarios` - both **OK, measured 5/6**, `reached_playing`
  and `invariants_held` PASS on each. `hull_section` is the example that needed
  `ChaseCamera` back; a prelude regression would boot it inert, and inert is
  exactly what probe reports as a failure.

**Reflection.** The 28-name list is the honest shape of the coupling, not a
tidy one: nova_gameplay's prelude now visibly re-exports bcs status-bar UI
helpers that only `nova_core` wants. That is worth leaving ugly - it names a
real dependency that the glob was hiding, and it is the kind of thing a future
step can move to `nova_core` depending on bcs directly. The step's estimate
that the blast radius was unmeasurable was right, and the measurement cost five
builds; nothing outside the workspace broke, which is the useful finding.

## Close-out - step 3: one capture idiom

**What.** The screenshot reel is deleted and `shoot` is the fleet's single
capture idiom.

- `crates/nova_autopilot/src/reel.rs` and `tests/reel.rs` are gone. The
  primitive they carried moved to a new `crates/nova_autopilot/src/capture.rs`:
  `capture_window`, the `NOVA_SHOT_DIR` resolution and its unit test,
  `CAPTURE_RESOLUTION`, and a new `capturing()` reading `CAPTURE_ENV`.
  `ScreenshotReelPlugin`, `ReelBeat` and `completion::REEL` are gone with the
  driver.
- `nova_debug::harness` gains `shoot(world, path)` - capture + log, gated on
  `capturing()` - which collapses the three identical `fn shoot(world,
  capturing, path)` copies (`screenshot_flight`, `screenshot_nova_os`,
  `screenshot_combat`) and `screenshot_ui`'s closure. `nova_reel`,
  `NovaReelPlugin`, `reel_beat`, `ReelCamera` and `hide_reel_chrome` are gone;
  `reel_pose_camera -> pose_camera` and `reel_freeze_bodies -> freeze_bodies`
  (now public) survive the driver they were named for, and
  `scenario_camera_present` became a public `Predicate` builder so a script can
  hold a step on it.
- The reel's non-beat responsibilities became named, shared pieces rather than
  plugin internals: `force_capture_resolution` (collapsing FOUR copies of
  `fn force_resolution`), `hide_hud` (from `screenshot_ui`'s local copy plus
  `hide_reel_chrome`'s HUD half), `freeze_bodies`.
- `screenshot_sections.rs` and `screenshot_scene.rs` are rewritten from
  `nova_reel(beats)` onto autopilot steps - the real cost of the step. Each is
  now one script (`section_script` / `scene_script`) that waits on
  `scenario_camera_present()`, then walks present/frame -> settle -> shoot per
  shot. Both files lost `nova_autopilot()`: the ONE script is now both paths.
- `examples/ui/widget_zoo.rs` shoots through `capture_window` (queued as a
  command, since the driving system holds `Commands`), deleting its own
  `NOVA_SHOT_DIR` resolution; its capture pass moved behind `--features debug`
  because `capture_window` lives there. `examples/ui/menu_scenarios.rs` uses
  `capturing()` + `shoot`.
- **Env rename, `NOVA_REEL -> NOVA_CAPTURE`** (`REEL_ENV -> CAPTURE_ENV`), and
  the staging dir in the docs/script `target/reel -> target/shots`. Also
  `HarnessMute`'s env list (`nova_gameplay/src/settings.rs`), the wiki
  automation-harness + development pages (with a new "Capturing: one idiom"
  section), `scripts/gen-web-screenshots.py`, and the CHANGELOG's
  still-Unreleased `BCS_* -> NOVA_*` entry, amended in place rather than given a
  second breaking note.
- Out of scope but found on the way: master did not compile. Step 2's prelude
  narrowing dropped two names still used by examples, so `cargo check --features
  debug --examples` failed on `screenshot_combat` (`PointRotation`) and
  `hud_range` (`DirectionalSphereOrbitOutput`). Both added to nova_gameplay's
  explicit bcs list - step 2's own stated procedure, just missed.

**Why.** A beat list is built away from the script that produces the state each
beat frames, so timing and framing lived in different files; as steps they read
act -> frame -> shoot in source order. `NOVA_REEL` naming the capture flag was a
lie once the reel was gone, and it had never shipped (the rename entry is still
under `[Unreleased]`), so amending it costs no released consumer.

**Alternatives.** (a) Keep `REEL_ENV`'s value and rename only the const -
rejected: a const/value mismatch is worse drift than either name alone.
(b) Give scene/sections a small "capture preset" plugin carrying the resolution,
chrome and freeze wiring - rejected as a mini-reel; the three pieces are three
named functions an example adds itself, which is what makes the file readable.
(c) Make `shoot` take `capturing: bool` like the copies it replaces - rejected:
every caller passed the same expression, so the gate belongs inside.
(d) Fold `crates/nova_scenario/src/actions/view.rs`'s third `NOVA_SHOT_DIR`
resolution onto `capture_window` - **NOT DONE**, see below.

**Deliberately not done.** `crates/nova_scenario/src/actions/view.rs:70` still
resolves `NOVA_SHOT_DIR` itself. `nova_scenario` is a SHIPPING crate and does
not depend on `nova_autopilot`; folding it would put an automation-driver crate
into the game binary to save a six-line path join. The task text flags it
parenthetically as a known third copy rather than instructing the fold, and this
reading is recorded here so the reviewer can overrule it cheaply.

**Difficulties.** The reel owned four things, not one, and only the beat cadence
was dead: the window resize, the overlay/HUD hide, the body freeze and the
scene-ready gate all had to land somewhere before the driver could go. The
freeze and the gate are the load-bearing ones - without the freeze the posed set
drifts between shots, and without the gate `pose_camera` warns and poses nothing
(the step then shoots an unframed frame rather than failing). Both converted
scripts hold their first step on `and(state_is(Playing),
scenario_camera_present())` for exactly that reason.

The first headless run appeared to prove the conversion broken - `step 'wait for
the drydock scene' stalled after 30.0s, state Loading`. It was the rig, not the
code: running `./target/debug/examples/screenshot_scene` directly resolves
assets against the binary's directory, so the load never completed. Under
`cargo run` from the repo root both examples pass.

**Evidence.**
- `cargo check --workspace --features debug --all-targets` clean;
  `cargo fmt --all` clean. `cargo doc --no-deps -p nova_autopilot -p nova_debug`
  produces the SAME 5 pre-existing `nova_autopilot is both a function and a
  crate` warnings as master (counted on both trees) - no new rustdoc warning.
- `cargo test -p nova_autopilot --lib capture` 1/1;
  `--test env_contract` 1/1 (rewritten for `CAPTURE_ENV`, `REEL` collector
  dropped); `cargo test -p nova_debug --lib harness` 5/5, including the new
  `shoot_captures_nothing_when_the_run_is_not_armed`.
- RUN, not just checked (Xvfb `:99`, `cargo run --features debug`):
  - `screenshot_scene` capture path (`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`): exit 0,
    `reached Playing`, `cycle complete, no panic (t=5.6s)`, all three PNGs on
    disk at 1920x1080.
  - `screenshot_sections` capture path: exit 0, `cycle complete (t=8.1s)`, all
    five section PNGs at 1920x1080.
  - Both SMOKE paths (`NOVA_AUTOPILOT=1` alone): exit 0, `cycle complete
    (t=1.7s)` each, ZERO `nova capture` lines and the shot dir never created -
    the one script really is both paths, and the smoke path is ~3-5x shorter.
  - Framing preserved across the rewrite: the new `feature-gravity.png` is the
    shipped `web/src/assets/feature-gravity.png` framing (same camera, same hero
    placement); the differences are rock mesh/lighting from the intervening
    gravity and scenario-lighting tasks, not from this step.
- DoD proof `test ! -f crates/nova_autopilot/src/reel.rs` passes. The companion
  grep `! rg -n "ScreenshotReelPlugin|ReelBeat" crates examples` has ONE
  surviving hit, reported rather than papered over:
  `crates/nova_gameplay/src/lib.rs:73`, the comment naming the bcs harness twins
  that must never be re-exported. `bevy_common_systems` still ships that type;
  the comment is a guard on the explicit list, so it stays.
- The step-4 proof `! rg -n "if capturing" examples/screenshots` still fails, as
  expected: the per-path settle split is exactly what step 4 unifies.

**Reflection.** The step's cost estimate was right about which part was
expensive and wrong about why. The two file rewrites were mechanical once the
step shape was settled; the expensive part was inventorying what the plugin did
BESIDES walk beats, because none of it was in the plugin's name. A driver that
also owns window sizing, chrome, physics and a readiness gate does not announce
those in its type - the only way to find them was to read `build`. Worth
remembering when the next "delete the driver" step is scoped: budget for the
driver's undeclared side jobs, not for its headline behaviour.

## Close-out - step 6: the smoke suite is gone, probe is the verdict

**What.** `tests/examples_smoke.rs` is deleted and the root `tests/` directory
with it. Its five per-category smoke tests, the six example lists (`SECTIONS`,
`SYSTEMS`, `UI`, `STRESS`, `SCREENSHOTS`, `NOT_SMOKED`), `smoke()` and `tail()`
are gone. The two display-free SOURCE gates it also housed moved intact to
`crates/nova_probe/tests/catalog_drift.rs`, which derives the repo root as
`env!("CARGO_MANIFEST_DIR")/../..`:

- `catalog_matches_disk`, now disk-vs-catalog alone (the smoke-list half died
  with the lists).
- `sections_assert_their_invariant_roster`, unchanged except that it reads its
  example set from the catalog (`category == "sections"`) instead of the
  deleted `SECTIONS` const - which makes it strictly better: a new range cannot
  be added without a roster.

CI's "Examples smoke test" step is now "Probe correctness sweep":
`xvfb-run --auto-servernum cargo run -p nova_probe --features
nova-protocol/debug -- run --all --out "$RUNNER_TEMP/probe-runs"`, followed by
an `if: always()` artifact upload of the run directories.

**Why the `--features nova-protocol/debug`.** probe builds each example itself
with `--features debug`, but the probe BIN links `nova-protocol` too. Linking
it with default features would flip `bevy/track_location` back off and pay for
a second Bevy variant inside the step - exactly the cost the Tests step's
comment exists to prevent.

**Why the artifact upload.** The smoke test printed a 48 KB stderr tail into
the job log on failure; probe prints the six-row check table and leaves the
`run.log` on disk. Without the upload a red CI run would be undiagnosable.
This is a requirement of the swap, not an extra.

**Coverage.** A superset, not a trade. `probe run --all` is the whole catalog,
so it also grades `scene_baseline`, which `NOT_SMOKED` deliberately skipped
("probe owns it"), and it grades every run on six checks where the smoke suite
asserted four things by grep. `log_clean` carries the smoke suite's one unique
assertion - `"Encountered an error in command"` at ANY level - which step 5
moved into it (`de288622`).

**Two step-text items resolved differently, on the owner's ruling.** Both are
recorded in the step above rather than done silently:
`every_category_has_a_probe_policy` was already deleted in step 5, and neither
`sections_assert_their_invariant_roster` nor
`crates/nova_autopilot/tests/autopilot_example.rs` was cut - the first is the
only gate that reddens when an invariant is dropped, and the second grades
`driven_app`, which is not in the root catalog and therefore unreachable by
probe. It is a second verdict over a DIFFERENT subject.

**Difficulties.** The step text's line numbers had all moved and one of its
targets no longer existed, because steps 4 and 5 landed against the same files.
Reading the tree first rather than trusting the plan is what caught it.

**Evidence.**

- `cargo test -p nova_probe --test catalog_drift` - 2/2 pass.
- FAIL-FIRST for the moved gate, not just a green run: dropping a stray
  `examples/systems/stray_uncataloged.rs` into the tree made
  `catalog_matches_disk` FAIL naming it, and removing the file made it pass
  again. That is the one case nothing else in the toolchain catches - with
  `autoexamples = false` an uncataloged example silently does not build.
- `cargo test -p nova_probe --lib` 99/99.
- `cargo check --workspace --all-targets --features debug` clean;
  `cargo fmt --all -- --check` clean.
- The CI command shape RUN, not just written: the exact invocation against the
  `sections` category (`cargo run -p nova_probe --features
  nova-protocol/debug -- run sections --out <tmp>`) - **aggregate OK**, all
  five examples OK at `measured 5/6`, process exit code 0. `--out` under a
  fresh temp dir finds no baseline and reports `fps_within_baseline N/A`, which
  is the honest answer for a correctness gate.

**Reflection.** The full `--all` sweep has NOT been run end to end locally -
this step proved the command shape on one category (5 examples, ~4 min
wall-clock) and relies on step 5's separate `screenshots` sweep for the other
new category. The first CI run is therefore the first `--all` of the whole
catalog, and the honest expectation is that it is where the runtime budget gets
measured for real. The 60-minute job timeout is unchanged and was sized for a
comparable number of `cargo run` example spawns, so the shape should hold; if
it does not, the fix is a narrower spec, not a longer timeout.

## Close-out - step 7: camera authority sets

**What.** One nova-owned chain now declares who writes the camera `Transform`
and in what order. New module `crates/nova_gameplay/src/camera_controller/authority.rs`:

- `CameraAuthority { Solve, Additive, Override }` - the phases, exported from
  `nova_gameplay`'s camera_controller prelude.
- `CameraAuthorityPlugin` - two `configure_sets` calls and nothing else. First,
  `(CameraShakeSystems::Restore, Solve, Additive, Override).chain()
  .before(TransformSystems::Propagate)`. Second, the fold of bcs's writers into
  the phases as SET-IN-SET, not bare edges:
  `(ChaseCameraSystems::Sync, WASDCameraSystems::Sync).in_set(Solve)` and
  `CameraShakeSystems::Apply.in_set(Additive)`.
- `enforce_scripted_camera_pose` (`crates/nova_scenario/src/loader/mod.rs`)
  drops `.after(WASDCameraSystems::Sync)` for `.in_set(CameraAuthority::Override)`.
- The plugin is added by `SpaceshipCameraControllerPlugin` and by
  `ScenarioLoaderPlugin`, each behind `is_plugin_added` (the `juice.rs`
  precedent for `CameraShakePlugin`). Either crate can be the only camera
  consumer in a test app, and plugin add order between them is the app's
  business, not the plugin's.
- The two hand-written duplicates of the propagate edge are gone: the
  `camera_framing_is_speed_invariant` rig (`framing.rs`) and the
  `indicator_projects_with_the_frames_final_camera_pose` rig
  (`hud/screen_indicator.rs`) now add `CameraAuthorityPlugin` instead of
  re-declaring `ChaseCameraSystems::Sync.before(Propagate)` by hand - so a test
  exercises the production ordering rather than a copy of it.

ZERO bcs changes, as the step required.

**Why.** The camera Transform had four writers and a PARTIAL ordering lattice;
the rest was executor readiness, i.e. a per-frame coin flip. The scripted pose
was ordered only against WASD sync, but `loader/lifecycle.rs` swaps the player
onto `SpaceshipCameraController` on spawn, so the writer it actually races is
CHASE - unordered against it. That race is the flicker. Two edges were missing
outright: neither `CameraShakeSystems::Apply` nor the scripted pose had any edge
to `TransformSystems::Propagate`, so a frame could render last frame's pose.

**Alternatives.** (a) Add `.after(ChaseCameraSystems::Sync)` beside the existing
WASD edge - rejected: it fixes the one race that was noticed and leaves the
lattice partial, which is the failure mode being closed. It also does nothing
for the two propagate edges. (b) `run_if`-gate the chase/WASD writers off while
a `ScriptedCameraPose` exists - rejected under the boundary rule "order, don't
disable": a gate breaks silently when bcs adds a writer, whereas an ordered
loser still runs, still writes, and is still overwritten. (c) Put the phases in
`nova_scenario` next to the override - rejected: `nova_scenario` depends on
`nova_gameplay`, so the set would be invisible to the crate that owns the chase
camera. (d) Bare ordering edges (`Chase::Sync.before(Override)`) instead of
set-in-set - rejected: set-in-set orders a bcs writer against every phase at
once, so a phase added later needs no second edit at each writer.

**Difficulties.** The set-in-set fold has to stay consistent with the edges bcs
declares for ITSELF (`Restore.before(Chase::Sync)`,
`Apply.after(Chase::Sync)`, `WASD::Sync.before(Propagate)`); an inconsistency is
a schedule cycle, which panics at the first `PostUpdate` and only in an app that
adds all three plugins. That is what
`the_chain_composes_with_every_bcs_camera_plugin` exists to catch, and it is why
`CameraShakeSystems::Restore` sits in the chain as itself rather than inside a
fourth phase - it must precede every base writer, and bcs pins it before
`Chase::Sync` only, which is no edge at all when the chase plugin is absent.

**Evidence.**

- FAIL-FIRST, not just green: `override_wins_the_frame_against_the_chase_camera`
  drives a real `ChaseCameraPlugin` + `CameraShakePlugin` app for 8 frames with
  an `Override` writer. With `CameraAuthorityPlugin` removed and the `in_set`
  dropped it FAILS on frame 1 (`assert_failed`, the chase camera overwrote the
  pose); restored, it passes. That is the flicker, reproduced and closed in a
  unit-shaped seam.
- `the_chain_composes_with_every_bcs_camera_plugin` - all three bcs camera
  plugins plus the chain, one `update()`. Passes; a cycle would panic here.
- `cargo test -p nova_gameplay --lib camera` 33/33 (includes the two rigs whose
  hand-written edge was replaced); `--lib screen_indicator` 27/27;
  `cargo test -p nova_scenario --lib loader` 27/27.
- `cargo check --workspace --all-targets --features debug` clean, zero new
  warnings; `cargo fmt --all -- --check` clean.
- RUN, not just checked (Xvfb `:99`,
  `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 cargo run --features debug --example
  screenshot_combat`): exit 0, all 13 PNGs written, twice. The FIRST posed shot
  (`wiki-radar.png`, the one the owner reported flickering) has an IDENTICAL
  camera pose across the two runs - same framing, same ship placement; the only
  differences are asteroid positions and the fps counter, i.e. physics timing,
  not camera authority.

**Reflection.** The step's "highest value-to-effort" claim held: the fix is two
`configure_sets` calls and one `in_set` swap, and it deletes more than it adds
(two hand-copied edges in test rigs, one stale `.after` in the shipping
enforcer). The non-obvious part was not the ordering, it was WHERE the chain can
live - it needs a crate both the chase camera and the scripted override can see,
which is `nova_gameplay`, and it needs to survive being added from either side,
which is what the `is_plugin_added` guard buys.

One thing deliberately NOT changed, noted so it is not re-derived: HUD screen
indicators still run `.after(ChaseCameraSystems::Sync)`
(`hud/screen_indicator.rs`), which is now INSIDE `Solve` - so under a scripted
pose they can project against the pre-override camera. Nothing observed it, the
capture scripts hide the HUD for most shots, and re-slotting a HUD consumer is a
different change from declaring the writer order. If it ever shows, the fix is
one edge: `ScreenIndicatorSystems.after(CameraAuthority::Additive)`.

**The last two phases are the reverse of the plan, on purpose.** The step's text
above chains `Solve -> Override -> Additive`, which puts shake LAST and lets
trauma jitter a scripted pose. `cd1bff21` reverses them to
`Solve -> Additive -> Override`: the scripted pose is the frame's final write
and WINS over shake, so photo mode, the capture scripts and the cinematic
framings are steady even with combat next to the camera. That is the owner's
call and it is what `authority.rs:34-37` documents.

## Review round 1 (20260806)

Three out-of-context lanes (behavior/proofs, correctness/security,
design/standards/docs) over the 33 commits carrying this task's ID, plus bcs
`e5da687`. 23 findings; verdict REQUEST_CHANGES. Full text and per-finding
responses in `REVIEW.md`.

**The blocker was mine, and `cargo check` could not see it.** Step 8 made nova
own `Health`, but `nova_probe` kept importing
`nova_gameplay::bevy_common_systems::health::Health`. Both types exist and both
compile, so nothing failed - the two consumers just stopped matching anything.
`invariants.rs`'s health-bounds check ran over zero entities in every run
(including the CI sweep step 6 had just made the sole correctness gate), and
`capture.rs`'s `combat_burst_driver` stopped healing combatants, so its
documented "keeps every combatant alive" guarantee was silently off under every
frame-time measurement. All three lanes found it independently by reading the
type paths; no run reported anything.

The generalisable part: a type-ownership move is invisible to the compiler when
the old type still exists, and its stale consumers can live in a crate the step
never names. A cross-crate grep for the moved symbol's OLD path is the check
that would have caught it, and step 8 did not run one.

**Fixed this round.** Both blockers, and 20 of the remaining 21 findings: the
DoD proofs that could never pass as written (both were matching guard comments,
not code), four false and five missing `[Unreleased]` CHANGELOG entries
including the ram-damage typing, a dead `HealthSystems` set, ~20
rustdoc sites still crediting bcs for code nova now owns, the untested
persistence key/path derivation, two destruction-pipeline tests the port
dropped, and the stale doc surfaces (`release`/`probe` skills, two wiki pages,
the harness env table).

**Answered rather than applied.** R1.19 is refused with reasoning - the
suggested fix does not compile across the crate boundary. R1.20 is fixed, but
not the suggested way: an `## Amendment` section on the existing `DECISION.md`
rather than a second decision file, which no task in `tasks/` has. Round 2
accepted both arguments.

**Evidence.** All four `cmd:` proofs pass (two only after being re-anchored).
`cargo check --workspace --all-targets --features debug` and
`cargo fmt --all -- --check` clean. Lib suites: nova_gameplay 801 (+1 ignored,
pre-existing), nova_probe 99, nova_assets 98, nova_scenario 151, nova_menu 76,
nova_ui 21, nova_debug 12, nova_core 2, nova_autopilot 45. Probe under Xvfb
with the health invariant actually live for the first time: `stress` 4/4 OK,
`sections` 5/5 OK, zero invariant violations across 3821 checked frames (1205
`stress` + 2616 `sections`). The
new leaf-derivation test was sabotage-checked - it fails with its `try_remove`
branch deleted.

## Review round 2 (20260806)

One out-of-context reviewer re-verified round 1's 23 responses against
`261c7e71` and `427bd2bb`: 20 confirmed fixed, both pushbacks accepted (R1.19's
cross-crate visibility argument, R1.20's one-DECISION.md-per-task convention),
three partial. Seven new findings, one MAJOR. All seven are fixed; details and
per-finding responses in `REVIEW.md`.

**The MAJOR is the interesting one, and it is the round-1 blocker's second
half.** Repointing the `Health` import made the invariant correct, but
`invariants_held` still printed `PASS / 0 violations over N checked frames`
whether the query matched 17 entities or none - which is precisely how the
original bug survived a whole task. A fix that restores a check without
restoring the ability to SEE the check running leaves the trap armed.

So the check now carries a delivery guard: `InvariantState` tracks
`health_subjects` and `velocity_subjects` (per-run peaks), they ride on the
`invariant_summary` timeline entry, and `invariants_held` reports them in its
`data` and its detail line. Reported, never gated - a UI example with no ships
legitimately has zero subjects, so a gate would be noise, but a zero is now
visible instead of indistinguishable from a clean pass. The paired test asserts
both the populated and the empty case.

The live numbers are the proof the blocker is actually fixed, not just
recompiled - `probe run sections`, per example: controller 4, thruster 4, hull
6, turret 10, torpedo 17 health subjects. Before `261c7e71` every one of those
was 0 and the check still said PASS.

**Two of my round-1 responses overclaimed** and are corrected in place in
`REVIEW.md`. R1.14 said "a repo-wide grep for `examples_smoke` is now empty" and
R1.9 said the bcs-credit sweep "also caught the stale function names" - both
were true of `crates/` and false of `examples/`, which I never swept. Round 2
found the four leftovers. A sweep that claims repo-wide scope should be recorded
as the command that produced it.

One round-2 finding also corrected something I had told the owner directly:
the CHANGELOG claimed ram damage now differs per section, and I had flagged that
to them as a play-test risk. It is false - `damage.rs:110` is
`(_, Kinetic) => 1.0` for every section, by design, as the feel-preserving
reference column. Routing ram through the typed path changes no numbers today;
it makes a ram subject to the same table every other weapon meets.

**Evidence.** All four `cmd:` proofs pass. `cargo check --workspace
--all-targets --features debug` and `cargo fmt --all -- --check` clean. Lib
suites: nova_probe 100, nova_gameplay 801 (+1 ignored, pre-existing), nova_assets
98, nova_scenario 151, nova_menu 76, nova_ui 21, nova_debug 12, nova_core 2,
nova_autopilot 45. `probe run sections` 5/5 OK under Xvfb, 2573 checked frames,
zero violations, with the subject counts above.
