# Retro: probe surface close-out (the strand's last layer)

- TASK: 20260719-211500
- BRANCH: feature/probe-closeout (squash-landed as f6733f36)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs)

## What went well

- The perf_web investigation at FILING time (perf.html's data-bin
  linkage) turned "remove this bin" into "keep it and make it
  unmistakable" BEFORE any code moved - the task body carried the
  finding, so implementation never even flirted with breaking the web
  capture. Investigating removals when the ask arrives, not when the
  knife is out, is the pattern.
- Retired verbs got pointed errors instead of silent removal - a
  three-line UX decision that converts every muscle-memory failure into
  a teaching moment. The pins assert the pointers, so they cannot rot
  into generic errors.
- Removing the aliases exposed Cmd::Run as producer-less - the compiler
  walked the dead code out (variant, dispatch arm, trace_table) with
  zero guesswork. Deleting the CALLERS first and letting rustc name the
  orphans beats hunting them by eye.
- Third stacked layer landed with the now-practiced rhythm: the
  stacked-squash TASK.md conflict was EXPECTED (both sides differ from
  the pre-squash merge-base), resolved to the landed version in one
  move, and the auto-merged docs were re-read for meaning.
- CHANGELOG-Unreleased-is-current-docs applied again: v0.8.0's notes no
  longer advertise aliases that never shipped in a release.

## What went wrong

- The sync merge was attempted with the implementation still uncommitted
  in the worktree (merge aborted harmlessly). Stacked branches make
  "commit before sync" a hard precondition, not a style preference -
  the abort cost one command, but on a dirtier tree it could have cost
  a confusing conflict state.

## What to improve next time

- In stacked flows, commit the layer's work BEFORE any sync merge, every
  time - treat an uncommitted worktree as unmergeable by definition.

## Action items

- [x] The probe strand is COMPLETE: spike, T1-T6, hardening,
      consolidation, multi-run aggregate, fleet wiring, depth markers,
      surface close-out. Two verbs, one report, the whole fleet
      evaluable.
- [ ] Queue returns to: content lint 20260718-152240 (p48), worktree
      ergonomics spike 20260719-002512 (p46), then the docs strand.
