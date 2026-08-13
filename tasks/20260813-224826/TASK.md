# Carve asteroids by damage and investigate destructible ship geometry

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.11.0,epic,destruction,asteroid,ship,physics,spike

## Goal

Replace asteroid hit-to-death-only visuals with persistent geometric damage:
weapons carve craters, impacts eject shards, and disconnected material becomes
physical chunks. Then investigate the harder extension to semantic spaceship
parts and correlate geometric loss with health and structural integrity.

This is a large staged epic. Each phase has a kill gate. Do not commit the full
runtime to a voxel representation before the prototype proves appearance,
collision quality, native cost, and WASM cost.

## Existing design

Consume `tasks/20260812-100256/DESIGN-round3.md`, Topic B2. That completed spike
already selected this first approach:

- Lazy per-asteroid local-space SDF grids derived from the analytic asteroid
  shape.
- Naive surface nets rather than marching cubes. It is simpler, avoids sliver
  triangles, and fits Nova's low-resolution faceted style.
- Spherical SDF subtraction per damage impact.
- Flat-normal remeshing and triplanar materials, because carved geometry has no
  stable UV unwrap.
- Rebuilt trimesh collider for the remaining asteroid.
- Six-connected flood fill for severed solid islands.
- Convex-hull dynamic bodies for large detached chunks; temporary shards and
  optional salvage for small islands.
- Existing slicing remains the death fallback and the baseline for unsupported
  objects.

The design's performance estimates are hypotheses, not accepted budgets. The
prototype must measure them on current native and WASM paths.

## Phase 0 - Prerequisite surface pipeline

- Implement or schedule the triplanar asteroid material and repeat sampler from
  Topic B1. Carved meshes must not depend on generated UVs.
- Add proof images for pristine and carved faceted asteroids under representative
  scenario lighting.

Gate: no carving integration until the material works on runtime-remeshed
geometry and WebGPU.

## Phase 1 - Asteroid carving prototype

Build an isolated, runnable example before touching production damage:

- One analytic asteroid field at 32^3 and 48^3.
- Inject repeatable local-space impact points and damage amounts.
- Apply spherical subtraction and coalesce impacts per fixed tick.
- Remesh with surface nets and flat normals.
- Swap render mesh and trimesh collider after a completed job.
- Record field update, remesh, collider-build, and swap times separately.
- Show repeated craters, tunnel/near-sever cases, grazing hits, and complete
  material removal.
- Keep at most one job in flight per body. Define how queued impacts merge or
  supersede stale output.

Gate: rendered output keeps the faceted art direction; collider follows the
visible crater; no stale async result restores removed material; native and
WASM measurements support an explicit update budget.

## Phase 2 - Detached chunks and impact debris

- Flood-fill solid cells after each accepted carve batch.
- Keep the largest/root island as the asteroid body.
- Mesh sufficiently large detached islands and spawn them as dynamic chunks.
- Inherit point velocity: `v + omega x r`.
- Use bounded convex-hull colliders for chunks.
- Convert small islands into bounded temporary shards. Add salvage only after
  defining deterministic ids or an id-less pickup path.
- Cap chunk count, shard count, retained grids, and work per fixed tick.

Gate: deterministic harness evidence proves conservation bounds, no duplicate
islands, bounded entity growth, and valid collider generation.

## Phase 3 - Production asteroid integration

- Add explicit asteroid authoring controls such as `carvable`, field resolution,
  carve scale, and optional debris/loot policy. Defaults preserve non-carvable
  mod behavior unless a migration decision says otherwise.
- Carry impact point, normal, source, and applied damage through the production
  damage seam without creating a second health pipeline.
- Keep scenario lifecycle exact: carving emits no destruction event;
  `OnDefeated`/`OnDestroyed` still follow the existing exact-once rules.
- Keep Health as the initial kill gate. Health zero destroys the current carved
  body through the existing finale or a measured replacement.
- Update mass properties and collider shape. Decide and test whether
  `BodyRadius`, gravity-well reach, targeting signature, and HUD range use the
  pristine envelope or remaining geometry.
- Ensure save/reload either serializes carving state or explicitly resets it.
  Do not leave persistence accidental.

Gate: a player-path harness shoots, craters, severs, reloads according to the
chosen persistence contract, and destroys an asteroid with correct scenario
outcomes.

## Phase 4 - Geometry-derived health experiment

Do not silently replace authored Health. Compare these models with a balance
harness:

1. Health remains authoritative; geometry is visual and physical evidence.
2. Remaining solid volume derives health fraction from authored maximum health.
3. Hybrid: impacts apply health damage, while low remaining volume or loss of a
   protected core forces destruction.

Measure tunneling, many-small-hit exploits, large-blast behavior, repair
implications, deterministic replay, and scenario compatibility. Select one
model explicitly. Any balance or content-schema break requires migration notes
and bundled-mod updates.

## Phase 5 - Semantic spaceship-parts spike

Asteroid SDF generation does not transfer directly to authored GLB parts.
Before implementation, compare:

- Offline voxel/SDF bake per semantic part, shipped as content.
- Runtime voxelization of the render mesh.
- Local mesh booleans or fracture cells without a persistent SDF.
- Keeping part-level destruction authoritative while carving remains cosmetic.

Required interactions:

- Authored link-point mates remain the sole source of structural adjacency.
- Carving cannot create mates.
- Define when a carve destroys or disables a link point.
- Re-derive connected components after lost link points; detached components
  become independent debris bodies.
- Preserve exact assembly through `render_mesh_transform`.
- Correlate remaining part volume, per-part Health, aggregate ship health, and
  controller/capability loss without double-counting damage.
- Bound memory for repeated instances of the same part, preferably by sharing a
  pristine baked field and allocating deltas only after damage.

Gate: one semantic part can be carved and one mate can be severed in an isolated
ship harness with deterministic integrity results and acceptable memory. Only
then schedule production ship carving.

## Out of scope until a phase promotes it

- Fully deformable planets.
- Carvable detached chunks.
- Repair, welding, or adding material.
- Unbounded voxel resolution or per-projectile remesh jobs.
- Replacing link-point structural adjacency with voxel contact.
- Removing slicing before carving has a production-safe fallback.

## Definition of done

This epic closes only when:

- Asteroid carving is production-integrated with bounded native and WASM costs,
  deterministic chunk severing, collision parity, lifecycle parity, and
  player-path evidence.
- The health model is explicitly selected and documented.
- The spaceship-parts spike has a recorded go/no-go verdict. Production ship
  carving may become a separate implementation task if the verdict is go.
- Content schema, modding docs, gameplay docs, examples, screenshots, and
  release notes match the shipped scope.
- Affected Rust checks, content lint, correctness probes, and rendered-output
  inspection pass.
