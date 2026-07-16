# Retro: The Ledger - four-chapter campaign mod

- TASK: 20260716-123535
- BRANCH: content/the-ledger (landed 7251d4fd)
- REVIEW ROUNDS: 2

## What went well

- Spike-then-author worked exactly as designed: every chapter beat named
  its proven source pattern in SPIKE.md, so 1700 lines of RON were
  mostly transcription, validated chapter-by-chapter (fail-fast after
  each file).
- The review round earned its keep on content: reading the ENGINE
  contract (salvage.rs: "the scenario despawns the crate itself") found
  a real double-count/soft-visual bug the load gates can never see.
- The generic gates from the decoupling flow covered the new mod with
  zero test changes: webmods_validation and the portal generator picked
  up the-ledger automatically.

## What went wrong

- The one real authoring bug (nonexistent "basic_hull_section") came
  from typing a prototype id from memory; the gates cannot catch it
  (spawn-time resolution). Cost: one near-miss caught by an ad-hoc
  catalog grep - now a scripted cross-check and a seeded lint task.
- The crate-despawn pairing was missed on first authoring because I
  copied the shakedown OnEnter shape from the BEACON handlers (which do
  not despawn) instead of its crate handlers.

## What to improve next time

- Content authoring gets the same verify-first rule as code: every
  cross-file identifier (prototype ids, chain targets, filter ids) is
  swept by script BEFORE review, not trusted from memory.
- When copying a handler shape, copy it from the handler with the same
  OBJECT KIND, not the same event name.

## Action items

- [x] Seeded tatr 20260716-191543: prototype-reference lint for the mod
      gates.
- [x] SPIKE.md fix record updated (both implementing tasks landed).
- [ ] First human playthrough = the tuning pass (wave sizes, boss
      toughness, channel readability, ending feel) - open invitation,
      not this session's to close.
