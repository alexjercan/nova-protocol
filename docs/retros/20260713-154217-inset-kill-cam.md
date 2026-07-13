# Retro: Inset kill cam

- TASK: 20260713-154217
- BRANCH: feat/inset-kill-cam (landed 380c215)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Making the driver's implicit state machine EXPLICIT (the
  InsetPanelState enum, resolved before side effects) made the fourth
  state nearly free and audited the three that had accreted across
  playtest rounds - the review's transition table fell straight out of
  the code shape.
- The spike's "presentation-only" boundary held perfectly: the diff
  touches one HUD file and the example; the option-D rejection (delaying
  the lock clear) never needed relitigating.
- Every pin passed on the first run - the plan's discriminator analysis
  (dead = despawned, cleared = alive) was done against the actual
  despawn paths (husk timing, direct despawn in the script) before
  writing code.

## What went wrong

- Nothing broke. The one open thread is process, not code: the inverted
  example pin is mechanism-verified but not live-run (the user's game
  instance holds the GPU); a live 12_hud_range run is owed once it
  closes.

## What to improve next time

- Component state that must die with its owner belongs ON the owner
  (both kill-cam components live on the panel entity) - this made the
  player-death teardown question answer itself; prefer that over Locals
  whenever the state has a natural owner.

## Action items

- [ ] Run 12_hud_range live once the user's game instance closes (also
  owed by 150343/154513).
