# AI engagement flight: standoff orbit/strafe envelope

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.4.0,ai,spike,handling


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 2)

Goal: replace pure pursuit (which converges to point-blank parking or ramming)
with a standoff envelope in the Engage state: approach when far, hold/orbit at
preferred weapons range by mixing a lateral component into the desired
direction, extend when too close. Tunable preferred-range constants; physics
test on the flight harness that the ship settles into the range band instead
of closing to zero.

Blocked on: 20260709-155921 (AI rotation path onto slew_rotation /
hull_turn_rate). Depends on: 20260709-225726 (skeleton).
