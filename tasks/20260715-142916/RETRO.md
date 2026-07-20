# Retro: Explore online tab - the family goal lands

- TASK: 20260715-142916
- BRANCH: feature/explore-tab (landed on master as e4e4aa29)
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES with one MAJOR, R2 APPROVE)

## What went well

- The layered-contracts approach paid off at the finish line: this task wired
  a UI onto event/resource APIs (163508), markers/action-area contracts
  (142911), and cache semantics (142906) that were all designed FOR it - the
  implementation was binding, not inventing.
- Visual verification as a first-class plan step (the 142911 retro's action
  item) was executed through the REAL pipeline - generated portal, localhost
  HTTP, production transport - not a synthetic mock.
- The reviewer's MAJOR (offline Update destroying an install) was a genuine
  product-logic hole: every layer worked as specified, but the COMPOSITION
  (stale catalog entries + full actions + Ready-gated installs) produced a
  destructive path no single layer owned. Adversarial review at composition
  level is where its value now concentrates.
- The implementer's R1.3 fix improved on the prescription (progress-reset
  timeout window instead of a flat clock) with the race analysis documented
  where the constant lives.

## What went wrong

- The offline-Update MAJOR: the plan promised stale entries would "render
  below a muted note" (browsing) and the implementation quietly extended them
  to full actions - a scope-exceeding convenience that composed into data
  loss. Root cause: an unstated invariant ("destructive multi-step actions
  require the resources to complete ALL steps") existed in nobody's head
  until the reviewer traced the composition.
- A parallel session relocated the reference docs into the web wiki
  mid-cycle, producing the family's first merge conflict at landing time. The
  resolution was mechanical (rename detection carried most edits), but the
  shared-checkout hazard the lessons warn about is now a lived experience:
  verify-branch-before-commit and conflict-resolution-on-the-branch both did
  their jobs.

## What to improve next time

- For any multi-step destructive action (update = uninstall + install),
  state the completability invariant in the plan: the action must not START
  unless every step's preconditions hold.

## Action items

- [x] LESSONS.md: new lesson `destructive-chains-check-completability`;
  bumped `out-of-context-review-pass`.
