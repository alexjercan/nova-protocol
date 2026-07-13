# AI engagement flight: standoff orbit/strafe envelope

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.4.0,ai,spike,handling


Spike: tasks/20260709-225508/SPIKE.md (wave 2)

Goal: replace pure pursuit (which converges to point-blank parking or ramming)
with a standoff envelope in the Engage state: approach when far, hold/orbit at
preferred weapons range by mixing a lateral component into the desired
direction, extend when too close. Tunable preferred-range constants; physics
test on the flight harness that the ship settles into the range band instead
of closing to zero.

Blocked on: 20260709-155921 (AI rotation path onto slew_rotation /
hull_turn_rate). Depends on: 20260709-225726 (skeleton).

## Steps

- [x] Rework `ai_desired_direction` (input/ai.rs) into a standoff envelope:
      radial term from the range error against a preferred range
      (AI_STANDOFF_RANGE, inside the turrets' effective range), blended
      with a tangential orbit term (stable handedness from
      los x Y, X fallback for polar approaches) weighted by how far
      outside the band (AI_STANDOFF_BAND) the ship is - far: approach the
      target; in band: orbit it; too close: extend away. Speed scales with
      the RANGE ERROR (not raw distance), clamped to a new AI_ORBIT_SPEED
      floor; the overshoot brake regime stays.
- [x] Update the existing harnesses for the new geometry: flip_world's
      player moves outside the band (approach regime keeps the 180-flip
      semantics); the swing physics test's player moves to approach range
      so nose-on-target remains the correct assertion.
- [x] Unit tests for the envelope: far -> points at the target; in band ->
      mostly tangential (small dot with the line of sight); too close ->
      points away; brake regime preserved; polar line of sight does not
      degenerate.
- [x] Physics test (full loop: acquisition + rotation + thrust + PD +
      impulses on the flight harness): a ship starting outside the band
      approaches and settles into the standoff band instead of closing to
      zero - assert the last simulated second stays inside a generous band
      and the ship never rams inside the inner envelope.
- [x] Verify: cargo fmt, cargo check --workspace, ai:: module tests (skip
      full local suite per user instruction; report skips honestly).

## Notes

- Relevant files: crates/nova_gameplay/src/input/ai.rs (all changes).
- Preferred range must sit inside fire discipline's effective range
  (default turret: 450 m) - start at 250 m, band 60 m; feel knobs.
- The orbit handedness is global (all ships circle the same way); opposing
  per-ship directions are polish for a later pass if orbits look uniform.
- Torpedo standoff interplay (blast self-harm at close range,
  20260709-140559) is the torpedo-usage task's concern (225732); this
  task only guarantees the ship no longer parks at zero range.
- Incident (20260710): the first attempt's worktree was lost mid-flight
  with uncommitted changes (session interruption during a backgrounded
  test run); this is the replay. Lesson: commit work-in-progress on the
  feature branch before long test runs.

## Resolution

Implemented in crates/nova_gameplay/src/input/ai.rs, all inside
ai_desired_direction (pure, unit-testable) plus tuning constants:

- Radial term from the range error against AI_STANDOFF_RANGE (250 m),
  blended with a tangential orbit term (los x Y, X fallback at the poles)
  by a radial weight that ramps over AI_STANDOFF_BAND (60 m). Far:
  approach; in band: orbit; too close: extend away.
- Speed budget scales with the RANGE ERROR (not raw distance), clamped to
  [AI_ORBIT_SPEED (8), AI_MAX_CHASE_SPEED (20)]; the overshoot brake
  regime is unchanged.
- Existing harnesses moved outside the band (flip_world player to z=800,
  swing physics player to x=1000) so their approach-regime assertions
  keep their old meaning.
- Five unit tests pin the envelope regimes and the polar fallback; a new
  standoff physics test runs the full diegetic loop (acquisition ->
  behavior -> PD torque -> aligned thrust -> impulses) for 45 simulated
  seconds and asserts the last second holds inside 2x the band with no
  dive under 100 m - pure pursuit closes to ~zero on the same harness.

Verified: cargo fmt --check clean, cargo check --workspace clean,
input::ai tests 33/33 green. Full local suite skipped per user
instruction (CI runs it on the PR/master).
