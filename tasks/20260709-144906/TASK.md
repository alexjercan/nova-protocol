# Overkill damage to one section propagates full amount and can kill the whole ship

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.4.0,bug,health

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

- [ ] Reproduce in a test: ship with two sections (100 hp each), hit one with
      1000 damage; assert the other section and the ship root survive.
- [ ] Decide the mechanism: clamp the propagated amount to the target's
      remaining health (bevy_common_systems health propagation - our crate,
      task in ~/personal/bevy-common-systems per AGENTS.md), or stop
      root-propagation for sectioned ships and derive root zero-health purely
      from `aggregate_ship_health`.
- [ ] Check the interaction with the "last section dies -> root dies" flow
      documented in integrity/glue.rs (it currently RELIES on the propagated
      fatal damage to mark the root; see the doc comment on
      `aggregate_ship_health`).
- [ ] Cover with a physics-level test and re-run the 06/11 range smokes.

## Notes

- Evidence: first version of
  `mass_properties_follow_a_section_destroyed_by_damage` (integrity/glue.rs)
  used 1000 damage and the ROOT despawned; exact-health damage fixed the test.
- Blast damage in play is <= 100, and sections have 100-200 hp, so this is
  currently masked; it will bite as soon as weapon damage scales up
  (20260708-162005 weapon variety) or section hp goes down.
