---
name: plan-release
description: Groom the tatr backlog against a stated goal and assemble a versioned release from it - triage stale tasks, create/merge tasks so the set matches the goal, then produce a `vX.Y.Z, release, meta` tracker task plus a tagged+prioritized+estimated member set, and stop at an awaiting-kickoff state. Use when the user says "plan vX.Y.Z with the goal of <X>" (or "groom the backlog into a release", "scope the next release"). This is release-level SCOPING - it does NOT define per-task DoD/Steps, spike anything, or cut a worktree; those wait for an explicit kickoff.
---

# Plan Release - Groom the Backlog Into a Versioned Release

Turn "plan vX.Y.Z with the goal of <X>" into a scoped release: read the whole
open backlog, reconcile it against the goal, close what is dead, create or merge
what the goal needs, and leave behind a release **tracker task** plus a tagged,
prioritized, estimated member set - then STOP and wait for kickoff. This skill
owns the SCOPING pass only. Per-task Definition of Done, Steps, and any spikes
are the NEXT pass (`/understand` for a spike, then `/plan`, then `/flow`), run
on the user's explicit go. There is no `/spike` skill - investigation lives in
`/understand`, which scaffolds the task's `SPIKE.md`.

This is the release-level front half of `/flow`: `/flow` step 1-2 is "pin the
goal, plan into tasks, gate". Here the "plan" is backlog grooming + a tracker,
and the gate is the awaiting-kickoff pause.

## Read first

- **`AGENTS.md`** (repo root) - the scheduling-tag rule is load-bearing: every
  task carries EXACTLY ONE of `backlog`(p0) or the active `vX.Y.Z` tag, with
  priority slotted RELATIVE to that release's tasks; topical tags
  (`feature`,`bug`,`hud`,...) come on top. Pulling a backlog task into a release
  = swap the tag, re-slot the priority.
- **The previous release tracker** - the format precedent. Find it with
  `tatr ls -f ':tags contains meta'`. Reference shapes: `20260724-083631`
  (v0.9.0, a full feature release) and `20260802-000300` (v0.9.1, the lighter
  patch shape).
- **The central knowledge repo** (`/home/alex/personal/agent-knowledge`) -
  `knowledge list`, then `knowledge show <type>/<slug>` on anything matching the
  goal's subsystem words. The planning/ lessons are the relevant family here
  (`a-planned-proof-must-name-a-real-command`, `every-consequence-needs-a-step`,
  `rejected-alternatives-are-requirements`). Advisory only - an unavailable
  knowledge repo never blocks the grooming.

## Steps

### 1. Pin the goal and the version

Restate the goal in one or two sentences and name the version. If the goal is
genuinely ambiguous (a real fork in what the release is about), ask now - this
is the cheapest moment.

### 2. Survey the whole open backlog

```sh
tatr ls -s priority -f "(not :status eq CLOSED)"
```

Read EVERY open task (`tatr show <id>`). For a large backlog, delegate the
reading to a subagent that returns a terse per-task inventory: title, tags,
1-2 line summary, spike-vs-concrete, cross-references to other task IDs, and
staleness signals. Cross-check with `git log --oneline -40` (what shipped
recently) and resolve every referenced ID's status - a task gated on now-CLOSED
work may be unblocked, orphaned, or superseded.

### 3. Triage against the goal

Sort every open task into:

- **Aligned** - directly serves the goal; a release candidate.
- **Adjacent** - same area, could support the goal; flag as maybe.
- **Stale / removable** - dead (owner said not pursuing), superseded (a chosen
  alternative already shipped), wrong-repo (work lives in a sibling repo, this
  one only bumps a pin), premise-removed (its dependency was closed wontdo), or
  blocked-on-user-not-code with no near-term path. Recommend CLOSE.
- **Mergeable** - two tasks that are the same question and architecturally
  inseparable (their own bodies often say "may merge"). Recommend merge into one.
- **Gaps** - parts of the goal NO open task covers. These need NEW tasks.

### 4. Present the shape and get the cut (confirmation gate)

Before mutating anything, present:
- the recommended CLOSES (with the one-line reason each),
- the recommended MERGES,
- the NEW tasks the goal needs but the backlog lacks,
- the proposed release shape: **Committed** strands, **Stretch** (cut-first),
  and **Out of scope**, in the user's stated preference order.

Get an explicit yes on the cut. Do NOT close, retag, or create on your own
judgment of "stale" - closing is the user's call (creating new tasks to fill a
goal gap is expected and fine). If they reshape the cut, re-present.

### 5. Execute the grooming

On confirmation:

- **Create the gap tasks** (`tatr new -b <body-file>`). Give each a Story and a
  paragraph of context; leave DoD/Steps light - the next pass authors them. Tag
  them into the release directly (or `backlog` if they are out-of-scope
  follow-ups surfaced during triage). One `tatr new` per Bash call.
- **Merge** inseparable pairs: close the secondary with
  `tatr close -x SUPERSEDED -O <survivor-id> <id>`, and append a note to BOTH
  (the closed one points to the survivor; the survivor records what it
  absorbed). Two tasks that are one architecturally inseparable question will
  re-merge on their own during work - do it here, on purpose, instead.
- **Close** the agreed stale tasks with a resolution, never by hand-editing
  status: `tatr close -x WONTDO -R "<why>" <id>` (dead / premise-removed /
  wrong-repo), `-x SUPERSEDED -O <id>` (a chosen alternative shipped), or
  `-x DUPLICATE -O <id>`. Append a `## Closed (<date>, <reason>)` note spelling
  out WHY. Never tick undone Steps to make a lint go green - the work is not
  done, and honest reporting beats a green check. New tasks do not get
  `tasks/EXEMPTIONS.md` entries; that list is historical records only.
- **Retag + prioritize + estimate** the member set: for each,
  `tatr edit <id> -t vX.Y.Z,<topical...> -p <priority>`. Priority encodes ORDER
  (higher = earlier / gates more); slot the gate spikes highest, independent
  small wins next, dependent features below, stretch lowest. Record a rough SIZE
  (S/M/L) per task - it lives in the tracker body, not in tatr.

### 6. Write the release tracker task

`tatr new "vX.Y.Z release tracker: <theme> - <one-line>" -t vX.Y.Z,release,meta
-p 1 -b <body-file>`. Body follows the previous tracker's shape:

- Header: `DATE`, `BASE` (master at the last version + head short-sha), `THEME`
  (a named, memorable release theme + what it delivers, and whether it is a
  features release or debt paydown).
- **Why this scope** - what the last release left, why this goal now; the
  owner's focus in preference order.
- **In scope, in execution order** - grouped into STRANDS. Each line:
  `**<task-id>** (p<priority>, SIZE <S/M/L>, <tags>) <title + one-line note>`,
  with dependencies called out (which task gates which). Mark the STRETCH strand.
- **Out of scope** - the backlog tasks NOT pulled, and the ones closed this
  pass, each with a one-line reason.
- **Planning - next step (pending owner OK)** - explicitly list what this skill
  did NOT do: spike the gate task(s) via `/understand`, author per-task DoD +
  Steps via `/plan` (each item naming its proof `test:`/`cmd:`/`manual:`, and a
  proof must name a REAL command), decide the stretch in/out once the
  headline's real size is known, then the `/flow` gate.
- **Definition of done (release-level; filled at planning)** - a skeleton, one
  bullet per strand, authored properly in the next pass. Do NOT invent DoD here.
- **Grooming history** - a dated entry recording the triage: what was closed,
  merged, created, and the assembled set.

Note in the tracker that THIS project's release convention is the
`vX.Y.Z, release, meta` tracker task, NOT `/flow`'s `GOAL.md`.

### 7. Verify and pause at awaiting-kickoff

- `tatr ls -s priority -f ":tags contains vX.Y.Z"` - confirm the set reads right.
- `tatr check` - the tree is not clean baseline-wide (hundreds of historical
  records sit behind `tasks/EXEMPTIONS.md`), so diff against what you found:
  fix only findings YOU introduced. The live rules are `bad-record-schema`,
  `bad-proof-syntax`, `inconsistent-gates`, `closed-missing-review`,
  `closed-missing-retro`, `unused-exemption`.
- Report: the theme, the member table (task | strand | prio | size), what was
  closed/merged/created, and the sequencing (what gates what).
- The tracker + task changes are UNCOMMITTED in the main checkout. Offer to
  commit just the `tasks/` paths (stage explicit paths, never `git add -A` in
  the shared checkout); committing is the user's call.
- **STOP.** Do not `/understand`, `/plan`, `/work`, or sprout anything. Wait for
  the user's explicit kickoff ("start the spike", "plan it", "go") before the
  next pass.

## Guardrails

- Scoping only. No DoD authoring, no spikes, no code, no worktree - all deferred
  to the kickoff pass.
- Never close on your own judgment - recommend, then close on the user's yes.
  Creating new tasks to fill a genuine goal gap does not need a separate yes; it
  is the point of the skill.
- Never tick undone Steps to satisfy `tatr check`. Close with a resolution
  (`tatr close -x ...`) and let the record say the work stopped.
- Honor the one-scheduling-tag rule and relative priorities (AGENTS.md).
- Keep the trail on disk: closes carry a reason note, the tracker carries the
  grooming history, so the scoping is resumable from the files alone.
