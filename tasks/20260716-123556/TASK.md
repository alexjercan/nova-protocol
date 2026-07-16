# Ammo/magazine HUD readout: show loaded type, rounds and reload state

- STATUS: OPEN
- PRIORITY: 32
- TAGS: v0.7.0,hud,ui,spike


## Goal

Multi-type magazines, reload and bullet-type switching shipped in v0.5.x
(20260712-133349), but the HUD shows no ammo state at all - weapon state is
only visible via the reticle. Add an ammo/magazine readout in the HUD's
instrument family (HudTier: Instrument, so it survives the Minimal tier like
the speed/mode chips): loaded bullet type, rounds remaining, and reload
state, for the turret and torpedo bay of the player ship. Small and bounded;
follow the existing chip/instrument styling (nav-cyan palette,
screen_indicator/chip family).

## Notes

- Spike: tasks/20260716-122954/SPIKE.md (v0.7.0 release scope)
- Plan: docs/plans/20260716-v0.7.0-plan.md, strand 3
- Ammo data lives in the section/magazine components from 20260712-133349;
  HUD tiers from 20260711-180501.
- Coordinate styling with diegetic HP (20260711-202901) if both land this
  release.
