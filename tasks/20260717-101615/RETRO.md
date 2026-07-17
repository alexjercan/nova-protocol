# Retro: Split the sound bank into UiSfx and WorldSfx

- TASK: 20260717-101615
- BRANCH: task-20260717-101615-sound-bank-split (squash-landed cad93a16)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

A smooth cycle; short retro to say so. See TASK.md/REVIEW.md for detail.

## What went well

- Prior lessons applied proactively paid off: the R1.3 finding from 002228
  (rigs mirroring production load paths) became the `load_world_sfx_bank`
  helper design UP FRONT - one path-convention site shared by production and
  all rigs, instead of three copies; `piped-cargo-masks-exit-code` meant every
  cargo check was read via output grep, not pipe exit; the crate-solo-tests
  lesson (x4) made the nova_scenario run `--features serde` on the first try.
- Scripted-edit asserts caught real drift: the CHANGELOG python edit
  assert-aborted because a parallel task had already created the Unreleased
  Modding section - the assert turned a would-be duplicate section into a
  clean hand-merge (verify-scripted-edits-applied working as designed).
- The regen check (run gen-placeholder-sounds.py, assert git shows only the
  staged renames) proved the script split byte-stable in one cheap step.

## What went wrong

- Discovered a docs miss in the PREVIOUS task (20260717-002228): its plan
  named CHANGELOG + wiki-modding-guide updates, the docs step was ticked, but
  neither was touched - both my self-review and the independent reviewer
  checked the docs that WERE changed rather than diffing the docs list the
  plan promised. Caught here only because this task rewrote the same story.
  Folded the missing lines in (CHANGELOG modding entry, wiki turret
  `fire_sound` field). Root cause: a ticked checklist item was verified by
  looking at the diff, not by re-reading the item's own enumerated targets.

## What to improve next time

- When reviewing a task's docs step, re-read the step's OWN list of promised
  surfaces and check each one against the diff - a docs step that names 5
  surfaces and touches 4 looks complete in the diff view.

## Action items

- [x] docs/LESSONS.md: keep-docs-in-sync-with-code bumped x3 -> x4 with the
  002228 occurrence.
