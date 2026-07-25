# Retro: drawer scroll clamps at content end

- TASK: 20260725-171900
- BRANCH: master
- REVIEW ROUNDS: 1

## What went well

- The regression reproduced the user-visible issue directly: the stored scroll
  offset advanced beyond the computed bottom.
- Checking Bevy UI's layout source gave the exact clamp formula instead of a
  hand-rolled estimate.

## What went wrong

- The previous scroll fix copied the repo's top-clamp input pattern without
  checking how Bevy clamps the bottom during layout.

## What to improve next time

- For UI input code that writes an engine-owned state component, read the engine
  layout/update code for every bound the input can cross.

## Action items

- [x] Added a lessons-ledger entry for reusing the engine's scroll bounds.
