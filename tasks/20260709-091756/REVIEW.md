# Review: One hit = one cue: dedup HealthApplyDamage propagation in audio + juice

- TASK: 20260709-091756
- BRANCH: fix/one-hit-one-cue (local branch, no worktree)

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...fix/one-hit-one-cue` (audio.rs +67, juice.rs +43,
TASK.md) against the task Goal and the spike's recommendation A.

Verified independently:

- Both commits on the branch are task work; no foreign/user commits rode along
  (the 224254 retro lesson - checked `git log master..fix/one-hit-one-cue`).
- The guard is the spike's exact shape in both observers, placed first so a
  propagated hop pays nothing, and propagation itself is untouched - ship
  death bubbling (`integrity/glue.rs`) and the AI threat tracker
  (`input/ai.rs:455` `on_damage_track_threat`, which *depends* on the event
  reaching the root) are unaffected. Swept all `On<HealthApplyDamage>`
  observers in crates/ and examples/: the only other one is the example 06
  owner-pair assert, which is hop-agnostic by construction. No missed cue
  observer.
- Tests assert behavior, not execution: the juice test pins trauma to exactly
  one `hit_trauma`, one flash, and the flash at the hit location (the phantom
  ring at the parent origin was the visible symptom); the audio test counts
  real `PlaySfx` triggers via a test observer plus the throttle-stamp shape.
  TASK.md documents a mutation check (guard disabled -> both tests fail with
  2 cues); I re-ran the suites myself: audio 9/9, juice 21/21,
  `cargo check --workspace --all-targets` green after touching the edited
  files, `cargo fmt --check` clean. Full workspace test suite + clippy left
  to CI per the standing instruction.
- Doc comments on both observers explain the caveat and warn future
  damage-cue observers to copy the guard, satisfying the task's doc step.
- No existing test was weakened; the diff adds only.

No findings. A clean, minimal diff that delivers the Goal; the deferred
shared-cue-seam extraction is correctly left out per the spike's
rule-of-three call.
