# AI torpedo threat response: point-defense + break-off burn

- STATUS: CLOSED
- PRIORITY: 64
- TAGS: v0.4.0,ai,spike,torpedo,turret


Spike: tasks/20260709-225508/SPIKE.md (wave 3)

Goal: the AI defends itself against torpedoes. Detect a hostile torpedo
whose target is me; prioritize it as a turret target (PDC role - this should
mostly fall out of target-type scoring in 20260709-225727) and/or break with
a perpendicular burn while it closes (stressing PN guidance). First consumer
of target-type scoring beyond ships.

Depends on: 20260709-225727 (target selection), 20260709-225731 (evade
maneuver machinery).

## User decision (20260710)

Turrets' PDC role is PRIMARY: hostile torpedoes take priority over ships
for turret aim and fire (the PDC's main purpose is defending against
torpedoes). Flight target selection stays ship-first - the hull keeps
chasing ships while the guns deal with inbound ordnance. Pulled forward
ahead of the standoff-flight task on user request.

## Steps

- [x] `AIPointDefenseTarget(Option<Entity>)` component in `input/ai.rs`,
      required by the AI marker (default None), registered + in prelude.
- [x] Pure `pick_point_defense_target(own_anchor, candidates)`: candidates
      are hostile COMMITTED torpedoes within AI_POINT_DEFENSE_RANGE
      (constant, inside the turrets' effective range); torpedoes whose
      `TorpedoTargetEntity` is ME outrank ones hunting someone else
      (tiered, like the ship/torpedo tiers); nearest wins within a tier.
- [x] System `update_point_defense_target` (chained after
      `update_ai_target`): per AI ship, gather hostile committed torpedoes
      via the relation model, resolve targeting-me from
      `TorpedoTargetEntity`, write the component.
- [x] Turret override: `update_turret_target_input` and
      `on_projectile_input` aim/fire at the point-defense target when one
      exists, else the primary `AITarget`; the velocity feed follows the
      same override. The burst cadence is BYPASSED while defending -
      point defense fires continuously; the discipline gates (range,
      aim-point alignment) still apply.
- [x] Tests: picker matrix (targeting-me outranks nearer other-victim
      torpedo; range gate; empty -> None); turret override (turret aims at
      the torpedo while the ship's AITarget stays the hostile ship); flight
      unaffected (AITarget still ship-first, existing test); cadence
      bypass (hold phase + pd target still fires).
- [x] Verify: cargo fmt, cargo check --workspace, ai:: module tests (skip
      full local suite per user instruction; report skips honestly).

## Notes

- The break-off/perpendicular-burn half of this task needs the evade
  maneuver machinery and stays with 20260709-225731 (noted there via the
  arc ordering); this task ships the point-defense half.
- Relevant: sections/torpedo_section/mod.rs (TorpedoTargetEntity,
  TorpedoTargetChosen), input/ai.rs.

## Resolution (20260710)

Shipped the point-defense half on user request (pulled ahead of standoff
flight): AIPointDefenseTarget (required by the marker), the tiered picker
(hunting-ME torpedoes outrank ones hunting someone else, nearest within a
tier, 400 m range inside the turrets' 450 m effective envelope), the
acquisition system over the relation model + TorpedoTargetEntity, and the
turret override - guns aim/fire at the inbound torpedo in EVERY behavior
state (an idle ship still defends itself), the lead velocity feed follows
the gun target, and the burst cadence is bypassed while defending (bursts
are discipline for ships, not inbound ordnance). Flight targeting is
untouched: the hull keeps chasing ships (pinned by test). 5 new tests;
full crate suite 218/218 green this once, fmt + check clean. Skipped per
user instruction: clippy.

The break-off/perpendicular-burn half stays with the evade machinery
(20260709-225731), as noted.
