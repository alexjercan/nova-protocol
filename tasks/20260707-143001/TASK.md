# Torpedoes commit to their launch target and never retarget

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0,torpedo,behavior

Reported while testing the PN guidance (PR #31): fire a torpedo with no target,
then shoot bullets - the loitering torpedo picks a bullet up as its target and
detonates on it. Mechanism: the player aim cast locks whatever collider it hits
(bullets included - they fly exactly down the aim ray), and
`update_torpedo_target_input` hands the lock to every torpedo that currently has
no `TorpedoTargetEntity`. After the target-loss fix, torpedoes without targets
loiter, so they keep re-entering that assignment pool.

Decision (user): bullets ARE valid targets - the fix is not target filtering but
target *commitment*. A torpedo's targeting decision happens once, at launch:
whatever the input system (player crosshair now, spaceship AI later) has locked at
that moment is the torpedo's target for life. If the target dies, the torpedo
keeps flying to the last known position (existing freeze behavior) and does NOT
re-acquire. A torpedo fired with no lock is committed dumb-fire and never picks
anything up later. (Future counterplay like flares can revisit this; out of scope.)

## Steps

- [x] Add a `TorpedoTargetChosen` marker component: inserted the first time the
      input targeting system processes a torpedo, with or without a lock. Once
      present, no targeting system touches the torpedo again.
- [x] Rework `update_torpedo_target_input`: query
      `Without<TorpedoTargetEntity> + Without<TorpedoTargetChosen>`, insert the
      marker on every owned torpedo it processes, plus the target when a lock
      exists. Target death keeps removing the link (stops the dead lookup) but the
      marker prevents re-acquisition.
- [x] Update the example autotargets (`06_torpedo_range`, `07_torpedo_guidance`)
      to the same contract (assign once + insert the marker).
- [x] Tests: the bullet regression (no lock at launch -> commit dumb-fire; a lock
      appearing later must NOT be assigned) and no-retarget-after-target-loss
      (committed torpedo whose link was removed must not get a new target).
- [x] Verify: torpedo tests + player tests green, clippy clean, 06 + 07 headless
      smoke runs still detonate as before.

## Notes

Source: `crates/nova_gameplay/src/input/player.rs` (`update_torpedo_target_input`),
`crates/nova_gameplay/src/sections/torpedo_section.rs` (component + freeze path).
Same branch/PR as the PN work (feature/torpedo-pn-guidance, PR #31) per user
request.

## Resolution

Added `TorpedoTargetChosen` (torpedo_section.rs, prelude-exported): the launch-time
targeting decision marker. `update_torpedo_target_input` now processes only
torpedoes without it, stamps it on every owned torpedo it sees, and assigns
`TorpedoTargetEntity` only when a lock exists - so a lock at launch is kept for
life, and a no-lock launch is committed dumb-fire. Target death still removes the
link (freeze-and-continue), but the marker blocks re-acquisition. Both example
autotargets (06/07) mirror the contract.

Tests: `dumbfire_torpedo_ignores_later_locks` (the bullet scenario),
`committed_torpedo_does_not_retarget_after_target_loss`, plus marker asserts in
the two existing player tests - 26 nova_gameplay tests green. Clippy clean.
Headless smoke: 06 = 3 fired / 3 detonated, 07 = 2 detonations, no panics.
