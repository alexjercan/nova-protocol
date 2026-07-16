# Remove deep mod-content behavior tests from core CI

- STATUS: OPEN
- PRIORITY: 80
- TAGS: testing, refactor

## Goal

Core CI stops deep-testing mod CONTENT. Delete the two per-mod behavior
tests; first preserve any ENGINE mechanics coverage they uniquely carry
as synthetic-fixture engine tests. Decision from audit task
20260716-141620 (FINDINGS.md section 2), user confirmed.

## Steps

- [ ] Coverage audit: list the engine mechanics
      `crates/nova_assets/tests/gauntlet_race.rs` (OnEnter gate handler
      registration/firing, OnStart event seeding) and
      `crates/nova_assets/tests/arena_combat.rs` (per-id OnDestroyed
      counting gated by a one-shot OnUpdate) exercise, versus existing
      engine tests: nova_scenario area.rs has an OnEnter-fires test
      (:112-193), nova_scenario owns the OnDestroyed root-bridge test
      (`destroying_an_asteroid_node_fires_on_destroyed_for_the_root`),
      loader.rs has handler-registration tests. Record the delta in this
      file before deleting anything.
- [ ] For each mechanic that would lose its only coverage, add an engine
      test with a SYNTHETIC inline scenario (no shipped mod data) in the
      crate owning the mechanic (expected: nova_scenario events/loader).
      Respect rig-supplies-precondition: when pinning OnStart mechanics,
      the loader must drive OnStart, not the rig hand-seeding state.
- [ ] Delete `crates/nova_assets/tests/gauntlet_race.rs` and
      `crates/nova_assets/tests/arena_combat.rs`.
- [ ] Sweep-then-delete: grep the repo for the deleted file/test names in
      comments, docs and CI config (historical RETRO/TASK files stay
      untouched).
- [ ] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      the new synthetic tests and webmods_validation (full suite is CI).

## Notes

- What still gates mod content generically (unchanged):
  webmods_validation.rs loads every webmods bundle recursively;
  demo_scenario.rs keeps loading the demo mod through the catalog.
- Policy for the retro/close notes: no per-mod behavior tests for future
  mods; content correctness is the mod author's concern, load-ability is
  the pipeline's.
