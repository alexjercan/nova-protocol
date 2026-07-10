# Bullets twitch badly at high spaceship velocity

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.5.0, rendering, physics, bug

## Goal

Playtest bug (user, 2026-07-10): bullets look funky - they twitch really
badly, especially at high spaceship velocity.

## Notes

- High relative velocity makes fixed-tick aliasing worst-case: a bullet
  moving hundreds of u/s covers multiple units per tick, and without
  render interpolation its rendered position staircases. Check avian's
  interpolation settings (TransformInterpolation / PhysicsInterpolation)
  for projectiles specifically - spawn bundles may be missing what ships
  have, or nothing has it.
- Same investigation umbrella as 20260710-231928/229/231; whichever is
  picked up first should spike the interpolation story once for all
  four.
