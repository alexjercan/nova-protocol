# Retro: Non-lingering cut for the asteroid_next relay bridges

- TASK: 20260717-201534
- BRANCH: linger-tuning
- REVIEW ROUNDS: 1 (APPROVE, one optional NIT)

## What went well

- The deliverable was the audit, not the two-flag edit, and treating it that
  way is what made the change defensible. Reading world.rs/loader.rs/lint.rs
  before touching anything turned "apply linger:false somewhere" into a bounded
  claim: only overlay-less handlers qualify, and the lint already encodes that.
- The A/B on the regression test was cheap and real: committed the fix first,
  flipped the bridge back to linger:true, watched the test fail at the linger
  assertion, restored via `git checkout`. The safety-ordering rule (commit
  before sabotage) meant the restore could not lose work.
- Review re-derived the load-bearing completeness claim independently (a fresh
  out-of-context agent re-enumerated all 28 transitions) instead of trusting
  the implementer summary - the shared-session blind-spot rule doing its job.
  It also caught that `assets/mods/example` was a surface the first audit had
  not explicitly named (it turned out clean, but that was verified, not
  assumed).

## What went wrong

- Nothing structural. Minor friction only: the harness auto-backgrounded cargo
  builds and my first `--exact` test filter matched nothing (it needs the full
  `module::path`), costing one extra run to confirm the test actually executed
  rather than being silently filtered (0 passed / N filtered reads like a pass).

## What to improve next time

- When confirming a single new test ran, assert on "1 passed", not exit 0 -
  a filtered-out test also exits 0. Use a substring filter, not `--exact` with
  a bare function name.
- For "apply X where it makes sense" tasks, write the bounding audit into the
  task record as the primary artifact and re-derive it repo-wide in review;
  the code change is the small part.

## Action items

- [x] Recorded the audit rule in docs/design/scenario-linger.md (in-task).
- [x] Ledger: added `audit-framed-task-delivers-the-audit`; bumped
  `sweep-content-repo-wide-not-just-assets`.
- No follow-up code tasks: the audit proved only the two bridges qualified,
  so the goal is fully delivered.
