# Controller-based RCS burn loop sound: design record

## What plays, and when

A sustained loop (an airy cold-gas hiss) that plays whenever the RCS primitive
is burning on a ship, at a volume that tracks the burn effort. It fires for
BOTH drivers of RCS:

- the player holding SHIFT and nudging (mouse/scroll), and
- the autopilot writing `RcsIntent` for ORBIT station-keep trim or the
  STOP/GOTO terminal settle.

Both write the same `RcsIntent` on the ship root, so gating the sound on that
one signal (plus the RCS verb) makes it driver-agnostic for free. That is what
"controller based" buys: the sound belongs to the flight computer, not to a
particular input path.

## Where it lives (controller-authored)

The controller section already owns the ship's computer voice - the
lock/radar/safety cues in `ControllerSectionSounds`. The RCS loop is one more
authored sound on that section (`ControllerSectionConfig::rcs_loop_sound` ->
`ControllerSectionSounds::rcs_loop`), AUTHORED-OR-SILENT like the rest: a
controller with no `rcs_loop` makes no sound. Base content authors
`self://sounds/rcs_loop.wav` via the `SectionMeshRefs` manifest (both the
catalog controller in `sections.rs` and the `craft.rs` controller builder), so
the shipped game has it and mods can reship or reference it.

## How it plays (models the thruster hum exactly)

The engine-hum loop was the template. The RCS loop reuses its whole shape:

- one looping audio entity per DISTINCT resolved handle (`RcsLoopSfx`,
  mirroring `ThrusterLoopSfx`) - ships sharing a sound share a loop;
- the volume logic is split into a headless-testable resource
  (`RcsLoopVolume`, reusing `HumLevels`) written by `compute_rcs_loop_volume`
  and pushed to the `AudioSink` by `apply_rcs_loop_volume` - so the volume can
  be unit-tested without an audio device;
- per-ship attribution (the loudest ship on a handle wins, not a sum),
  distance attenuation from the `SfxListenerMarker` camera with the player's
  own ship exempt, and the same ~8 units/s exponential smoothing;
- the compute pass gates on `WithheldVerbs::granted(FlightVerb::Rcs)`, the same
  capability check as `rcs_burn_system`, so a hull that cannot RCS is silent;
- the systems join `SpaceshipSectionSystems` (scenario-gated: silent in the
  editor, plays wherever a scenario is live including the menu ambience), and
  `RcsLoopSfx` is added to the pause/resume sink sweep so the hiss mutes behind
  the pause overlay.

Volume maps `RcsIntent.length()` (clamped to 1) to `RCS_MAX_VOLUME = 0.22`, a
touch under the engine's 0.3 because RCS is a gentle push.

## The asset

`assets/base/sounds/rcs_loop.wav` is a generated placeholder from
`scripts/gen-placeholder-sounds.py` (a new seamless-looping `"hiss"` synth:
one-pole low-passed white noise with a 20 ms overlap crossfade at the loop seam
to kill the boundary click). Deterministic (seeded by name). Swap the file at
the same path for a real asset with no code change, exactly like the other
placeholders.

## Alternatives considered

- A GLOBAL default RCS sound (a loaded resource played for any RCS burn).
  Rejected: it would break the codebase's uniform authored-or-silent model and
  give mods no way to reship the sound. The controller-authored path is
  consistent with every other world cue.
- Reusing the thruster `loop_sound`. Rejected: RCS should read distinct from
  the main drive; a separate hiss synth keeps them apart by ear.
