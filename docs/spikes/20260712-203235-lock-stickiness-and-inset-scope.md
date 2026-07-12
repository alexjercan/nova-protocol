# Spike: Sticky target lock + ship-only inset scope

- DATE: 20260712-203235
- STATUS: RECOMMENDED
- TAGS: spike, targeting, hud, camera, playtest

## Question

Two coupled playtest annoyances with the target lock / focus and the new
target inset (task 20260710-104421):

1. The inset (and the lock) can land on things that are not enemy ships -
   beacons (authored `LockSignature`) and committed torpedoes - so the "scope"
   zooms a waypoint or a passing missile. We probably want a flag for what is
   worth scoping (probably just ships).
2. Focus shifts too easily: a torpedo streaking across the aim ray STEALS the
   aim-driven lock and RESETS the 1.5 s focus dwell, closing the inset. The
   player wants the lock to stay on the thing they first committed to unless
   they deliberately move off it.

A good answer picks: (a) how the inset decides what is zoomable, and (b) a
lock-feel change that stops accidental focus theft while keeping the liked
aim-to-acquire behaviour - grounded in the existing lock machinery, not a
rewrite. Explicitly buy-in-first: recommend and seed tasks, do not build yet.

## Context (what exists, from input/targeting.rs + hud/target_inset.rs)

- **The lock is aim-driven.** `update_spaceship_target_input` runs
  `pick_target` (nearest body to the aim ray inside an 18-degree cone) over
  ALL lockable candidates - ships, committed torpedoes, beacons, asteroids -
  with NO ship/hostility filter on the cone pick (confirmed
  targeting.rs:464). So a torpedo crossing the aim ray wins the lock. The
  empty-cone fallback (`pick_signature_target`) IS hostile-filtered, but the
  cone pick is not.
- **Focus resets on any lock change.** `tick_lock_focus` zeroes the dwell
  whenever `focus.target != lock` (targeting.rs:576). A one-frame lock steal
  therefore resets the 1.5 s dwell, and the inset (which needs
  `focus.focused_on(target)`) closes.
- **A "sticky" gate already exists.** When the player CTRL+scrolls to cycle the
  SHIP lock (`TargetCycleModifier` + wheel -> `step_target_lock`), it sets
  `pinned_until = now + 4 s`; while pinned, `update_spaceship_target_input`
  SKIPS the aim re-pick (`if !pinned { ... }`, targeting.rs:460). Range
  hysteresis (`range_hysteresis`) also holds the incumbent a little past its
  gate. So the plumbing for "aim picker stands down, lock holds" is already
  there - it is just only triggered by a manual cycle today.
- **The multi-target set is already ship+hostile only.**
  `rank_ship_candidates` filters to `is_hostile && is_ship`; the CTRL+scroll
  cycle walks that list. So "cycle only lands on enemy ships" is already true;
  only the AIM-driven primary pick is unfiltered.
- **The inset consumer.** `hud/target_inset.rs::drive_inset_camera` frames
  whatever is focused; it does not care what kind of body it is. Gating it is
  a one-line predicate on the target entity.

## Options considered

### Part 1 - what is zoomable (inset scope)

- **A1. Reuse `SpaceshipRootMarker`.** Gate the inset on
  `q.get(target).has::<SpaceshipRootMarker>()`. Zero new authoring - every
  ship root already has it. Con: conflates "is a ship" with "is worth
  scoping"; a future boss asteroid / station could not opt in, and you could
  not opt a friendly ship OUT.
- **A2. New `InsetZoomable` (scope-target) marker (recommended).** A dedicated
  flag component authored on ship roots where they spawn (the section/ship
  spawn path), gating the inset. Matches the user's explicit "flag component
  for things that can be zoomed in on". Con: one extra insert at ship spawn
  (and anywhere a modded scenario wants a zoomable non-ship). Cheap and
  future-proof; decouples scope-worthiness from ship-ness.
- **A3. Do nothing.** The inset keeps zooming beacons/torpedoes. Rejected -
  it is the reported annoyance and reads as a bug.

### Part 2 - lock feel (stop accidental focus theft)

- **B1. Protect a FOCUSED lock from aim-steal (recommended core).** Extend the
  existing `pinned` gate so the aim picker also stands down while
  `focus.focused_on(lock)` - i.e. once the 1.5 s dwell completes, the lock
  holds (subject to the same range gates + hysteresis) until the target dies /
  leaves range or the player manually CTRL+scrolls off it. Aim-to-acquire is
  UNCHANGED before the commit; after the commit the lock is sticky. This is
  exactly the user's "does not move off the thing they first locked on unless
  they scroll off". Reuses machinery that already exists; ~one condition.
  - Con: after focusing, you can no longer switch ships by just aiming at a
    new one - you must CTRL+scroll (or the target must die). The user asked
    for precisely this, but it is a real feel change; needs a playtest.
  - Open sub-choice: should a focused lock hold even when the target leaves
    the aim cone (locked ship now behind you, inset still on it)? The `pinned`
    path already holds out-of-cone as long as in range, which is the "scope
    what I chose" behaviour we want. Keep that.
- **B2. Exclude committed torpedoes (and other transient non-designation
  bodies) from the AIM auto-pick (recommended complement).** The cone pick
  should not AUTO-acquire a committed torpedo streaking past; torpedoes stay
  MANUALLY lockable for point defense (aim + they are still in the candidate
  set for an explicit lock), but they do not steal an existing/forming lock.
  This protects the 1.5 s dwell too (B1 only protects AFTER focus; a torpedo
  can still steal DURING the dwell without B2). Small, low-risk, helps even
  without B1.
  - Note: must NOT exclude beacons/asteroids from the cone pick - the lock is
    the GOTO/torpedo DESIGNATOR and you designate those by aiming. Only the
    transient fast-movers (committed torpedoes) are the problem for auto-steal.
    Beacons are handled by Part 1 (inset scope), not by delocking.
- **B3. Angular hysteresis on the cone pick (smaller alternative/complement).**
  Today the cone pick has range hysteresis but not ANGULAR - a challenger even
  slightly closer to the aim ray instantly wins. Add "incumbent holds unless
  the challenger is decisively closer to the ray" (the `snap_pick` pattern the
  section fine-lock already uses). Reduces flicker generally without the full
  sticky change. Weaker than B1 for the user's stated want (they want manual
  shift, not just less flicker), but a cheap standalone polish.
- **B4. Inset remembers the last-focused ship.** Decouple the inset from live
  focus: keep showing the last focused ship until a new ship is focused / it
  dies. A band-aid on the SYMPTOM (inset flicker) that leaves the lock feel
  unchanged - and it would show a stale ship while the reticle sits on the
  torpedo, which is confusing. Redundant once B1 makes focused locks sticky.
  Rejected as a primary; not needed if B1 lands.
- **B5. Sticky-by-default (lock never auto-switches once acquired, even before
  focus).** Most sticky; aim only picks when there is NO lock. Rejected: it
  removes the liked "look at the next thing to acquire" aim-assist for INITIAL
  acquisition, which is a bigger change than the problem warrants. B1 keeps
  aim-to-acquire and only sticks after the deliberate 1.5 s commit.
- **B0. Do nothing / just bump `range_hysteresis`.** Minimal; does not deliver
  the manual-shift feel. Rejected.

## Recommendation

Ship all three, as separate tasks so they can land and be playtested
independently, smallest/least-risky first:

1. **Inset scope via a dedicated `InsetZoomable` marker (A2).** Author it on
   ship roots; gate `drive_inset_camera` on it. Independent of the lock work
   and safe to land first - it stops the beacon/torpedo scope immediately.
2. **Exclude committed torpedoes from the aim auto-pick (B2).** Small, protects
   the forming dwell, low feel-risk. Torpedoes stay manually lockable for point
   defense.
3. **Sticky focused lock (B1)**, with B3 (angular hysteresis) as an optional
   sub-step if B1 alone feels too binary. This is the feel-critical one; land
   it last and playtest the "must CTRL+scroll to switch after committing"
   feel. CTRL+scroll off is the existing manual shift; no new input needed.

Together: aim to acquire (unchanged) -> 1.5 s dwell to commit (now protected
from torpedo theft by B2) -> sticky on your chosen ship, inset scoped to it,
until you CTRL+scroll off or it dies (B1). The inset only ever scopes ships
(A2). Reversible - each piece is a gate/condition, not a rewrite.

## Open questions

- Does B1 feel too sticky in practice (no aim-to-switch after commit)? Needs a
  playtest; if so, add a "sustained aim far off the locked target releases it"
  grace, or fall back to B3-only. Decide from the playtest, not up front.
- Should `InsetZoomable` ever go on non-ships (a boss asteroid, a station)?
  Not today; the marker exists so that is a one-line authoring change later.
- Does excluding torpedoes from the auto-pick hurt point-defense ergonomics
  (you now must aim AT the torpedo deliberately)? Believed fine (that is how
  you engage a specific torpedo anyway), but confirm in the point-defense
  playtest.

## Next steps

Direction-level tasks seeded (parked OPEN; `/plan` breaks into steps when
picked up). Buy-in requested before implementing - the user asked to explore
first.

- tatr 20260712-203345: inset scope - `InsetZoomable` marker, ship-only scope
- tatr 20260712-203349: committed torpedoes do not auto-steal the aim lock
- tatr 20260712-203353: sticky focused lock - focused lock resists aim-steal,
  manual CTRL+scroll to shift off

## Fix record

(none yet - tasks not started; buy-in pending)
