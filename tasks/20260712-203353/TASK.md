# Sticky focused lock: a focused lock resists aim-steal, manual CTRL+scroll to shift off

- STATUS: OPEN
- PRIORITY: 26
- TAGS: v0.5.0, targeting, spike

## Goal

The lock is STICKY FROM ACQUISITION: the aim-driven picker acquires a lock only
when there is no current valid lock; once you are locked on something, it HOLDS
(subject to the existing range gates + hysteresis) until the target dies /
leaves range or the player deliberately shifts off it with CTRL+scroll (which
already cycles the ship lock). Aim still makes the FIRST acquisition - it just
stops re-picking a new target under you afterwards. This is the "does not move
off the thing I first locked on unless I scroll off" feel from the playtest.

Direction (see spike + user refinement 2026-07-12 - this is option B5,
sticky-from-acquisition, NOT B1 sticky-after-focus): in
`update_spaceship_target_input` (input/targeting.rs), skip the aim re-pick when
the current `**res_target` is still a valid (collectible/in-range) candidate -
reusing the `pinned` gate's "incumbent still collectible" check. Only run the
cone/signature pick when there is no held lock. The manual CTRL+scroll cycle
(`step_target_lock`) and target death/out-of-range still change the lock. A
held lock persists even when the target leaves the aim cone (locked ship behind
you, inset still on it).

Why B5 and not B1: the user's rationale - torpedo theft becomes a non-issue and
torpedoes stay lockable for point defense - only holds if the lock sticks the
moment it is acquired; B1 (after the 1.5 s dwell) leaves the pre-focus window
stealable. This supersedes task 20260712-203349 (no-torpedo-autolock), now
closed won't-do.

Open feel-risk to PLAYTEST: sticky-from-acquisition removes aim-to-switch
entirely (you must CTRL+scroll to change targets). If that feels stuck, the
fallbacks are (a) a "sustained aim far off the locked target for N seconds
releases it" grace, or (b) fall back to B1 (sticky only after the focus dwell).
Decide from the playtest.

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
