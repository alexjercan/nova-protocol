---
name: nova-review
description: Review a Nova change with a panel of parallel reviewer agents. Use only when the user requests Nova Review.
disable-model-invocation: true
---

# Nova Review

Dispatch parallel reviewer agents over one change range, then adjudicate their
findings in this session.

The invocation IS the request to dispatch subagents. The standing "no subagents
unless the user asks" directive does not apply inside this skill.

## Resolve the range first

Never fan out over an unresolved range.

- No argument: the range this session produced. Name it, state it, continue.
  Ask only when the session cannot name it.
- `<base>..<head>`: that range.
- `--task <id>`: the commits named in `tasks/<id>/TASK.md`.
- `--worktree`: uncommitted changes, tracked and untracked.

Stop above 2000 changed lines and offer a narrower range or a commit-by-commit
pass. `origin/master..HEAD` and the last release tag are both too wide to be a
default.

## Build the bundle once

Write the evidence to the scratchpad and give every lane the same paths. A lane
must not re-derive the range.

```bash
git log --oneline <range>
git diff --stat <range>
git diff <range>
```

Add the task body when the range belongs to a task.

## Dispatch the lanes

Send every lane in ONE message so they run concurrently. Give each the range,
the bundle paths, and two repo-relative brief paths to read: the shared
`.agents/skills/nova-review/lanes/reviewer.md`, and its own.

| Lane | Brief | When |
|-|-|-|
| Craft | `lanes/craft.md` | always |
| Performance | `lanes/performance.md` | always |
| Correctness | `lanes/correctness.md` | always |
| Contracts | `lanes/contracts.md` | always |
| Red team | `lanes/red-team.md` | `--play` |
| Feel | `lanes/feel.md` | `--play` |

- Reviewers are read-only. They report; they never edit, stage, commit, or fix.
- One lane measures at a time. Performance holds the measurement slot; red team
  and feel wait for it. A shared GPU turns a 93ms frame into 291ms.
- No lane runs the workspace test suite or Clippy.

## Adjudicate

Do this in this session, not in a lane.

- Drop every finding that is not grounded in the diff or the tree. A plausible
  smell is not a finding.
- Merge duplicates across lanes and keep the strongest evidence.
- Rank `BLOCKER`, `MAJOR`, `MINOR`. Record why a finding was not raised higher.
- Re-derive a load-bearing claim yourself before it reaches the verdict.

## Report

Give the verdict, the findings by severity, what was verified, and what was
skipped. Name each skipped check; a skip is not a pass.

Write `tasks/<id>/REVIEW.md` when the range belongs to a task, in the shape the
existing records use: round, reviewer, verdict, findings, verified, proofs
rerun, verdict rationale. Otherwise report inline and write no file.

Fixing is a separate step. Change nothing until the user picks the findings to
act on.
