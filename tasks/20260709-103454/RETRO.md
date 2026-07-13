# Retro: Maneuver instruments v1 (telemetry seam, chips, ORBIT holo ring)

- TASK: 20260709-103454
- BRANCH: flight-instruments (squash-merged to master as d943fe8)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES with 1 MAJOR + 5 MINOR,
  round 2 APPROVE)

First task of the diegetic-instruments arc (spike 20260710-174523), which
also opened with its own design spike - the fastest spike-to-shipped
turnaround of the project so far.

## What went well

- **The spike prevented scope drift in both directions.** Naming the
  rejected options (all-chips, all-3D, cockpit) meant implementation never
  wobbled between them, and bounding the 3D pilot to "a torus" kept the
  holo language cheap to prove.
- **The telemetry seam paid for itself immediately.** Publishing
  ManeuverTelemetry from autopilot_system (compute where the truth lives,
  render dumb) made every chip driver a trivial pure-ish system and the
  whole ring lifecycle headless-testable. The keybind-hints task should
  copy this shape: resolve availability where the verbs live.
- **Process rules from earlier retros were applied, and they worked**: the
  affected-modules list included input::ai (orbit retro's
  signature-change rule), check ran with --examples (gravity retro), and
  neither regressed.

## What went wrong

- **R1.1 (MAJOR): the flip marker did not solve the autopilot's own
  equation.** goto_flip_point used `standoff + v^2/(2a)` while the
  arrival rule brakes at `v*lead + v^2/(2a)` - the marker sat seconds of
  coast short of the real flip. Root cause: the helper was written from
  the physics textbook (brake distance) instead of from the function the
  ship actually flies (arrival_speed_limit, ten lines above it in the
  same file). The instrument's one job was agreement with that function.
- The test then had to move its sampling window because the corrected
  flip arrives earlier - the original test had silently validated the
  wrong physics.

## What to improve next time

- **When an instrument/readout mirrors a control law, derive it from the
  control law's code, not from first principles** - ideally by calling
  the same function or sharing its terms; the review caught this one, but
  "solve the same equation the controller solves" should be the starting
  point, not the fix.

## Action items

- [ ] Playtest: readout chip width (260px, NoWrap) on small windows; ring
  visibility against bright skyboxes (unlit cyan, alpha 0.9); whether the
  FLIP marker's disappearance at brake onset reads as intended.
- [ ] Next in the arc: 20260710-174646 (keybind hints - copy the
  resolver-then-render shape), then 20260710-174629 (holo expansion).
