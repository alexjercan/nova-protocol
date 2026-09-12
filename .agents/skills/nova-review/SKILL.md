---
name: nova-review
description: >-
  Review a Nova change with parallel reviewer agents.
  Use only when the user requests Nova Review.
disable-model-invocation: true
---

# Nova Review

The request authorizes reviewers, not edits. Read [Verify](../verify/SKILL.md).
If reviewer tools are unavailable, report that limit; do not claim a panel.

## Resolve and bundle

- Default: this session's change. Name it; ask only if the range is unclear.
- `<base>..<head>` selects commits; `--task <id>` uses its TASK.md commits.
- `--worktree` includes tracked and untracked changes.
- Above 2000 changed lines, offer a narrower or commit-by-commit review.
  Never default to `origin/master..HEAD` or the last release.
- Give every lane one scratch bundle: log, diff stat, full diff, untracked
  contents if in scope, and the task body if present. Do not re-derive ranges.

## Dispatch

Send lanes concurrently with the resolved range, bundle paths, and briefs:
- All read `lanes/reviewer.md` and their own brief under `lanes/`.
- Always run `craft.md`, `performance.md`, `correctness.md`, and `contracts.md`.
- Add `red-team.md` and `feel.md` for `--play`; supply steps and run budgets.
- Reviewers never edit, stage, commit, or fix. No workspace tests or Clippy.
- One lane runs the game at a time. Performance goes first; play lanes wait.

## Adjudicate here

Drop ungrounded claims, merge duplicates, and recheck load-bearing evidence.
Rank BLOCKER, MAJOR, MINOR; explain why each finding is not rated higher.
Report the verdict, findings, evidence, and named skipped checks.
For a task, write `tasks/<id>/REVIEW.md` in its existing report format;
otherwise report inline. Change no code until the user selects findings.
