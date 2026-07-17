# Review: flip remaining player scenarios to finite auto-reloading ammo

- TASK: 20260717-101414
- BRANCH: feature/scenarios-finite-ammo

## Round 1

- VERDICT: APPROVE

Verified independently (implementer == reviewer):

- **No player scenario left on infinite ammo.** A full-tree grep for
  `infinite_ammo: true` returns exactly one hit: `nova_scenario/src/loader.rs:1930`,
  inside the `a_scenario_config_round_trips_through_ron` serialization test - a
  test/debug fixture, correctly kept per the user's "infinite stays for
  testing/debug." Broadside (RON + Rust builder) and the example mod are the only
  player scenarios that had it, and both are flipped.
- **No softlock risk.** Both flipped scenarios' players fire
  `better_turret_section`, which carries auto-reload (task 20260717-085640), so a
  spent magazine recovers. Confirmed the prototype reference in each RON.
- **Parity holds.** `content_ron_parity` 8/8 - the Broadside Rust builder and RON
  now agree on `infinite_ammo: false` (a mismatch would have failed here).
  `content_lint_gate` 2/2 (both RON files still parse).
- **The inverted assertion is meaningful.** `broadside_assault` asserts
  `!player_controller.infinite_ammo`; it passes because Broadside flipped and
  would fail if it were still infinite - a real guard, not a rubber stamp. 2/2.
- **Docs in sync.** CHANGELOG (Scenarios & Objectives) and NOTES.md record what
  flipped and what was kept; the player wiki's existing "Ammo & reloading" line
  ("some tutorial or sandbox ships fly with unlimited ammo") remains accurate.

No BLOCKER/MAJOR/MINOR findings. One thing for the user's attention (not a
defect): flipping the **example mod** arena is a judgment call - it is a modding
showcase, not a combat mission. It reads "more real" and demonstrates a mod
inheriting reload; if it should stay a no-pressure sandbox, revert only that one
line (noted in NOTES.md).
