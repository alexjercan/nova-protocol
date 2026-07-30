# Decision: how the combat-lock drop is fixed and made visible

- STATUS: ACCEPTED
- DATE: 2026-07-30
- TASK: 20260730-123009

## Context

The owner reported the ship "sometimes loses radar focus on locked enemies".
Understanding found six candidate mechanisms (see TASK.md). Two load-bearing
forks had to be settled before any code was cut, because the candidate shapes
are mutually exclusive.

## Fork 1: where the fire-reset fix lands

`CombatDecay` has exactly one writer in the workspace (`input/targeting.rs`),
but three doc comments there promise that FIRING resets the idle clock, citing
task 20260713-082337 - which is CLOSED without the wiring. Since
`WeaponsRaised` is only true while the combat button is HELD, and
`WeaponsHot = raised OR combat lock`, a player fighting with the stance lowered
loses the lock after 30 s mid-fight.

**Decision: fix it in THIS task.** The change is small (reset `CombatDecay` on
the player's fire, mirroring the raised-stance reset) and it is the mechanism
the owner actually reported; splitting it out would ship a diagnosis with no
cure. Rejected: closing this task with the diagnosis and filing the fix
separately.

## Fork 2: the concrete artifact of the wind-down cue

The owner wants the 30 s rule KEPT but VISIBLE. Three shapes were on the table
and they are mutually exclusive - a permanent countdown arc is a new widget
with its own marker, spawn and layout surface, whereas tinting the existing
reticle adds no node at all, and the two cannot both be "the" cue without
double-signalling the same state.

**Decision: drive the EXISTING combat reticle.** The cue is a system that
reads `CombatDecay` and drives the alpha of the `ImageNode` carrying
`TorpedoTargetReticleMarker` (`hud/torpedo_target.rs`, tinted
`RETICLE_COMBAT_COLOR`): solid through the early window, then over the last
seconds the alpha ramps down and pulses faster, and at the threshold the
existing unlatch ghost pops off. No new marker, no new spawn, no new layout.

Rejected:

- A new countdown arc styled like `TorpedoTargetFocusMeterMarker`, visible for
  the whole 30 s - always-on screen furniture for a state that only matters at
  the end.
- The same arc shown only in the last seconds - still a new widget, and it
  duplicates what the reticle ramp already says.

`COMBAT_DECAY_SECS` stays 30.0 either way.
