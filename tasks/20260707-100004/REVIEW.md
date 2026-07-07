# Review: Torpedo despawns silently when its target dies mid-flight

- TASK: 20260707-100004
- BRANCH: feature/torpedo-target-loss

## Round 1

- VERDICT: APPROVE

Delivers the Goal. `update_target_position` now drops the dead `TorpedoTargetEntity`
link instead of despawning the torpedo, so a torpedo whose target dies keeps flying
toward the frozen `TorpedoTargetPosition` (freeze-and-continue), and - no longer
matching `With<TorpedoTargetEntity>` - stops re-looking-up the dead entity and
spamming the log. Minimal, correct, and the intended behavior from the task.

Verified independently in the worktree:

- `cargo test -p nova_gameplay torpedo`: 7/7 pass, including the new
  `torpedo_survives_target_loss_and_freezes_position` (target alive -> tracked;
  target despawned -> torpedo survives, position frozen at last value, dead link
  removed). Asserts behavior, deterministic.
- `cargo clippy -p nova_gameplay`: clean.
- Range smoke (`BCS_AUTOPILOT=1 ... 06_torpedo_range --features debug`, Xvfb): still
  3 fired/armed/detonated, cycle complete, no panic, and the old
  `not found in q_target` per-frame spam is gone (0 occurrences).

Checked the obvious regression risk: the removed `despawn()` was not doing scenario
cleanup - torpedoes get `ScenarioScopedMarker` via `on_add_entity_with::<TorpedoProjectileMarker>`
(loader.rs:87), so they are still torn down on scenario change. No leak.

No BLOCKER/MAJOR. One observation.

- [ ] R1.1 (NIT) crates/nova_gameplay/src/sections/torpedo_section.rs:438 - dropping
  the link makes the torpedo eligible for re-targeting (the game's player targeting
  and the range's `range_autotarget` both assign to `Without<TorpedoTargetEntity>`
  torpedoes), so a torpedo whose target dies will re-acquire the ship's current
  target if one is selected rather than strictly freezing. This is reasonable
  (redirects to a live target) and is called out in the code/TASK.md, so it is
  intended, not a defect. Noting it so a future reader knows the re-acquire is
  deliberate; no change requested.
  - Response:
