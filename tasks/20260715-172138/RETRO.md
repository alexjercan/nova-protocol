# Retro: regenerate shakedown RON (parity drift)

- TASK: 20260715-172138
- BRANCH: fix/parity-drift (landed on master as ff9c61b7)
- REVIEW ROUNDS: 1 (APPROVE, mechanical verification)

Smooth ten-minute cycle; the interesting part is upstream of it.

## What went well

- The 142906 implementation agent flagged the failure instead of absorbing it,
  A/B-verified it pre-existing, and the filed task carried enough evidence
  (culprit commit, failing test, regeneration mechanism) that planning was a
  lookup, not an investigation.
- The parity test's write-on-missing design made the fix a delete + re-run,
  and the same test is the reviewer for its own output.

## What went wrong

- Upstream: 713ac855 changed a builder whose output is committed generated
  data without re-running the generator in the same change. The parity gate
  exists and caught it - but only on a later full-suite run.

## What to improve next time

- A change to any builder behind a committed generated artifact regenerates
  the artifact in the SAME commit; greppable rule: if you touch
  `scenario.rs`/`sections.rs` builders, run content_ron_parity before
  committing.

## Action items

- [x] LESSONS.md: bumped `generate-data-from-code` with the
  same-commit-regeneration variant.
