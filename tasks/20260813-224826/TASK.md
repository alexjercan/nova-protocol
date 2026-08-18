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

### Phase 4 - asteroids - DONE (2026-08-17)

- Analytic seed, subtract at MARKS (not at level - see "The level"), naive
  surface nets with flat normals, trimesh collider swap. `SignedField` in
  `nova_gameplay/mesh/field.rs` is the representation; `asteroid_carve.rs` is
  the glue.
- TRIPLANAR, done here after all (`DESIGN-round3.md` B1). Emitting the carved
  surface through `TriangleMeshBuilder` did give it the same UV CONVENTION as
  the shipped mesh, which was enough to compile and not enough to look right:
  those UVs are planar per TRIANGLE, so their scale follows the triangle's
  size, and a surface-nets triangle is not a subdivision triangle. A carved
  rock therefore wore a visibly finer texture than the uncarved one beside it.
  `AsteroidSurfaceMaterial` samples by POSITION instead and consults no UV at
  all, which fixes that mismatch and the per-triangle quilting every rock has
  always had, in one move.
- The SILHOUETTE was the other half of the same complaint and is fixed with it.
  `PlanetHeight` is a planet generator whose output is NON-NEGATIVE, so
  displacing a sphere by it could only add material and the base sphere showed
  through wherever the noise bottomed out - every rock read as a ball with
  growths on it. `RockHeight` replaces it: four octaves of plain fBm, SIGNED,
  with a per-seed anisotropic stretch. Same size (the pinned
  `ASTEROID_GEOMETRIC_FACTOR_MIN..MAX` is load-bearing for campaign clearances,
  editor placement and orbit gates, and is re-pinned against the new
  generator), new shape.
- That also cut the seeding cost from 28-39 ms to 4.9-7.4 ms, because the
  planet graph carried 25 permutation tables a rock never needed. Remesh
  2.4-3.4 ms and collider 3.1-4.5 ms on a fuller surface (7.7k-10.7k tris).
- `BodyRadius` only ever shrinks: re-derived from the surviving surface and
  written only when smaller, so gravity SOI and orbit bands stay valid without
  recomputation.
- MEASURED, native, at 32^3 (`carve_asteroids`, `RUST_LOG=nova_scenario=debug`):
  seed 28-39 ms once per rock on its first hit, remesh 2.3-3.5 ms, collider
  build 1.6-2.8 ms, ~4000 triangles out. Remesh and collider land inside the
  design's estimates; SEEDING is the one the design did not budget for and is
  the largest single cost.
- NOT done, and deliberately: the async offload. It is worth doing on the
  numbers above - the seeding spike especially - but it is a perf-hardening
  step, the numbers had to come first, and a synchronous version that can be
  measured is what produced them. Coalescing is already there for free: a
  frame's worth of marks is one list and one list is one carve.
- NOT measured: wasm. It cannot be run from here. The resolution is already
  pinned at the design's wasm cap so the shape is identical on both, but the
  timings are a native-only claim and must not be quoted as covering the web.
- Island severing, chunk spawning and loot (`DESIGN-round3.md` B2 stage 3) stay
  out. This phase never asked for them.

### Phase 4b - carving that throws real bodies (2026-08-18)

Owner-approved after the Phase 4 review. Phase 4 made a crater; the review asked
why the material that came out of it was cosmetic when a DESTROYED body already
breaks into rigid pieces. Eight steps, in order.

#### 1. Every rock is one shape - DONE

- The pristine mesh is now `pristine_rock_mesh(seed)`, which is
  `pristine_field(seed)` meshed. It used to be a subdivided octahedron displaced
  by the same noise, and the two agreed only to within a cell: the first hit on
  a rock moved its silhouette and changed the size of every facet on it, which
  read as the rock being swapped rather than dented.
- The field is DROPPED after meshing (140 KB a rock, and most rocks are never
  touched) and rebuilt from the same seed on the first hit. Same function, same
  grid, so the reseed reproduces the mesh exactly.
- `RockHeightNoise::reach` replaces the fixed `ROCK_SURFACE_MIN` bound: it
  measures where THIS rock's surface can be, which sizes the field's domain and
  lets the sampler settle a point's sign outside that shell without paying for
  the noise.
- The reach is a BOUND, not an estimate, and getting that wrong is silent: a
  rock reaching past its domain is clipped flat against the grid wall. Measured
  against a 200k-direction spread over 64 seeds, a sampled peak falls short by
  up to 6.4% at 512 directions and 2.3% at 2048, converging like one over the
  root of the count. So: 2048 directions, widened 10% inward and 5% outward, and
  a test that checks the bound holds against a 40x finer spread.
- MEASURED, native (`carve_asteroids`, `RUST_LOG=nova_scenario=debug`): 6.0-9.3
  ms to mesh a rock at spawn, against 4.9-7.4 ms to seed a field alone, so the
  surface nets and the collider build are about a millisecond of it. The
  largest shipped scatter is 40 rocks (`weave`), so the worst load cost is about
  280 ms, once, behind a loading screen.
- Retired with it: the `NoiseFn` adapter on `RockHeightNoise` (nothing displaces
  a sphere by a rock any more) and `ROCK_SURFACE_MIN`.
- `compare_asteroids` and the geometric-factor sweep both moved onto the
  production mesh path. The sweep runs 24 seeds instead of 256: meshing a rock
  costs a hundred times what displacing an octahedron did, and the analytic
  sweep in `asteroid_surface` still covers the noise across seeds at full width.

#### 2 and 3. Severed pieces, and ejecta by volume - DONE

Landed together: they are one question - what comes off a carve, and is it worth
simulating - answered in two places that have to agree on the threshold.

- `SignedField::split_off_islands` flood-fills the solid corners 6-CONNECTED
  (face adjacency; two lumps meeting at a grid diagonal share no surface and are
  not attached), keeps the biggest piece on the parent and hands the rest back
  as fields of their own. Each is then meshed by the same surface nets the rock
  is, so a piece is exactly the geometry that left it.
- `integrity/chunk.rs` is where a piece becomes a body. A chunk spends
  `CHUNK_GRACE_SECS` (0.5) KINEMATIC and colliderless, then goes dynamic and
  grows its collider: it is born inside the collider it came off, and a dynamic
  body spawned interpenetrating another gets shoved out hard enough to read as
  the parent kicking its own debris. Matters most for SHIPS - a section's convex
  collider has an inside - and costs an asteroid nothing, its collider being a
  hollow trimesh.
- A chunk is handed back UNDRESSED and the caller inserts the material, because
  there is no one type to take: a rock's pieces want its triplanar
  `ExtendedMaterial`, a section's want a plain `StandardMaterial`.
- Severed rock pieces DO wear the parent's triplanar material, unlike death
  fragments. The shader samples by the body's own local position and a piece has
  a new origin, so it reads the rock's grain from a different place - which for
  noise is invisible, and what it buys is the texture sitting still on a
  tumbling piece.
- `CHUNK_MIN_VOLUME` (1.0 cubic units = 80 hp at the cladding's toughness) is
  the dust/debris line, and the shipped weapons sit far from it rather than near
  it: a PDC round spends 4 and throws dust, a torpedo spends 750-2000 and throws
  rubble. `CarveSpew` carries the volume so ships and rocks read the same rule.
- Ejecta is sized off the crater (`EJECTA_OF_CRATER` 0.3) and capped at
  `EJECTA_MAX` 3, each piece squashed differently by a hash of the crater. The
  first attempt threw ONE lump holding the whole removed volume, which came out
  as a 4.6-unit ball beside a 7-unit rock - the material a carve takes is mostly
  pulverised, and a lump the size of the hole reads as the body calving.
- Islands BELOW the threshold are announced as carves instead, so they become
  dust. A cut does not end at a clean line: the gallery's cut left eighteen
  crumbs round the rim where the slab thinned out, and eighteen rigid bodies of
  a few cells each is litter with a solver cost.
- `fragment_collider` moved to `chunk_collider` and is shared with the finale.
  One answer to "what collider does a piece get", including the coplanar
  zero-mass guard that took down a capital fight.
- `carve_asteroids` gained a sixth column: a rock CUT IN TWO. A ring of surface
  craters cannot do it - cutting deep enough to reach the axis makes each crater
  nearly as wide as the rock, and their union swallows the caps it was supposed
  to leave, so the rock does not come apart, it goes away. The cut is a salvo
  walked across ONE PLANE through the body, which is what a torpedo does: a
  blast resolves at a point in space, not on a surface.
- MEASURED: the connectivity pass is 0.3-1.6 ms per carve on a 32^3 field, on
  top of the 2-4 ms remesh. It runs on every carve, not only on ones that sever.

#### 4. No fallback, and the slicer survives - DONE

- `spawn_fallback_burst` is gone. An empty geometry walk emits NOTHING and logs
  an error. The cubes were not bad because they looked bad; they were bad
  because they looked like SOMETHING, so a body that had silently failed to come
  apart was indistinguishable from one that had come apart badly - which is how
  the bug Phase 0 fixed survived every playtest that saw it.
- `destruction_finale`'s sixth invariant is renamed from "no death emitted both
  fragments and cubes" to "no death came apart into nothing", and it is counted
  at the BODY (`Add<ExplodeFragments>` with an empty list) rather than on the
  field - an empty walk leaves nothing on the field to count. The roster in
  `catalog_drift.rs` moved with it. Verified live: 16 fragments, 0 empty walks,
  at most 4 from one body.
- The random-plane slicer is KEPT, and its module now says why. It was never the
  fallback - the cubes were. It cuts a real mesh into convex chunks that
  reassemble the original, and it is the only fragmenter that works on glTF art:
  a signed field can be carved and severed, but ship sections are authored models
  with no field behind them.

#### 5. Spike: a signed field off authored art - DONE, PASSES

`mesh/solidify.rs`: `field_from_mesh` builds a parry `TriMesh` with
`TriMeshFlags::ORIENTED` and samples the grid by point query. `is_closed`
gates it.

- WATERTIGHTNESS, measured on the shipped glTF: three of five models have no
  boundary edge (`hull-01`, `torpedo-bay-01`, `turret-pitch-01`) and two do
  (`turret-barrel-01` 48, `turret-yaw-01` 2). Only `turret-pitch-01` is a clean
  2-manifold; `hull-01` (12) and `torpedo-bay-01` (4) carry non-manifold edges
  where panels meet, which still separates space. The procedural cut-cube parts
  most sections draw with are all closed.
- So step 6 PASSES for the bodies it targets. HULL art is field-able; the two
  meshes that are not are turret parts, and step 6 does not carve turrets.
- COST: 5-20 ms to build a 32^3 field off a section mesh, native, measured live
  against the `destruction_finale` rig. The same order as an asteroid's own
  field (6-9 ms), so the same build-on-first-hit cadence carries it.
- Vertices are WELDED by position first. A glTF index buffer describes the
  render topology - split at every crease and UV seam - so an unwelded cube
  reads as 24 vertices and every edge of it looks like a boundary.
- parry trap: `project_local_point` does NOT apply the pseudo-normals, so
  `is_inside` comes back false everywhere and the field has no interior at all.
  Only `project_local_point_and_get_location` runs that step. Nothing about the
  API says so, and the failure looks like a correct distance field.

#### 6. Hull sections carve - DONE

`sections/damage_carve.rs`, fitted from a new `DamageEffect::Carve`. Shipped on
hull sections and on the cut-cube parts whose role is Hull; nothing else.

- The field is read out of the section's own drawn mesh at 24^3 and DROPPED
  never - unlike a rock's, it stays, because a section's art is authored and
  cannot be regenerated from a seed. Built on the first hit, so an unhit ship
  pays nothing.
- Costs 10-16 ms to solidify one section mesh, measured live in the gallery.
- Meshes that are not closed are refused ONCE and marked, so a section whose art
  has open faces does not pay the weld-and-check every frame. In practice the
  skin PLATES are the ones refused - a plate is an open shell - which is right:
  they carve through their own path.
- THE COLLIDER DOES NOT FOLLOW, deliberately. A section's mass comes from its
  authored collider's volume and the link-point graph is built against the
  authored shape, so a collider that changed under fire would move a ship's mass
  and inertia every time it was shot and change which sections count as
  attached. The visible cost is a round passing through the air inside a deep
  crater and still being stopped - the same lie a shot-off plate already tells.
- A mark is priced at the CLADDING's toughness (`DAMAGE_PER_UNIT_VOLUME`, 80 hp
  a cubic unit) because one sphere is shared by everything it reaches. A hull
  section is 200 hp to the cubic unit, so `bite_of` scales the radius by the
  cube root of the ratio: without it, a hit that left a hull at half health
  carved the whole thing away. Found by looking at the render, not by reasoning
  about it.
- FIXED on the way: `DamageMarks` was inserted by the SKIN builder, on the
  theory that an unclad ship has no shape to carve. It became a requirement of
  `SpaceshipRootMarker` instead - an unclad ship recorded no hits at all, so its
  hull sections could not carve and the bare gallery column showed nothing.
- The gallery gained a sixth, UNCLAD column at level 0.0 for exactly this: on a
  clad ship the hull's crater is behind the plates until they are shot off,
  which is correct behaviour and impossible to judge. It takes 150 of its 200
  hit points - more would destroy the section rather than carve it.
- HONEST about the look: a carved section loses the panel detail the artist put
  in and the corners left standing around a deep crater read as spikes. It is
  unmistakably a bite out of a hull and it is not authored art. That is the
  trade the effect exists to make, and it is why turrets do not carry it.

#### 7. Cracks replace scorch - DONE

`sections/damage_cracks.rs` + `assets/shaders/section_cracks.wgsl`.
`sections/damage_tint.rs` is deleted and `DamageEffect::Scorch` is gone;
`DamageEffect::Cracks` is the default in its place.

- The tint said "this is damaged" by reddening and darkening a whole body. That
  is information rather than a picture of anything - a hull at 60% looked like a
  hull painted red - it fought every authored paint scheme, and it disagreed
  with the geometry now that a section can be visibly bitten into.
- The fracture field is three octaves of centred value noise sampled by LOCAL
  position, and the crack is its ZERO SET: a surface through the volume, so it
  draws continuous lines on a face and carries across a seam onto the next one.
  Local space for the reason the rock shader is - a world-space pattern swims
  across a ship as it flies.
- TUNED BY LOOKING. The first width was a third of the field's RANGE, which
  covered more than half the surface and drew orange blotches: the octaves
  cluster tightly about zero. 0.09 puts a dead section at about a fifth cracked
  and a half-dead one at a twentieth. Verified live: dark veins at 0.5, hot
  glowing fractures at 0.9.
- A cracked mesh keeps a `FragmentMaterial` pointing at its PRISTINE standard
  material. The finale can only draw debris with a `StandardMaterial`, and a
  section that swapped to an extended one would otherwise break into anonymous
  grey.
- The shipped effect table is now:
  Hull = Cracks + Carve; Turret, Torpedo bay, Controller = Cracks + Sparks;
  Thruster = Cracks + Sparks + Plume.
- Sparks are UNCHANGED and were never the problem: at level 0.9 the interval is
  ~0.21 s against a 0.35 s lifetime, so they are already continuous at low
  health. The threshold is on `DamageLevel`, where 0.0 is pristine.

#### 8. The finale inherits the body's motion - DONE

- A death's fragments leave with `v + omega x r` now, read off the nearest
  ancestor that has a velocity at all. A section carries none of its own - it is
  a child of the ship's rigid body - so the inheritance is a walk rather than a
  lookup. Without it a ship dying at speed left its debris hanging where it was
  hit while the wreck flew out from under it, which reads as the pieces being
  spawned rather than shed.
- That closes the phase-5 bullet the epic opened with. The rest of phase 5 -
  deleting the slicer - is RESOLVED THE OTHER WAY: see step 4.

### Phase 4c - sustained fire makes holes - DONE (2026-08-19)

Owner review found that the shipped PDC path did not visibly carve shipped
rocks. The mechanism worked only in the gallery because it used 600-damage
synthetic hits, a small rock and 100,000 health.

Accepted correction, in order:

- A hit whose centre is inside an existing crater adds its paid volume to that
  crater: `r' = cbrt(r^3 + s^3)`. A separate hit opens a separate crater while
  the 24-mark budget has room. At the budget, the nearest crater gains the
  volume without moving its centre. The old bounding-sphere merge is rejected:
  one distant hit could inflate a crater across the body.
- Accumulation and remesh throttling ship together. Marks update on every hit,
  but asteroid and section fields remesh only after their quantized
  `solid_volume` loses at least one grid cell since the last successful remesh.
  A change too small for the grid to draw must not pay for surface generation,
  island splitting or collider rebuilding.
- Durable scatter rocks use radius-cubed health, anchored at 100,000 hit points
  for nominal radius 3. Scripted objective rocks retain explicit fixed health,
  so objectives cannot soft-lock. Invulnerable bodies keep the existing flag.
  Fixed 100,000 for every size is rejected: relative removable volume would
  vary by `radius^3`, allowing small rocks to outlive their geometry while
  large rocks barely change before death.
- `carve_asteroids` must exercise sustained 4-damage PDC fire against a
  shipped-size rock, retain a torpedo-scale one-hit case and retain severance.
  `wfc_arena` must show the same result. Frame cost under sustained fire is a
  measured claim, not an estimate.

Carve and gate delivery:

- `DamageMarks` now adds radius-cubed volume to the crater containing a hit,
  or to the nearest crater at the budget. Repeated rounds no longer disappear;
  a distant budget merge no longer creates a whole-body bounding sphere.
- Asteroid and section fields track marks applied separately from volume last
  meshed. They keep every sub-cell field change but remesh only after a grid
  corner changes sign. In the real-fire gallery, 354 landed PDC rounds caused
  two asteroid remeshes, not one remesh per round.
- `carve_asteroids` now holds a real shipped `better_turret_section` on one
  point of a radius-3 durable rock until at least 300 4-damage rounds land. A
  captured run paid 1,416 damage after in-flight rounds settled, accumulated
  into three depth-wise craters with a 1.52u largest radius, and showed a clear
  dark cavity against the identical pristine control. The same row retains one
  750-damage torpedo crater and the severing cut. Every rendered capture was
  opened and inspected.
- Sustained-fire frame capture, native Vulkan RTX 3060 Ti, dev profile,
  1280x720, one accumulating 4-damage hit per rendered frame: 300 frames,
  mean 18.40 ms, p95 23.82 ms, p99 30.84 ms, max 39.72 ms. During warm-up and
  capture, grid-cell throttling reduced hundreds of mark changes to nine
  remeshes. This is a native measurement, not a wasm claim.

Health delivery:

- `AsteroidConfig.health` is replaced by explicit `durability`: `Durable` or
  `Fixed(hit_points)`. This is intentionally format-breaking; no implicit
  numeric convention decides whether a rock belongs to an objective.
- Shipped scatter and belt dressing use `Durable`. The asteroid-field objective,
  campaign anchors, the Shakedown derelict and mod-scripted targets retain
  `Fixed` health. Editor belts and the WFC arena use the durable rule.
- Base content was regenerated from the Rust builders. Bundled example and web
  mods were migrated to explicit fixed values, preserving their authored beats.

Lag diagnosis:

- Reproduced in `damage_levels`. Its first damage batch synchronously solidified
  five section meshes in one frame: 18.0, 10.7, 10.4, 10.3 and 10.3 ms, 59.7 ms
  total before remeshing. This is the reported huge first-hit ship spike.
- The grid-cell remesh throttle does not fix that one-off cost. The next design
  choice is async solidification versus a bounded across-frame queue; no fix is
  claimed in this phase.

### Phase 4d - the rock is its remaining material - ACCEPTED (2026-08-19)

Owner playtest found that carving is the fun asteroid mechanic and health death
undercuts it: a visibly solid rock reaches zero hit points and abruptly enters
the unrelated random-plane finale. Asteroids now use their signed geometry as
their sole durability authority.

Accepted correction:

- Normal asteroids carry `DamageMarks` but no `Health` or `ExplodableEntity`.
  Every accepted hit is material work. Kinetic and pierce rounds still stop at a
  healthless rock; rams explicitly keep collision events. Invulnerable
  planetoids remain uncarvable.
- Remove `AsteroidDurability` and `AsteroidConfig::durability` entirely. This is
  intentionally format-breaking. One global rock material-toughness constant
  converts weapon damage to volume; preserve the current 80 hp per cubic unit
  first so the approved crater look does not move. Author mineral hardness only
  when the game has real material variants to distinguish.
- A rock remains the primary carvable body while its largest connected field is
  collider-buildable and above the existing world-space debris threshold.
  Disconnected islands become chunks or dust as they do now. An empty,
  unmeshable or sub-threshold final remnant becomes dust, fires the asteroid
  root's `OnDestroyed`, and removes the root. No health finale and no random
  slicing.
- Field, mesh, collider, radius, islands and finalization commit as one
  transaction. Build from a cloned candidate; a rejected candidate cannot spawn
  chunks, desynchronize the visible body or retry expensive work every frame.
- Health-bearing bodies carve only damage their health pool actually absorbed.
  Geometry-only asteroids accept the whole hit as material work. This distinction
  belongs in the post-C+A damage-accounting correction recorded in `REVIEW.md`.
- Replace Shakedown's asteroid-kind rehearsal hulk with an inert neutral inline
  ship of three connected light hull sections. It keeps the scenario id and
  teaches section damage before the pirate arrives.
- Ordinary scenario rocks all use the one geometry lifecycle. Replace Asteroid
  Field's five-health-rock grind with one small marked ore rock whose geometric
  exhaustion fits the exercise; the rest of the field remains a free combat and
  gravity sandbox. Migrate bundled mods, examples, fixtures and docs without a
  compatibility parser.
- The correctness gate is the real destruction range: a healthless rock must
  survive ordinary carving, finish only when no viable primary field remains,
  emit no `ExplodeFragments`, leave bounded carve debris and fire `OnDestroyed`
  exactly once. The existing real-PDC gallery remains the player-path look gate.

Rejected alternatives:

- A renamed hidden integrity counter: the same arbitrary death under another
  name.
- Finalization at a remaining-volume percentage: health expressed as voxels.
- Finalization on the first meaningful severance: one chip would stop further
  carving of a mostly intact rock.

Delivery:

- `AsteroidConfig` has no durability field. Normal collider nodes carry marks
  and collision events but no health or explodable marker; invulnerable ones
  carry neither marks nor health. Bundled content and docs use the breaking
  schema directly.
- Candidate fields split, mesh and build their collider before any island or
  replacement becomes observable. Rejected colliders wait for a new mark. An
  empty or sub-one-cubic-unit largest solid emits its final debris, reuses the
  common destruction cue, fires the root's `OnDestroyed` once and despawns.
- `destruction_finale` first leaves a healthless rock alive with an ordinary
  crater, then exhausts it and asserts no random fragments, bounded carve
  chunks and exactly one scenario event. The old health path failed this range
  by deleting the rock after the first 1200-damage carve.
- The real-PDC gallery now counts sourced turret hits rather than health loss.
  Its geometry-authoritative run completed and the PDC, torpedo and cut captures
  were opened; the sustained-fire cavity remains clear.
- Shakedown's rehearsal target is a neutral, controller-less line of three light
  hull sections. Asteroid Field instead marks one radius-0.35 ore rock and asks
  the player to break it; normal field rocks remain long-lived carveable cover.
- Live correctness runs passed for `destruction_finale`, `player_path`,
  `scenario_grammar`, `outcomes`, `turret_gunnery` and `torpedo_launch`.
  `wfc_arena` completed; its unrelated destroyed-bay particle lookup was then
  downgraded from a false engine error to the expected omitted optional effect.

### Phase 5 - the finale, and delete the slicer - DONE (2026-08-18)

- What is left of a body at death comes apart into bounded debris with
  inherited `v + omega x r`. Done in Phase 4b step 8.
- ~~Delete the random-plane slicer~~ KEPT, with the reason recorded in its
  module and in Phase 4b step 4: the cubes were the fallback, the slicer never
  was, and it is the only fragmenter that works on glTF art. What was deleted is
  `spawn_fallback_burst`.
- Fragment budgets are per BODY, not per primitive, and `destruction_finale`
  asserts a real death against `BODY_FRAGMENT_BUDGET` rather than a copy.

## Out of scope

- ~~Located craters and any hit-point plumbing.~~ PULLED IN (2026-08-17). See
  "The level" above for why the gate forced it.
- SHED, as an effect. Cut in Phase 3 for want of art, not deferred.
- ~~Island severing on a carved rock: a chunk cut free by two craters meeting is
  still part of the body. `DESIGN-round3.md` B2 stage 3.~~ PULLED IN
  (2026-08-18) - see Phase 4b.
- Repair, welding, or adding material back.
- Health derived from geometry, volume-authoritative health, or any
  rebalancing of authored damage.
- Persistent per-body fields, offline fracture bakes, per-level authored
  models.
- Carvable debris. Detached material is debris and is not itself erodible.
- Replacing link-point structural adjacency with geometric contact.

## Definition of done

- DONE. One documented finale contract covers every shipped and modded section
  representation; destruction is keyed to semantic destructibility, not to
  where a `Mesh3d` happens to sit. Phase 0, and step 4 removed the fallback that
  was hiding its failures.
- DONE. Every destructible body shows its own health as geometry, for every
  allegiance, and the damage tint is GONE rather than merely retired - see step
  7. Rocks carve and sever, hulls carve, cladding carves, and everything cracks.
- DONE. Damage effects are authored per section as a composable list; the
  shipped set is recorded in step 7 and in `web/src/wiki/modding/sections.md`.
- DONE. The random-plane slicer is KEPT and its survival is documented with the
  reason, in its own module and in step 4.
- PARTLY. Fragment budgets bound a capital-scale collapse and
  `destruction_finale` asserts it. Native costs are measured throughout (rock
  mesh 6-9 ms, section solidify 10-16 ms, remesh 2-4 ms, connectivity 0.3-1.6
  ms). WASM IS NOT MEASURED and cannot be from here: the field resolution is
  pinned at the design's wasm cap so the SHAPE is identical on both, but every
  timing in this record is a native claim.
- DONE. Player-path range evidence, rendered output opened and inspected, on
  every step that changed a look.
- DONE. Content schema, modding docs and examples match the shipped scope.
  Release notes are not written here - the per-version News post is.
