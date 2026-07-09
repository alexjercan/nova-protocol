# AI rotation path: adopt slew_rotation and hull_turn_rate (and fix delta-into-absolute input)

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0,ai,handling

From review R1.8 of the flight-feel retune (20260709-095043). The AI brain
(input/ai.rs) rewrites `ControllerSectionRotationInput` every frame with no
slew - the exact PD-saturation regime the player path was fixed for (flip
wobble), now at 2.5x less torque than when the AI was written. Exposure today
is editor-only (no shipped scenario spawns `SpaceshipController::AI`), but any
AI combat work (20260708-162012) will hit it immediately.

Also pre-existing, spotted in the same review: input/ai.rs:92 writes a DELTA
(`Quat::from_rotation_arc(...)`) into an input that every other writer treats
as an ABSOLUTE world rotation.

## Steps

- [x] Fix the delta-vs-absolute bug (command the absolute look-at rotation).
      Done command-anchored like the autopilot: the goal carries the
      command's own forward onto the desired direction and evolves from the
      command's previous state, never the hull (roll regulation).
- [x] Slew the AI's command through `flight::slew_rotation` at
      `flight::hull_turn_rate` (same queries as the player path: strongest
      live computer's torque, body's max principal inertia). Dead-helm
      freeze matches the player path.
- [x] Physics test: AI ship swings to a target attitude without limit-cycling
      (reuse the flight test harness). Plus command-level tests: one-frame
      slew step at the derived rate, absolute look-at convergence, dead-helm
      freeze. Also moved the AI's own side of the chase vector onto
      `live_structure_anchor` (rotation + thruster systems), per the 150711
      review note.

## Notes

- Related: 20260708-162012 (smarter enemy AI), 20260709-095043 (retune).

Note (20260709, from the 150711 review): AI ships still measure chase/aim
direction FROM their own root origin (to_player = player_anchor - own
translation). When picking up this task, move the own side onto
live_structure_anchor too (sections/mod.rs) so both ends of the vector track
live structure.

## Resolution (20260709)

Shipped the slewed, command-anchored, absolute rotation path for the AI
brain, mirroring the player path (turn rate from strongest live computer +
max principal inertia; freeze on dead helm) plus the own-side
live-structure anchor in the rotation and thruster systems. 4 new tests
(3 command-level with manual time, 1 physics-level on the integrity/flight
harness).

Difficulty worth recording: the physics test's original acceptance
(residual spin ~0) is unreachable today - after the swing the hull keeps a
small pure-ROLL oscillation (0.23 rad/s in the stock rig, worse when
over-torqued) that the bcs PD never damps. Diagnosed by sampling the spin
axis: it is the nose axis with alternating sign, i.e. the exact
'PD cannot damp roll' bcs bug already filed as 20260709-125640 (whose own
evidence says even a pure damper fails). The test now pins what THIS task
owns - the nose converges onto the player and stays there for a second -
and bounds the roll residual at 0.5 rad/s with a pointer to 125640 to
tighten when the bcs fix lands.

Verified: cargo fmt, cargo check --workspace, ai:: module (6 tests, incl.
the two pre-existing turret-anchor tests) green. Skipped honestly per user
instruction: full local suite and clippy (CI runs the suite).
