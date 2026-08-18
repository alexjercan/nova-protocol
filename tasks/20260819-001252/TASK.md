# The sandbox runs at 2 FPS and no measurement has found it

- STATUS: OPEN
- PRIORITY: 99
- TAGS: v0.11.0,performance,bug,scenario

Epic: `20260818-220812`. **Blocks play. Highest priority on the board.**

Owner, 2026-08-19, flying the game by hand: the sandbox scenario runs at
**2 FPS**, on master AND on `perf-sandbox`. Not 25, not 45 - two.

## Why nothing has found this

Everything measured so far measured something else.

- `20260818-221021` fixed asteroid CARVE cost, measured on `carve_asteroids`
  holding PDC fire on one radius-3 rock: 83.5 -> 42.8 ms worst frame. Real, and
  irrelevant here. It was landed as `b61547fc` on its own merits.
- The probe's own `scene_baseline asteroid_field` pass reports about **20 ms a
  frame** for the same scenario the owner measures at **500 ms**. It showed no
  delta before or after the carve fix, which was reported at the time and not
  acted on.

**A 25x gap between the probe and the player is the finding.** The probe is not
running what the player runs, and until that is explained no measurement from
it can be trusted - including every budget `20260818-221027` is currently
setting.

## Do this first, before any fix

Reproduce the owner's case AS A PLAYER. Not a probe fixture, not an example, not
a headless pass: load the sandbox the way the main menu loads it, with a player
ship, and confirm the frame time is ~500 ms. If it is not, find what the owner's
run has that yours does not - build profile, render scale, resolution, mods,
settings, display - and say so before going further.

Then explain the gap. Diff the player path against `scene_baseline
asteroid_field` until the difference is named. Candidates, unverified: the probe
may pose a camera and never spawn a player ship, may run at a reduced render
scale or software render, may run too few frames to reach the state that stalls,
or may load a different scenario entirely.

## Only then, find the 2 FPS

500 ms a frame is not a subtle cost. It is one thing, or a small number of
things, and it will be obvious in a profile of the right run. Do NOT assume it
is carving - carving was today's suspect and today's suspect has already been
wrong twice.

Known content of `crates/nova_authoring/src/base_content/scenarios/sandbox/asteroid_field.rs`:
20 scattered rocks at radius 1.0-3.0, plus one radius-20 invulnerable gravity
well past the scatter cube, plus the "destroy 5 asteroids" objective.

## Done when

- The player-path frame time is measured before and after, and the sandbox is
  playable by hand. The OWNER's verdict decides that, not a number.
- The probe/player gap is explained and the probe case is corrected, so
  `scene_baseline asteroid_field` reports what a player experiences. A budget on
  a case that does not reproduce is worse than no budget.
- Whatever is found is written up with the measurement behind it.
