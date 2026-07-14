# Retro: prototype references + component modifications + serde-default

- TASK: 20260714-113411
- BRANCH: modding/section-prototypes
- REVIEW ROUNDS: 2 (APPROVE)

Process only; what/why in TASK.md, family status in the spike (110502) fix-record.

## What went well

- **Asking the design fork surfaced a better answer than mine.** I proposed
  modifications as config-transforms; the user proposed modifications-as-components
  (insert a component, an observer applies it where relevant, inert elsewhere). Their
  model is more ECS-native and extensible, and it's what shipped. The AskUserQuestion
  checkpoint on a genuine fork paid for itself.
- **The implementing agent caught + fixed a real correctness bug during impl.** The
  DisableVerb accumulation bug (a component is unique per entity, so one insert per
  verb drops all but the last) was found via the shakedown e2e and fixed by merging
  into one `SectionDisableVerb(Vec)`. Good adversarial self-check, not left for review.
- **Applied last cycle's lessons and they held.** `git add -A` in the ISOLATED
  worktree (not the main checkout) caught everything including any lock change -
  master landed clean, no stale-`Cargo.lock` (the `stage-lock-with-manifest` miss from
  113408 did not recur). Verified with `cargo test --workspace --no-run`; used
  out-of-context multi-agent review; both reviewers re-derived the load-bearing
  behavior-preservation claim.

## What went wrong

- **R1.1: the bug fix was only guarded downstream (e2e), not at its own boundary.**
  The DisableVerb accumulation fix had an e2e test (`goto_unlocks_...`) but the
  module-level unit test used a SINGLE verb - it would pass even under the
  last-write-wins bug. Root cause: a regression test was added at the scenario level,
  but the fix itself (multi-verb merge in `insert_all`) had no failing case pinned at
  the module boundary.
- **R1.2-R1.4: task/spec lists drifted from what shipped.** `SetMass` was listed in
  the step but correctly deferred (density-derived) with no note; the serde-default
  scope (Option/Vec only, domain-defaults deferred) and the parity narrative
  ("proves lowered byte-identity") were left aspirational. Root cause: the plan's
  wishlist wasn't reconciled to reality at close-out - the SECOND cycle running with
  this pattern (see 133028's retro).
- **serde-default was low-value-for-the-safe-part.** The safe subset (omit None/empty)
  trimmed modestly; the high-value part (f32/bool domain-defaults like `health: 100.0`)
  is the risky part and was deferred. That safe/risky split only became clear mid-impl.

## What to improve next time

- Pin a bug fix with a test at ITS OWN boundary (a unit test that fails under the bug),
  not only a downstream e2e - especially when the existing unit test passes under the bug.
- At close-out, reconcile the plan's aspirational lists (which variants/scope shipped)
  with reality BEFORE handing to review; treat "start with X, Y, Z" as provisional.
- When a bundled feature splits into a cheap-safe part and a valuable-risky part
  (serde-default: None-omission vs domain-defaults), name that split at PLANNING so the
  scope decision is deliberate, not discovered.

## Action items

- [x] Lessons ledger: added `pin-the-fix-at-its-boundary`; bumped `reconcile-plan-to-shipped`.
- Deferred (not tasked, noted here + in TASK.md): `SetMass` section modification
  (needs a density/mass override design) and f32/bool serde domain-default omission.
  File as tasks only if a concrete need appears.
- Family continues at 113414 (whole-ship prototypes) / 113418 (typed multi-file bundles);
  the verb-flags refactor spike is 123535.
