# Retro: Ledger chapter 2 encounter rework

- TASK: 20260717-112630
- BRANCH: work/ledger-ch2-rework (landed b415d321)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 1 NIT, both fixed)

## What went well

- Computed-pin-first: writing the geometry invariant test alongside the
  layout caught a real design flaw (the Mule 52u off the heavies' fire
  lane) that the layout's own author had just eyeballed as fine - and
  fixing it forced the stray-fire model to be stated precisely (danger =
  hostile->player line EXTENDED past the player, since misses overshoot).
- The reviewer's independent recomputation reproduced every number,
  including the fail-first catch to the decimal (51.7u vs the claimed
  52u) - the NOTES' evidence survived a hostile audit.
- Verifying leash semantics in source BEFORE authoring (center = patrol
  centroid, spaceship.rs:330) meant the mechanism was used correctly;
  the one leash mistake was arithmetic (a 665u anchor with a 650 leash),
  not conceptual.

## What went wrong

- R1.1: the leash radii were chosen by feel after all the other numbers
  were computed - the one un-computed quantity was the one the review
  caught. Root cause: the invariant ("the leash sphere covers the arena")
  was in my head but not in the test, so nothing checked it.
- The first Mule station was inherited from the old file's geometry
  instead of being derived against the new threat lanes - carried-over
  positions deserve the same scrutiny as new ones.

## What to improve next time

- When a layout quantity has an invariant worth stating in prose (NOTES
  said "full aggression in the arena"), turn that sentence into a computed
  assertion in the same sitting; prose invariants are where reviews find
  the holes.

## Action items

- [x] leash-covers-spawn is now a pinned assertion for every leashed
  hostile in the encounter test (prevents recurrence mechanically).
- [x] docs/LESSONS.md: new lesson prose-invariant-becomes-pin (x1); bumped
  authored-vs-derived-values (the carried-over Mule station variant).
