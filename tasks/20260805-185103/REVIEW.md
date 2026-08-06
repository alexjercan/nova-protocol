# Review: Cleanup and maintenance: close the engine gaps the screenshot pipeline routed around

- TASK: 20260805-185103
- BRANCH: task/20260805-185103

## Round 1

- REVIEWER: out-of-context (three lanes: behavior/proofs, correctness/security,
  design/standards/docs)
- VERDICT: REQUEST_CHANGES

Scope: the 33 commits carrying this task's ID between `cafae048` and
`task/20260805-185103`, over 112 files, plus bcs `e5da687`.

- [ ] R1.1 (BLOCKER) crates/nova_probe/src/invariants.rs:44 - imports
  `nova_gameplay::bevy_common_systems::health::Health`, but step 8 (`5f67c75a`)
  moved the game onto `nova_gameplay::integrity::health::Health` and nothing
  spawns the bcs type any more. The health-bounds invariant at `:216-218`
  queries a component no live entity carries, so `invariants_held` silently
  stopped checking health in every run - including the CI sweep step 6 made the
  sole correctness gate. Repoint the import at
  `nova_gameplay::integrity::health::Health`.
- [ ] R1.2 (BLOCKER) crates/nova_probe/src/capture.rs:25 - the same stale bcs
  `Health`. `combat_burst_driver`'s force-heal at `:581` matches nothing, so its
  documented "keeps every combatant alive" guarantee is dead: a combatant can
  die inside the measurement window and the burst fizzles, changing every
  `fps_within_baseline` number the sweep grades. Repoint the import and correct
  the now-false comment at `:22`.
- [ ] R1.3 (MAJOR) crates/nova_probe/src/invariants.rs:400 - the health-bounds
  tests (also `:415`, `:419`, `:423`, `:565`, `:590`, `:623`) spawn the bcs
  `Health` themselves, so they pass against a type the game no longer uses and
  cannot fail while R1.1 stands. Repoint the spawns so the test and the
  production query name one type.
- [ ] R1.4 (MAJOR) tasks/20260805-185103/TASK.md:419 - the DoD proof
  `! rg -n "pub use bevy_common_systems::prelude" crates` exits 1: it matches
  the explicit named re-export at `crates/nova_gameplay/src/lib.rs:77`. The
  intent (no glob) is delivered, but the proof as written can never pass.
  Narrow it to `! rg -n 'pub use bevy_common_systems::prelude::\*' crates`.
- [ ] R1.5 (MAJOR) CHANGELOG.md:123 - three `[Unreleased]` entries are false
  after steps 4.1 and 5: `CategoryPolicy { probed, frame_time }`, "`--all`
  skips unprobed categories and records each as excluded" and "a bare
  `probe run screenshots` now errors" (`af4e2c16`, `cf28c543`), plus `:131`'s
  `fps_exempt` -> `fps_skipped` rename naming a field 4.1 deleted, and `:152`'s
  "`widget_zoo` joined the CI smoke list" after step 6 deleted the list. Amend
  all four in place, as step 3 did for the `BCS_*` -> `NOVA_*` entry.
- [ ] R1.6 (MAJOR) CHANGELOG.md:15 - no `[Unreleased]` entry exists for steps
  4.1, 5, 6, 7 or 8. Add one line each for the runtime coverage contract and
  the `UNPROBEABLE` verdict, the smoke-suite deletion and CI probe sweep
  **(breaking)**, the camera-authority chain, the nova-owned health/integrity
  pipeline, the single persistence store, and the deletion of
  `AppBuilder::with_main_menu` and `nova_ui`'s `debug` feature. Ram/impact
  damage now routes through `DamageType::Kinetic` and meets the per-section
  resistance table (`crates/nova_gameplay/src/integrity/core.rs:151`) - a
  player-visible balance change that owes its own line.
- [ ] R1.7 (MAJOR) tasks/20260805-185103/TASK.md:861 - step 7's record quotes
  the shipped chain as `(CameraShakeSystems::Restore, Solve, Override,
  Additive).chain()`; `crates/nova_gameplay/src/camera_controller/authority.rs:61-66`
  ships `Restore, Solve, Additive, Override`, reversed by `cd1bff21` - an
  in-range code commit that carries no task ID and appears on no step's
  `commits:` line. Correct the order in the record and add `cd1bff21` to step
  7's commits.
- [ ] R1.8 (MAJOR) crates/nova_gameplay/src/integrity/health.rs:71 - the
  `HealthSystems` set is declared, exported at `:21`, and applied to nothing:
  `NovaHealthPlugin::build` registers types and one observer, and no system in
  the workspace orders against it. Delete the enum and its prelude entry.
- [ ] R1.9 (MAJOR) crates/nova_gameplay/src/lib.rs:9 - the run took ownership of
  health and integrity but left the rustdoc crediting bcs for nova's own code:
  the crate doc still says nova "builds on the reusable `bevy_common_systems`
  layer for integrity, health"; `integrity/glue.rs:119,125` attribute
  `on_health_depleted_insert_disabled` and `on_damage` to bcs though they are
  `core.rs:170` and `health.rs:103`; `damage.rs:169` says "bcs's `on_damage`
  then subtracts". Two tests assert on nova code while naming bcs -
  `damage.rs:493`'s `bcs_subtracts_the_prescaled_amount_nova_triggers` and
  `neutralized_bullet_mass_makes_bcs_emergent_kinetic_negligible`. Rewrite each
  site to name the nova module and rename both tests.
- [ ] R1.10 (MINOR) tasks/20260805-185103/TASK.md:421 - the reel DoD proof exits
  1 on the deliberate guard comment at `crates/nova_gameplay/src/lib.rs:73`
  naming `ScreenshotReelPlugin`. Step 3's close-out disclosed the comment but
  left the proof unfixed. Anchor the proof to code, e.g. match
  `use .*ScreenshotReelPlugin|add_plugins\(.*ScreenshotReelPlugin`.
- [ ] R1.11 (MINOR) crates/nova_assets/src/persist.rs:56 - `config_path` and
  `storage_key` are private and untested, so the byte-compatibility with the two
  stores this replaced survives only as a doc comment; a typo there silently
  orphans every player's saved mods and settings with a green suite. Make both
  `pub(crate)` and assert `config_path("enabled_mods")` ends in
  `nova-protocol/enabled_mods.ron` and `storage_key("settings") ==
  "nova_protocol.settings"`.
- [ ] R1.12 (MINOR) crates/nova_gameplay/src/integrity/health.rs:131 - the port
  dropped bcs's `a_lethal_hit_still_bubbles_to_zero_a_matching_parent`, the
  invariant that the new overkill clamp does not block a fatal bubble when
  parent HP equals child HP. It survives only downstream in `glue.rs`'s
  ghost-ship tests. Add the two-entity case beside the other three.
- [ ] R1.13 (MINOR) crates/nova_gameplay/src/integrity/core.rs:258 -
  `derive_integrity_leaves`' `try_remove::<IntegrityLeafMarker>` branch is never
  driven from an inserted state; bcs's `leaves_are_derived_from_the_connection_count`
  was not ported. Add a test that a 1-neighbour node gains the marker and loses
  it on a 2nd neighbour.
- [ ] R1.14 (MINOR) .claude/skills/release/SKILL.md:56 - tells the operator to
  confirm CI's "windowed `examples_smoke` run under Xvfb"; step 6 replaced that
  step with the probe correctness sweep (`.github/workflows/ci.yaml:101`).
  `.claude/skills/probe/SKILL.md:186` likewise cites
  `sections_assert_their_invariant_roster (tests/examples_smoke.rs)`, now
  `crates/nova_probe/tests/catalog_drift.rs:171`. Fix both strings.
- [ ] R1.15 (MINOR) web/src/wiki/dev/architecture.md:93 - documents
  `with_main_menu(bool)` as the menu override; `4de50263` deleted it. Rewrite
  against the surviving `use_default_plugins` path.
- [ ] R1.16 (MINOR) web/src/wiki/dev/scenario-system.md:161 - "the
  screenshot-reel example drives exactly this"; the reel is deleted. Point it at
  the `pose_camera` + settle + `shoot` idiom the automation-harness page
  documents.
- [ ] R1.17 (MINOR) crates/nova_probe/src/contract.rs:27 -
  `NOVA_PERF_CONTRACT` is a new harness variable on no doc surface, while
  `web/src/wiki/dev/automation-harness.md:45` claims "The table above is the
  whole contract". Add a row.
- [ ] R1.18 (MINOR) crates/nova_probe/src/contract.rs:100 -
  `ProbeContract::iter` and `is_empty` (`:107`) have no caller outside this
  file; `iter` is reached only by the test at `:224` and `is_empty`'s own
  docstring says it is "only reachable for a contract built by hand". Delete
  both.
- [ ] R1.19 (MINOR) crates/nova_assets/src/persist.rs:125 - the public
  path-taking `load_from` / `save_to` have no production caller; their only
  users are `crates/nova_menu/src/settings_store.rs`'s tests, which can call
  `backend::load_from` / `backend::save_to`. The owner's ruling asked for
  load/write access rather than a plugin, which the key-taking `load` / `save`
  already deliver. Delete the path-taking pair.
- [ ] R1.20 (MINOR) tasks/20260805-185103/DECISION.md - three owner rulings in
  step 8 reverse this record on load-bearing points: bcs `integrity` and
  `ui/health_display` do NOT move and nova re-implements instead; persistence
  is nova's own store rather than a swap onto bcs `PersistPlugin`; bcs gets
  commits but no tag and no pin bump. They live only as prose inside a step.
  Add a superseding decision record for the duplicate-capability choice, with
  the supersede link on both records.
- [ ] R1.21 (NIT) crates/nova_debug/src/harness.rs:115 -
  `NOVA_SCREENSHOT_SETTLE_FRAMES = 30` and `SETTLE_FRAMES = 30` (`:126`) are two
  constants of one value and near-identical meaning, against step 4's "ONE
  scene-settle value". Point `nova_screenshot()` (`:242`) at `SETTLE_FRAMES` and
  delete the older const.
- [ ] R1.22 (NIT) crates/nova_gameplay/src/integrity/health.rs:224 -
  `destructible_body` is a public bundle fn sitting after the `#[cfg(test)] mod
  tests` block, and its docs are about the integrity graph rather than the HP
  pool. Move it above the tests or into `integrity/components.rs`.
- [ ] R1.23 (NIT) bevy-common-systems src/modding/events.rs:309 - the reflow
  left a ~120-column line in a file wrapped at ~80. Re-wrap.

Process signal: the bcs-`Health` -> nova-`Health` move is invisible to
`cargo check` (both types exist and both compile) and its two stale consumers
live in a crate the step did not name. A cross-crate grep for the moved
symbol's old path belongs in any type-ownership migration.

Process signal: four in-range commits carry no task scope - `cd1bff21`,
`26bc29e0`, `2bf65b5e`, `dc02f5f9` - and only `26bc29e0` reached a `commits:`
line.

Process signal: steps 1-8 landed straight on `master`, interleaved with tasks
`20260805-105154`, `20260805-213432` and `20260806-121625`, so
`cafae048..HEAD` is 61 commits of which 33 are this task's. There is no
contiguous range to review; the scope had to be reconstructed by grepping the
task ID. DoD item 1 ("every step above has a child task") cannot be ticked as
written - no task carries `PARENT: 20260805-185103`.

Out of scope: `crates/nova_assets/src/portal/catalog.rs:155-183` carries a
third private `load_from` / `save_to` pair over the same native/wasm split the
new store owns. Step 8 named exactly two copies and this is not one of them.

Out of scope: `on_impact_collision_deal_damage` (`core.rs:124`) has no
`Without<NovaBlast>` on `q_body`, so a blast reaching the mass math as `body1`
can produce a NaN amount. Faithful to the bcs original, unchanged in behavior
by this diff, and the NaN lands on the blast's own health-less collider.

Out of scope: HUD screen indicators still run `.after(ChaseCameraSystems::Sync)`
inside `Solve`; step 7's close-out records that as deliberate.

Pending user checks (both `manual:` DoD items, not resolvable by review):

- Owner confirms the child set is complete.
- Owner watches the capture run and confirms the flicker is gone.

Re-derived by the recording pass, not taken on a lane's word: R1.1/R1.2's stale
imports and their zero-match queries; `HealthSystems` having no reference
outside its declaration; the four `cmd:` proofs (3 and 6 pass, 4 and 5 fail);
the camera chain order against `authority.rs`; `cd1bff21` absent from TASK.md;
the `contract.rs` and `persist.rs` caller sets; every stale doc string above;
and the persistence key/path derivation against both pre-diff stores, which is
byte-identical - no player data is lost.

Checks run by the recording pass: `cargo check --workspace --all-targets
--features debug` clean, `cargo fmt --all -- --check` clean, and `--lib` suites
for nova_gameplay (799 passed, 1 pre-existing ignored), nova_scenario 151,
nova_assets 97, nova_menu 76, nova_ui 21, nova_debug 12, nova_core 2. TASK.md's
recorded 798 was correct at `9b1f69f2`; `cd1bff21` added the extra test. No
`probe run` sweep was re-run, so step 5's and step 8's OK grades stand on the
implementer's record alone - and R1.1/R1.2 mean the health invariant behind
those grades was not actually being checked.
