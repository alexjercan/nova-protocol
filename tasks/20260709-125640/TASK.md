# Residual roll after autopilot release: PD cannot damp fast roll (bcs)

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.4.0,handling,bug,bcs


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

## Notes

- In-game impact today: after a hard STOP/GOTO the ship may slowly corkscrew
  until the player moves the mouse; mid-maneuver behavior is clean.
- Fix location: ~/personal/bevy-common-systems (pd_controller.rs), then bump
  the git rev in nova's Cargo.toml. A per-axis torque clamp (preserving the
  damping direction) is the leading candidate.
- No Steps on purpose: needs a minimal repro first (/plan when picked up).
