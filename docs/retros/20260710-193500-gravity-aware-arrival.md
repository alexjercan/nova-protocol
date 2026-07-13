# Retro: Gravity-aware GOTO/STOP arrival planning

- TASK: 20260710-193500
- BRANCH: fix/gravity-aware-arrival (squashed to master as 9302a0a)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES: 1 MAJOR, 4 MINOR, 3 NIT; round 2 APPROVE)
- SPIKE: docs/spikes/20260710-204802-gravity-aware-arrival.md

## What went well

- The spike-first order paid: option (a) from the task's own sketch
  (current-position feedforward) is precisely the mechanism of the crash,
  softened - writing the options out caught that before any code existed.
- Deriving from the owning system's actual code (the AGENTS.md rule) fixed
  the plan mid-implementation: the planned DominantWell read is empty at
  the flip, which is planned from OUTSIDE the SOI. An all-wells scan at
  the rest point replaced it; the deviation is recorded in the Steps.
- The honesty check - force-zero the gravity budget and watch the new
  integration test reproduce the playtest crash (hull at 12u from the
  center of a 40u body), then restore - cost two minutes and turned "the
  test passes" into "the test discriminates". The reviewer reproduced it
  independently. Keep this as a standing pattern for physics fixes: a
  regression test for a plan-quality bug must be shown to fail on the old
  plan.
- The ManeuverTelemetry seam delivered what it promised two cycles ago:
  the FLIP marker and ETA corrected themselves with zero HUD changes.

## What went wrong

- R1.1 (MAJOR): the promised eta=None degradation was not implemented -
  arrival_eta's braking-regime fallback caught the None from
  goto_flip_point and served a confident, wrong ETA on an unstoppable
  leg, while comment and docs claimed blank instruments. Root cause: the
  degradation contract was written in prose (comment, docs, TASK step)
  and assumed to hold through composition, but never asserted; each
  helper was tested alone and the fallback path silently swallowed the
  degenerate case. The same lesson as the spawn-flash finding one cycle
  earlier, generalized: EVERY claimed behavior of a degraded/edge state
  needs its own assertion, especially when the state is produced by one
  function and interpreted by another.
- Smaller doc-honesty drift (write-only brake_accel described as "the
  decel chip", per-tick log described as "once", frame-parity control
  claim): all prose that outran the code by a few edits. Cheap to fix,
  but three of five MINOR findings were this category.

## What to improve next time

- When a plan step promises a degradation/edge behavior ("X goes blank",
  "logs once"), turn that clause into a test or a grep-verifiable code
  fact in the same commit that claims it done - the checkbox is not
  tickable on the happy path alone.
- Before writing "instrument Y self-corrects", grep for a reader of the
  field. A seam with zero consumers is still worth publishing correctly,
  but say that instead.

## Action items

- [ ] Playtest follow-through: GOTO the Gravity Rock hot from outside the
  SOI (the original repro) and watch the earlier flip by eye.
- [ ] The grazing-leg under-budget (goal outside a well, path passing
  near it) is a recorded limit in the spike and docs; revisit only if
  playtests show sloppy stops near wells.
- [x] Tasks 20260710-202408 (surface-relative standoff) and
  20260710-195954 (GOTO parks into ORBIT) touch this same arrival region
  and should re-read docs/retros/20260710-gravity-aware-arrival.md first -
  already noted in 202408's own task notes.
