# Spaceship rendering is twitchy at high velocity

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.5.0, rendering, physics, bug

## Goal

Playtest bug (user, 2026-07-10): the spaceship itself renders twitchy at
high velocity.

## Notes

- The camera chases per-frame (ChaseCamera lerp in Update) while the
  hull's Transform steps per physics tick - at high speed the per-tick
  steps are large and the smoothed camera makes them visible as judder.
  The canonical fix is transform interpolation on the physics bodies
  (avian supports it) or moving the camera sample to the same clock.
- Root of the twitching family: fix this one first and re-test
  20260710-231928 (HUD text) and 20260710-231930 (bullets) - they may
  collapse into it. Consider a single /spike covering all four twitch
  tasks before implementing any.
