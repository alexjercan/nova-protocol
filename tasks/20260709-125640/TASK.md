# Residual roll after autopilot release: PD cannot damp fast roll (bcs)

- STATUS: CLOSED
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

## Diagnosis (2026-07-11, tick trace via diag_residual_roll_after_release)

The corkscrew is a period-2 bang-bang limit cycle in the discrete PD, not a
torque-direction bug: post-release the spin sits at a constant 1.414 rad/s
and FLIPS SIGN EVERY TICK. The PD output is saturated at max_torque (100 in
the test rig) and correctly opposes the previous tick's spin, but one tick's
impulse (max_torque * dt / I_roll = 100 / 0.5 / 64 = 3.1 rad/s) overshoots
the 1.4 rad/s spin through zero to the mirrored state; the frozen ~0.4 rad
attitude error keeps the demand pinned at the clamp, so the cycle sustains
itself indefinitely. bcs's gains are discretely unstable at this tuning
(kd * dt = 72/64 > 1). The shipped max_torque 40 sustains a ~0.6 rad/s
cycle (40 * dt / I = 1.25 = 2 * 0.625) - the slow in-game corkscrew. This
also explains evidence anomaly 3: the extra -spin*40 despin torque pumped
the cycle to a higher amplitude instead of despinning. Fix lives in bcs as
backward-Euler gain conditioning: bcs task 20260711-094942.

## Steps

- [x] Land the frame-order fix in bevy-common-systems (its task
      20260711-091519, landed as bcs 13e33e5). Its crate-level repro showed
      the corkscrew does NOT reproduce at the crate boundary with default
      tuning, which pointed back at the release scenario specifics; the
      tick trace above then identified the real mechanism.
- [x] bcs task 20260711-094942 closed PREMISE-FALSIFIED: the limit cycle
      was a symptom of the frame-order bug, not discrete instability. The
      decisive A/B (cargo path patch to the fixed bcs) killed the
      corkscrew entirely: the flip stays planar and the ship parks at
      0.000 rad/s. Mechanism detail: avian's eigen sort returns
      principal = (2.5, 0.5, 2.5) with local_frame =
      Quat(0.5, 0.5, 0.5, 0.5) - a cyclic axis permutation - even for
      this axis-aligned ship, so the old L * R composition mangled the
      tensor for every off-axis rotation.
- [x] Removed the temporary diag_residual_roll_after_release diagnostic
      from crates/nova_gameplay/src/flight.rs.
- [x] Bump the pinned `bevy_common_systems` rev in
      crates/nova_debug/Cargo.toml, crates/nova_events/Cargo.toml,
      crates/nova_gameplay/Cargo.toml and crates/nova_scenario/Cargo.toml
      (all four pin 4c58835708feb888f3a1872e74d6ae5fd742dd0c today), then
      `cargo update -p bevy_common_systems` to refresh the lockfile.
      Requires the bcs fix pushed to GitHub first.
- [x] Tighten the regression guard in
      crates/nova_gameplay/src/flight.rs::high_speed_stop_settles_without_tumbling
      from `spin < 2.0` to `spin < 0.5` and rewrite its comment (the
      "known issue, filed as its own task" note no longer applies).
- [x] Re-check the two evidence anomalies against the fixed PD: confirm a
      hull released at 1.5 rad/s now despins in the flight harness. If the
      direct-despin sign surprise (`apply_torque(-spin * 40)` grew the
      spin) turns out to be application-side in nova rather than explained
      by the bcs fix, file it as its own task rather than widening this
      one.
- [x] Run `cargo check` + `cargo fmt --check` and the touched flight tests
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

## Outcome (2026-07-11)

What changed in nova: the pinned bevy_common_systems rev in the four crate
Cargo.tomls moved 4c58835 -> a35b74c (frame-order fix 13e33e5 + coverage
d9e13e1), the high_speed_stop_settles_without_tumbling release guard
tightened from spin < 2.0 to < 0.5 (measured: the release now parks at
0.000 rad/s, so 0.5 has margin for timing jitter), and its comment now
records the real mechanism. The temporary diagnostic test used for the
tick trace was removed after serving its purpose.

Anomaly follow-ups from the original evidence:

- "Pure damper did not decay": explained - the pre-fix tensor was mangled
  through the cyclic-permutation local frame, and only pure-roll-with-
  roll-rotation states (a commuting subspace) escaped it; the general
  post-maneuver states did not.
- "apply_torque(-spin * 40) made the spin GROW": consistent with adding
  torque on top of the mangled-PD flip-flop, which pumped the cycle. Not
  reproducible on the fixed rev (the state it required no longer arises);
  no separate nova-side task filed.

Also tightened (review R1.1) the AI settle test's residual-roll bound in
crates/nova_gameplay/src/input/ai.rs from 0.5 to 0.05: the rig now
measures ~5e-6 rad/s where the pre-fix roll was ~0.23.

Verification: cargo check, cargo fmt --check, and the full flight module
(55 tests) green on the bumped rev, including the tightened guard. Full
suite runs in CI per the local-test policy.

Difficulties: two wrong theories died on the way - see the two bcs task
outcomes (20260711-091519: the crate-level repro that refused to
reproduce; 20260711-094942: the backward-Euler premise falsified by the
path-patch A/B). The tick-trace instrumentation pattern (ignored #[test]
printing per-tick spin / command error / PD output) was what actually
cracked it.

Reflection: the fix was one line in the dependency; almost all the value
of this task was diagnostic discipline. The one avoidable detour: after
the bcs frame fix landed, the next experiment should have been re-running
the nova trace against it (5-minute cargo path patch) BEFORE reading the
trace for new mechanisms - recorded as the lesson of bcs retro
20260711-094942.
