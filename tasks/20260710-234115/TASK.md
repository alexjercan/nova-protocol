# Engaged-state shader tint across the flight instrument family

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,hud,ux,spike


## Goal

Make engaged-vs-manual readable at a glance from the instruments
themselves: the velocity sphere, trajectory ribbon, orbit ring, and flip
gate share a material/shader treatment (hue or intensity shift) while the
autopilot is engaged, reverting when control is manual. This reinforces
the ship-anchored mode chip (task 20260710-231926) diegetically, per the
user's "chip + shader tint" questionnaire choice.

## Notes

- Spike: docs/spikes/20260710-234019-diegetic-flight-status.md
- The holo meshes (ribbon, ring, gate) are plain unlit StandardMaterial;
  the spheres use custom material extensions (hud/velocity.rs). A simple
  per-frame color swap is the honest v1; a shared material extension is
  the richer option - decide at /plan time.
- Depends on nothing, but lands best after 20260710-231926 so the mode
  chip and the tint ship as one language.
- Piggyback playtest (retro 20260710-231926): the new chip offsets (120px
  right of the ship, mode row stacked above) and the spoke thickness were
  set headless; give them a by-eye check while tuning the tint.
