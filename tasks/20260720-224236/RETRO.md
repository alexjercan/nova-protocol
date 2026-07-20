# Retro: relocate nova_meta_gen to tools/

- TASK: 20260720-224236
- BRANCH: refactor/meta-gen-to-tools (landed 6f41f47a)
- REVIEW ROUNDS: 1 (out-of-context APPROVE, 1 NIT fixed)

See TASK.md Outcome for what changed; process only here.

## What went well

- The spike's up-front finding did its job: because round 2 established that
  feature unification (the `wav` feature) REQUIRES workspace membership, the
  design was obvious before any code moved, and the one real risk had a
  concrete pass/fail proof - run the tool from `tools/` and check `.wav.meta`
  is written. An abstract "unification might break" became a one-command gate.
- The out-of-context reviewer independently re-ran BOTH key risks (the
  `.wav.meta` write and the `default-members`-vs-`members` cross-check for a
  silently-dropped game crate) rather than trusting the close-notes - exactly
  the checks that matter for a workspace-config change.
- `git mv` kept history (the four files landed as renames), and applied the
  prior lesson: committed REVIEW.md on the branch before `sprout land` so it
  rode the squash.

## What went wrong

- R1.1 (NIT): my DoD scoped the doc sweep to README + AGENTS + 152304, but the
  crate table also lives in `web/src/wiki/dev/project-tour.md` and
  `architecture.md`, which still listed `nova_meta_gen` among the game crates.
  Root cause: I hand-picked a subset of doc surfaces instead of grepping the
  NAME tree-wide. This is the THIRD same-session instance of a doc sweep missing
  a surface (152240: source `//!` comments; 231555: a mod README; here: two
  more wiki crate tables).

## What to improve next time

- A "crate table" is not one place - it appears in README, AGENTS, AND multiple
  wiki pages (project-tour, architecture). When a crate moves/renames, grep the
  crate NAME across the WHOLE tree and fix EVERY table, don't enumerate a
  subset. The recurrence-despite-enforcement (keep-docs-in-sync is already
  enforced in AGENTS.md and still reached x7) says prose is not holding this -
  a doc-surface lint / `tatr check`-style grep guard is the real fix.

## Action items

- [x] LESSONS.md: bumped `keep-docs-in-sync-with-code` to x7 with this instance;
  added the "recurs despite enforcement -> needs a tooling guard, not more
  prose" note.
- [ ] Candidate follow-up (not filed): a doc-surface sweep check (grep the
  moved/renamed symbol across README + AGENTS + web/src/wiki + skill files),
  runnable in CI or `tatr check`. Raise if it recurs again.
