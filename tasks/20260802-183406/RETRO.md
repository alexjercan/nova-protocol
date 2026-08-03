# Retro: Retire the BCS harness surface and refresh the automation docs

- TASK: 20260802-183406
- BRANCH: docs/retire-bcs-harness-surface
- REVIEW ROUNDS: 2

## What went well

The retained-`bevy_common_systems` audit landed as a table in the close-out
rather than as a claim, so the next reader gets the surviving list without
re-deriving it - and both review rounds could check it against a live sweep in
one grep. Linking `development.md` to the `automation-harness` page instead of
restating the env table kept one contract with one home.

Diff stayed proportionate: prose plus a CHANGELOG entry plus one stale test
comment. No production code changed, which is what a retirement pass should
look like when the migration task ahead of it did its job.

## What went wrong

**The absence proof hid half the repository.** The DoD's sweep was a plain
`rg`, which skips dot-directories by default. It reported green while
`.claude/skills/probe/SKILL.md` still taught `BCS_HARNESS_DEADLINE` - an env
`nova_probe` no longer sets, so the documented override was dead - and
`.github/workflows/ci.yaml` and `.gitignore` still named `BCS_AUTOPILOT` and
`BCS_SHOT`. That was the round-1 BLOCKER. The decision seemed sound when
written: every surface the plan could name lived under `web/`, `AGENTS.md` or
`CHANGELOG.md`, and `rg`'s default felt like "search the repo".

**Two proof clauses were unsatisfiable as planned.** `debug::harness` is a
substring of Nova's own `nova_debug::harness` adapter, and `CHANGELOG.md` must
spell the dead names its breaking entry documents. Both were diagnosed and
narrowed at work time and recorded in DECISION.md - but the narrowing happened
after the PLAN gate, which is a plan-time miss, not a work-time one.

**DECISION.md ignored the tatr record schema**, leaving `tatr check` red with
ten errors against a convention all 50 other decision records follow. Caught in
round 1 as MAJOR.

**Round 2 found the "one searchable home" claim was half true.** The wiki told
readers the CHANGELOG spells out the old names; the entry only carried the
`BCS_* -> NOVA_*` glob, so `BCS_SHOT` and `BCS_REEL` were greppable nowhere in
the live tree. A stuck script would have found nothing.

## What to improve next time

**Breadth.** The diff did not grow; it grew *sideways* twice, into `.claude/`
and `.github/`, because the Step list enumerated files by hand (`web/`,
`AGENTS.md`, `CHANGELOG.md`) instead of deriving them from the sweep the DoD
already specified. When a task's DoD is an absence proof, the Steps should be
"fix every hit", not a hand-copied file list - the proof is the work-list.

**Churn.** Both review rounds trace to one plan-time question never asked of
the DoD command itself: *run this proof on the base branch and read every
hit*. Doing that would have shown the dot-directory blind spot (R1.1, R1.3,
R1.5), the `debug::harness` ambiguity, and the CHANGELOG exclusion, all before
the gate. The task's Notes did record a base-branch proof status ("hits ~35
files"), but as a count rather than a read list, so the missing surfaces were
invisible in the summary. A red proof's *hits* are plan-time evidence; its
*count* is not.

**Context.** No pressure observed. No checkpoint, compaction warning or
delegation beyond the two out-of-context review rounds, both of which returned
inside their budget.

## Action items

- Any repo-wide `rg` used as a DoD criterion carries `--hidden` and an explicit
  `--glob '!.git/**'`. A sweep that silently skips `.claude/`, `.github/` and
  dotfiles reads green while the contract is still wrong somewhere a reader
  will find it. Submitted to central knowledge.
- Absence proofs match fully-qualified paths (`bevy_common_systems::x::y`), not
  bare module tails, when the tail is or may become a substring of a local
  module. Submitted to central knowledge.
- Task-record references to another file's content name the content (a heading,
  the entry's opening words), not a line number, when the same task edits that
  file. Round 2's R2.2 was a line reference the task's own CHANGELOG edit
  invalidated.

## Landing message

```
docs(autopilot): retire the BCS harness surface from the prose

Every live doc surface now teaches the NOVA_* harness contract:
development.md, guide-add-section.md, automation-harness.md, AGENTS.md,
the probe skill, the CI smoke comment and .gitignore. The CHANGELOG
records the nova_autopilot crate and spells all four BCS_* -> NOVA_*
renames as a breaking change for anyone with a scripted run.

The pinned bevy_common_systems dependency stays for gameplay, the
inspector and the wireframe pass; the surviving imports are tabulated in
the task's close-out. No bevy_common_systems harness or completion
import remains.
```
