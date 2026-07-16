# Broadside playtest tuning: drop the player torpedo bay, give infinite turret ammo

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.7.0,balance,scenario,playtest


## Goal

User playtest verdict (2026-07-16, first hands-on of Broadside): "too hard" -
(1) the player should NOT have a torpedo bay - story-wise it is not unlocked
yet; torpedoes stay the ENEMY's weapon this chapter, which also keeps the
PDC-screening fantasy pure (you defend against torpedoes, you don't trade
them); (2) the player needs more ammo - go infinite_ammo like Shakedown
(chapter one's precedent; no resupply mechanic exists to make scarcity a
managed pressure yet; finite-but-bigger would need a new ammo modification
surface, deferred until ship management exists).

## Steps

- [ ] broadside.rs player_ship(): remove the torpedo section + its input
      binding; infinite_ammo: true; amend the loadout doc comment.
- [ ] Regenerate broadside.content.ron via content_ron_parity (delete + run
      twice).
- [ ] Flip the structural pin: on_start_stages_the_slice asserts NO torpedo
      bay on the player + infinite_ammo, keeping the gunship's tubes pinned.
- [ ] Sweep prose claims of a player torpedo (wiki/CHANGELOG/tutorial) -
      the PDC-screening copy stays true.
- [ ] Verify: parity x2, broadside_assault suite, one live example-19 walk.

## Notes

- Playtest VERDICT record per flow discipline: slice shipped f53fa5e8;
  feedback arrived same day, filed at p90 per the v0.7.0 plan's policy.
- Depends on: 20260708-203659 (CLOSED, landed).
