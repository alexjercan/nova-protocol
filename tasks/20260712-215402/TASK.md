# Cyclable nav bodies: notable asteroids/beacons join the CTRL+scroll cycle at range (non-sticky, no debris)

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.5.0, targeting, navigation, spike

## Goal

Let CTRL+scroll reach notable signed nav bodies (asteroids, beacons) at range,
so you can flick to a far body you cannot pixel-aim and GOTO it - without adding
unsigned debris (annoying at 500+ m). Combat targets (ships, committed
torpedoes) stay STICKY; nav bodies join the cycle as NON-sticky entries (they
ride the existing 4 s `pinned_until` after a cycle press, long enough to press
G for GOTO, then the aim pick resumes).

Direction (see spike): in `update_spaceship_target_input` (input/targeting.rs),
broaden the ranked cycle set (`rank_combat_targets` -> `entries`) from
`is_hostile && is_combat_target` to ALSO include signed non-debris bodies above
a signature/size threshold; keep the sticky `held` gate COMBAT-ONLY so nav bodies
stay aim-re-designatable. Debris (unsigned) is excluded for free (no
`LockSignature`). Guard asteroid-field clutter: a signature threshold for cycle
eligibility and reserve cap slots for combat targets so a field cannot crowd
ships/torpedoes out of the 5-slot cycle + edge indicators.

## Notes

- Spike: docs/spikes/20260712-215256-combat-travel-lock-separation.md
  (Part A, option A1).
- This is a stopgap the combat/travel mode toggle (a future direction in the
  spike doc, not yet a task) later subsumes; worth landing on its own.
- Relevant: `TARGET_CANDIDATE_COUNT` (cap), `maintain_candidates`,
  `rank_combat_targets`, the `is_hostile && is_combat_target` filter, the
  LockSignature range model; check against 04_asteroids for clutter.
- Playtest: does non-sticky-but-pinned (4 s) feel right for GOTO, or does travel
  want a longer hold (an input into whether the mode toggle - a future spike
  direction - is needed sooner)?
