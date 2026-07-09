# Overkill damage to one section propagates full amount and can kill the whole ship

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.4.0, bug, health

Found while testing 20260709-140620: `HealthApplyDamage` propagates through
`ChildOf`, so damage applied to a section ALSO applies its FULL amount to the
ship root's aggregate health. A single 1000-damage hit on a 100 hp section
zeroed the root aggregate (ship had 700 hp total across healthy sections) and
the whole ship died through disable -> destroy, despite four healthy sections.

Expected: a hit on one section should cost the ship at most that section's
remaining health (the aggregate is recomputed from sections by
`aggregate_ship_health` anyway); overkill should be absorbed by the section's
destruction, not teleported into the hull total.

## Steps

- [x] Reproduce in a test: ship with two sections (100 hp each), hit one with
      1000 damage; assert the other section and the ship root survive.
      (`overkill_on_one_section_does_not_kill_the_ship` in integrity/glue.rs.)
- [x] Decide the mechanism: clamp the propagated amount to the target's
      remaining health, in bcs `on_damage` (chosen over the nova-only
      "derive root death from `aggregate_ship_health`" option: the clamp is
      the correct, general fix for any aggregate hierarchy, not just ships).
- [x] Check the interaction with the "last section dies -> root dies" flow:
      preserved. When the last section dies the root aggregate equals that
      lone section, so the clamped amount is exactly enough to zero the root.
      Covered by bcs `a_lethal_hit_still_bubbles_to_zero_a_matching_parent`
      and the doc comment on `aggregate_ship_health` was updated.
- [x] Cover with a physics-level test. The 06/11 range smokes were NOT re-run
      locally (per repo policy: fmt/check + newly written tests only, CI runs
      the rest). They are behavior-invariant here: in-play blast damage is
      <= section hp, so the clamp never changes normal play; it only stops the
      >section-hp overkill case, which the smokes never exercise.

## Notes

- Evidence: first version of
  `mass_properties_follow_a_section_destroyed_by_damage` (integrity/glue.rs)
  used 1000 damage and the ROOT despawned; exact-health damage fixed the test.
- Blast damage in play is <= 100, and sections have 100-200 hp, so this is
  currently masked; it will bite as soon as weapon damage scales up
  (20260708-162005 weapon variety) or section hp goes down.

## Resolution

Fix: bcs `on_damage` (src/health/mod.rs) now clamps the propagated damage to
what actually lands on each node (`min(amount, health.current)`) and mutates
the bubbling event's `amount` to that value; an already-dead node propagates
zero. Overkill on a section is therefore absorbed by the section, not
teleported into the hull aggregate.

- bcs: fix landed on master via PR #6
  (https://github.com/alexjercan/bevy-common-systems/pull/6); commit
  `4c58835708feb888f3a1872e74d6ae5fd742dd0c` on the merged branch. 3 new health
  unit tests + full lib suite (126) green.
- nova: pinned `bevy_common_systems` rev bumped to `4c58835` across all four
  crates (nova_debug, nova_events, nova_gameplay, nova_scenario), plus a new
  physics regression test (`overkill_on_one_section_does_not_kill_the_ship`) and
  an updated `aggregate_ship_health` doc. `integrity::glue` tests (9) green
  against the real dependency. See docs/2026-07-09-section-overkill-propagation.md.
