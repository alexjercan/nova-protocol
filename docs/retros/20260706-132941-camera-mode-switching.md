# Retro: camera mode switching (task 20260525-132941)

## What was asked
Camera should be WASD-controlled with no spaceship, chase cam with one.

## What happened
The scenario camera already switched to chase on spawn, but the "back to WASD on
destroy" path was broken: `on_player_spaceship_destroyed` triggered on
`Add<HealthZeroMarker>` (only added to sections, never the ship root) and guarded on
`entity == root`, so it never fired. Rewrote it to trigger on
`Remove<PlayerSpaceshipMarker>`, the same signal the HUD cleanup observers use.

## Lessons
- When a fix is needed, check whether the codebase already solves the *same* problem
  elsewhere. The HUD's `On<Remove, PlayerSpaceshipMarker>` observers were the ready-made
  correct pattern; matching them beat inventing a presence-polling system.
- `On<Add, HealthZeroMarker>` fires per-section, not per-ship. "The player ship is gone"
  is `Remove<PlayerSpaceshipMarker>`, not "a section hit zero health".
- Runtime behavior still unverified (no display); correctness rests on matching the
  established marker lifecycle. A headless smoke test would let us actually assert the
  camera swap.
