# Retro: Base as the implicit universal dep://base target (Option A, mechanism)

- TASK: 20260717-000416
- OUTCOME: landed (squash 94d12718), review APPROVE round 1, suites green.

## What was built

Reversed the `dep://base` rejection: base is the implicit universal dependency,
so `dep://base/<path>` is allowed without a `meta.dependencies` entry. base is
injected into every owning bundle's dep scope (runtime + static lint) and exempted
from the portal's declared check. Mechanism only - proven with synthetic bundles;
real resolution against moved base art is the next task.

## What went well

- Small, surgical inversion of the already-reviewed `dep://` gate. The unified
  `RefScope` from task 20260716-215423 meant the change was "exempt `base` from
  the declared check" in the two `mod_refs` functions + supply base in the two
  callers + the portal - four consistent edits, no new machinery.
- The existing test structure made flipping "base rejected" -> "base resolves"
  cheap, and the end-to-end integration test (real `register_bundles` with a
  synthetic base catalog entry) proves the caller injection, not just the pure
  `mod_refs` logic.
- Recognizing that base's `resource_base="base"` is already CORRECT once art
  moves under `assets/base/` (task 2) - so no root-relative trick (that was
  Option B); the mechanism is just "allow base as an implicit dep".

## What went wrong / difficulties

- Cold-build tax: a fresh sprout worktree recompiles all of bevy from scratch
  (~4 min for check, longer for the test binary with codegen), so the
  verify loop is slow. Every task in this flow pays it once. Not fixable within
  the task; noted for pacing.

## What to improve next time

- For a multi-task flow that keeps sprouting fresh worktrees, expect and plan
  around the per-worktree cold-build cost (a shared target dir / sccache would
  amortize it, but that is a separate infra change).
