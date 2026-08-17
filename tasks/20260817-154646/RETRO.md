# Retro

## What worked

- Starting from Avian's `ColliderOf`, `ColliderTransform`, and mass-property
  schedule avoided render-hierarchy guesses.
- A dedicated rendered range made the physical result visible and supplied
  player-path probe markers.
- Repeated-sever and redundant-ring tests caught assumptions that a one-cut
  bridge test could not.

## Bugs and fixes

- New fragment COM data was initially stale. Force mass recomputation after
  reparenting and before velocity restoration.
- Equal-mass momentum assertions hid the real asymmetric case. Use actual
  computed masses and survivor rigid-point momentum.
- The ship adapter duplicated generic leaf destruction. Restrict its immediate
  destruction command to depleted non-leaves.

## Next time

- Treat observer ownership as an explicit design dimension. For every marker,
  name one owner for each graph state before adding another observer.
- Test repeated topology changes in the first implementation batch, not after
  the first bridge case passes.
