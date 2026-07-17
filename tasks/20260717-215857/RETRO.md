# Retro: multi-muzzle firing (task 215857)

Landed as commit e882e2cf on `turret-arbitrary-joints`.

## What went well

- The joint-tree core made this small: the tree already SPAWNED all muzzles, so
  the whole task was "iterate the list the core already had" in three sites (fire,
  aim, and the new component). A day-one design that records every muzzle in the
  spawn observer (even while the fire path used one) paid off directly.
- Keeping `TurretSectionMuzzleEntity` as the PRIMARY muzzle alongside the new
  `TurretSectionMuzzles` list meant zero churn in ai.rs / turret_lead.rs /
  audio.rs - the single-point consumers kept their single point. Adding a Vec
  beside the scalar beats replacing the scalar when consumers differ in arity.

## What went wrong / bugs

- None of note. The one thing to watch (and the spec called it out) was the
  borrow checker: the muzzle Entity list has to be copied out of the component
  before the inner loop, or `q_muzzle`/`commands` can't borrow inside. Done up
  front, so no thrash.

## What to do differently

- Nothing structural. The shared-magazine invariant was the one thing worth a
  dedicated assertion (total bullets == capacity, NOT capacity x barrels); writing
  that test FIRST framed the whole fire-loop refactor correctly. Kin of
  `prose-invariant-becomes-pin`.

## Follow-ups still open

- 20260717-215920 editor/lint + joint-tree well-formedness lint (last task).
