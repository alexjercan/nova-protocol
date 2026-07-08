# Review: Minimal example for nova_gameplay crate

- TASK: 20260525-133009
- BRANCH: feat/gameplay-minimal-example

## Round 1

- VERDICT: APPROVE

Diff adds `examples/10_gameplay.rs` and registers it in `examples_smoke`. Example-only.

Verified:

- Delivers the task exactly: a ship with sections (controller + hull + thruster + turret,
  assembled inline via `SpaceshipConfig`), health (aggregate 500/500 read out), and one weapon
  (the turret). Building the ship inline is the right call - it documents nova_gameplay usage,
  which `03_scenario` (load-a-named-scenario) does not.
- The mechanics actually run end to end, confirmed headless: `reached Playing`, the turret
  auto-aims the nearest asteroid and clears `3 -> 2 -> 1 -> 0`, ship health holds at 500/500,
  `cycle complete, no panic`. So it exercises sections + health + weapon + collision damage +
  the integrity destroy pipeline in one place.
- The spawn pitfall was caught and fixed, not shipped. The first run had all three asteroids
  self-destruct on spawn (impact damage ~250 between the two close pairs, while the far pair was
  untouched) - the oversized asteroid collider vs its nominal radius. Spacing them ~30 units
  apart fixes it, and the diagnosis (with the collider/radius mismatch flagged as a possible
  separate bug) is recorded in TASK.md.
- Harness wiring matches the other examples (env-gated autopilot/screenshot, `autopilot_fire`
  holds Space in Playing); `aim_turret_at_nearest_asteroid` runs `after(SpaceshipInputSystems)`
  like `08`'s range aim; the readout is guarded to skip until the ship exists (no misleading
  `0/0` pre-spawn line).
- Green: `cargo clippy --workspace --all-targets` clean in both feature modes (only the pre-
  existing `hull_section.rs` warning); `cargo test --workspace` with `examples_smoke` running
  `10_gameplay` (63.9s for six examples under Xvfb).

Scope is honest: `examples_smoke` asserts clean-exit, not the specific asteroid-count decline
(the run is deterministic in practice with the static ship + fixed targets). Thrust/flight is
present as a section but not driven - reasonable for a "minimal" example.

No BLOCKER/MAJOR/MINOR findings.
