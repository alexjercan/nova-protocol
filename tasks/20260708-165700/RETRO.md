# Retro: Screen-projected-indicator widget (HUD substrate)

- TASK: 20260708-165700
- BRANCH: weapons-hud (shared arc branch; no per-task squash-merge by user
  instruction - the whole weapons-HUD arc lands from this branch)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 2 MINOR + 2 NIT, all addressed;
  round 2 APPROVE)

What shipped is in the task's Resolution and
tasks/20260708-165700/NOTES.md.

## What went well

- **Deciding architecture with the user before any code.** The spike ended in
  four concrete questions (location, migration scope, off-screen scope,
  sizing scope); all four answers went into the spike doc and the TASK.md
  steps, and none of them was re-litigated during implementation or review.
  Cheapest possible way to burn ambiguity.
- **Fabricated-camera projection tests.** `Camera.computed` (bevy_camera
  0.19) is fully public, so a hand-built `clip_from_view` +
  `RenderTargetInfo` gives real `world_to_viewport` behavior in plain World
  tests - no render backend, no windowing. That unlocked whole-system
  behavioral tests (anchor lifecycle, clamp + arrow rotation, apparent size
  vs the projection matrix) that previous HUD code never had.
- **Pure-function core.** `place(viewport, projected, view_pos, policy)`
  isolates the entire visibility/clamp policy matrix from the ECS; the system
  is a thin shell. The review found zero correctness issues in the policy
  logic.
- **Earlier retro lessons held.** The range example was written
  mandatory-expects-first with an asserted-at-exit backstop (com-range
  lesson), REVIEW.md responses were written only after fixes and re-runs
  (bookkeeping lesson), and the docs enumerated per-consumer behavior deltas
  up front - the reviewer's docs-precision finding streak (3 cycles) ended at
  zero this time.
- **Verify conventions from dependency source, not memory.** The
  arrow-rotation math depended on how `UiTransform` applies `Rot2`; reading
  `compute_affine` in bevy_ui settled it (`Mat2::from(rotation)` in y-down
  coords) instead of shipping a maybe-mirrored arrow.

## What went wrong

- **The only red test was the test's own math.** The apparent-size
  expectation forgot the aspect-ratio division in the horizontal projection
  (x_ndc = x / (z * aspect)). Root cause: deriving the expected value from a
  mental model of the projection instead of from the same matrix the code
  uses. Hand-derived expected values need the derivation written in the test
  comment - writing it down is what exposed the error.
- **Headless example run went to the wrong directory twice.**
  `cd <worktree> && Xvfb :99 ... &` backgrounds the entire chain including
  the `cd`, so cargo ran in the main checkout; and the first failed run was
  misread because cargo's stderr was piped through a grep that matched
  nothing, hiding the real error. Root causes: compound one-liners around
  `&`, and filtering unknown output before capturing it. Rule: start the
  display in its own command, and capture first-run output to a file, grep
  afterwards.
- **Old habits copied into new generic code (R1.1).** The widget initially
  wrote `Visibility` unconditionally every frame because the code it
  generalized did; for a substrate that every HUD indicator will use, that
  dirties change detection globally. Migrating code is also the moment to
  question its habits, not just relocate them.
- **`Option<Single>` semantics almost shipped a silent failure mode (R1.2).**
  With two tagged cameras, `Option<Single>` skips the whole system - every
  indicator freezes with no diagnostic. Bevy gotcha worth remembering:
  `Option<Single>` is None on zero matches but SKIPS on multiple; ambiguity
  needs an explicit query + warn when freezing is unacceptable.

## What to improve next time

- When a test's expected value is hand-derived from math, derive it in the
  test comment from the exact formula the implementation uses.
- Headless app runs: dedicate a command to the display server, `cd` and run
  cargo in a separate command, capture output to a file before filtering.
- When generalizing existing code into a shared component, list the habits
  the old code had (write-every-frame, hardcoded camera, per-overlay layers)
  and decide each one deliberately.
- Remember the `Option<Single>` multi-match skip; grep for it when reviewing
  systems that must degrade loudly.

## Action items

- [x] Promotion to bevy_common_systems already seeded as tatr
  20260709-164608 (by the architecture spike).
- [ ] Tasks 20260708-165701 (pip) and 20260708-165702 (readout) consume the
  widget next; their drivers should follow the pure-math + thin-driver
  pattern this cycle validated.
