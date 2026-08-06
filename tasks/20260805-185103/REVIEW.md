# Review: Cleanup and maintenance: close the engine gaps the screenshot pipeline routed around

- TASK: 20260805-185103
- BRANCH: task/20260805-185103

## Round 1

- REVIEWER: out-of-context (three lanes: behavior/proofs, correctness/security,
  design/standards/docs)
- VERDICT: REQUEST_CHANGES

Scope: the 33 commits carrying this task's ID between `cafae048` and
`task/20260805-185103`, over 112 files, plus bcs `e5da687`.

- [x] R1.1 (BLOCKER) crates/nova_probe/src/invariants.rs:44 - imports
  `nova_gameplay::bevy_common_systems::health::Health`, but step 8 (`5f67c75a`)
  moved the game onto `nova_gameplay::integrity::health::Health` and nothing
  spawns the bcs type any more. The health-bounds invariant at `:216-218`
  queries a component no live entity carries, so `invariants_held` silently
  stopped checking health in every run - including the CI sweep step 6 made the
  sole correctness gate. Repoint the import at
  `nova_gameplay::integrity::health::Health`.
  - Response: fixed in this round's commit - `invariants.rs:44` now imports `nova_gameplay::prelude::Health`, the same path the game's own code resolves. The module doc's "bcs's `on_damage` clamps" line was stale in the same way and now names nova's.
- [x] R1.2 (BLOCKER) crates/nova_probe/src/capture.rs:25 - the same stale bcs
  `Health`. `combat_burst_driver`'s force-heal at `:581` matches nothing, so its
  documented "keeps every combatant alive" guarantee is dead: a combatant can
  die inside the measurement window and the burst fizzles, changing every
  `fps_within_baseline` number the sweep grades. Repoint the import and correct
  the now-false comment at `:22`.
  - Response: fixed in this round's commit - `capture.rs` imports `Health` from `nova_gameplay::prelude` alongside the other three names, and the comment that justified the old path is replaced by one naming the failure it caused.
- [x] R1.3 (MAJOR) crates/nova_probe/src/invariants.rs:400 - the health-bounds
  tests (also `:415`, `:419`, `:423`, `:565`, `:590`, `:623`) spawn the bcs
  `Health` themselves, so they pass against a type the game no longer uses and
  cannot fail while R1.1 stands. Repoint the spawns so the test and the
  production query name one type.
  - Response: fixed by R1.1 - the tests are `use super::*`, so repointing the module's import moved the spawns onto nova's `Health` with the production query. There is now exactly one `Health` path in the crate, so the two cannot diverge again.
- [x] R1.4 (MAJOR) tasks/20260805-185103/TASK.md:419 - the DoD proof
  `! rg -n "pub use bevy_common_systems::prelude" crates` exits 1: it matches
  the explicit named re-export at `crates/nova_gameplay/src/lib.rs:77`. The
  intent (no glob) is delivered, but the proof as written can never pass.
  Narrow it to `! rg -n 'pub use bevy_common_systems::prelude::\*' crates`.
  - Response: fixed in this round's commit - the proof is now `! rg -n '^\s*pub use bevy_common_systems::prelude::\*' crates`. Anchored to line start because the guard comment at `lib.rs:70` quotes the glob form deliberately. Runs clean.
- [ ] R1.5 (MAJOR) CHANGELOG.md:123 - three `[Unreleased]` entries are false
  after steps 4.1 and 5: `CategoryPolicy { probed, frame_time }`, "`--all`
  skips unprobed categories and records each as excluded" and "a bare
  `probe run screenshots` now errors" (`af4e2c16`, `cf28c543`), plus `:131`'s
  `fps_exempt` -> `fps_skipped` rename naming a field 4.1 deleted, and `:152`'s
  "`widget_zoo` joined the CI smoke list" after step 6 deleted the list. Amend
  all four in place, as step 3 did for the `BCS_*` -> `NOVA_*` entry.
  - Response: fixed in this round's commit - the `CategoryPolicy` entry is rewritten in place as the coverage HANDSHAKE it became, absorbing the `fps_exempt`/`fps_skipped` entry (both fields are gone, so a rename entry for them was doubly false), and "`widget_zoo` joined the CI smoke list" is dropped with the list.
- [x] R1.6 (MAJOR) CHANGELOG.md:15 - no `[Unreleased]` entry exists for steps
  4.1, 5, 6, 7 or 8. Add one line each for the runtime coverage contract and
  the `UNPROBEABLE` verdict, the smoke-suite deletion and CI probe sweep
  **(breaking)**, the camera-authority chain, the nova-owned health/integrity
  pipeline, the single persistence store, and the deletion of
  `AppBuilder::with_main_menu` and `nova_ui`'s `debug` feature. Ram/impact
  damage now routes through `DamageType::Kinetic` and meets the per-section
  resistance table (`crates/nova_gameplay/src/integrity/core.rs:151`) - a
  player-visible balance change that owes its own line.
  - Response: fixed in this round's commit - five entries added under Internals & Tooling for the smoke-suite deletion **(breaking)**, the camera-authority chain, the nova-owned health/integrity pipeline including the ram-damage balance change, the single persistence store with its "saved data survives" note, and the `with_main_menu` / `nova_ui debug` deletions **(breaking)**.
- [x] R1.7 (MAJOR) tasks/20260805-185103/TASK.md:861 - step 7's record quotes
  the shipped chain as `(CameraShakeSystems::Restore, Solve, Override,
  Additive).chain()`; `crates/nova_gameplay/src/camera_controller/authority.rs:61-66`
  ships `Restore, Solve, Additive, Override`, reversed by `cd1bff21` - an
  in-range code commit that carries no task ID and appears on no step's
  `commits:` line. Correct the order in the record and add `cd1bff21` to step
  7's commits.
  - Response: fixed in this round's commit, record-only. The shipped order is the owner's deliberate change in `cd1bff21`: the scripted pose is the frame's LAST write and wins over shake, so a posed shot is steady - which is what `authority.rs:34-37` documents and what the code does. The step-7 record now quotes `Restore, Solve, Additive, Override`, `cd1bff21` is on step 7's commits line, and a close-out paragraph records that the last two phases are the reverse of the step's plan text and why.
- [x] R1.8 (MAJOR) crates/nova_gameplay/src/integrity/health.rs:71 - the
  `HealthSystems` set is declared, exported at `:21`, and applied to nothing:
  `NovaHealthPlugin::build` registers types and one observer, and no system in
  the workspace orders against it. Delete the enum and its prelude entry.
  - Response: fixed in this round's commit - `HealthSystems` and its prelude entry are deleted. It was never applied to a system nor ordered against.
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
  - Response: fixed in this round's commit - every site rewritten across `lib.rs`, `damage.rs`, `integrity/glue.rs`, `sections/turret_section/firing.rs`, `sections/torpedo_section/projectile.rs` and `input/ai/threat.rs`, and both tests renamed (`neutralized_bullet_mass_makes_the_emergent_kinetic_negligible`, `the_health_store_subtracts_exactly_the_prescaled_amount`). The sweep also caught stale bcs function names (`handle_destroy`, `handle_parent_destroy`) and a set of blast comments whose premise no longer holds at all: bcs's `IntegrityPlugin` is not added anywhere, so there is no bcs blast observer to avoid double-counting against. The bcs mentions left in `crates/` are real ones - the PD controller, the harness twins, `DespawnEntityPlugin`. CORRECTED after round 2: the sweep stopped at `crates/` and missed one in `examples/`. See R2.5.
- [x] R1.10 (MINOR) tasks/20260805-185103/TASK.md:421 - the reel DoD proof exits
  1 on the deliberate guard comment at `crates/nova_gameplay/src/lib.rs:73`
  naming `ScreenshotReelPlugin`. Step 3's close-out disclosed the comment but
  left the proof unfixed. Anchor the proof to code, e.g. match
  `use .*ScreenshotReelPlugin|add_plugins\(.*ScreenshotReelPlugin`.
  - Response: fixed in this round's commit. Note for the next proof author: the obvious `(use|add_plugins\()` form breaks `tatr proofs` parsing - its `(cmd: ...)` extraction stops at the first unbalanced paren and the proof silently vanishes from the list. The shipped form is paren-free.
- [x] R1.11 (MINOR) crates/nova_assets/src/persist.rs:56 - `config_path` and
  `storage_key` are private and untested, so the byte-compatibility with the two
  stores this replaced survives only as a doc comment; a typo there silently
  orphans every player's saved mods and settings with a green suite. Make both
  `pub(crate)` and assert `config_path("enabled_mods")` ends in
  `nova-protocol/enabled_mods.ron` and `storage_key("settings") ==
  "nova_protocol.settings"`.
  - Response: fixed in this round's commit - `config_path` is `pub(super)`, `storage_key` is hoisted out of the wasm backend so it compiles (and is tested) on native, and `the_storage_locations_match_the_stores_this_replaced` pins all four literals.
- [x] R1.12 (MINOR) crates/nova_gameplay/src/integrity/health.rs:131 - the port
  dropped bcs's `a_lethal_hit_still_bubbles_to_zero_a_matching_parent`, the
  invariant that the new overkill clamp does not block a fatal bubble when
  parent HP equals child HP. It survives only downstream in `glue.rs`'s
  ghost-ship tests. Add the two-entity case beside the other three.
  - Response: fixed in this round's commit.
- [x] R1.13 (MINOR) crates/nova_gameplay/src/integrity/core.rs:258 -
  `derive_integrity_leaves`' `try_remove::<IntegrityLeafMarker>` branch is never
  driven from an inserted state; bcs's `leaves_are_derived_from_the_connection_count`
  was not ported. Add a test that a 1-neighbour node gains the marker and loses
  it on a 2nd neighbour.
  - Response: fixed in this round's commit - `a_leaf_that_gains_a_second_neighbour_stops_being_one`. Sabotage-checked: deleting the `try_remove` branch fails it.
- [ ] R1.14 (MINOR) .claude/skills/release/SKILL.md:56 - tells the operator to
  confirm CI's "windowed `examples_smoke` run under Xvfb"; step 6 replaced that
  step with the probe correctness sweep (`.github/workflows/ci.yaml:101`).
  `.claude/skills/probe/SKILL.md:186` likewise cites
  `sections_assert_their_invariant_roster (tests/examples_smoke.rs)`, now
  `crates/nova_probe/tests/catalog_drift.rs:171`. Fix both strings.
  - Response: fixed in this round's commit - both strings updated; CORRECTED after round 2: I claimed a repo-wide grep was empty, but I had only swept `.claude/`, `web/`, `README.md` and `AGENTS.md` - `examples/` still had two mentions. See R2.2.
- [x] R1.15 (MINOR) web/src/wiki/dev/architecture.md:93 - documents
  `with_main_menu(bool)` as the menu override; `4de50263` deleted it. Rewrite
  against the surviving `use_default_plugins` path.
  - Response: fixed in this round's commit.
- [x] R1.16 (MINOR) web/src/wiki/dev/scenario-system.md:161 - "the
  screenshot-reel example drives exactly this"; the reel is deleted. Point it at
  the `pose_camera` + settle + `shoot` idiom the automation-harness page
  documents.
  - Response: fixed in this round's commit.
- [x] R1.17 (MINOR) crates/nova_probe/src/contract.rs:27 -
  `NOVA_PERF_CONTRACT` is a new harness variable on no doc surface, while
  `web/src/wiki/dev/automation-harness.md:45` claims "The table above is the
  whole contract". Add a row.
  - Response: fixed in this round's commit. The suggested row was wrong on one detail and the shipped row corrects it: `NOVA_PERF_CONTRACT` names a FILE path (probe passes `probe-contract.json`), not a directory.
- [x] R1.18 (MINOR) crates/nova_probe/src/contract.rs:100 -
  `ProbeContract::iter` and `is_empty` (`:107`) have no caller outside this
  file; `iter` is reached only by the test at `:224` and `is_empty`'s own
  docstring says it is "only reachable for a contract built by hand". Delete
  both.
  - Response: fixed in this round's commit - both deleted. The accumulation test now asserts through `declares` plus `to_json`, which keeps the enum-order and no-duplicate claims pinned without a public accessor that only a test reached.
- [x] R1.19 (MINOR) crates/nova_assets/src/persist.rs:125 - the public
  path-taking `load_from` / `save_to` have no production caller; their only
  users are `crates/nova_menu/src/settings_store.rs`'s tests, which can call
  `backend::load_from` / `backend::save_to`. The owner's ruling asked for
  load/write access rather than a plugin, which the key-taking `load` / `save`
  already deliver. Delete the path-taking pair.
  - Response: Pushback. The suggested fix does not compile: `backend` is private to `nova_assets`, and the only round-trip tests live in `nova_menu::settings_store`, a different crate. The path-taking pair is that crate's only seam for testing a store without touching the real config dir. Kept, and now exercised by R1.11's test too.
- [x] R1.20 (MINOR) tasks/20260805-185103/DECISION.md - three owner rulings in
  step 8 reverse this record on load-bearing points: bcs `integrity` and
  `ui/health_display` do NOT move and nova re-implements instead; persistence
  is nova's own store rather than a swap onto bcs `PersistPlugin`; bcs gets
  commits but no tag and no pin bump. They live only as prose inside a step.
  Add a superseding decision record for the duplicate-capability choice, with
  the supersede link on both records.
  - Response: Fixed differently, and the difference is deliberate. `tatr` scaffolds one DECISION.md per task and no task in `tasks/` has two, so a second file would invent a convention rather than follow one. The three rulings are recorded as an `## Amendment` section on the existing record instead - they change THIS decision's answer, not a later one, so a reader who finds the original also finds the reversal. Contents: why integrity is re-implemented rather than moved (bcs `examples/15_integrity.rs` is a live consumer, so the move was impossible as planned) and what the port deleted, why the persistence store is not bcs's plugin, and the commit-but-do-not-tag rule for bcs.
- [x] R1.21 (NIT) crates/nova_debug/src/harness.rs:115 -
  `NOVA_SCREENSHOT_SETTLE_FRAMES = 30` and `SETTLE_FRAMES = 30` (`:126`) are two
  constants of one value and near-identical meaning, against step 4's "ONE
  scene-settle value". Point `nova_screenshot()` (`:242`) at `SETTLE_FRAMES` and
  delete the older const.
  - Response: fixed in this round's commit - `NOVA_SCREENSHOT_SETTLE_FRAMES` deleted, `nova_screenshot()` reads `SETTLE_FRAMES`.
- [x] R1.22 (NIT) crates/nova_gameplay/src/integrity/health.rs:224 -
  `destructible_body` is a public bundle fn sitting after the `#[cfg(test)] mod
  tests` block, and its docs are about the integrity graph rather than the HP
  pool. Move it above the tests or into `integrity/components.rs`.
  - Response: fixed in this round's commit - moved above the `#[cfg(test)]` block.
- [x] R1.23 (NIT) bevy-common-systems src/modding/events.rs:309 - the reflow
  left a ~120-column line in a file wrapped at ~80. Re-wrap.
  - Response: fixed in bevy-common-systems `6f09461`. Unpushed and untagged, per the owner's ruling; nova's pin stays at `v0.19.5`.

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

## Round 2

- REVIEWER: out-of-context (single reviewer, verifying round 1's responses)
- VERDICT: REQUEST_CHANGES

Round 1's 23 findings re-verified against `261c7e71` and `427bd2bb`: 20
confirmed fixed, 2 pushbacks accepted (R1.19, R1.20), 3 partial - R1.5, R1.9
and R1.14 each left residue, carried below as R2.2 through R2.5. Those three
stay unticked until round 3 confirms the residue is gone.

Two of my round-1 responses overclaimed and are corrected in place: R1.14's
"the grep is now empty" and R1.9's "the sweep caught them" were both true of
`crates/` and false of `examples/`.

- [ ] R2.1 (MAJOR) crates/nova_probe/src/invariants.rs:216 - the health-bounds
  check still reports `PASS / 0 violations over N checked frames` when its query
  matches nothing, which is exactly how R1.1 hid for a whole task; the fix
  repointed the type but added no delivery guard. Record the matched-entity
  count per invariant and emit it in `checks.json`'s `invariants_held` data, so
  an empty query is visible instead of reading as a clean gate.
  - Response: fixed in this round's commit. `InvariantState` gains
    `health_subjects` and `velocity_subjects` (per-run peaks), the
    `invariant_summary` timeline entry carries both, and `invariants_held` puts
    them in its `data` and its detail line ("peak subjects: N health, M
    velocity"). Reported, never gated - a UI example with no ships legitimately
    has zero, so a gate would be noise. Pinned by
    `subject_peaks_separate_a_bound_that_held_from_a_query_that_matched_nothing`,
    which asserts both the populated and the empty case.
- [ ] R2.2 (MINOR) examples/ui/menu_scenarios.rs:389 and
  examples/screenshots/shared/kit.rs:10 - both still cite
  `tests/examples_smoke.rs`, so R1.14's response claim is false. Repoint both.
  - Response: fixed in this round's commit. `kit.rs` points at
    `crates/nova_probe/tests/catalog_drift.rs`; `menu_scenarios.rs` describes
    what the probe sweep grades, since the sentence was about coverage rather
    than about a file. A grep across `crates/`, `examples/`, `web/`, `.claude/`,
    `README.md` and `AGENTS.md` now returns one hit, `catalog_drift.rs:3`, which
    names the deleted file correctly as history.
- [ ] R2.3 (MINOR) CHANGELOG.md:174 - "ramming a Hull and ramming a Turret no
  longer deal the same damage" is false: `damage.rs:110` is `(_, Kinetic) => 1.0`
  for every section, so typed ram damage is identical across sections. Drop the
  clause.
  - Response: fixed in this round's commit, and this one was worth catching -
    the false clause had also reached the owner as a claimed play-test risk. The
    entry now says impact damage routes through the typed path, that no numbers
    move today because Kinetic is the 1.0 reference column, and that a ram is
    now subject to the same table as every other weapon.
- [ ] R2.4 (MINOR) CHANGELOG.md:162 - "named by its two emitters and the test
  that greps for it": no test greps `REACHED_PLAYING` any more. Drop the clause.
  - Response: fixed in this round's commit - the entry ends at "its two
    emitters". The grepping test went with the smoke suite in step 6.
- [ ] R2.5 (NIT) examples/sections/hull_section.rs:371 - names bcs's
  `handle_destroy`; nova's is `destroy_a_disabled_leaf`. Rename in the comment.
  - Response: fixed in this round's commit.
- [ ] R2.6 (NIT) tasks/20260805-185103/TASK.md:983 - "20 of the remaining 21
  findings" counts R1.20 as fixed while the next paragraph lists it under "Not
  fixed, and why". Make it consistent.
  - Response: fixed in this round's commit - the count stands at 20 and the
    paragraph is retitled "Answered rather than applied", which is what R1.19
    and R1.20 actually are.
- [ ] R2.7 (NIT) tasks/20260805-185103/TASK.md:1003 - "2616 checked frames" is
  the `sections` total alone; `stress` adds 1205. Use 3821 or attribute 2616.
  - Response: fixed in this round's commit - 3821, with the split named. Both
    figures re-derived from the run logs.

Process signal: R1.14's and R1.9's responses both claimed a completed sweep that
stopped at `crates/`. A doc sweep that claims repo-wide scope should be recorded
as the command that produced it, not as prose.

Out of scope: `crates/nova_probe/tests/catalog_drift.rs:3` and the released
CHANGELOG entry naming `examples_smoke` are correct history, not stale mentions.
