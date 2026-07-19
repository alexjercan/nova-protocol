# Retro: profiled pass (chrome-trace top-N + samply)

- TASK: 20260719-112253
- BRANCH: feature/probe-profiling (squash-landed as f3af2485)
- REVIEW ROUNDS: 2 (APPROVE; round 2 was the user-requested profiling-profile addendum)

## What went well

- Empirical-first debugging, three times over: (1) the zero-system-spans
  mystery survived three source-reading theories and died to one 30-second
  throwaway probe app; (2) the RUST_LOG-overrides-the-filter question was
  settled by re-running the real binary, not by trusting folklore; (3) the
  "still no names" report was answered by driving samply's own tokened
  symbolication API headlessly - proving the committed pipeline resolves
  full demangled names in 0.85 s, which localized the user's issue to the
  load-side workflow instead of triggering another build change.
- The user field-testing DURING the cycle (pause-for-review workflow)
  surfaced two real-host blockers the CI-style e2e could not:
  perf_event_mlock_kb on a 24-core box, and raw-address flamegraphs from
  the dev profile's debuginfo tradeoff. Both fixed and documented in-cycle.
- Reading the ARTIFACT, not the exit codes: the first e2e was all-green by
  exit status while producing an empty table (render-output-eyeball). The
  emptiness was the thread that unraveled the bevy_ecs=warn discovery.

## What went wrong

- The first e2e shipped a beautiful pipeline around an empty payload: my
  assumed span-name format was verified in bevy SOURCE but the runtime
  filter interaction (EnvFilter governs spans, not just logs) was not - a
  runtime behavior no amount of span-emission source reading would show.
- The plan SAID "degrades gracefully when samply is unavailable or perms
  block it", but the first script only handled the missing-binary case;
  the perms case died under set -e and was found by the user. A stated
  failure path that was never forced is an untested claim.

## What to improve next time

- When a plan claims a degrade/fallback path, FORCE that failure once
  before shipping (fake the missing tool, revoke the perm) - same shape as
  would-it-fail-without-it, applied to error handling.
- Runtime composition facts (filter layers, feature unification) need an
  empirical probe even when the emission site is source-verified; budget
  the 30-second probe app up front instead of after the theories fail.

## Action items

- [x] Lessons: new env-filter-governs-spans (domain), new
      degrade-paths-need-a-forced-failure, bumped render-output-eyeball.
- [ ] T5 inherits: R1.2 (parent/child span shares overlap - rank, never
      pie-chart) plus the earlier T2/T3 notes.
- [ ] Surfaced for 20260718-004856 (broadside hitch): insert_asteroid_collider
      at 67 ms/call is a concrete suspect, straight from the new table.
