# Retro: document modding meta-conventions

- TASK: 20260718-231601
- BRANCH: docs/modding-meta-conventions (landed f77e8bbe)
- REVIEW ROUNDS: 1 (out-of-context APPROVE, zero findings)

See TASK.md for what was documented; process only here.

## What went well

- Delegated the docs work to a subagent with STRONG code pointers (the task
  spec named file:line for every "why"), and required verify-then-write. The
  subagent honestly flagged two task-premise discrepancies rather than parroting
  them: Gauntlet is 1.2.0 (a minor bump), not the "2.0" the bundle comment
  loosely says, and convention 5 was already documented so it made no change.
- The out-of-context reviewer verified all five "why" claims against code with
  file:line citations and re-ran `npm run ci` - a docs task graded on
  factual accuracy, not prose polish, which is exactly what the risk is. Zero
  findings on a first round because the implementer verified first.

## What went wrong

- Process slip (mine, orchestration): I authored REVIEW.md into the WORKTREE but
  did not commit it on the branch before `sprout land`. The land squashes only
  committed branch state, so REVIEW.md did not ride the squash, and `sprout
  land` then removed the worktree (the "fatal ... use --force" was only the git
  worktree-remove step; sprout still cleaned the dir), taking the uncommitted
  file with it. Recovered by recreating REVIEW.md on master. Root cause: treated
  "write the file" as done without the commit, and landed in the same breath.

## What to improve next time

- Commit REVIEW.md on the feature branch as its own step BEFORE `sprout land`,
  and confirm `git status` is clean in the worktree before landing - an
  uncommitted review/retro file is silently dropped by the squash and lost with
  the worktree. This is the review skill's "commit REVIEW.md after each round"
  rule; the failure mode is losing it entirely, not just a missing commit.

## Action items

- [x] LESSONS.md: added `commit-review-retro-before-land` (x1).
- [x] REVIEW.md recreated on master (post-land).
