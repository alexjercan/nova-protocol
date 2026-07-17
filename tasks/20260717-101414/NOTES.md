# Notes: flip remaining player scenarios to finite auto-reloading ammo

## Flipped to finite (`infinite_ammo: false`)

- **Broadside** (chapter two combat scenario): `broadside.content.ron` and the
  Rust builder `crates/nova_assets/src/scenario/broadside.rs`. Its guard test
  `broadside_assault.rs` was inverted (was "infinite turret ammo (chapter-one
  precedent)"; the precedent - Shakedown - is now finite).
- **Example mod arena** (`assets/mods/example/example.content.ron`): the shipped
  worked-RON example and a playable arena. Flipping it makes it "more real" and
  demonstrates that a mod inherits the base catalog's auto-reload.

Both players fire the `better_turret_section` prototype, which carries
auto-reload (task 20260717-085640, ~3s reload-to-full), so a spent magazine
recovers - no softlock from the flip. Verified the section source in each RON.

## Deliberately kept `infinite_ammo: true` (testing/debug, per user direction)

- `crates/nova_scenario/src/loader.rs` inside
  `a_scenario_config_round_trips_through_ron`: an arbitrary value in a
  serialization round-trip test, not a player scenario.

The `infinite_ammo` flag, `PlayerControllerConfig.infinite_ammo`, and the
ammo-strip build path are all untouched - unlimited ammo stays available to
authors for testing/debug ships.

## Verification

`content_ron_parity` 8/8 (Broadside Rust builder and RON now agree on
`infinite_ammo: false`), `content_lint_gate` 2/2 (both RON files still parse),
`broadside_assault` 2/2 (the inverted assertion passes and would fail if
Broadside were still infinite). CHANGELOG updated.

## Open decision

The example-mod flip is a judgment call (it is a modding showcase, not a combat
mission). If it should stay a no-pressure sandbox, revert only that one line in
`example.content.ron`.
