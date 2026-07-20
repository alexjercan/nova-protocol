# lessons: resolve 6 pending promotions

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,chore

## Story

As the maintainer, I want a decision pass over the 6 pending promotions (x3+)
in LESSONS.md, so that they get folded into guidance/tooling or retired. One of
them (`out-of-context-review-pass`, x31) is already de-facto flow Round 1
practice and should simply be annotated as promoted.

## Steps

- [ ] Review the 6 pending lessons (out-of-context-review-pass x31, prose-from-diff, render-output-eyeball, verify-stale-brief, authored-vs-derived-values, advertised-but-unwired).
- [ ] For each: promote (AGENTS.md / skill / tool) or retire; annotate with the promotion marker.
- [ ] Mark out-of-context-review-pass promoted (already flow practice).

## Definition of Done

- Every x3+ pending lesson is annotated promoted or retired (cmd: `tatr check --ledger LESSONS.md` clean).

## Notes

- render-output-eyeball and prose-from-diff-not-intent are good candidates for the work/review skills - consider filing generic fixes against nix.dotfiles.
