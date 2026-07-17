# Flip remaining player scenarios off infinite_ammo to finite auto-reloading ammo

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.7.0,scenarios,weapons

## Goal

Now that weapons auto-reload (task 20260717-085640), finite ammo is
non-terminal, so the remaining player-facing scenarios that still set
`infinite_ammo: true` can run real ammo and read "more real" without risking a
softlock. Flip them; keep `infinite_ammo` as a mechanism for testing/debug
(user direction 2026-07-17). Shakedown Run was already flipped in the mechanic
task; this does the rest.

## Steps

- [x] Flip Broadside (chapter two, a combat scenario) to finite: set
  `infinite_ammo: false` in `assets/base/scenarios/broadside.content.ron` and
  the Rust builder `crates/nova_assets/src/scenario/broadside.rs`.
- [x] Invert the Broadside guard test: `crates/nova_assets/tests/broadside_assault.rs`
  asserts `player_controller.infinite_ammo` ("chapter-one precedent"); the
  precedent (Shakedown) is now finite, so assert `!infinite_ammo` and update the
  message.
- [x] Flip the example mod arena to finite:
  `assets/mods/example/example.content.ron` `infinite_ammo: false` (it inherits
  the base catalog turret's auto-reload, so it recovers).
- [x] Leave `infinite_ammo` for testing/debug: the round-trip test fixture
  `crates/nova_scenario/src/loader.rs` (inside
  `a_scenario_config_round_trips_through_ron`) keeps `infinite_ammo: true` as an
  arbitrary serialization value - it is not a player scenario. The flag,
  `PlayerControllerConfig.infinite_ammo`, and the ammo-strip path stay intact.
- [x] Run `content_ron_parity` (compares the Rust builder to the RON) after the
  Broadside flip; run the broadside_assault test.
- [x] Docs: CHANGELOG line (Broadside + the example mod now fly real,
  auto-reloading ammo); note in tasks/20260717-101414/NOTES.md which scenarios
  flipped and which infinite_ammo uses were deliberately kept.

## Notes

- Depends on: 20260717-085640 (auto-reload) - CLOSED.
- Related: 20260716-123556 (readout reload-state) - the flipped scenarios now
  show the diegetic gauge + reload sweep instead of no gauge.
- Decision surfaced: the example mod is a modding showcase; flipping it to
  finite makes it "more real" and demonstrates mods inheriting reload. If it is
  meant to stay a no-pressure sandbox, revert just that one line.

## Implementation record

Landed on branch `feature/scenarios-finite-ammo`. Flipped Broadside (RON + Rust
builder) and the example mod arena to `infinite_ammo: false`; inverted the
Broadside guard test. Both players fire `better_turret_section`, which
auto-reloads, so they recover rather than dying dry. Kept `infinite_ammo: true`
only in the loader serialization round-trip test fixture (test/debug, per user
direction); the flag and ammo-strip path are untouched. Details +
kept/flipped inventory: tasks/20260717-101414/NOTES.md.

Verification (per standing skip-local-full-suite instruction): `content_ron_parity`
8/8, `content_lint_gate` 2/2, `broadside_assault` 2/2 (inverted assertion, would
fail if Broadside were still infinite); CHANGELOG updated. Full suite in CI.
