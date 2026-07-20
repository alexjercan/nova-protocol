# Retro: static mod portal - generator, wire schema, first webmods mod

- TASK: 20260715-142900
- BRANCH: feature/mod-portal (landed on master as 315ccde4)
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES with one MAJOR, R2 APPROVE)

## What went well

- Two plan-time design calls prevented expensive reworks: asking "what will the
  deploy job have to COMPILE" produced the engine-free nova_mod_format split
  before any generator code existed (the re-export made the extraction
  invisible to every consumer), and grepping demo's test entanglement before
  honoring the spike's "demo moves online" produced a scope correction instead
  of a mid-implementation unravel.
- Applying the ledger up front: the determinism test exists BECAUSE
  verify-generator-stability-before-commit-diff is in LESSONS.md; the reviewer
  then sharpened it further (sorted-output assertions beat probabilistic
  byte-identity at small N).
- The reviewer's toolchain investigation (stable-toolchain action vs the
  nightly rust-toolchain.toml pin) settled a deploy-job risk by reading the
  ACTION'S source - the verify-engine-guarantees-in-source habit applied to CI
  actions.

## What went wrong

- R1.1 (MAJOR): the content-path check validated EXISTENCE on the filesystem
  when the question was MEMBERSHIP in the set the portal serves - an escaping
  `../` path existed, passed, and was never copied, so a broken mod published
  with exit 0. Root cause: the check was written against "does the author's
  path resolve" instead of "will the client be able to fetch this"; the two
  differ exactly on paths that leave the served tree. The reviewer proved it
  empirically before claiming it.
- Two promised test assertions (entries-sorted; a duplicate-id case) quietly
  degraded between plan and implementation - one forgotten, one impossible by
  construction but unnoted. Both were caught only by review cross-checking the
  plan against the tests.

## What to improve next time

- When validating user-supplied paths for a serve/copy pipeline, validate
  against the set that will actually be served (membership), never bare
  filesystem existence.
- When a planned test case turns out impossible or gets dropped, say so in the
  step at tick time - silent substitutions read as forgotten coverage.

## Action items

- [x] LESSONS.md: new lesson `validate-membership-not-existence`; bumped
  `out-of-context-review-pass` and `verify-generator-stability` gains the
  sorted-assertion variant.
