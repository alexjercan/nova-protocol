# Erode every destructible body by its own health

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.11.0,epic,destruction,asteroid,ship,torpedo,render,physics,spike

## Goal

Replace three unrelated destruction paths with one. A body's visible geometry
degrades as its own health falls, and HOW it degrades is authored per section
rather than special-cased per code path.

Absorbs task `20260817-154330` (representation-independent destruction
visuals), closed as merged. That task's seam is this one's Phase 0: the two
were one feature seen from opposite ends - 154330 asked how geometry reaches
the destruction system, this asked what damage does to it, and neither is
answerable alone.

Damage itself is UNCHANGED. Bullets, blast and ram keep dealing their authored
amounts. Erosion takes the place the red damage tint holds today, not the place
the damage numbers hold.

## What is there now

Three paths that share nothing:

- ASTEROID: no damage visual at all; death slices the mesh on random planes
  into four convex chunks (`mesh/explode.rs`).
- SHIP SECTION: a material tint while alive (`damage_tint.rs`); death spawns
  eight generic gray cubes for two seconds. That fallback is a filter bug, not
  a design - `on_explode_entity` requires `Mesh3d` on the gameplay entity, and
  sections render through `SectionRenderOf` descendants carrying
  `WorldAssetRoot`, so every section falls through to it.
- TORPEDO: sliced only when the meshed child happens to die first, else cubes.

And one thing that already works that nothing calls destruction: skin plates
(`shell_skin.rs`) are per-cell `SectionFixture`s with their own health that
come off when shot. Derived ONCE on the spawn batch; lost plates stay lost.

A clad hull section renders `WorldAssetRoot(meshes.hull)` UNDER its plates.
Nothing hides the body, so ships need the general mesh path too - the lattice
covers the cladding only.

## Accepted design

### The level

- `level = 1 - health_fraction`, per destructible entity, off its own health.
- SUPERSEDED (2026-08-17, by Phase 2's gate). This said ONE input - no hit
  point, no impact history - with located craters as a later refinement that
  must not be designed for now. That cannot work, and the gate is what proved
  it: a single per-body number can only drive geometry that changes everywhere
  at once, so a clad hull came out with every plate sagging by the same
  proportion. It kept its outline, lost its relief, and read as a smaller,
  plainer ship rather than a damaged one. Damage has to ADD detail - a rim, a
  bite, a hole - and no scalar can put detail anywhere in particular. Hit
  points are now plumbed through `apply_damage` and stored as `DamageMarks`.
- So damage is TWO readings, and they drive different things: the LEVEL (how
  far gone a body is) grades whole-body effects, and the MARKS (where it was
  hit) drive anything that changes shape.
- Geometry is DERIVED, never the reverse. Damage itself is still untouched: a
  bullet, a blast and a ram deal exactly what they authored, and health is
  still the kill gate.
- Shown for every body regardless of allegiance. The `TintMode::Full` /
  `TintMode::DeadOnly` split goes away; enemies now show their damage.

### The level has many consumers, and they are not one system

- The level is ONE number. What reads it is any number of independent DAMAGE
  EFFECTS, each a component a section carries, each turning the same scalar
  into its own kind of "more or less". A section composes as many as its art
  supports.
- This is not an enum. There is no mode set to close, no `match` the engine
  owns, and a mod adds a new look by adding a component rather than by
  extending a variant list. `Intact` is not a mode - it is a section with no
  geometry-consuming effect attached.
- Effects known to be wanted. Names are provisional, the SHAPE is not:
  - EROSION - material comes off the body itself; craters, then holes. Hull
    sections, asteroids, props.
  - SHED - expendable pieces leave with level. The core is untouched.
    Turrets, thrusters. REQUIRES art with separable pieces; whether the
    shipped turret and thruster have any is a Phase 3 content finding.
  - SPARKS - emissive and particle damage that rises with level. The turret
    and thruster answer, and the reason neither needs its geometry touched.
  - PLUME - a damaged thruster's exhaust goes ragged before it fails.
  - SCORCH - material darkening. What survives of `damage_tint.rs`, reduced
    from a whole-section health readout to a local effect among others.
- The rule that keeps it honest: ONLY NON-FUNCTIONAL MATERIAL IS REMOVED. The
  fixture/section line (`fixture.rs`: "if shooting it off should cost the ship
  a capability it is a section, not decoration") taken one step. A turret that
  has lost most of its mass still shoots, because what it lost was cowling and
  the barrel was never expendable. An effect that would remove functional
  geometry is the wrong effect for that section.
- Effects that both remove material (EROSION and SHED on one section) share
  one budget rather than each spending the level in full, or a section at high
  level has nothing left. Decide the split when a section first wants both.

Why this shape rather than one mode per section: it composes, it is moddable
by addition, and each effect gates independently. If voxel remeshing turns out
to wreck authored art, EROSION is dropped from glTF bodies and every other
effect is unaffected - the verdict costs one component, not the plan.

### Representations

- SKIN PLATES: step the plate's own `ShellShape` samples down with level.
  Every shape this can reach is one the vocabulary already draws, meshes are
  already built on demand and cached by shape id, and collider and mass follow
  from `ShellShape::volume` automatically. Health stays as spawned (it comes
  off the PRISTINE volume); only the shape follows the level. Derive-once
  holds: the generator picks the pristine skin at spawn and damage only ever
  subtracts. Neighbouring plates share boundary samples, so a sagging plate
  opens a seam against an intact one - acceptable, plates already die
  individually and leave holes between intact neighbours.
- SECTION BODIES AND PROPS: authored glTF, so the general mesh path.
  Voxelize into a SCRATCH grid at first damage, subtract at level, remesh with
  surface nets and flat normals, drop the grid.
- ASTEROIDS: the pristine field is analytic
  (`d(p) = |p| - (1 + PlanetHeight(p/|p|))`), so there is no voxelization step
  - seed, subtract at level, remesh, drop.
- NOTHING RETAINS A GRID. The damaged shape is a pure function of
  `(pristine, seed, level)`, so `DESIGN-round3.md` B2's per-rock memory table,
  lazy allocation and 32^3 wasm cap do not apply. The grid is a remesh scratch
  buffer. This is the same idiom `shell_skin.rs` already uses: derive, do not
  store.

### Boundaries

- The section graph (`ConnectedTo` / link points) is the SOLE structural
  authority. Erosion never creates or destroys structural adjacency. A lump of
  material that erosion disconnects is debris, never structure.
- Ship severing belongs to task `20260817-154646` and is not touched here.
  This task lands AFTER it and rewrites its "retain their health-derived
  visual tint" line into the health-derived erosion level.
- Proximity torpedo detonation stays direct consumption into its blast, never
  a shard burst.
- Repair is out of scope. Lost material stays lost. A future repair shop or
  nova_os ship-app action may re-add it; nothing here anticipates that.

## Phases

ONE task, ONE landing. This is a build order, not a shipping order.

### Phase 0 - the seam

- Resolve destruction geometry through `SectionRenderOf` and descendants, not
  `Mesh3d` on the gameplay entity.
- One finale contract: exact-once, never both fallback cubes and mesh
  fragments for one death, and a deterministic fallback when render meshes
  have not loaded.
- Every section kind, procedural and glTF, articulated turret joints, clad
  sections, and a torpedo through EITHER child produce the same result.
- `ExplodableEntity` propagates to parent roots, so relaxing the mesh filter
  must not slice a whole hierarchy or leave a meshless root alive.

Gate: the eight-cube burst is no longer the default for anything; it survives
only for unloaded or unsupported geometry.

### Phase 1 - the level

- `level = 1 - health_fraction` as one derived input, replacing the tint's
  role. Pure function, unit- and snapshot-testable.
- One consumer, to prove the wiring: skin plates are the cheapest, because
  stepping a `ShellShape`'s samples down needs no new mesh generator at all.
  A preference for the fastest signal, not a constraint - any model can carry
  the first effect.

### Phase 2 - the prototype range (THE GATE)

An `examples/systems/` range that walks a hull cube, a plate, a turret, a
thruster and an asteroid through the level range and holds them there to be
looked at.

Two independent risks, so prototype two effects, not one:

- EROSION on material bodies - a plate, a hull cube, an asteroid. Does
  health-driven material loss read as battle damage or as mush? Is voxel
  remeshing acceptable on authored glTF, or does that art need the additive
  fallback (pristine mesh kept, craters and scorch added, silhouette
  unchanged)?
- SPARKS and SHED on a turret and a thruster. Does a section read as
  progressively wrecked WITHOUT its geometry being touched? Does the shipped
  art have separable pieces at all?

Also answer, for both: how many levels are actually perceptible? The useful
number is likely far smaller than the scalar suggests.

Gate: the effect set is fixed HERE, from what the range shows. Each effect
gates separately - a negative verdict on EROSION over glTF selects the
additive fallback for those bodies and leaves every other effect standing.

### Phase 3 - effects on real sections - DONE (2026-08-17)

- Author the effect list per section in content, moddable, defaulting to
  `[Scorch]` and not to nothing. The phase originally said "defaulting to
  nothing so a third-party section is never worse than it is today", which is
  self-contradictory: scorch applied to EVERY section before it was authorable,
  so an empty default is exactly what would make a third-party section worse.
  `[Scorch]` is what preserves the behaviour the sentence was protecting. The
  empty list stays sayable, because "I want none" and "I did not say" are
  different statements.
- SPARKS, PLUME and SCORCH on the sections whose geometry must stay whole.
  PLUME is new here: it grades the exhaust cone `thruster_section` already
  draws, cutting it back and guttering it, and never to nothing - a drive
  showing no plume is a drive that has SHUT DOWN, which must not be confusable
  with one that is failing.
- SHED is CUT, not deferred. Its rule is that only non-functional material
  comes off, and no shipped section has any: a turret is a mount and a barrel,
  a thruster is a bell, and taking a piece off either means taking a working
  part off. The sections that genuinely carry expendable material are the clad
  ones, and cladding already sheds - a plate is shot off and stays off - so
  SHED is delivered where it makes sense under another name. Reopening it needs
  ART with separable pieces, not code.

### Phase 4 - asteroids

- Analytic seed, subtract at level, surface nets with flat normals, trimesh
  collider swap.
- Triplanar material or an equivalent, because remeshed geometry has no usable
  UVs (`DESIGN-round3.md` B1).
- `BodyRadius` only ever shrinks, so gravity SOI and orbit bands stay valid.
- Measure remesh and collider-build cost separately, native AND wasm. At most
  one job in flight per body; a newer level supersedes a queued one.

### Phase 5 - the finale, and delete the slicer

- What is left of a body at death comes apart into bounded debris with
  inherited `v + omega x r`.
- Delete the random-plane slicer (`mesh/explode.rs`) once the replacement
  covers the death case. Fragment budgets are per SECTION, not per primitive,
  so a capital collapse cannot multiply one death into unbounded bodies.

## Out of scope

- ~~Located craters and any hit-point plumbing.~~ PULLED IN (2026-08-17). See
  "The level" above for why the gate forced it.
- SHED, as an effect. Cut in Phase 3 for want of art, not deferred.
- Repair, welding, or adding material back.
- Health derived from geometry, volume-authoritative health, or any
  rebalancing of authored damage.
- Persistent per-body fields, offline fracture bakes, per-level authored
  models.
- Carvable debris. Detached material is debris and is not itself erodible.
- Replacing link-point structural adjacency with geometric contact.

## Definition of done

- One documented finale contract covers every shipped and modded section
  representation; destruction is keyed to semantic destructibility, not to
  where a `Mesh3d` happens to sit.
- Every destructible body shows its own health as geometry, for every
  allegiance, and the damage tint's role is retired.
- Damage effects are authored per section as a composable list, with the set
  chosen from Phase 2's rendered evidence and recorded here.
- The random-plane slicer is gone, or its survival as the single unsupported-
  geometry fallback is documented with the reason.
- Fragment budgets bound a capital-scale collapse; native and wasm costs are
  measured, not estimated.
- Player-path range evidence, rendered output opened and inspected.
- Content schema, modding docs, gameplay docs, examples, screenshots and
  release notes match the shipped scope.
