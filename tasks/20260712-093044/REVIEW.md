# Review: Nav beacon and salvage crate scenario objects

- TASK: 20260712-093044
- BRANCH: shakedown-run (commit 5031480 vs master)

## Round 1

- VERDICT: APPROVE (fresh-context agent review; findings below are
  non-blocking)

- [x] R1.1 (MINOR) crates/nova_scenario/src/actions.rs:459 - the despawn
  loop calls `world.entity_mut(entity).despawn()` on collected matches;
  `entity_mut` panics on a stale entity, reachable if two matches are ever
  ancestor/descendant (the first recursive despawn takes the second).
  Unreachable today (scenario objects spawn as roots) but one line of
  defensiveness: `if let Ok(e) = world.get_entity_mut(entity) {
  e.despawn(); }`.
  - Response: fixed in the round-1 follow-up commit (get_entity_mut +
    graceful skip).
- [ ] R1.2 (NIT) crates/nova_gameplay/src/hud/beacon_chips.rs:127 - the
  chip chevron re-implements edge_indicators' private `edge_arrow` at chip
  scale; a parameterized chevron builder in screen_indicator.rs would
  dedupe. Optional.
  - Response: deliberate for now (the builder is worth doing when a third
    consumer appears); left as-is.
- [ ] R1.3 (NIT) crates/nova_gameplay/src/hud/beacon_chips.rs:94 - one
  full-screen layer node per beacon vs edge_indicators' single shared
  layer. Harmless (Pickable::IGNORE) and makes per-beacon despawn
  trivial; noted, not requested.
  - Response: keeping per-beacon layers for the trivial lifecycle.

Verified clean by the reviewer (independent re-derivation): the
scope-restriction claim (ship sections DO carry EntityId,
spaceship.rs:100); the arrow-drive claim (update_arrows walks descendants
and drives the chip chevron); targeting-gate safety (workspace grep: only
the beacon combines Static + LockSignature, so nothing else becomes
lockable; the regression test's point-blank unsigned case exercises the
gate, not the range check); observer timing (single-bundle insert means
Add observers see the full config); HUD lifecycle (Remove fires on
despawn; Chrome tier honored); tests are real (delivery guard present);
conventions (no non-ASCII; patterns match siblings); docs and TASK.md
honest. Tests observed: 8/8 new tests pass, check + fmt clean.
