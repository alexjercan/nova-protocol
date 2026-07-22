# Review: only piloted ships feel gravity wells

- VERDICT: APPROVE

Out-of-context review (round 1). Reconstructed on master: the reviewer wrote
this file in the worktree after the implementation commit, so it was uncommitted
when `sprout land` removed the worktree and had to be recovered from the review
output (the clean-before-land lesson - see RETRO).

The gravity re-keying is correct and well-tested. Findings by focus area:

1. Reliability (PASS) - The only production ship spawn is
   `spaceship_scenario_object` (nova_scenario/src/objects/spaceship.rs:249),
   which routes through `insert_spaceship_sections`; at spaceship.rs:406-460 it
   inserts `PlayerSpaceshipMarker`/`AISpaceshipMarker` unconditionally per
   controller (None -> neither). Every other ship-root spawn in the workspace is
   `#[cfg(test)]`. The editor deliberately uses a markerless preview. Both pilot
   markers `#[require(SpaceshipRootMarker)]`, so they are a strict subset of ship
   roots - no piloted ship can be missed.

2. Timing (PASS) - Observers fire on component Add whenever it lands, so there
   is no ordering hole; a later-added AI marker (the sibling loiter task) auto-
   opts in.

3. Regression (PASS, exact) - Verified every scenario. broadside/lifeline have
   zero wells. final_tally/menu/shakedown have wells but zero None ships. The one
   well+None co-location is asteroid_field: rock at (250,0,0) r=20 -> SOI 160u,
   None ship at (10,0,0) = 240u away, outside the SOI. Felt no force before or
   after. The Fix note's claim is precisely right.

4. Double-insert (PASS) - `GravityAffected` is a unit struct; the controller is
   one enum variant so only one pilot marker attaches; both observers
   `try_insert` idempotently.

5. Tests (PASS) - The two new unpiloted tests spawn a bare root with no pilot
   marker; before the fix the old `SpaceshipRootMarker`-keyed observer would opt
   them in and the float assertions would fail. `cargo test -p nova_gameplay
   --lib gravity::`: 18 passed, 0 failed, 1 ignored (perf).

6. Scope (PASS) - A preventive guarantee rule is the right call; it is the
   foundation for sibling task 20260722-092432. The lifeline "crash" attribution
   to knockback (no well, haulers at rest) is sound.

## MINOR (non-blocking, follow-up filed)

- The "None ship never parked inside a well's SOI" authoring invariant is not
  asserted anywhere - asteroid_field clears the SOI by only 80u, and a future
  content nudge could silently flip its behaviour. A content_lint / scenario
  invariant guard would make the guarantee load-bearing. Filed as a follow-up
  task, not a defect in this change.
