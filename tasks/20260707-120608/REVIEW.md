# Review: Torpedo still vanishes on target loss (second despawn path)

- TASK: 20260707-120608
- BRANCH: feature/torpedo-target-despawn

## Round 1

- VERDICT: APPROVE

Correctly identifies and fixes the real culprit the 100004 fix missed:
`update_torpedo_target_input` despawned every un-targeted player torpedo when the
aim lock was `None`. The no-lock branch is now a no-op return, so a torpedo whose
target died (link dropped by `update_target_position`) keeps flying toward its
frozen position instead of blinking out. Consistent with the freeze-and-continue
behavior from 100004, and matches the diagnosis (the asteroid husk masked the bug).

Verified independently in the worktree:

- `cargo test -p nova_gameplay`: 15/15 pass, including the two new player tests -
  `no_lock_does_not_despawn_untargeted_torpedo` (None lock -> torpedo survives, no
  target assigned) and `lock_assigns_target_to_owned_torpedo` (Some lock -> owned
  torpedo gets `TorpedoTargetEntity`). Deterministic, assert behavior.
- `cargo clippy -p nova_gameplay`: clean.
- Range smoke (`BCS_AUTOPILOT=1 ... 06_torpedo_range --features debug`, Xvfb): no
  regression - 3 fired/armed/detonated, cycle complete, no panic.
- Grepped the torpedo/targeting systems: the only remaining despawns are the
  intended detonation (`torpedo_detonate_system`) and the `TempEntity` lifetime.
  No despawn-on-target-loss path remains.

No BLOCKER/MAJOR. One follow-up observation.

- [ ] R1.1 (NIT) A torpedo fired with no lock at all now flies toward its initial
  `TorpedoTargetPosition` (world origin, `Vec3::ZERO`) rather than being deleted -
  not a vanish, but flying to the origin is odd. The proper behavior (fly straight
  ahead when unlocked) is guidance work, already tracked by PN guidance
  (`20260525-133021`); this fix only needed to stop the vanish, which it does. No
  change requested here.
  - Response:
