# Retro: only piloted ships feel gravity wells

- Landed: f328797d (squash), 1 review round, out-of-context APPROVE.

## What changed and why

Owner playtest: neutral/unpiloted ships should not crash into gravity wells.
Fixed by re-keying the gravity opt-in observer from `SpaceshipRootMarker` (every
ship root) to the pilot markers `PlayerSpaceshipMarker` / `AISpaceshipMarker`,
so only piloted ships feel a well and unpiloted bystanders float.

Cross-crate constraint drove the design: nova_gameplay (where gravity lives)
cannot see nova_scenario's `SpaceshipController` enum (deps run scenario ->
gameplay, one way). But nova_gameplay OWNS the pilot markers that nova_scenario
attaches per controller. Keying on those markers is the clean way to let "the
controller kind" decide gravity from the gameplay side. Bonus: it is
self-maintaining for the sibling loiter task - a hauler that gains an AI pilot
opts back into gravity automatically.

## Difficulties / findings

- VERIFY-FIRST PARTIALLY FALSIFIED THE REPORT. The literal claim ("the Ceres
  Queen falls into a gravity well in Broadside") is impossible: broadside and
  lifeline have no gravity wells at all, and no shipped scenario parks a
  controller:None ship inside a well's SOI. So the change alters zero current
  visible behaviour - it is the guaranteed RULE the owner asked for, not a
  visible-bug fix. Stated plainly in the Fix note and CHANGELOG (filed under
  Gameplay & Flight as a rule, not Fixes). The owner's OBSERVED convoy "crash"
  is knockback drift (no well; haulers at rest until bumped), routed to the
  active-loiter sibling task 20260722-092432. This is the "a cycle may end in a
  falsification" path - the fix still went through review and test, but it is
  honest about what it does and does not do.

- CLEAN-BEFORE-LAND RELAPSE. The out-of-context reviewer wrote REVIEW.md into
  the worktree AFTER my implementation commit, so it was uncommitted when
  `sprout land` ran. `sprout land` squashes only committed state and then
  removes the worktree, so REVIEW.md was dropped AND the worktree was deleted -
  I had to reconstruct REVIEW.md on master from the review output. This is
  exactly the `worktree-clean-before-land` lesson. The land itself succeeded
  (f328797d); only the review artifact was lost and recovered.

## Self-reflection - what to do differently

- COMMIT THE REVIEW ARTIFACT BEFORE LANDING. When a review agent writes
  REVIEW.md into the worktree, `git add -A && commit` it (or verify the worktree
  is clean) BEFORE `sprout land`. Better: have the reviewer return findings and
  I write+commit REVIEW.md myself in the same commit flow, so nothing the agent
  leaves behind is uncommitted at land time. This bit a previous task too; it
  needs to be a reflex, and it applies to the remaining task in this flow.
- Reproduce-first on a BUG task means checking the world state (do the named
  scenarios even have the mechanism?) before assuming the report is literal. A
  five-minute grep for `surface_gravity: Some` would have surfaced the
  falsification before I framed the task around "the Ceres Queen falls in" -
  though the fix landed the same either way, the framing would have been honest
  from the start.

## Follow-ups

- Filed 20260722-105556 (backlog): a content-lint guard flagging a
  controller:None ship parked inside a well's SOI, to make the float guarantee
  load-bearing against future content nudges (review MINOR).
- The real convoy-drift fix is task 20260722-092432 (next in the flow).
