# Residual roll after autopilot release: PD cannot damp fast roll (bcs)

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.5.0, handling, bug, bcs

## Goal

After an autopilot maneuver ends, the ship can be released with a residual
spin of ~1.5 rad/s (mostly roll about the nose) that the controller section's
PD never damps - it corkscrews indefinitely with a frozen command. Mid-
maneuver attitude is now clean (the slewed, command-anchored reference from
fix/pd-flip-wobble), so this is an endgame/release problem only. The fix
almost certainly belongs in bevy-common-systems' `compute_pd_torque` (and
possibly in how torque is applied), not in nova.

## Evidence gathered 2026-07-09 (instrumented physics tests, flight.rs)

- A hull released at ~1.5 rad/s with the command parked ON its attitude keeps
  spinning at a constant rate forever; 0.7 rad/s cases damp fine. Theory: the
  clamp `normalize(P + D) * max_torque` starves the roll component (smallest
  principal inertia scales it down before the length-normalization), so above
  some spin the P noise dominates the applied direction and net roll work is
  ~zero.
- Pure-damper experiment (command tracks the hull exactly every tick, P = 0):
  spin still did NOT decay - which contradicts the simple starving theory and
  suggests the inertia-frame sandwich in `compute_pd_torque`
  (`rot_inertia_to_world = inertia_local_frame * from_rotation`; check the
  multiplication order) or the per-axis `* inertia_principal` scaling
  mangles the damping direction for off-principal spins.
- Direct despin attempt in nova (`Forces::apply_torque(-spin * 40)`) made the
  spin GROW (1.5 -> 3.2) - either a sign/frame convention surprise in avian's
  `apply_torque` on this path or an interaction with the PD output being
  applied in the same tick. Worth a minimal avian-only repro before touching
  the PD.
- A regression guard exists: `high_speed_stop_settles_without_tumbling`
  asserts post-release spin < 2.0 rad/s; tighten it to < 0.5 when this lands.

## Steps

- [ ] Land the root-cause fix in bevy-common-systems (its task
      20260711-091519: closed-form pure-damper unit tests, an avian
      integration despin repro, and the `compute_pd_torque` fix - leading
      candidate is the inertia frame composition order at
      pd_controller.rs:136).
- [ ] Bump the pinned `bevy_common_systems` rev in
      crates/nova_debug/Cargo.toml, crates/nova_events/Cargo.toml,
      crates/nova_gameplay/Cargo.toml and crates/nova_scenario/Cargo.toml
      (all four pin 4c58835708feb888f3a1872e74d6ae5fd742dd0c today), then
      `cargo update -p bevy_common_systems` to refresh the lockfile.
      Requires the bcs fix pushed to GitHub first.
- [ ] Tighten the regression guard in
      crates/nova_gameplay/src/flight.rs::high_speed_stop_settles_without_tumbling
      from `spin < 2.0` to `spin < 0.5` and rewrite its comment (the
      "known issue, filed as its own task" note no longer applies).
- [ ] Re-check the two evidence anomalies against the fixed PD: confirm a
      hull released at 1.5 rad/s now despins in the flight harness. If the
      direct-despin sign surprise (`apply_torque(-spin * 40)` grew the
      spin) turns out to be application-side in nova rather than explained
      by the bcs fix, file it as its own task rather than widening this
      one.
- [ ] Run `cargo check` + `cargo fmt --check` and the touched flight tests
      (per the local-test policy: newly written/changed tests only; CI
      runs the full suite).

## Notes

- In-game impact today: after a hard STOP/GOTO the ship may slowly corkscrew
  until the player moves the mouse; mid-maneuver behavior is clean.
- Fix location: ~/personal/bevy-common-systems (pd_controller.rs), then bump
  the git rev in nova's Cargo.toml. A per-axis torque clamp (preserving the
  damping direction) is the leading candidate.
- Depends on: bevy-common-systems task 20260711-091519 (separate repo).
- Plan-time analysis (2026-07-11): the composition-order suspect at
  pd_controller.rs:136 is real (`inertia_local_frame * from_rotation`
  composes principal-to-world in the wrong order), but note even the wrong
  order gives a symmetric positive-definite tensor, so it cannot BY ITSELF
  explain a pure damper sustaining a spin - the bcs repro test decides what
  the actual mechanism is before the fix is written.
- Pushing the bcs fix commit to GitHub is a hard prerequisite of the rev
  bump (cargo fetches git deps from the remote).
