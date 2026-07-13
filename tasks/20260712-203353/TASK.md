# Sticky focused lock: a focused lock resists aim-steal, manual CTRL+scroll to shift off

- STATUS: CLOSED
- PRIORITY: 26
- TAGS: v0.5.0, targeting, spike

## Outcome (CLOSED 2026-07-12)

Implemented B5 (sticky-from-acquisition) in `update_spaceship_target_input`
(input/targeting.rs) with a `held` gate: the aim pick (cone + signature) now
only runs when `!pinned && !held`, where `held` = the current
`SpaceshipPlayerTargetLock` target is still a collectible candidate AND is a
SHIP (review R1.1 - `is_ship` in the candidate tuple). Ship-only because the
lock doubles as the GOTO/torpedo nav designator: asteroids/beacons must stay
aim-driven so you can re-point a GOTO by aiming (CTRL+scroll cycles only hostile
ships). So:

- Aim makes the FIRST acquisition (no lock -> `held` false -> pick runs).
- Once locked, a body crossing the aim ray (passing torpedo, another ship) no
  longer steals the lock or resets the focus dwell (`held` true -> pick skipped).
- The lock returns to the picker only when its target dies / leaves range
  (drops from candidates -> `held` false -> re-acquire).
- Deliberate switches stay on the existing CTRL+scroll cycle (`step_target_lock`),
  which also keeps torpedoes lockable for point defense (why the separate
  no-torpedo-autolock task 20260712-203349 was dropped).

One existing test encoded the OLD behaviour (`pinned_lock_holds_against_the_aim_
pick_until_expiry` asserted an expired pin re-aims to a different target); that
is exactly what B5 changes, so it was rewritten as
`an_expired_pin_leaves_the_lock_sticky_not_re_aimed` (pin clears at its deadline
but the lock stays sticky; delivery guard: losing the target re-acquires).
Added `a_held_lock_is_not_stolen_by_a_closer_body` (a challenger closer to the
aim ray does not steal a held lock; delivery guard proves the picker still
moves once the held target leaves).

Verified: `cargo test -p nova_gameplay targeting` 43 pass; `12_hud_range` +
`10_gameplay` autopilots green (lock acquires, inset opens, everything hides on
target death); `fmt --check` + non-debug `cargo check --workspace` clean.

PLAYTEST NOTE (spike open question): B5 removes aim-to-switch entirely - you
must CTRL+scroll to change targets (or the target must die / leave range). If
that feels stuck, the fallbacks are a "sustained aim-away releases the lock"
grace or reverting to B1 (sticky only after the focus dwell). Decide at playtest.

## Steps

- [x] Add a `held` gate in `update_spaceship_target_input`: skip the aim pick
      when the current lock target is still a collectible candidate.
- [x] Keep first-acquisition, death/out-of-range re-acquire, and the CTRL+scroll
      cycle working (they fall out of `held` being false / `step_target_lock`).
- [x] Rewrite the pin-expiry test for sticky semantics + add a held-lock-not-
      stolen test (both delivery-guarded).
- [x] Verify: targeting unit tests + 12_hud_range/10_gameplay autopilots.

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

- Spike: tasks/20260712-203235/SPIKE.md
  (Part 2, options B1 + optional B3).
- Relevant files: `crates/nova_gameplay/src/input/targeting.rs`
  (`update_spaceship_target_input` `pinned` gate at ~line 447-482;
  `SpaceshipPlayerLockFocus::focused_on`; `step_target_lock` for the existing
  CTRL+scroll shift). No new input binding needed.
- Feel-critical: land after the two smaller tasks and PLAYTEST the
  "must CTRL+scroll to switch after committing" feel; see the spike's first
  open question (add a "sustained aim-away releases it" grace if too sticky).
- Buy-in requested before implementing.
