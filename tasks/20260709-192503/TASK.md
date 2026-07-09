# Hybrid lock acquisition: aim cone + signature-range proximity

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.4.0,targeting,gameplay,spike


Spike: docs/spikes/20260709-192358-component-lock-vats-lite.md

Extend `update_spaceship_target_input` (input/player.rs): keep the instant
aim-cone pick; when the cone finds nothing, auto-acquire the nearest
`AISpaceshipMarker` ship root within a shorter SIGNATURE_RANGE (heat-signature
close-range lock; start ~500-600 m vs the 2000 m cone range). Minimal hostile
definition = AI ships until the faction model (20260708-203708) lands.
Consider the mechanical rename of `SpaceshipPlayerTorpedoTargetEntity` to a
general target-lock name here (three systems consume it after this arc).

## Steps

- [ ] Extract targeting into `crates/nova_gameplay/src/input/targeting.rs`:
      move `update_spaceship_target_input`, `pick_target`, the TARGETING_*
      constants and the lock resource there from player.rs (mechanical move,
      registered from the same plugin; player.rs is ~970 lines and the arc
      adds focus/component state next to this code).
- [ ] Rename `SpaceshipPlayerTorpedoTargetEntity` ->
      `SpaceshipPlayerTargetLock` across the workspace (torpedoes, HUD
      reticle/readout driver, examples): after this arc three systems consume
      it, so the torpedo-specific name lies. Keep the resource semantics
      identical.
- [ ] Add the signature fallback in the acquisition system: when the cone
      pick returns None, lock the nearest `AISpaceshipMarker` ship root
      within `TARGETING_SIGNATURE_RANGE` (new const, start 550.0 m) of the
      ship's live-structure anchor (150711 helper). Pure helper
      `pick_signature_target(origin, max_range, candidates) -> Option<Entity>`
      so the rule is unit-testable; hostile = AI ships until the faction
      model (20260708-203708).
- [ ] Tests: cone pick still wins when both would match; fallback picks the
      nearest hostile in range; asteroids/torpedoes/controller-less ships are
      never signature-acquired; out of range -> no lock. Adapt the existing
      pick_target tests to the move/rename.
- [ ] Verify: cargo fmt, cargo check --workspace, new + touched targeting
      tests only (report skips). examples/12_hud_range.rs keeps passing
      unchanged - its target ship is controller-less, so it exercises the
      cone path (signature acquisition is covered by the world tests; noted
      honestly).

## Notes

- Depends on: 20260709-150711 (anchor helper for the fallback origin).
- The rename touches many files; do it as its own commit on the branch so
  the review can see the mechanical change separately from the behavior
  change.
