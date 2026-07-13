# Radar UX polish: CTRL hint row, concentric crosshairs, inset faction line, distance-only sweep label

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,hud,ux,playtest

## Outcome (CLOSED 2026-07-13)

Playtest batch (user, 2026-07-13; done directly on master):

- **[CTRL] RADAR in the keybind cluster**: FlightVerbHints gains a `radar`
  hint (fixed "CTRL" label like the wheel rows; lit while the computer
  grants the Lock verb); the cluster grows to six rows (RADAR before
  COMPONENT). The fuller hold/tap teaching stays with 20260713-090653.
- **Concentric lock brackets**: `ScreenIndicatorSize::ApparentSize` gains a
  `scale` multiplier (min_px stays the floor); the travel crosshair tracks
  at 1.35 vs the combat reticle's 1.0, so an overlapped pair stays two
  concentric rings at ANY target size instead of converging to the same
  pixels and shimmering (the min-px difference only helped at the floor).
- **Faction line on the inset**: the viewfinder caption is now
  "<NAME> - HOSTILE/OWN/NEUTRAL", colored by relation, shown whenever a
  combat lock exists (gesture-independent) - restoring on the rich surface
  the information the retired reticle relation-tint carried.
- **Distance-only sweep label**: the radar box label is "<dist>m" for BOTH
  slots (revising Q6a: the name read as clutter, and the combat/travel
  asymmetry read as a bug); names live on the faction line + readout.

Tests re-pinned (box label both-slots, faction line incl. unnamed-neutral
delivery guard, six hint rows); 471 lib tests green; fmt clean. The
12_hud_range live run was SKIPPED: the user's game instance is running and
the earlier contention flake is documented in 20260713-124000 - re-run
after the session closes.

## Notes

- Revises spike 20260713-110039's Q6a (gesture-time name+distance) per
  playtest; the spike carries the note.
- ApparentSize.scale is a widget-level addition; existing call sites pass
  1.0 (combat reticle) - only the travel crosshair scales.
