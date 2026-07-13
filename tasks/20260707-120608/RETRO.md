# Retro: Torpedo second despawn path (incomplete target-loss fix)

- TASK: 20260707-120608
- BRANCH: feature/torpedo-target-despawn
- PR: #30 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, one NIT deferred to guidance)

This is a follow-up to `20260707-100004`, which the user found only fixed the bug
for asteroids. The retro is mostly about *why the first fix was incomplete*.

## What went wrong (the important part)

The `100004` fix changed `update_target_position` to drop the target link instead
of despawning - but there was a **second despawn path**, `update_torpedo_target_input`
in `input/player.rs`, that deletes every un-targeted player torpedo when the aim
lock is `None`. It even carried a `TODO(...): Maybe think of something better then
just despawning the torpedo?`. My `100004` fix made the torpedo un-targeted on loss,
which fed it straight into that second despawn. Asteroids masked the whole thing
because their `RigidBody` husk (`20260706-212910`) never despawns, so the link is
never dropped and neither despawn path fires.

Two root causes:

1. **I fixed the despawn in the function the task named, without grepping for other
   despawns gated on the same condition.** A `grep despawn` across the torpedo +
   targeting systems would have surfaced `player.rs` immediately - the confessing
   TODO was right there.
2. **The verification exercised a proxy, not the reported scenario.** The `100004`
   unit test despawned the target and ran `update_target_position` *in isolation*,
   proving that one system no longer despawns - but never ran the full targeting
   pipeline where the second despawn lives. The range smoke didn't catch it either,
   because the range's `range_autotarget` always re-locks, so a torpedo is never
   left un-targeted with no lock. Both masked the exact failing case.

## What went well

- Once the user reported it, diagnosis was fast and evidence-based: `grep despawn`
  over nova_gameplay found `player.rs:190`, and reasoning through the
  freeze-fix + husk interaction explained the asteroid-only symptom exactly.
- Filed a proper follow-up task with the full root-cause writeup rather than quietly
  patching, and covered it with deterministic tests that drive the actual system
  (both the no-lock survive case and the lock-assigns case).

## What to improve next time

- For any "stop X from deleting Y when C" fix: `grep` every despawn/delete of Y in
  the codebase and check each against condition C. Do not assume the one in the
  task's named function is the only one - especially when a `TODO` elsewhere admits
  to the same behavior.
- Make the regression test reproduce the *reported* scenario end to end (target with
  a lifetime that despawns, no re-lock), not just the one system in isolation. An
  isolated-system test can pass while a sibling system still triggers the same
  symptom.

## Action items

- [ ] NIT R1.1 -> PN guidance (`20260525-133021`): a torpedo fired with no lock now
      flies to the world origin; "fly straight ahead when unlocked" belongs there.
- [ ] The asteroid husk (`20260706-212910`) masks target-loss behavior and made this
      bug asteroid-invisible. Worth doing partly to remove that confound from future
      torpedo testing. Already tracked.
