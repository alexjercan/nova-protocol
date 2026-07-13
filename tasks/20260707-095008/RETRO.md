# Retro: PDC turret test range

- TASK: 20260707-095008
- BRANCH: feature/turret-range
- PR: #32 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, two intentional NITs)

Short, smooth cycle. See `tasks/20260707-095008/TASK.md`.

## What went well

- The torpedo range (`06_torpedo_range`) was a ready-made template: same scenario
  scaffolding, gate spawning, moving-gate driver, autopilot/screenshot wiring. The
  turret range fell out of it quickly - the only new surface was the aim wiring
  (`TurretSectionTargetInput`) and the telemetry.
- Reading the input plugin first paid off: `update_turret_target_input` (the
  crosshair) competes for the same `TurretSectionTargetInput`, and
  `SpaceshipInputSystems` is a public set, so ordering `range_aim.after(...)` makes
  the range aim deterministically win. Cheaper than discovering the jitter at
  runtime.
- Picked a telemetry metric that *is* the diagnosis: throttled aim error in degrees
  against the sweeping gate. The headless log alone shows the turret catching to
  ~7 deg then oscillating to ~20 - the "clunky" complaint made quantitative, and it
  turned straight into a fix task with a concrete acceptance criterion.
- Kept the example honest to its job: it exposes the problem and files the fix
  (`20260707-150001`) rather than smuggling a gameplay change into an example.

## What went wrong

- Nothing notable. Single review round, no rework.

## What to improve next time

- When a range's purpose is to expose a tuning problem, design the telemetry to be
  the acceptance criterion for the eventual fix (here, "aim error stays single
  digits against a crosser"). It makes the follow-up task self-defining and the
  range a permanent regression check.

## Action items

- [ ] `20260707-150001` - turret aim lead/smoothing (the diagnosed lag); the range's
      aim-error readout is its acceptance test.
- [ ] `20260707-150002` - live tuning sliders for the range (deferred step 3).
