# Retro: Modding scenario-dispatch perf (benchmark + handler index)

- TASKS: 20260714-083331 (benchmark), 20260525-133014 (index), 20260714-083339 (deferred)
- BRANCH: modding-perf + bevy-common-systems master @ 4c81117
- REVIEW ROUNDS: 1 (APPROVE)

Process notes only; the what/why/numbers live in TASK.md and
`docs/modding-perf-report.md`.

## What went well

- **The gating benchmark earned its cost immediately.** It caught the first
  index design (entity-ids + `Query::get`) regressing at scale: a clean-looking
  O(N)->O(matching) change *lost* to the baseline's cache-friendly linear
  archetype scan at 5000 handlers, even though it won at 500. Without measuring
  across the full scale range that regression ships silently. The snapshot
  redesign (contiguous handler clones, no ECS touch) was measured green.
- **Deferral on data.** 083339 was declined because the benchmark showed the
  filter/condition micro-costs are noise at realistic once-per-frame rates - a
  legitimate outcome of the gate, not an unfinished task. Adding a nested-
  condition bench during review (62 ns vs 26 ns) confirmed the defer holds even
  for the worst case.
- **Isolating the signal.** Splitting the dispatch bench into a realistic
  1-event/frame group (frame-overhead-dominated, index-neutral) and a burst
  group (scan-isolated, where the index bites) kept the report honest about
  where the win does and does not exist.

## What went wrong

- **Pushed a bcs commit that failed CI on `cargo fmt --check`.** Root cause: I
  ran `cargo fmt` on the index change, *then* added the test module and committed
  without re-running fmt. The lint gate ran before the last edit, not after it.
  Cost: a second bcs push (fmt fix) and a third nova rev-bump re-pin. For a repo
  whose only gate is remote CI, the local pre-push check has to mirror what CI
  runs (fmt + clippy --all-targets + tests), as the final step.
- **Cross-repo churn.** Three bcs pushes and two nova rev bumps for one logical
  change. The rebase-onto-master path was clean, but the fmt slip turned one
  landing into three round-trips.

## What to improve next time

- Re-run the full lint/test gate as the *last* action before any commit that a
  remote CI will police - especially after edits made post-format (a test
  module, a doc block). Never let "I already ran fmt" cover an edit made after.
- When benchmarking an algorithmic change, sweep the full scale range (and both
  regimes) before believing a win; one data point can invert.

## Action items

- [x] Report + tasks record the measured deferral of 083339 (no code).
- [x] Nested-condition bench added (review R1.3).
- [ ] Next bcs touch: fold in the R1.4/R1.5 doc caveats on `EventHandlerIndex`
  (snapshot semantics + ~2x handler storage). Deferred to avoid a 4th rev churn.
