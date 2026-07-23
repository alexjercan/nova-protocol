# Investigate: firing torpedoes drifts the ship ~20 u/s with no engaged drive

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: wontdo,physics,torpedo

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

- [x] Reproduce with a minimal rig: the 10_playable scene (or a pared-down
      ship + torpedo bay), log the ship's LinearVelocity each frame during a
      volley alongside every impulse applied to it (diagnostic-first, real
      numbers before theorizing).
- [x] Identify the writer (spawner recoil path, collider coupling in the
      launch window, blast impulse reach, or an unexpected system) and
      decide whether the magnitude is intended.
- [x] Fix or document-as-designed; if fixed, a fail-first regression test
      (App-level velocity assertion during a scripted volley) plus a
      10_playable assertion tightening (the script currently tolerates the
      drift by geometry).

## Notes

- Discovered 2026-07-13; the 10_playable geometry was chosen to be robust
  to the drift, so the smoke suite does not currently pin its absence.
- The 05_torpedo_section range fires the same bay but never asserts the
  ship's own velocity - the drift may exist there too, unmeasured.


## Record (2026-07-13): falsified - not a physics bug

The user spotted the real mechanism: the shipped `FlightBurnInput` action
binds `[KeyW, Space, RightTrigger]` app-wide
(crates/nova_gameplay/src/input/player.rs:588-591), and the example fire
scripts hold SPACE - so every fire-hold also drove the global burn action,
and `manual_burn_system` allocated that intent onto the ship's unbound
main drive. The "drift" was the ship's own engine burning under the shared
key, behaving exactly as designed.

Corroboration: the acceleration ran exactly while Space was held and
coasted after release; the weapon ranges (whose ships carry NO thruster
section) never drifted; only 10_playable's ship (which has a main_drive)
did. The gamepad RightTrigger shares the same collision (burn + the
examples' fire mapping).

CLOSED wontdo: the engine behavior is correct. The residual observation -
example input_mappings that bind fire to Space overlap the global burn
key, which is confusing in demos - is recorded in 10_playable's comments;
rebinding example fire keys is cosmetic and left to a future examples
pass.
