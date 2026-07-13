# Retro: Conveyance gold text readability

- TASK: 20260712-152340
- BRANCH: gold-text-readability (landed as fadcdb4)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Playtest feedback -> diagnosis was immediate because the shipped code
  was fresh: "gold and white" mapped straight onto the cyan->gold
  cross-mix's near-white mid-blend and the breathing 12px label. The fix
  cycle (task, sprout, fix, tests, review, land) took one pass.
- The ledger's check-all-targets-for-struct-field lesson (filed by a
  parallel session THIS day) was applied on first use.

## What went wrong

- The washed mid-blend was designed in: the original spike text said
  "lerp toward white", the implementation switched the pulse target to
  gold for palette purity, and nobody re-asked what the INTERMEDIATE
  frames of a cyan->gold sweep look like. Endpoint reasoning approved a
  transition whose middle was the problem.
- The first delivery-guard advance (period/4) landed exactly on the wave
  crest, where the alpha factor is 1.0 - indistinguishable from spawn -
  and the "did it move" guard failed against a working system.

## What to improve next time

- When reviewing/altering a color transition, evaluate the intermediate
  blend colors, not just the endpoints - cross-hue RGB lerps pass
  through desaturated blends that read as white on dark HUDs.
- Time-advance delivery guards must avoid the wave's stationary points
  (crest/trough); pick an eighth period, not a quarter.

## Action items

- [x] Ledger: `endpoint-only-color-reasoning` (x1) and a wave-guard note
      folded into the existing exact-instant family.
