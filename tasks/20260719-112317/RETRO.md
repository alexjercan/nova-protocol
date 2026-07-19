# Retro: the probe front door (runner CLI)

- TASK: 20260719-112317
- BRANCH: feature/probe-runner-cli (squash-landed as 02513ce4)
- REVIEW ROUNDS: 1 (APPROVE; R1.1 pid-derived display fixed in-round)

## What went well

- The family's contract discipline made the capstone cheap: run_report
  called in-process (same crate), the recorder/invariants armed by the same
  env surface T2/T3 defined, the trace pass reusing T4's two field facts
  (bevy_ecs=info override, separate-build rule) - zero rework in any
  producing layer, and the whole task fit one sitting.
- Three scope adaptations were made EXPLICITLY with owners and reasoning
  (wrap scripts, no web on `run`, no --export) instead of either silently
  shipping less or grinding out low-value rewrites - the
  audit-framed-task-delivers-the-audit spirit applied to scoping.
- The un-unit-tested composition (the --profile call site) was identified
  as the risk and bought its cold-build e2e deliberately; the pure env
  fns carried the rest of the confidence cheaply - a good split of what
  gets a unit pin vs what gets one real run.
- Ledger lessons showed up as design inputs, not post-hoc fixes: recorded-
  PID Xvfb guard (pkill lesson), wasm stub in the same edit (T5), timeline-
  never-overwritten pin (shared-artifact hygiene).

## What went wrong

- The hardcoded :97 display (R1.1) - a concurrency hazard the parallel-
  sessions environment makes real - only surfaced in review. The env
  assembly was pure and tested, but the DISPLAY allocation was not
  treated as part of that tested surface.
- Minor: the first e2e background job was reported "completed exit -1"
  after the session rename mid-run; the results were all on disk and
  intact. Reading the artifacts rather than trusting the job status
  avoided a pointless rerun.

## What to improve next time

- Resource ALLOCATION (displays, ports, temp names) is part of the pure,
  testable surface - anything two concurrent runs could contend on gets
  derived-from-pid/unique treatment at first writing, not at review.

## Action items

- [x] Family complete (T1-T6); spike fix record closes the loop.
- [ ] Optional follow-up surfaced to the user: wire nova_frametime() into
      one or two more examples so `probe run --fps` covers them (one line
      each, inert without env).
