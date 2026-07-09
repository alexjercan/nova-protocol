# AI rotation path: adopt slew_rotation and hull_turn_rate (and fix delta-into-absolute input)

- STATUS: OPEN
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

- [ ] Fix the delta-vs-absolute bug (command the absolute look-at rotation).
- [ ] Slew the AI's command through `flight::slew_rotation` at
      `flight::hull_turn_rate` (same queries as the player path: strongest
      live computer's torque, body's max principal inertia).
- [ ] Physics test: AI ship swings to a target attitude without limit-cycling
      (reuse the flight test harness).

## Notes

- Related: 20260708-162012 (smarter enemy AI), 20260709-095043 (retune).

Note (20260709, from the 150711 review): AI ships still measure chase/aim
direction FROM their own root origin (to_player = player_anchor - own
translation). When picking up this task, move the own side onto
live_structure_anchor too (sections/mod.rs) so both ends of the vector track
live structure.
