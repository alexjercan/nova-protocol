# Bullets affected by gravity wells

- STATUS: OPEN
- PRIORITY: 40
- TAGS: feature,gameplay,spike,v0.5.0

Make turret rounds (bullets) feel gravity wells, the same way ships and
torpedoes already do. Today only ship roots and torpedo projectiles opt into
`GravityAffected`; turret rounds and section debris deliberately skip it.

## Start with a spike

This is not a straight extension - it reverses an explicit design decision, so
it needs a short spike before any code.

- The current behaviour is intentional (spike decision 5 in
  `docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md`): turret
  rounds skip gravity because flight times are short and per-bullet well
  queries were judged "pure cost for imperceptible curvature". See the
  `GravityAffected` doc comment at `crates/nova_gameplay/src/gravity.rs:83`.
- The spike should answer: is the curvature actually perceptible at bullet
  speeds/ranges near a well, or does this only matter for long-range/slow
  shots? What is the per-frame cost of running the well-force system over the
  (potentially many) live rounds, and does it scale? Should debris opt in too,
  or bullets only? Any gameplay-balance angle (curving shots as a mechanic vs.
  a frustration)?
- Output: a decision (do it / don't / do it conditionally) plus, if go, the
  concrete tasks that `/plan` then breaks into steps.

## Implementation sketch (pending the spike)

- Insert `GravityAffected` on turret rounds at spawn, mirroring
  `insert_gravity_affected_on_torpedo` in
  `crates/nova_gameplay/src/gravity.rs`. Round spawning lives in
  `crates/nova_gameplay/src/sections/turret_section.rs`.
- The existing `gravity_well_system` already integrates any `GravityAffected`
  entity, so no new force math is needed - only opt-in plus whatever
  performance guard the spike decides on.
- Update the `GravityAffected` doc comment (it currently states rounds skip
  v1) so it stays accurate.

