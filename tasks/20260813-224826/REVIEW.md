# Erosion-level follow-up review

Review handoff for the agent completing the C+A section-carve performance fix.
Use this after that work is merged into `erosion-level`. Re-check the final code
before changing it because C+A can overlap section traversal and mark reach.

This record does not cover the proposed asteroid health and mining redesign.
That is a separate design discussion.

## Reviewed state

- Branch: `erosion-level`
- Review head: `4ae35bf6`
- Relevant delivery:
  - `265c085c` - accumulated crater volume and grid-cell remesh throttling
  - `8cd4987e` - explicit asteroid durability migration
  - `4ae35bf6` - shipped-size real-PDC asteroid gallery
- Method: static trace through damage, health, marks, signed fields, ship and
  asteroid consumers, debris, and destruction.
- Runtime verification was not repeated during this review.

## Expected C+A change

The separate profiling work found that one ship-root mark made every carving
section walk and solidify all descendant art in one frame. The accepted C+A
correction is expected to:

- test each mark sphere against cached art bounds before solidifying;
- use a conservative radius so the gate can make false-positive work but never
  skip reachable geometry;
- spend at most one successful solidification globally per frame.

C+A attacks irrelevant first-hit work. It does not automatically fix the damage
accounting, blast coalescing, debris fan-out, or asteroid remesh transaction
below. Inspect the merged implementation before assuming any item remains.

## Landing blockers

### 1. Carving uses requested damage instead of absorbed damage

Locations at the reviewed head:

- `crates/nova_gameplay/src/damage.rs:276-290`
- `crates/nova_gameplay/src/integrity/health.rs:117-150`
- `crates/nova_gameplay/src/integrity/carve.rs:177-214`

`apply_damage` queues `record_damage_mark` with the requested amount before
`HealthApplyDamage` reaches `on_damage`. The health observer later clamps that
amount to the target's remaining health, but the mark and `CarveSpew` have
already been priced from the unclamped value.

Concrete failure:

1. A 100-damage kinetic round reaches a plate with 20 HP.
2. The ship root records a 100-damage carve.
3. Health absorbs 20 and the round carries 80 onward.
4. The remaining round can hit the hull and record another 80-damage carve.
5. One 100-damage projectile bought 180 damage worth of visible removal.

A hit on an already spent node has the same defect: health changes the applied
amount to zero, but the hit can still grow a crater and emit debris.

Required invariant:

- A mark and its ejecta are priced from damage actually absorbed by the target,
  not damage offered to it.
- A spent target that absorbs zero adds no mark and emits no carve debris.
- A kinetic round cannot buy more total carve volume than its damage budget.

Recommended reproduction:

- Add an `examples/systems/` range case for a high-overkill kinetic round
  crossing a low-health plate into a hull.
- Assert health absorbed, projectile remainder, root mark volume, and emitted
  debris volume together.
- Add a spent-target case.

Recommended direction:

- Make applied damage an explicit result/event from the health application
  seam, carrying the hit location and attribution needed by carving.
- Record the mark only after the target pool has computed the absorbed amount.
- Do not pre-read `Health` at weapon call sites. That would duplicate the health
  store's clamp and race same-frame damage.

The exact event interface is a design decision. Record it in `TASK.md` before
implementation.

### 2. Blast marks and debris fan out per collider, not per body

Locations at the reviewed head:

- `crates/nova_gameplay/src/damage.rs:592-635`
- `crates/nova_gameplay/src/integrity/carve.rs:182-214`
- `crates/nova_gameplay/src/integrity/spew.rs:310-375`

Blast resolution calls `apply_damage` once per overlapping target collider.
Many ship colliders resolve through `mark_owner` to the same ship-root
`DamageMarks`. Each target therefore grows the same root crater and emits a
separate `CarveSpew` at the blast centre.

Consequences:

- One blast's shared crater is priced from the sum of descendant hits, then the
  enlarged sphere is read by every intersecting carving mesh.
- Each descendant with at least `CHUNK_MIN_VOLUME` damage can emit up to three
  physical chunks.
- One capital-ship blast can create a large set of 30-second rigid bodies.

C+A can reduce how many art meshes consume the root mark. It does not fix the
root mark being added repeatedly unless the merged work explicitly coalesces
blast damage by mark owner.

Required invariant:

- Health remains per target section.
- Located geometry damage and carve ejecta are coalesced once per damage source,
  mark owner, and impact location.
- One blast cannot multiply its crater or debris count by the number of child
  colliders it overlaps.

Recommended reproduction:

- Add a range case with one ship root, several health-bearing descendant
  colliders, and one blast.
- Assert that all intended health pools take pressure.
- Assert that the root receives one coalesced geometric contribution.
- Assert that carve debris is bounded per body, not per collider.

Recommended direction:

- Aggregate blast carve contributions by `(blast, mark owner)` before recording
  marks.
- Use the maximum applied pressure reaching that owner for the shared blast
  sphere, rather than summing pressure across descendants. Summing applies each
  descendant's payment back to all geometry and recreates the multiplier.
- Emit `CarveSpew` once from the coalesced body-level contribution.

The coalescing key and maximum-pressure rule are interface decisions. Confirm
and record them in `TASK.md` before implementation.

### 3. Asteroid remesh failure mutates state before validation

Locations at the reviewed head:

- `crates/nova_scenario/src/objects/asteroid_carve.rs:369-423`
- `crates/nova_gameplay/src/mesh/field.rs:183-218`

`split_off_islands` mutates the parent field. The caller then queues severed
chunks before it builds and validates the replacement trimesh collider. If
`Collider::trimesh_from_mesh` returns `None`, the old visible mesh and collider
remain while the internal field has already lost the islands.

Consequences:

- Spawned chunks can overlap geometry still present in the old parent mesh.
- Internal field, visible mesh, and collider disagree.
- `meshed_volume` remains stale, so the system retries surface and collider
  construction every frame.
- Carving the field completely empty naturally reaches the unusable-collider
  branch.

Required invariant:

- Field, mesh, collider, published radius, and severed chunks commit as one
  transaction.
- A failed candidate leaves the previous complete state intact.
- Failure does not retry expensive work every frame without a new mark.

Recommended reproduction:

- Add an asteroid range or focused integration test that carves a field to an
  empty or otherwise unusable surface.
- Assert no chunk is committed when the replacement collider is rejected.
- Assert field/mesh state remains coherent and the failed candidate is not
  rebuilt every frame.

Recommended direction:

1. Clone the current field into a candidate when a remesh threshold is crossed.
2. Split islands from the candidate.
3. Build candidate surface and collider.
4. If validation fails, retain the current field, mesh, collider, radius, and
   chunk set. Mark the failed signature as handled or define a terminal-empty
   result so it does not retry each frame.
5. If validation succeeds, commit the candidate and then spawn its islands.

A 32^3 field clone is small compared with remeshing and collider construction.
Prefer the transaction over partial rollback.

## Performance follow-ups

These are measurement items, not landing blockers without evidence.

### Carve shard population

Every accepted PDC mark emits at least two shard entities with a 2.5-second
lifetime. At 100 rounds per second, one turret can sustain about 500 shard
entities. Multiple firing ships multiply that count even when remeshing is
properly throttled.

Measure a multi-ship sustained fight after the blockers. If this is material,
coalesce cosmetic dust per body and rendered frame. Keep immediate hit feedback
through existing sparks and audio.

### Permanent section traversal

At the reviewed head, any non-empty root mark list makes every `DamageCarve`
section collect descendant art every frame. Per-art signatures avoid repeated
subtraction but do not avoid the hierarchy walk.

After C+A merges, verify both states:

- unchanged signature and no budget-deferred art performs no full-tree work;
- a changed signature or pending budget continues exactly the required work.

A global one-solidify budget also needs eventual progress across multiple ships.
Measure or test that stable query ordering cannot starve a later ship forever.

## Verification gaps

- The accepted Phase 4c design says `wfc_arena` must show sustained PDC carving.
  The dedicated `carve_asteroids` gallery was run and inspected, but a literal
  `wfc_arena` player-path result is not recorded.
- WASM timing remains explicitly unmeasured.
- Add the literal arena playtest after C+A and the correctness blockers are on
  the same branch.

## Minor cleanup

`crates/nova_scenario/src/objects/asteroid.rs` still documents
`AsteroidHealth` as coming from removed `AsteroidConfig::health`. Point it at
`AsteroidConfig::durability` when that file is next edited.

## Suggested order after C+A

1. Sync or merge C+A into `erosion-level` and re-review the overlapping section
   path.
2. Reproduce and fix absorbed-damage mark accounting.
3. Reproduce and fix body-level blast coalescing and ejecta bounds.
4. Make asteroid remesh and severance transactional.
5. Measure shard population and unchanged-mark traversal.
6. Run the literal `wfc_arena` player path.
7. Run only affected checks, inspect rendered output, update task evidence, and
   commit in shippable slices.
