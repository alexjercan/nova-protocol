# Retro: Diegetic objective reveal

- TASK: 20260721-211520
- BRANCH: feat/objective-reveal
- REVIEW ROUNDS: 2 (round 1 out-of-context APPROVE with non-blockers; round 2
  in-session after addressing them)

(What/why + evidence in TASK.md close-out; design decisions in NOTES.md; this is
process only.)

## What went well

- Reusing the `screen_indicator` placement idiom (screen position via px
  `Node.left/top`, `UiTransform` for scale + rotation only) avoided a coordinate
  rabbit hole - I never had to answer "does a UI node's `GlobalTransform` include
  its `UiTransform`". This is exactly the `reuse-known-good-stack` pending
  promotion from the previous task (20260724-102304) paying off one cycle later:
  I looked for the in-repo node-to-screen-point mover FIRST and copied it.
- Hooking the reveal into `objective_feedback`'s single `GameObjectives` diff kept
  one detection point (no second change-detector to drift), and made the
  gold-ghost removal a local, well-contained swap.
- The out-of-context review APPROVEd with only a MINOR + 2 NITs, all real and
  cheap; addressing them (doc the centering assumption, add the y-axis test
  assertion, comment the orphan node) was a few minutes and left the branch
  tighter than "good enough".

## What went wrong

- Two test-clock false starts, both fixed in the rig not the code:
  1. The reveal advances on the default `Res<Time>`, which barely moves under
     microsecond real-time `app.update()`s, so the lifetime/despawn assertions
     never fired. Needed a manual `TimeUpdateStrategy::ManualDuration` clock (the
     comms/feedback tests already do this - I should have copied their rig setup
     wholesale from the start, again the `reuse-known-good-stack` lesson).
  2. `clear_reveals_on_teardown` (run_if `resource_changed::<GameObjectives>`)
     fired on the empty-INIT frame - a freshly `init_resource`'d resource reads as
     changed - and despawned the reveal before the assertions ran. Root cause:
     didn't account for `resource_changed` being true on frame 1. Fixed by seeding
     objectives non-empty first (mirroring production, where a reveal only exists
     alongside objectives).

## What to improve next time

- A `run_if(resource_changed::<T>)` system that acts on an EMPTY/default `T`
  (teardown-on-empty, reset-on-clear) will fire on the resource's init frame -
  either guard it against the default state or, in tests, drive `T` to a
  non-default value before the behavior under test.
- When a system reads a clock, copy the nearest existing test's clock setup
  (manual-duration + step) verbatim before writing assertions - two of this
  cycle's iterations were clock-shaped and both were already solved in sibling
  test modules.

## Action items

- [x] Review findings R1.1-R1.3 addressed on the branch.
- [x] Lessons ledger updated (see below); no follow-up code task - the reveal's
  siblings (comms log 102309, drawer z-order 121541) are already queued.
