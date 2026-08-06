# Retro: Dress Shakedown Run to screenshot density: a slalom belt around a near planetoid

- TASK: 20260805-213432
- BRANCH: master
- REVIEW ROUNDS: 2

## What went well

- The pin was falsified before it was trusted, twice. Knot 5 at the sketch's
  (-40, 10, -300) was checked against the new test and failed; after the review
  corrected the metric, knot 2's old centre was restored and failed with
  `-6u of air`. A geometry pin that has never been made to fail is a decoration.
- The engine fix beat the content fix on the second finding. Requiring a 32u gap
  between knot boxes was tried first and a search showed it forces knot 1 to
  x=-85 and knot 4 to z=-770 - it would have deleted the slalom to satisfy a
  rule. Moving the placed set into `NovaEventWorld` made abutting knots legal
  instead, with no new config knob.
- The owner look caught what no test could. Round 1 was too cluttered AND the
  rocks exploded on spawn; both were reported from one play, and both were real.
- Numbers were recomputed independently in the review rather than re-read.
  That is what turned up the fourth overlapping knot pair and the beacon-1
  intrusion the original pin was passing.

## What went wrong

- The belt was sized against `radius` while everything physical uses
  `radius * ASTEROID_GEOMETRIC_FACTOR` (3.5-6.0x). Authored 3.4 nominal rocks
  are 20u bodies, so they spawned inside each other and blew each other apart.
  The same authored-vs-derived confusion then produced `min_separation` as an
  unplanned mid-task feature.
- Both review MAJORs were the same mistake in a different costume: the test
  measured a PROXY that read like the constraint. `half_extent.max_element()`
  is not a box's reach, and one action's copies are not the field. Both proxies
  passed while the real quantity was negative.
- Knot 4 was moved from x=200 to x=170 during the work because its margin was
  "0.9u" - a number the review could not reproduce under any metric. It was
  probably right to move it, but the reason recorded was not the reason.

## What to improve next time

- Write the quantity the PHYSICS uses, not the one that is easy to compute. For
  a scattered field that is: distance from the pocket to the region SURFACE,
  minus the pocket radius, minus the body's derived collider extent.
- When a pin's margin is small, say the number in the record and show the
  derivation. A margin quoted without its formula cannot be checked later, and
  this one could not be.
- A per-action invariant is suspicious whenever content authors N sibling
  actions as one thing. Ask what the FIELD needs, not what the action can see.

## Action items

- [ ] File the `ROCK_OFFSETS` debris cluster overlap: hand-placed pairs ~33u
      apart with combined worst-seed extents ~33u - the same latent
      spawn-overlap the belt hit, pre-existing and unfixed here.
- [ ] File the leftover review MINORs: the 240u coast shell is absent from the
      pin's `pockets` list with no comment saying why (F3); a dropped scatter
      copy logs at `debug!` and `// always the authored count` is now false
      (F5); `SEPARATION_ATTEMPTS` is `pub` with one consumer (F6); the
      determinism doc sentence explains the wrong thing (F7).
- [ ] `~760u` for the planetoid distance is 751.8u in both `shakedown/mod.rs`
      and the CHANGELOG. Fix both or neither.
- [ ] Content lint has no geometry check for scattered bodies at all
      (`close-spawn` covers ships only), which is why every clearance question
      has to live in a scenario-specific pin.
