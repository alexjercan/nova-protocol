# Remove deep mod-content behavior tests from core CI

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.7.0, testing, refactor

## Goal

Core CI stops deep-testing mod CONTENT. Delete the two per-mod behavior
tests; first preserve any ENGINE mechanics coverage they uniquely carry
as synthetic-fixture engine tests. Decision from audit task
20260716-141620 (FINDINGS.md section 2), user confirmed.

## Steps

- [x] Coverage audit: list the engine mechanics
      `crates/nova_assets/tests/gauntlet_race.rs` (OnEnter gate handler
      registration/firing, OnStart event seeding) and
      `crates/nova_assets/tests/arena_combat.rs` (per-id OnDestroyed
      counting gated by a one-shot OnUpdate) exercise, versus existing
      engine tests: nova_scenario area.rs has an OnEnter-fires test
      (:112-193), nova_scenario owns the OnDestroyed root-bridge test
      (`destroying_an_asteroid_node_fires_on_destroyed_for_the_root`),
      loader.rs has handler-registration tests. Record the delta in this
      file before deleting anything.
- [x] For each mechanic that would lose its only coverage, add an engine
      test with a SYNTHETIC inline scenario (no shipped mod data) in the
      crate owning the mechanic (expected: nova_scenario events/loader).
      Respect rig-supplies-precondition: when pinning OnStart mechanics,
      the loader must drive OnStart, not the rig hand-seeding state.
- [x] Delete `crates/nova_assets/tests/gauntlet_race.rs` and
      `crates/nova_assets/tests/arena_combat.rs`.
- [x] Sweep-then-delete: grep the repo for the deleted file/test names in
      comments, docs and CI config (historical RETRO/TASK files stay
      untouched).
- [x] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      the new synthetic tests and webmods_validation (full suite is CI).

## Notes

- What still gates mod content generically (unchanged):
  webmods_validation.rs loads every webmods bundle recursively;
  demo_scenario.rs keeps loading the demo mod through the catalog.
- Policy for the retro/close notes: no per-mod behavior tests for future
  mods; content correctness is the mod author's concern, load-ability is
  the pipeline's.

## Coverage-audit delta (recorded before deletion)

- Already engine-pinned, unaffected: the area->body OnEnter contact bridge
  (nova_scenario/src/objects/area.rs
  `an_area_spawned_around_a_body_fires_on_enter`) and the destroy->OnDestroyed
  root bridge (asteroid.rs
  `destroying_an_asteroid_node_fires_on_destroyed_for_the_root`).
- Uniquely exercised by the deleted tests, now pinned synthetically in
  nova_scenario/src/filters.rs (3 new tests through the real
  GameEventsPlugin dispatch, handlers built exactly as the loader builds
  them): (1) a handler's filters must ALL pass - entity id AND expression
  guard, held in both directions with an unfiltered delivery probe;
  (2) an expression over an undefined variable fails closed (the
  soft-lock-not-misfire property gauntlet_race documented); (3)
  `n = n + 1` VariableSet re-evaluates against the current value across
  events (the kill-counter pattern).
- Deliberately dropped WITH the files (content assertions, not engine):
  gauntlet's gates-are-sequential-in-the-data, gauntlet's OnStart
  spawns-player+seeds-gate structural pin, the arena's one-shot win
  wiring. Load-ability of both mods stays gated generically
  (webmods_validation.rs; demo_scenario.rs catalog pipeline).

## Close notes (2026-07-16)

What changed: deleted gauntlet_race.rs and arena_combat.rs (per-mod
behavior tests); added the three synthetic filter/action machinery tests
in nova_scenario/src/filters.rs (which had zero tests despite owning the
semantics); updated broadside_assault.rs's header (cited arena_combat.rs
as its division-of-labor referent; now states the base-vs-mod coverage
policy directly).

Policy recorded: BASE story content keeps deep behavior tests in core CI
(broadside_assault.rs stays); MOD content gets the generic load gate
only - no per-mod behavior tests for future mods.

Verification: new tests 3/3 green (run paired `-p nova_scenario -p
nova_menu` per the crate-solo unification lesson - solo -p nova_scenario
does NOT compile, reconfirmed), check --all-targets + fmt green,
webmods_validation green. Full suite is CI's job per the standing
instruction.

Reflection: the coverage audit found the real gap was not the deleted
tests' assertions but that filters.rs never had its own tests - the
content tests had been standing in for engine coverage. Deleting without
the audit would have shipped a silent coverage hole.
