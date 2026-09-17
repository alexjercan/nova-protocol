---
name: nova-review
description: Review a Nova change with parallel reviewer agents.
disable-model-invocation: true
---

# Nova Review

Use only when the user requests Nova Review. The request authorizes reviewers,
not edits. If reviewer tools are unavailable, report that limit; never claim a
panel that did not run.

## Resolve and bundle

- Default to this session's change. Name it; ask only when the range is unclear.
- `<base>..<head>` selects commits; `--task <id>` uses its task commits;
  `--worktree` includes tracked and untracked changes.
- Above 2000 changed lines, offer a narrower or commit-by-commit review. Never
  default to `origin/master..HEAD` or the last release.
- Give each lane one scratch bundle: log, diff stat, full diff, untracked files
  in scope, and the task body when present. Do not re-derive ranges per lane.

## Dispatch

Send lanes concurrently with the resolved range, bundle paths, and briefs:

- All lanes read `lanes/reviewer.md` and their own brief under `lanes/`.
- Always run `craft.md`, `performance.md`, `correctness.md`, and `contracts.md`.
- Add `red-team.md` and `feel.md` for `--play`; supply steps and run budgets.
- Reviewers never edit, stage, commit, or fix. Do not run workspace tests or
  Clippy. Run one game or GPU measurement at a time; performance goes first.
- Review replaced behavior for leftovers: adapters, aliases, dual paths,
  implicit defaults, obsolete tests, stale comments, and stale documentation.

## Adjudicate here

Drop ungrounded claims, merge duplicates, and recheck supporting evidence. Rank
BLOCKER, MAJOR, or MINOR; explain why each finding is not rated higher. Report
the verdict, findings, evidence, and named skipped checks.

For a task, write `tasks/<id>/REVIEW.md` in its report format. Otherwise report
inline. Change no code until the user selects findings.
