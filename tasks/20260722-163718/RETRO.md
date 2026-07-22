# Retro: per-beat objective pacing gaps

- TASK: 20260722-163718
- BRANCH: fix/per-beat-pacing-gaps (merged to master, ff)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

See TASK.md / NOTES.md for what changed. Process observations only.

## What went well

- The compounding paid off: this task's whole reason to exist was fixing the
  PRIOR task's stale-RON miss discipline. This time I ran `content gen` and the
  `content_ron_parity` integration test (via `--test`, which `--lib` skips)
  BEFORE landing - the two ledger lessons from last cycle
  (`edit-the-builder-not-the-generated-ron`, `local-merge-skips-the-guarding-ci`)
  worked exactly as intended.
- Applied `would-it-fail-without-it` proactively: noticed `settle_beat` advances
  past the LONGEST gap, so the existing walk would pass even under a uniform
  gap - it did not pin the split. Added a focused test that advances only
  `INSTRUCTION_GAP` and posts the objective, which fails if reverted. Caught my
  own test-meaningfulness gap before review.
- The heavy design judgment was done up front by the out-of-context pacing
  review, so implementation was mechanical classification - and cheap to verify
  (grep the stamp sites, count the RON gap literals: 8x4.0 + 2x6.0).
- Honest about the probe situation instead of claiming a clean run: verified the
  lifeline RON is byte-identical to master, which proves the render OOM is
  environmental (an unchanged scenario OOMs the same way).

## What went wrong

- A `replace_all` on `stamp_gate()` set the OnStart SEED stamp to
  INSTRUCTION_GAP, contradicting TASK.md's "the seed stays as-is". Harmless (the
  seed is overwritten by the 1->2 transition before any gate reader fires; the
  reviewer confirmed), but it is plan-vs-diff drift a bulk edit introduced
  silently. Root cause: `replace_all` cannot distinguish the one site the plan
  excluded.
- Could not get clean probe evidence for the heavy combat scenes: lifeline and
  broadside both reach Playing then hit an identical wgpu render OOM at frame 47
  under software rendering. Environmental (22Gi RAM free; the lighter
  menu_newgame ran clean), but it means the local gameplay-probe signal for
  combat scenes rests on reached_playing + unit tests + CI.

## What to improve next time

- After a bulk `replace_all`, reconcile the touched sites against the plan - if
  the plan excluded a site, verify the bulk edit did not silently change it (or
  amend the plan). This is `reconcile-plan-to-shipped` at the edit granularity.

## Action items

- [x] Ledger: bumped `gpu-example-local-skip` (heavy scenes OOM'd locally
  again - one smoke attempt, rely on CI).
- [ ] Manual (batched on the umbrella): owner playtests shakedown - do the
  instruction objectives land as you read to the keypress?
