# ch5: restore bigger planetoid wells once AI is gravity-aware (revert the tiny-well tuning of 20260723-223954)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, content, scenario

## Story

The ch5 raid's planetoids were deliberately shrunk to tiny, gentle wells
(radius 8-9, gravity 1) and moved out of the combat area, and the base was made
thrusterless, because the AI cannot fly a gravity well yet - AI ships just fall
in (task 20260723-223954, the third gravity iteration). The user wants the
bigger, more dramatic planetoids back once that is fixed.

DEPENDS ON: 20260723-224003 (AI ships handle gravity wells: engage-flight
resists/uses wells instead of falling in). Do not pick this up until that lands
- bigger wells with the current AI just recreate the "fighters fall in" bug.

## Notes / pointers

- What to revert/restore when 20260723-224003 is done: the planetoid
  radius/gravity + positions in `ledger_ch5_the_raid.content.ron` (the pre-shrink
  values are in the git history of task 20260723-200643 / the round-2 diff), and
  reconsider whether the base can hold station in a real well again (it needs
  gravity-aware AI + RCS, the approach the round-2 task reverted). Bump the bundle
  version and update the ch5 rig's base assertion accordingly.
- Re-check the geometry the same way round 2 did: with bigger wells, make sure
  the combatants that SHOULD stay clear still do, and the ones meant to fly wells
  (now that AI can) behave.
- Also a good moment to re-hide ch5 (`hidden: false` -> `hidden: true`) if it has
  not already been re-hidden before the 0.8.0 release.

## Definition of Done (sketch - refine when picked up)

- Bigger planetoid wells restored in ch5; AI ships (fighters, and the base if it
  station-keeps in a well) handle the gravity without falling in; lint + ch5 rig
  green; playtest confirms the drama is back without the chaos.
