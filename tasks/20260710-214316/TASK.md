# Holo ribbon should terminate at the arrival park point, not the target center

- STATUS: OPEN
- PRIORITY: 25
- TAGS: v0.5.0, hud, polish

## Goal

Review finding R1.4 of 20260710-202408 (2026-07-10): the trajectory
ribbon (hud/holo_instruments.rs, sync_trajectory_ribbon) draws its last
segment to the target CENTER, so on a big body it visually plunges
radius + standoff past the actual park point. Terminate the ribbon at the
arrival park point instead: goal minus closing_dir * (arrival_standoff +
resolved target radius) - the same surface-relative geometry the arrival
now flies (docs/2026-07-10-surface-relative-standoff.md).

## Notes

- Pre-existing behavior (the ribbon always overshot by the standoff);
  only worth doing as HUD polish. The park point is derivable from
  ManeuverTelemetry (goal, distance is surface-relative) without new
  physics plumbing - check whether telemetry needs to publish the
  resolved radius or effective standoff explicitly.
