# Investigate: firing torpedoes drifts the ship ~20 u/s with no engaged drive

- STATUS: OPEN
- PRIORITY: 40
- TAGS: bug,physics,torpedo

## Goal

While building 10_playable (task 20260712-211352), the timeline probe
showed the player ship drifting from z = -0.1 to z = -22 between t1.5 and
t4.0 (peaking around 20 u/s) with NO autopilot engaged, no manual burn, no
gravity wells, and the only activity being a torpedo volley (2 launches,
Space held, ship's main drive idle). The drift then stopped on its own.
Either the torpedo spawner/launch couples momentum into the ship far beyond
any plausible recoil, a blast impulse reaches the ship, or something else
writes the ship's velocity during a volley. The measured timeline (z -0.1 -> -22 over t1.5..4.0, two launches) is
recorded above; reproduce it with the rig in the first step.

## Steps

- [ ] Reproduce with a minimal rig: the 10_playable scene (or a pared-down
      ship + torpedo bay), log the ship's LinearVelocity each frame during a
      volley alongside every impulse applied to it (diagnostic-first, real
      numbers before theorizing).
- [ ] Identify the writer (spawner recoil path, collider coupling in the
      launch window, blast impulse reach, or an unexpected system) and
      decide whether the magnitude is intended.
- [ ] Fix or document-as-designed; if fixed, a fail-first regression test
      (App-level velocity assertion during a scripted volley) plus a
      10_playable assertion tightening (the script currently tolerates the
      drift by geometry).

## Notes

- Discovered 2026-07-13; the 10_playable geometry was chosen to be robust
  to the drift, so the smoke suite does not currently pin its absence.
- The 05_torpedo_section range fires the same bay but never asserts the
  ship's own velocity - the drift may exist there too, unmeasured.
