# Show-don't-tell lock HUD: inset-on-lock viewfinder, state styling, lock sfx, status text retired

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.5.0,hud,ux,audio,spike

## Goal

Playtest feedback (2026-07-13): too much text; the RTT inset should carry
the state ("if we have RTT we are not in DUMB fire mode"), and the lock
cleared cue should work without words. Strands B1 + C1 from the spike:

- Inset appears the moment a COMBAT lock exists (drop the focus-dwell gate
  for the panel only; dwell keeps gating the component fine-lock), including
  live during a radar sweep - the viewfinder.
- Frame carries the state: color = safety hot/safe, presence = guided
  torpedoes, small distance numeral, target name only while the radar is
  held.
- Reticle armed-ticks as the universal non-inset fallback (beacon locks,
  sub-Chrome HUD tiers).
- The drive_weapons_status text block is REMOVED ("TORP: DUMB" dies without
  replacement text; ammo-gauge pip is the reserved fallback knob).
- Lock cleared: crosshair unlatch animation (scale-up + fade, per slot color,
  staged tap reads as two pops) + LockOff sfx; LockOn at threshold-lock;
  extend scripts/gen-placeholder-sounds.py + NovaSfx. Text toast retired
  (LockClearedToast stays as the internal message the animation subscribes
  to).
- Absorbs the safety OFF->ON audio blip from 20260713-090653 (same sfx
  batch, same non-text-status theme).

## Notes

- Spike: docs/spikes/20260713-110039-show-dont-tell-radar-ux.md (strands
  B1 + C1 + open playtest knobs).
- Depends on: 20260713-110330 (live lock changes what the HUD must show:
  provisional/committed visuals collapse, viewfinder-while-sweeping).
- Main surfaces: hud/lock_crosshairs.rs, hud/target_inset.rs
  (drive_inset_camera gating), hud/torpedo_target.rs, audio.rs, the
  placeholder-sound generator.
- Coordinate with 20260713-090653 (drop the blip line there when this lands).
- /plan before implementation.
