# Retro: reload-state on the diegetic ammo readout

- TASK: 20260716-123556
- BRANCH: feature/ammo-readout-reload-state (landed 4e43a619)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **Choosing the new constant to be test-compatible up front.** Setting
  `RELOAD_ALPHA = 0.5`, deliberately below the shipped tests' lit-vs-dim
  threshold `(0.95 + 0.16)/2 = 0.555`, let the third pip state slot in without
  touching `lit_pip_count` / `first_lit_pip_color` or any shipped drive test. The
  regression guard (`driver_at_rest_reload_is_identical_to_no_reload`) is
  therefore honest rather than a rewrite of the baseline. Picking the constant to
  fit the existing oracle is cheaper than re-deriving the oracle.
- **The spike made this task tiny and additive.** Because the mechanic task
  exposed `SectionReload::progress()`, the readout change was one query + one
  branch in the single ammo-state read point - exactly the "purely additive"
  outcome the spike predicted. Designing the seam two tasks earlier paid off.
- **One round, no findings.** The pure `reload_fill_segments` + A/B sweep tests
  left nothing for review to catch.

## What went wrong

- **The player wiki gap surfaced a task late.** The finite-ammo/auto-reload
  feature had no player-facing wiki description at all: the mechanic task
  (20260717-085640) updated CHANGELOG + dev wiki but not `combat-weapons.md`, and
  I only noticed while doing this task's docs sweep. Root cause: the mechanic
  task's docs sweep judged "what does MY diff invalidate" (a scenario flag, a
  config field - dev-facing) and missed "what does the PLAYER now experience"
  (finite ammo that reloads), which is a player-wiki surface. Across a task
  family the player wiki is the seam that falls between siblings.

## What to improve next time

- On any task in a multi-task feature, run the docs sweep against the WHOLE
  feature's player-facing surfaces, not just the current diff - especially the
  last task, which should backstop the family. Ask "what does the player now see
  or do differently" for the feature, not just the commit.

## Action items

- [x] Sharpened `keep-docs-in-sync-with-code` in docs/LESSONS.md with the
  task-family player-wiki-seam variant (already an enforced x3 lesson).
- [x] Backfilled the player wiki here (`combat-weapons.md` "Ammo & reloading"),
  so no follow-up task is needed - the family's docs are now complete.
