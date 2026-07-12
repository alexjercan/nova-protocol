# Sticky focused lock: a focused lock resists aim-steal, manual CTRL+scroll to shift off

- STATUS: OPEN
- PRIORITY: 26
- TAGS: v0.5.0, targeting, spike

## Goal

Once the 1.5 s focus dwell completes on a target, the aim-driven picker stops
overwriting the lock: it holds on the committed target (subject to the existing
range gates + hysteresis) until the target dies / leaves range or the player
deliberately shifts off it with CTRL+scroll (which already cycles the ship
lock). Aim-to-acquire is UNCHANGED before the commit. This is the "does not
move off the thing I first locked on unless I scroll off" feel from the
playtest.

Direction (see spike): extend the existing `pinned` gate in
`update_spaceship_target_input` (input/targeting.rs) so the aim re-pick is
also skipped while `focus.focused_on(**lock)` - the same code path that a
CTRL+scroll cycle already uses (`pinned_until`). A focused lock should hold
even when the target leaves the aim cone (locked ship now behind you, inset
still on it), matching the `pinned` path's in-range-holds behaviour.

Optional sub-step (B3): if B1 alone feels too binary, add angular hysteresis to
the cone pick (the `snap_pick` incumbent-holds pattern the section fine-lock
uses) so pre-commit acquisition also flickers less.

## Notes

- Spike: docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md
  (Part 2, options B1 + optional B3).
- Relevant files: `crates/nova_gameplay/src/input/targeting.rs`
  (`update_spaceship_target_input` `pinned` gate at ~line 447-482;
  `SpaceshipPlayerLockFocus::focused_on`; `step_target_lock` for the existing
  CTRL+scroll shift). No new input binding needed.
- Feel-critical: land after the two smaller tasks and PLAYTEST the
  "must CTRL+scroll to switch after committing" feel; see the spike's first
  open question (add a "sustained aim-away releases it" grace if too sticky).
- Buy-in requested before implementing.
