# Retro: RCS fine-adjustment core primitive

- TASK: 20260718-122906
- BRANCH: feat/rcs-core (landed as master 96ce2034)
- REVIEW ROUNDS: 1 (APPROVE, findings all non-blocking docs/comment)

Process notes only; what/why/evidence live in TASK.md + NOTES.md, the family
status in the spike's fix record.

## What went well

- The spike resolved all four design forks WITH THE USER up front (via a
  single AskUserQuestion), so plan and work carried zero design rework - the
  cycle was mechanical from the plan onward.
- The plan verified the load-bearing physics facts against the dependency
  source BEFORE writing steps (`apply_linear_impulse` acts at the COM with no
  torque; it wakes sleeping bodies; `ComputedMass` can be co-queried with
  `Forces` as a shared read). This is the `verify-first-plan-steps` discipline
  paying off - the mechanic compiled and passed on the first real run, no
  "the model of the system was wrong" round.
- Reused existing machinery almost 1:1 instead of inventing: the manual-burn
  speed-cap taper generalized to three axes, and the FlightVerb/WithheldVerbs
  capability model carried `Rcs` for free (no parse table, no match sites -
  grep confirmed). Small diff, high confidence.
- Reading pose THROUGH the mutable `Forces` query item (`rotation()`,
  `linear_velocity()`) sidestepped the aliasing conflict a separate
  `&LinearVelocity` query would have caused - found by reading the avian query
  data source, not by trial-and-error compiles.

## What went wrong

- Split the squash-land into `merge --squash` -> inspect -> commit across three
  tool calls, exactly the `shared-checkout-write-leak` anti-pattern. Got lucky
  (no parallel job ran `git add -A` in the window), but a concurrent craft_racer
  job DID land on master mid-cycle, so the race was demonstrably live. Root
  cause: wanted to eyeball the staged set before committing - but the safe place
  to inspect is the BRANCH, before landing, not the staged main-checkout index.
- Duplicate `velocity_of` test helper broke the first test build - added a helper
  without grepping the (large) flight.rs test module for an existing one. Cheap
  to fix, but a wasted compile cycle (~3 min).
- `cargo test -p nova_scenario` failed to compile standalone because the
  `ScenarioConfig` serde derives are feature-gated and only unified on in the
  workspace build; needed `--features serde`. A minute lost decoding a
  Serialize-bound error that had nothing to do with my change.
- Minor: created the spike/core task files in the main checkout before sprouting
  (cwd resets between calls), then had to remove and recreate inside the
  worktree - the `sprout-first-then-tatr-new` rule again.

## What to improve next time

- Land in ONE command (`git merge --squash <b> && git commit`); inspect the diff
  on the branch beforehand, never leave a staged index sitting in the shared
  checkout across calls.
- Before adding a test helper to a big test module, grep it for the name first.
- When running a single crate's tests, remember gated derives: pass the crate's
  `--features` (or run the workspace build CI uses) instead of debugging a
  spurious trait-bound error.

## Action items

- [x] Bumped `shared-checkout-write-leak` (x2) and added
  `per-crate-test-needs-gated-features` + `grep-test-module-before-adding-a-helper`
  to LESSONS.md.
- No follow-up code tasks: the deferred items (RcsSpeedCap scenario authoring,
  the deflection->cap feel decision) are already routed to the sibling tasks
  (-122912) and NOTES.md; the RCS family's remaining work (input/HUD/autopilot)
  was already seeded by the spike.
