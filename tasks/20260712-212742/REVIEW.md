# Review: Committed torpedoes join the combat target set

- TASK: 20260712-212742
- BRANCH: feature/torpedo-combat-target

## Round 1

- VERDICT: APPROVE

Independent verification (shared-session blind-spot guard): checked the claim
the whole feature rests on - that a production enemy torpedo is actually
HOSTILE to the player (else it would be Neutral and never enter the
`is_hostile && is_combat_target` set, making this a no-op in-game). Confirmed
`sections/torpedo_section/mod.rs:686-690`: "the torpedo COPIES the shooter's
allegiance", `projectile.insert(allegiance)`, with a test
(`launched_torpedo_copies_the_shooter_allegiance`). So an enemy-fired torpedo
carries `Allegiance::Enemy`, `relation(Player, Enemy) == Hostile`, and it enters
the cycle set. Feature is real.

Also verified: the `is_combat_target = is_ship || is_torpedo.is_some()` combine
is sound because uncommitted torpedoes are already `return None`-d earlier in
the collection, so every torpedo reaching the tuple is committed; nav bodies
(no ship marker, no torpedo marker) are correctly excluded, keeping GOTO
re-designation aim-driven (the review R1.1 invariant from 20260712-203353).
Tests are delivery-guarded (the sticky-torpedo test re-acquires the ship once
the torpedo is gone), and the test that encoded the old "torpedoes stay out"
contract was correctly rewritten, not deleted.

- [ ] R1.1 (NIT) input/targeting.rs - the candidate `entries` cap
  (`TARGET_CANDIDATE_COUNT` = 5) is now shared between enemy ships and incoming
  torpedoes, so a torpedo swarm could push enemy ships out of the top-5 cycle
  (and the edge-indicator overlay). This is arguably correct - torpedoes are the
  urgent threat you want to cycle to - and the current lock always stays a
  member, so it is a feel/tuning consideration for playtest, not a bug. If it
  crowds in practice, a separate cap or a ship/torpedo interleave is the knob.
  - Response:

Check suite (repo policy: full suite + clippy in CI): `cargo test -p
nova_gameplay targeting` 45 pass (added `a_committed_torpedo_lock_is_sticky`,
rewrote `candidates_track_hostile_combat_targets_including_torpedoes`);
`12_hud_range` + `10_gameplay` autopilots PASS/no-panic; `fmt --check` clean.
