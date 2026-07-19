# Review: continuous invariant checks

- TASK: 20260719-114931
- BRANCH: feature/probe-invariants

## Round 1

- VERDICT: APPROVE

Shared-session caveat: implementer and reviewer are one session; the
load-bearing claims were re-derived rather than read off the diff:

- **The invariant set matches what the engine guarantees, not what the
  design aspires to** - the crux of this task's honesty. Re-checked each
  against the surface map's citations: health bounds ARE hard (bcs
  on_damage clamps at health/mod.rs:135, aggregate recomputes) so
  asserting them is sound; the speed cap is NOT hard (manual taper at
  flight.rs:2126-2131, RCS per-axis, autopilot explicitly unregulated) so
  the check correctly asserts only a 10x absurdity bound with the
  rationale in the doc comment; monotonicity has NO type guarantee
  (variables are free-form) so the opt-in registration is the only honest
  shape. The tempting-but-wrong versions (hard cap assert, inferred
  monotonicity, root==sum aggregate with its mid-despawn flake) are each
  named and rejected in the module docs and TASK.md.
- **Would-it-fail audit**: every check has a test that violates it in
  isolation (negative/NaN/over-max health = 3 distinct violations counted;
  NaN velocity + absurd speed while 4x-cap stays clean; monotonic 2->1
  fires while 0->1->2 and unregistered-variable regressions stay clean;
  NaN variable; strict #[should_panic]); the healthy rig pins zero
  violations; the unarmed plugin pins no-op. Deleting any check fails its
  test. 36 tests pass.
- **Ordering on the exit frame**: checks -> summary -> (recorder) variable
  diff -> run_end, pinned by the summary test asserting invariant_summary
  precedes run_end positionally in the file.
- **Half-ticked-step catch (self)**: the first commit ticked the
  summary-entry clause without implementing it; caught on the post-commit
  TASK.md re-read and implemented + tested in the follow-up commit
  (a89d20f6) rather than amending the step away.
- **E2E healthy baseline**: armed 10_playable run (recorder + invariants,
  monotonic target_down/leg) completed with ZERO violations over the full
  24 s window - the report's `invariants held` check has its green
  reference.
- **CI safety**: both examples' new plugins are env-gated (smoke suite
  sets no NOVA_PERF env); wasm target check clean (invariants cfg'd out
  with the recorder).

Findings: none blocking. One observation for T5 (not a finding here): a
violating entity that persists violates EVERY frame (the -1-health test
counts 1 violation per update), so the report should present violation
COUNTS per invariant name, not raw totals, to avoid a single stuck entity
reading as thousands of failures.
