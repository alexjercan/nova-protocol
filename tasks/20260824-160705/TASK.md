# Redesign Autopilot pacing and probe contracts

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.13.0, autopilot, probe

## Goal

Audit Nova Autopilot pacing contracts and make evidence-based cleanup changes. Prefer observable state when it is the real condition, but retain `frames` for work that is inherently frame-based.

Scheduled into v0.13.0 (2026-08-31 planning round) as the cycle's internal
work.

## Context

The original follow-up audit of task 20260824-011329 found 115 `frames(..)` call sites across 35 files. The tree has continued to grow: a 2026-09-03 read-only recount found 144 `.until(frames(..))` sites across 42 Rust files, including 28 `frames(1)` sites. Re-audit landed master before changing anything and keep the exact inventory with this task.

Frame counts are not categorically wrong. They are the real contract for deterministic media duration, renderer pipeline work, raster or shader synchronization, and deliberate separation between an action and the assertion that tests it. They are wrong when they guess how long hidden application state takes to change.

## Scope

- Inventory every current `.until(frames(..))` site and classify it as:
  - tautological next-frame pacing;
  - observable state or event acknowledgement;
  - render, capture, shader, or raster frame work;
  - deterministic media duration;
  - deliberately weaker pre-assert separation.
- Remove tautological `frames(1)` waits. An unqualified step already advances on its first driven frame.
- Replace a frame wait only when an existing observable condition states the real contract more accurately.
- Add a narrow, public, read-only probe only when the audit proves that a settle count hides state which has no suitable observable surface. Check `widget_zoo`, `system_nova_os`, and `pointer_pin`; do not assume each needs a new probe.
- Keep intentional frame-work waits deterministic. Record why each retained category needs frames rather than elapsed time or application state.
- Improve a deadline diagnostic only where the audit finds a real opaque stall and a useful observed value is available.
- Update affected examples, automation-harness documentation, changelog, and task proof together.

## Out of scope

- Removing or renaming the generic `frames` predicate.
- A broad Autopilot API redesign without a defect found by the audit.
- Replacing deterministic frame work with elapsed-time sleeps.
- Adding probe state only to make a frame-count total smaller.

## Constraints

- Preserve deterministic capture lengths and rendered output.
- Do not gate an assertion on the exact invariant it is intended to prove.
- Do not add sleeps or rename frame counts without changing their semantics.
- Keep subsystem probes read-only and avoid dependency cycles.
- Keep outcome slugs stable unless an intentional contract change is documented and reviewed.

## Done when

- The task evidence contains a current, complete classification of pacing call sites.
- Tautological `frames(1)` waits are gone.
- Every changed wait names a narrower real condition and has focused proof.
- Every retained frame-count category has an explicit frame-work or assertion-separation reason.
- `frames` remains public with its constrained role documented.
- Focused unit tests and affected software-rendered probe ranges pass through `nix develop`.
- Generated reports and any affected representative still/video artifacts are inspected, with commit-bound evidence kept under this task.
