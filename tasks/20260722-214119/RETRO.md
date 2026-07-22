# Retro: Ledger close-out (20260722-214119)

## What went well

- Writing all prose from the actual diff (`git diff 803a4e0c~1..HEAD --
  webmods/the-ledger`) rather than the task summary caught the thing that
  mattered most: the README's chapter-4 line ("sell it or burn it, then survive
  the Auditor. Two endings") was FALSE after the ch4 rework - the burn ending no
  longer fights. The player-facing description of the flagship mod would have
  shipped a lie about its own headline feature. The reviewer cross-checked the
  new README against the shipped `ledger_ch4.content.ron` (exactly one auditor
  spawn, in the sell branch) and confirmed it now matches.
- Version discipline held: content rework -> MINOR bump (1.5.0 -> 1.6.0) per the
  documented convention, and the ch2 rig's version assertion is a RANGE
  (`> [1,0,0]`), so the bump did not need a test edit - the
  `sibling-change-leaves-stale-fixture` lesson already paid off (the pin was
  written as a range on purpose).
- The catalog was regenerated AND verified locally (entry at 1.6.0, 8 files with
  sizes/hashes), with the live publish correctly deferred to the owner - the
  landing scope the umbrella pinned. icon/screenshots empty is pre-existing
  (gauntlet is identical), not a regression - verified, not assumed.

## What went wrong / was tricky

- LOW: the doc-sweep grep scoped to `web/src/wiki/` and omitted `web/src/news/`,
  which does contain Ledger facts (in the dated `0.7.0.md`). The verdict for that
  file is correct (a per-release news post is append-only history, like the root
  CHANGELOG - you do not rewrite a shipped release's notes), but the SWEEP TABLE
  should have listed it with that reasoning rather than silently excluding the
  directory. A sweep that narrows its own scope can hide a miss; the honest form
  greps wide and records "left, because dated history".

## Lessons / what to do differently

- A `keep-docs-in-sync` sweep greps the WHOLE doc tree (wiki + news + READMEs +
  CHANGELOGs), then classifies each hit as fix / leave-because-history - it does
  not pre-narrow the grep to one subdir. The append-only surfaces (root
  CHANGELOG, dated news posts, `tasks/`) are LEFT with a reason, not excluded
  from the search. (Sharpens `keep-docs-in-sync-with-code`.)
- The single highest-value close-out check is "does the player-facing README/
  CHANGELOG claim match the shipped behavior of the headline feature" - verify it
  against the code, not the task brief.

## Follow-ups

- None blocking. The owner's Finish steps (live portal publish/push + native/web
  over-the-wire install, and the visual/feel replays) are batched in GOAL.md
  Manual acceptance.
