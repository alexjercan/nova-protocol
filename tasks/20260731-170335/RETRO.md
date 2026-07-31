# Retro: KISS: nova_gameplay HUD - chrome and objective surfaces

- TASK: 20260731-170335
- BRANCH: refactor/kiss-hud-chrome
- REVIEW ROUNDS: 2

## What went well

Widening DoD 3 before starting, not after. The sibling (20260731-170329)
shipped a `//.*<tatr-id>` grep and review found four provenance clauses it
structurally could not see. This task's DoD carried the widened form (tatr
IDs, bare `YYYY-MM-DD`, `review Rn.n`, `docs/`) from the plan, and it
immediately earned its keep: `(owner playtest 2026-07-30)`, `(playtest round
4)` and `measured ... on 2026-07-30` would all have survived the narrow form.
Widening the PROOF to match the claim, rather than narrowing the claim to
match the proof, was the cheap fix.

Reducing "no behavior change" to one re-derivable number. Every changed line
in the diff starts with `//`, `///` or `//!` except one deleted blank line.
That is a single command the reviewer ran independently, twice, instead of
re-reading 134 hunks of judgment calls.

Taking the DoD 4 exception rather than inventing a seam. `keybind_dock.rs`
(1911) was the obvious split, and the evidence said no: `keycap_sizing_tests`
exists to assert the dock and the verb cues size keycaps through one path,
and it does `use super::tests::{...}`. Splitting means breaking that
guarantee or extracting a shared rig, which is the new abstraction this pass
forbids. NOTES names the seam for whoever splits it later.

## What went wrong

**A mechanical ID-strip hit a live deferred-work marker.** The pass treated
`(20260710-231927)` in `key_glyphs.rs` as provenance and demoted it to a bare
`NOTE:`. It was deferred work, and the epic rubric's deferred-work row says
to KEEP the ID; 20260710-231927 is still OPEN, so the surviving note pointed
at nothing. The decision seemed sound because the other hits in the same
sweep really were provenance and the two shapes are textually identical - the
difference is whether the referenced task is open and whether the sentence
describes work not yet done. Classify each hit by that question before
stripping; a sweep is the wrong instrument for a judgment.

**Fixing the wrapping falsified the records.** The R1.4 rewrap shortened
`flight_status.rs`, `objective_feedback.rs` and `objective_stack.rs` by one
line each. NOTES held a 15-row measured table and a 12-row marker inventory
with line numbers, all measured BEFORE the reflow, so three rows and three
markers silently stopped reproducing - a MAJOR in round 2. Measuring right
after the pass felt like the careful thing to do; the failure is that a
record full of measured numbers has to be re-measured after the LAST edit to
the files it measures, not after the edit that motivated the measurement.

**Two smaller record defects of the same family.** A stated count (143) came
from a wider pattern than the sentence named (140 for the four listed
categories), and a stated method ("first `#[cfg(test)]`") did not reproduce
`mod.rs`'s own row. Both were written from memory of how the number was
obtained rather than from the command that obtained it.

## What to improve next time

- Before a scripted provenance strip, split the hits into "provenance" and
  "deferred work with an open task" and hand-handle the second set. The tatr
  ID's presence is not the signal; the referenced task's STATUS is.
- Re-measure every figure in a record after the last edit of the round, and
  keep the command that produced it next to the number so the next reader can
  re-run rather than re-derive.
- Post-substitution, grep the added comment lines for over-fill length AND a
  leading `- ` before reading anything. That two-command check found all four
  ragged lines and the CommonMark hazard here.

## Diagnosis

**Breadth.** 15 files but 240 insertions / 271 deletions, comment text only.
The epic had already sliced the HUD into three children by surface and this
was one of them; the diff is wide-but-thin by the nature of a comment pass,
not by a missed split.

**Churn.** Both rounds' findings were the pass's own fixes, not the plan's
design. No plan-time question would have prevented R1.1 (it needs the tatr
STATUS of a task the plan never names) or R2.1 (it needs the reflow to have
happened first). What would have prevented them is a verify-step habit:
re-measure last, and classify before stripping. Both are recorded as lessons
rather than plan changes.

**Context.** This session compacted twice, and the handoff after the first
compaction asserted two things the tree did not support: that the record was
at FLOW STEP PLANNED (it was already WORKING) and that a prior substitution
had duplicated a clause in `beacon_chips.rs` (re-read both edited regions; no
duplication). Verifying both against the tree instead of acting on the
summary cost two commands and avoided a wrong flow transition and a wrong
"fix". A handoff summary is a claim about the tree, not the tree.

## Action items

- 20260731-232634 (backlog) - `holo_instruments`'s ribbon doc may describe a
  superseded state now that the gravity-aware arrival task (20260710-193500)
  is CLOSED. Deferred per the epic's out-of-scope rule; checking it means
  reading the autopilot's arrival solve.
- Process signal for landing: the branch carries d2c3d333 and 13da1583, whose
  patch-ids match master's link-RAM fix. Rebase before merge so the history is
  not duplicated.
- Process signal: `tatr proofs 20260731-170335` returns an empty list (exit 0)
  despite six DoD proof lines, so the reviewer enumerated proofs by hand.
  Worth checking whether the proofs parser handles this DoD block shape.
