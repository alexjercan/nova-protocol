# Review: Examples rework - testable curriculum

- TASK: 20260712-211352
- BRANCH: refactor/examples-testability

## Round 1

- VERDICT: APPROVE (was REQUEST_CHANGES at round-1 filing; all seven
  findings verified fixed in a4d725b - see the close-out at the bottom)

- [x] R1.1 (MAJOR) docs/development.md:15 (plus README.md:52, AGENTS.md:44,
  docs/architecture.md:79, docs/development.md:93+97,
  crates/nova_debug/src/harness.rs:52+92) - the checked-off "sweep all old
  example names (sweep-then-delete)" step is incomplete. Live references to
  pre-rework names remain in current docs and comments: the everyday-command
  blocks in README.md, AGENTS.md and docs/development.md all still say
  `cargo run --example 03_scenario` (a command that now errors);
  docs/architecture.md points at `examples/03_scenario.rs`; the harness
  module docs name 03_scenario twice; and the bug-pin section that this very
  branch ADDED to development.md names `13_menu_newgame` and
  `11_com_range`/`12_hud_range` - old numbering, where `12_hud_range` no
  longer exists and `11_hud_range`/`12_menu_newgame` now denote different
  examples than those names used to. Suggested change: replace 03_scenario
  with 08_scenario in the three command blocks, architecture.md and
  harness.rs; in development.md's bug-pin section use 12_menu_newgame and
  07_com_range/11_hud_range. (Historical records - old CHANGELOG entries,
  closed tasks, docs/plans, bevy-0.19-migration.md - are fine as-is.)
  - Response: fixed - all seven sites swept (README, AGENTS.md,
    architecture.md, development.md quick-start + bug-pin section,
    harness.rs module docs); the bug-pin section now names 12_menu_newgame
    and 07_com_range/11_hud_range.

- [x] R1.2 (MINOR) docs/development.md:82 - "Every example is HARNESSED ...
  carries at least one behavior assertion that panics on failure (not just
  'reached gameplay'), plus a backstop that fails the run if the script never
  completed" overstates the current set (TASK.md's Record repeats the claim).
  Counterexamples: 09_editor's placement check only `warn!`s on failure
  (examples/09_editor.rs:192) and has no completion backstop - a script that
  stalls at CreateShip still exits green; 06_torpedo_guidance has
  `assert_scenario_loaded` but no fired/detonated outcome assertion and no
  backstop; 12_menu_newgame has no explicit backstop (defensible: reaching
  Playing IS its completion criterion, and the panic error handler is the
  pin). NOTES.md's audit table is honest about all three ("unchanged").
  Suggested change: either scope the docs claim ("except 06/09, tracked as
  follow-up") or bring 06/09 up to the stated contract (a RangeOutcome-style
  detonation assert for 06; promote 09's warn to a panic plus a backstop).
  - Response: fixed - development.md now says ten of twelve carry
    panic-on-failure assertions with backstops and names 06/09 as
    load/reach-gameplay level; strengthening 06/09 is left to the
    curriculum's next pass rather than this branch.

- [x] R1.3 (MINOR) examples/01_controller_section.rs:18,
  examples/02_thruster_section.rs:18-22, examples/03_hull_section.rs:30,
  examples/10_playable.rs:38 - `cargo check --workspace --all-targets`
  (default features) now emits 5 new warnings that master does not have:
  unused `avian3d::prelude` (01, 02), unused `ExtendedMaterial` +
  `ThrusterExhaustMaterial` (02), dead `PARTIAL_HIT` (03), dead `WINDOW_SECS`
  (10). All are used only inside `#[cfg(feature = "debug")]` code. CI's
  clippy runs with `--features debug` so it stays quiet, but the default
  check is now noisy. Suggested change: gate the debug-only imports/consts
  with `#[cfg(feature = "debug")]`.
  - Response: fixed - the debug-only imports/consts in 01/02/03/10 are
    cfg-gated; default-feature check is warning-free again (only the
    pre-existing proc-macro-error2 future-incompat note remains, as on
    master).

- [x] R1.4 (MINOR) examples/10_playable.rs:19 - the header's "look for" line
  says `playable: prey destroyed, beacon locked, waypoint reached`, but the
  script emits `playable: prey destroyed, waypoint locked, GOTO closing at
  ...` (line 421). Anyone verifying a headless run by the header's grep line
  will conclude it failed. Suggested change: update the doc line to the real
  message (it also better reflects the rescoped headless contract).
  - Response: fixed - the header's look-for line now matches the emitted
    message.

- [x] R1.5 (MINOR) tasks/20260710-143138/TASK.md:8+59,
  tasks/20260713-203709/TASK.md:9, docs/plans/20260713-v0.5.2-plan.md:50,
  .github/workflows/ci.yaml:86 - open follow-up work still speaks the old
  names: the CI-taffy task (which this task's Notes say depends on THIS
  branch's final set) pins its containment to "03_scenario"; 20260713-203709
  points at examples/13_menu_newgame.rs; the v0.5.2 plan names 03_scenario.
  And ci.yaml's comment now asserts as fact that "08_scenario panics inside
  taffy" on GitHub runners - the observed repro was against the OLD
   03_scenario; the rebuilt 08_scenario has not demonstrated it. Suggested
  change: refresh the two OPEN tasks' example names, and hedge the ci.yaml
  comment ("the pre-rework 03_scenario panicked ...; re-verify against
  08_scenario when re-enabling the gate").
  - Response: fixed - both open tasks, the v0.5.2 plan and the ci.yaml
    comment now say the panic was observed on the OLD 03_scenario and must
    be re-verified against the rebuilt 08_scenario.

- [x] R1.6 (NIT) examples/11_hud_range.rs:771 - the velocity-sphere assert
  message literal contains a 14-space run mid-sentence ("does              not
  follow"), a wrapped string rejoined without the `\` continuation the file
  uses elsewhere (cf. line 537). Suggested change: add the `\` continuation.
  - Response: fixed - the assert message is a single-spaced string again.

- [x] R1.7 (NIT) tasks/20260713-220512/TASK.md:17 - "Numbers and the full
  timeline are in tasks/20260712-211352's history (the probe2 log)" points at
  a temp probe that was never checked in (no probe2 remnants exist in the
  tree - correctly swept). The load-bearing numbers are already inline in the
  Goal. Suggested change: drop or reword the dangling pointer so a future
  session does not hunt for a log that is not there.
  - Response: fixed - the dangling probe2-log pointer is replaced by the
    inline numbers.

## Verification (reviewer-run, this worktree)

- `cargo fmt --check`: PASS.
- `cargo check --workspace --all-targets`: PASS (exit 0; 5 new warnings, see
  R1.3).
- Full smoke suite under Xvfb (`cargo test -p nova-protocol --test
  examples_smoke --features debug`, DISPLAY=:95): PASS, 1 test (all 12
  examples sequentially), 122.67s - matches the Record's ~130 s claim.
- Smoke list vs disk: HARNESSED_EXAMPLES names exactly the 12 on-disk
  examples (12 .rs files + the 04_turret_section/ slider module dir).
- Honesty spot-checks all held up: the fire-dead-ranges reasoning matches
  crates/nova_gameplay/src/input/player.rs (On<Start> latch + cold-press
  deny) and camera_controller.rs/targeting.rs (WeaponsHot derived from the
  HELD combat input, so wait-for-hot-then-press is the correct gesture); the
  radar pick is indeed purely angular (targeting.rs:756 scores by cos to the
  look ray only); the asteroid id-on-root/health-on-child hierarchy matches
  crates/nova_scenario/src/objects/asteroid.rs (08_scenario damages the
  health child, 04/05 tag the AsteroidMarker node, which is the health
  carrier); 20260713-220512 exists and is coherent.
- Backstop margins are sound: probes read the same virtual-time clock the
  autopilot uses (bevy_common_systems autopilot adds capped `Time` deltas,
  max 0.25 s/frame, and runs the input closure before the exit check), so a
  0.3-0.5 s margin cannot be jumped over in one frame and cannot fire on a
  run that would otherwise succeed. 01/02/03/08/10 check the backstop before
  the Playing gate; 04/05 assert behind the gate, but a never-Playing run
  still fails the suite on the missing "reached Playing" line.
- Assertion strength spot-checks: 01's frozen-hull case is excluded by the
  swept > 0.6 rad delivery guard (tolerance 0.35 rad vs 0.18 measured lag,
  worst-case one-frame skew ~0.09 rad at llvmpipe frame rates - sane); 02
  cannot pass vacuously (baseline-relative speed growth; plume default
  thruster_input is 0.0, so the == 1.0 check requires the production sync);
  03's exact-drop arithmetic, despawn check and root/controller counts each
  fail on a broken pipeline stage; 08 asserts every grammar layer end-state
  (seed, tally == 2, beat == 3); 10's beats are all event-driven and the
  rescoped "GOTO engaged and closing" contract is honestly documented in the
  example, TASK.md and NOTES.md (the unexercised OnEnter arrival is called
  out and left for interactive runs).

## Round 1 close-out (reviewer verification of a4d725b)

All seven findings re-verified against the a4d725b diff and ticked above.

- R1.1: all seven stale-name sites confirmed fixed (README.md, AGENTS.md,
  docs/architecture.md, development.md quick-start AND bug-pin section -
  now 12_menu_newgame + 07_com_range/11_hud_range - and both harness.rs
  doc-comment sites). No 03_scenario/old-numbering references remain
  outside historical records.
- R1.2: development.md now scopes the contract honestly ("ten of the
  twelve ... 06_torpedo_guidance and 09_editor assert at the
  scenario-load / reach-gameplay level only"). Accepted as filed; note for
  the record that TASK.md's Record still carries the blanket phrasing,
  which this REVIEW and the corrected development.md now supersede, and
  that strengthening 06/09 was deferred without a tracked follow-up task -
  the scoped docs sentence is the durable, accurate contract either way.
- R1.3: re-ran in this worktree: `cargo fmt --check` PASS; `cargo check
  --examples` (default features) exit 0 and warning-free bar the
  pre-existing proc-macro-error2 future-incompat note (matches master);
  `cargo check --examples --features debug` exit 0, clean. The gated
  imports narrow to exactly the probe-used items (Rotation;
  LinearVelocity/Rotation + ExtendedMaterial + ThrusterExhaustMaterial).
- R1.4: the header look-for line now matches the emitted
  "playable: prey destroyed, waypoint locked, GOTO closing at ..." message.
- R1.5: both OPEN tasks, the v0.5.2 plan and the ci.yaml comment now
  attribute the taffy panic to the pre-rework 03_scenario with an explicit
  re-verify note against the rebuilt 08_scenario.
- R1.6/R1.7: whitespace run gone; the dangling probe2-log pointer replaced
  with the inline measured timeline.

Smoke suite deliberately not re-run for this round: a4d725b touches only
comments/docs, cfg-gating of debug-only imports/consts (a no-op under the
`--features debug` build the suite uses, confirmed by the clean debug
check) and one assert-message string. The round-1 full-suite PASS
(12/12, 122.67s) stands as the behavioral evidence.

- ROUND 1 VERDICT (final): APPROVE
