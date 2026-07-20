# lessons: resolve 6 pending promotions

- STATUS: IN_PROGRESS
- PRIORITY: 0
- TAGS: backlog, chore

## Story

As the maintainer, I want a decision pass over the 6 pending promotions (x3+)
in LESSONS.md, so that they get folded into guidance/tooling or retired. One of
them (`out-of-context-review-pass`, x31) is already de-facto flow Round 1
practice and should simply be annotated as promoted.

## Steps

- [x] Reviewed the 6 pending lessons (out-of-context-review-pass x31, prose-from-diff, render-output-eyeball, verify-stale-brief, authored-vs-derived-values, advertised-but-unwired).
- [x] Promoted each: 5 folded into AGENTS.md Conventions, out-of-context-review-pass -> flow round-1; all annotated. None retired.
- [x] Marked out-of-context-review-pass PROMOTED (already flow round-1 practice).

## Definition of Done

- Every x3+ pending lesson is annotated promoted or retired (cmd: `tatr check --ledger LESSONS.md 2>&1 | grep -c promotion-stalled` -> 0).

## Notes

- render-output-eyeball and prose-from-diff-not-intent are good candidates for the work/review skills - consider filing generic fixes against nix.dotfiles.
