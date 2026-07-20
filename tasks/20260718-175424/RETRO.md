# Retro: ephemeral docs/ model (compile-to-LESSONS + wipe + guard)

- TASK: 20260718-175424
- BRANCH: docs/ephemeral-docs
- REVIEW ROUNDS: 1 (APPROVE, two cosmetic NITs)

## What went well

- **Asked the two real forks up front, and it paid.** A big multi-part task with
  genuine decisions (automated-vs-agent compile; keep-live-content vs full-wipe)
  - one AskUserQuestion at the start changed what got built ("full wipe now"
  meant folding the LIVE v0.8.0 plan into a tracker task, not deferring it). The
  flow's "clarify a real fork at the cheapest moment" earned its keep.
- **Resolved the DoD's own contradiction explicitly.** "docs/README.md describes
  the model" and "docs/ holds only LESSONS.md" cannot both be literally true;
  instead of silently honoring one clause, I named the resolution (docs/ keeps
  TWO permanent meta files) and wired the wipe + guard to it. A DoD written
  before the work can contain a latent conflict - surface it, don't pick.
- **Out-of-context assessment + my verification.** An Explore agent triaged the
  9 design docs (wiki-already / migrate / distill / delete) in one pass; I
  spot-verified before acting (checked `SectionSource::Inline/Prototype` in
  spaceship.rs before writing the migrated wiki paragraph). Fast AND grounded.
- **Deleting a whole tree swept its references.** After removing docs/design +
  docs/plans I grepped the repo and redirected every live dangling pointer
  (config, Cargo desc, `//!` comments, AGENTS, wiki) to the wiki/lesson; the git
  rename detection (v0.8.0 plan -> tracker task, 97% similar) confirmed the
  content moved intact rather than vanishing.
- **Verified the mechanism, not just wrote it.** Ran the wipe (idempotent), the
  guard (exit 0 clean / 1 on junk), and `npm run ci` (wiki renders) before
  landing.

## What went wrong

- Nothing material. One process wrinkle: I first ran `npm run ci` without
  `npm install` in the fresh worktree (prettier not found, exit 127) - the same
  fresh-worktree gotcha as a prior cycle; re-ran with install and it was green.

## What to improve next time

- In a fresh sprout worktree, `npm install` is a prerequisite for `npm run ci` -
  bundle them (`npm install && npm run ci`) rather than assuming node_modules.

## Action items

- No new ledger slug: the tree-delete reference sweep is another instance of the
  promoted `sweep-then-delete` (x11); the DoD-contradiction resolution is a
  one-off worth remembering but not yet recurring. Both recorded here.
- The model is live: future cycles write freely in docs/, distil to LESSONS.md +
  wiki at release, run scripts/wipe-docs.sh; the release guard enforces it.
