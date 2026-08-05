# Decision: Dress Shakedown Run to screenshot density: a slalom belt around a near planetoid

- DATE: 20260805-213551
- STATUS: ACCEPTED
- TASK: 20260805-213432
- TAGS: content, scenario, gravity, scatter

## Context

Shakedown Run - the scenario New Game opens - has 9 rocks and one distant
planetoid, and reads as empty black sky. The shipped screenshot scene
`rock_hollow` (`tutorial-radar-lock.png`) reaches its look with 74 seeded rocks
in two size layers. The owner wants scenario 1 at that density, and wants the
beacon-1 -> beacon-2 leg to become a slalom through a large banana-shaped belt
bending around a planetoid. Beats, script and objectives stay as they are.

Two engine facts block the straight path (full detail in NOTES.md):

- `ScatterRegion::Ring` is hard-centered on world origin; the sampled position
  overwrites the template position (`nova_scenario/src/actions/spawn.rs:274`), so
  a ring around a body at `(1240, ...)` cannot be authored.
- `soi_radius = soi_factor * body_radius` off the runtime noise-mesh radius
  (`nova_gameplay/src/gravity.rs:67`), which varies 3.5-6.0x nominal by seed. The
  planetoid's SOI is therefore anywhere in 560-960u, and the shakedown geometry
  pins are written against that whole range. Bringing the planetoid near beacon 2
  would swallow the salvage crates and the derelict in its well - which playtest
  round 2 explicitly moved it away to avoid.

## Decision

Move the planetoid in, and fix the two engine shapes that stop it, inside this
task:

1. **SOI derives from mass, not radius, and is overridable.** Authored
   mass/`mu` sets the well's reach; the mesh radius stops driving it. Exact
   formula settles in planning; the candidate is `soi_radius = sqrt(mu / a_min)`
   with an explicit per-body override. This removes the 1.7x seed lottery from
   every gravity number in the campaign.
2. **`ScatterRegion::Ring` gains `center: Vec3`,** defaulted to `Vec3::ZERO` so
   every existing scatter is unchanged.
3. **The belt is built by stacking 2-4 seeded Ring/Box scatters** into a banana
   around the near planetoid, with a separate far parallax layer - mirroring the
   reference's two-scale split (far 4-10u, near 1.2-3.2u). No new region variant.
4. **The slalom is a real but forgiving hazard.** Rocks stay collidable and
   destructible; gaps are sized generously against the 25 u/s starter speed cap.
   Clipping one costs hull, it does not end a first flight.
5. **Clear pockets are preserved** wherever a beat needs air: the crate cluster,
   the pirate fight, the orbit ring.

**Definition of Done is a timebox, not a metric: two or three iteration rounds.**
Land a first cut, look at it, retune, look again; done is the owner calling it
after those rounds. The task is explicitly not "reach a measured density" - it is
"make it lookable, then iterate".

## Alternatives considered

- **Shrink the planetoid instead of fixing the SOI.** Cheapest - one const and
  retuned rings, no engine change. Rejected: shrinking is the wrong direction for
  a body whose job is to be the landmark anchoring the turn, and it leaves the
  SOI a per-seed lottery.
- **Keep the planetoid big and push the later beats out.** Preserves the awe but
  re-tunes half the level's geometry and lengthens legs at a 25 u/s cap.
- **Keep the planetoid big and let gravity be felt at the crate beat.** Reverses
  playtest round 2 finding 1 ("the player was fighting gravity while weaving
  crates").
- **Add a first-class `ScatterRegion::Arc`.** Exactly expresses a banana belt and
  is reusable, but stacked rings approximate it well enough - YAGNI until a
  second scenario asks. Fallback if stacking proves unworkable during iteration.
- **Hand-author a 60-80 entry offset table.** Total control, no engine change,
  trivial tests - but unworkable to iterate, and iteration is the point.
- **Scatter offsets by template position for all regions.** One rule instead of a
  new field, but silently changes what every existing world-space `Box` scatter
  means.

## Consequences

- Every scenario that authors `surface_gravity` must be re-checked against the
  new SOI derivation. The set is not yet enumerated - a planning step.
- `beat4_geometry_holds_across_the_derived_radius_range`
  (`shakedown/tests/pins.rs:414`) is rewritten: with a radius-independent SOI it
  collapses from a 3.5-6.0x range sweep to single numbers.
- Rock count rises well past the current 9 in a scenario the player is guaranteed
  to load. Scatter is never thinned by graphics tier, so this is a perf question -
  `nova_probe` on the shakedown player path is the check each round.
- The debris cluster may stay a hand-placed table (it carries the crate beat's
  clear pocket) even as the belt around it goes seeded. Decided by looking at it.
- A forgiving hazard on beat 2 is new failure surface in the tutorial. Hull
  damage from clipping a rock is now something a first-time player can meet.
