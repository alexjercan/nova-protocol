# Review: Controller-based RCS burn loop sound

- TASK: 20260718-201532
- BRANCH: feature/rcs-burn-sound

## Round 1

- VERDICT: APPROVE

Reviewed the diff (commit 2566e9ca) against master. `audio::` suite 30 passed
(3 new), `content_ron_parity` 2 passed, gen is deterministic (only rcs_loop.wav
is new, no other placeholder changed). No warnings.

Independently verified the load-bearing claim - "it fires for both the player
and the autopilot ORBIT/STOP":

- The audio layer is driver-agnostic BY CONSTRUCTION. `compute_rcs_loop_volume`
  reads `RcsIntent` on the ship root (`q_intent.get(root)`), and both drivers
  write that same component on the same `SpaceshipRootMarker` root: the player
  modal (input/player.rs) and the autopilot (flight.rs, both the STOP/GOTO
  settle and the new ORBIT trim, task 20260718-151102, which writes `&mut
  RcsIntent` on the root). There is no separate "player" vs "autopilot" path at
  the audio layer - both collapse to "non-zero RcsIntent on the root," so the
  player-driven unit test is representative of the autopilot case too.
- Schedule ordering is safe: the autopilot writes `RcsIntent` in FixedUpdate;
  the audio compute reads it in Update. The component persists between
  schedules, so the read sees the latest written value, and when the autopilot
  zeroes the intent (not trimming) the loop mutes. `decay_player_rcs_intent`
  only touches `RcsActive` (player) ships, so it never bleeds the autopilot's
  intent out from under the sound.
- The gate matches `rcs_burn_system` exactly (`WithheldVerbs::granted(Rcs)`),
  so the sound plays iff the burn would - proven by the silent-without-verb
  test (which fails if the gate is dropped) and the plays-then-mutes test
  (which fails if the intent read or the reset-to-zero is wrong).

Design and consistency:

- Reuses the thruster engine-hum machinery verbatim (one loop per handle,
  `HumLevels`, per-ship loudest-wins, distance attenuation with the player
  exempt, ~8/s smoothing, split volume-resource for headless testing). The
  parallel system rather than a forced abstraction is the right call - the two
  differ in what they read (`ThrusterSectionInput` vs root `RcsIntent` + verb
  gate).
- Controller-authored (`ControllerSectionConfig::rcs_loop_sound` ->
  `ControllerSectionSounds::rcs_loop`) is consistent with the codebase's
  authored-or-silent world-sound model and the controller's existing
  lock/radar/safety voice. Verified every `ControllerSectionConfig` literal
  compiles with the new field (`check-all-targets-for-struct-field`): the two
  builders got the field; the three other constructions use `..default()`.
- `RcsLoopSfx` is added to the pause/resume sweep, so the hiss mutes behind the
  pause overlay like the engine hum.

Observations (non-blocking):

- Whether the placeholder hiss actually READS as RCS in-game (level, timbre vs
  the engine hum) can only be judged by ear in a live playtest - inherent to an
  audio change; the file is a swappable placeholder by design. Not a blocker.
- All ships share the one base `rcs_loop` handle, so the scene collapses to a
  single loop at the loudest ship's volume - identical to the thruster-hum
  design and intentional (distinct mod sounds would get their own loop).

No BLOCKER/MAJOR/MINOR findings. Ship it.
