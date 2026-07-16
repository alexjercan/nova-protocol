# Broadside playtest tuning: drop the player torpedo bay, give infinite turret ammo

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.7.0, balance, scenario, playtest

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

- [x] broadside.rs player_ship(): remove the torpedo section + its input
      binding; infinite_ammo: true; amend the loadout doc comment.
- [x] Regenerate broadside.content.ron via content_ron_parity (delete + run
      twice).
- [x] Flip the structural pin: on_start_stages_the_slice asserts NO torpedo
      bay on the player + infinite_ammo, keeping the gunship's tubes pinned.
- [x] Sweep prose claims of a player torpedo (wiki/CHANGELOG/tutorial) -
      the PDC-screening copy stays true.
- [x] Verify: parity x2, broadside_assault suite, one live example-19 walk.

## Notes

- Playtest VERDICT record per flow discipline: slice shipped f53fa5e8;
  feedback arrived same day, filed at p90 per the v0.7.0 plan's policy.
- Depends on: 20260708-203659 (CLOSED, landed).

## Close record (20260716)

Player ship: torpedo bay + its binding removed, infinite_ammo: true, doc
comment carries the story rationale (torpedoes unlocked later; screening
stays pure defense; no resupply mechanic = dry magazines are frustration).
RON regenerated via parity (x2 green). Pins flipped and extended: the
structural test asserts NO player torpedo + the better turret + infinite
ammo, and a new test pins the gunship's tubes (>= 2) so the enemy half of
the verdict cannot silently regress. Prose sweep clean (the PDC-screening
copy was already player-torpedo-free); the unreleased CHANGELOG entry stays
accurate. Example 19 unaffected by design (kills via HealthApplyDamage);
re-ran the live walk green.

Alternative considered: finite-but-bigger magazine via a new ammo
SectionModification - deferred until ship management/resupply exists to
make scarcity a real mechanic (the ammo HUD task 20260716-123556 pairs
with that future).
