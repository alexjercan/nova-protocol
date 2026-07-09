# Multi-thruster autopilot: per-engine directions, fastest-path group planner

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, handling, autopilot, spike

Spike: docs/spikes/20260709-121746-multi-thruster-autopilot.md (design calls
settled with the user)

## Goal

The autopilot stops assuming the main drive points out of the nose: it
clusters the ship's live thrusters into direction groups (each section's
local `Transform.rotation` gives its thrust axis; no graph needed), scores
each group against the needed burn (`rotation_time * rotation_bias +
burn_time`), rotates the *winning group* - not the nose - onto the velocity
error, and fires every engine currently inside the alignment cone. A
retro-equipped ship brakes small overspeeds without flipping; big burns still
flip to the strongest drive; a lateral thruster kills a lateral crumb inside
the deadband instead of the residual being accepted. The GOTO arrival plan
takes both its braking authority and its lead time from the group it would
actually brake with (dynamic lead replaces the fixed `flip_lead_time`).
Torque from off-center engines is deliberately ignored in v1 (the PD fights
it; torque-aware allocation via section positions/COM is the recorded
follow-up).

## Steps

- [x] Pure helpers in `flight.rs`: `ThrusterGroup { world_dir, authority }`,
      greedy `cluster_thrusters` (existing ~25 degree cone), and
      `group_time_score` (rotation angle / est turn rate * bias + burn time);
      unit tests for clustering and for the bias tradeoff (retro wins small
      trims, main wins big burns).
- [x] Settings: replace `flip_lead_time` with `rotation_bias` (1.5),
      `est_turn_rate_deg` (90.0), and `arrival_spool_pad` (0.5); registered
      and documented as retune-owned.
- [x] Rework `autopilot_system`: thruster query reads the section's local
      `Transform` (world dir = root rotation * local dir); group scan
      replaces the forward-only authority sum; attitude command rotates the
      chosen group's direction onto the error (outside the deadband, as
      today); per-engine firing - every live engine inside the `align_cos`
      cone (own-input hysteresis) fires with a shared throttle from the
      firing set's summed authority, others spool to zero; deadband
      generalizes to "fire any engine already on the error, accept only if
      none"; GOTO arrival accel + dynamic lead from the scored brake group.
      Manual burn keeps main-drive-only semantics via the local -Z check.
- [x] Physics-level tests: retro-equipped ship brakes a small overspeed with
      zero hull rotation; a large burn still flips to the main drive;
      a side thruster kills a lateral crumb inside the deadband; destroying
      the retro group falls back to flip-and-burn; existing suite (bound
      thrusters, cool-on-release, GOTO arrival) stays green.
- [x] Update `docs/2026-07-09-diegetic-autopilot.md` (group planner section,
      knob list, torque non-goal).
- [x] Verify: cargo check + fmt + targeted flight tests only (full suite and
      clippy are CI's job per AGENTS.md).

## Notes

- Relevant: crates/nova_gameplay/src/flight.rs (autopilot_system, settings,
  tests), docs/2026-07-09-diegetic-autopilot.md.
- The keybind exclusion stays manual-only (settled last cycle); in autopilot
  every engine is the computer's.
- 20260709-095043 (retune) additionally owns rotation_bias and
  est_turn_rate_deg.

## Close record (2026-07-09, restored - the original close chain silently died on a grep exit code)

What changed: autopilot_system clusters live engines into direction groups
(section-local Transform gives each thrust axis), scores each group with
rotation_time * rotation_bias + burn_time, rotates the winning group (not
the nose) onto the velocity error, and fires every engine inside the
align_cos cone with a shared throttle; the GOTO arrival curve takes its
deceleration and a dynamic lead from the scored brake group (flip_lead_time
removed; rotation_bias 1.5, est_turn_rate_deg 90, arrival_spool_pad 0.5
added). Deadband generalized (any engine on the crumb kills it); manual burn
switched to a section-local -Z check; engineless ships disengage. Squash
commit 26d7c3b; review round 1 APPROVE (1 NIT deferred to the torque-aware
follow-up).

Lessons already captured in docs/retros/20260709-121842-multi-thruster-autopilot.md.
Process note for the record: chaining `grep -c` with `&&` broke this task's
original close-out (grep -c exits 1 on zero matches) - the ticks, status and
close record were all lost while the commit still landed.
