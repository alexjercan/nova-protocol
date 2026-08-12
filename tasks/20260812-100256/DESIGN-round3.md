# Design round 3 (2026-08-12): planets in-game + destructible asteroids

Design spike, no code. Owner questions after reviewing the compare examples:
(A) "planets look really nice, won't be to scale - what can we do with them";
(B) "asteroids have weird UVs" + future "carve them by damage (marching
cubes), they spew out pieces and small parts" instead of slice-on-destroy.

## Ground truth (measured this round)

- bevy 0.19; default reversed infinite-Z depth (far precision is not the
  constraint older engines make it). Web build uses the WEBGPU backend, not
  WebGL2 (`crates/nova_core/Cargo.toml` wasm block; bevy_hanabi needs
  compute). wasm is single-threaded by default -> async pools run on the
  main thread there.
- Custom-shader precedent exists: `MaterialExtension` over StandardMaterial +
  `assets/shaders/*.wgsl` (`ThrusterExhaustMaterial`,
  `crates/nova_ship/src/sections/thruster_section.rs:642`).
- Asteroid mesh (`insert_asteroid_collider`, asteroid.rs:325): octahedron
  res 3 = 512 tris / 1536 verts (verts duplicated per face -> flat normals
  already). Noise displaces outward 0..5 in unit space; surface radius
  bounded by factor 3.5..6.0 (`ASTEROID_GEOMETRIC_FACTOR_*`). UVs are
  per-triangle planar (builder.rs `uvs()`): seams at every edge BY
  CONSTRUCTION, cannot wrap equirect maps.
- Collider: `Collider::trimesh_from_mesh` at spawn, `ColliderDensity(1.0)`;
  small rocks Dynamic, well rocks Static (on rails).
- Destruction today: Health 0 -> `IntegrityDestroyMarker` ->
  `ExplodeMesh{fragment_count:4}` -> random planes through the origin slice
  the whole mesh (`mesh/explode.rs`, `mesh/slice.rs`) -> fragments spawn as
  Dynamic bodies with `convex_hull_from_mesh` colliders; husk despawns.
- Gravity (`GravitySettings` defaults): default_mass 4000 (SOI = sqrt(mu /
  0.25) = 126 u), min_well_radius 5, soi_cutoff_accel 0.25,
  max_surface_gravity 10 -> mu cap = 10 * body_radius^2. ORBIT autopilot
  circularizes around the dominant well (wiki gravity-wells.md).
- Skybox: one stacked cubemap per camera (`SkyboxConfig`), faces are
  4096 px; scenario-swappable (`PendingSkyboxSwap`).
- Lighting: authored `light` scenario objects (directional key/rim/fill,
  `aim` helper). NO automatic link between skybox sun and light direction.
- Salvage crate: static sensor pickup with a scenario id (OnEnter); not
  spawnable as anonymous loot today (needs an id when spawned by script).
- Cross-ref (read-only): ship-parts SPIKE (task 20260812-100246) defines
  parts-objects - `PartsBodyMarker`, one rigid body + part children +
  `ConnectedTo` graph, severing by connected components, debris = parts
  object without a controller.

## Topic A: planets

Play space is meters-scale; true planetary scale (1e6+ u) is out. Four
routes analyzed.

### A1. Skybox-baked planets

Render/composite planets into the space-3d cubemap offline.

- Gameplay value: none (pure backdrop). No parallax, no orbit, no lock.
- Cost: zero runtime. Content: a bake script compositing SBS sprites (2D
  Planet Pack, CC0, round 1) or rendered spheres onto cubemap faces,
  ~0.5-1 day; rebake per scenario skybox.
- Numbers: a face spans 90 deg at 4096 px -> a 10-deg planet gets ~455 px.
  Sharp enough at background sizes; a 30-deg giant (~1365 px) starts to
  show sprite softness.
- Risks: lighting baked in (sun direction frozen); planet welded to the
  skybox choice; any change = rebake; invisible to gameplay forever.

### A2. Far-layer impostors

Planets that read huge-and-distant and are never reachable.

- (a) Sky-anchored, same camera (RECOMMENDED variant): planet entity renders
  in the main 3D world at `camera_pos + authored_offset` (one system
  re-anchors it every frame). No collider, no well, no LockSignature, no
  radar. Distance never closes -> zero parallax, reads as infinitely far.
  - Float check: offset ~2000-20000 u. f32 ulp at 2e3 is ~0.00024 u, at
    2e4 ~0.0024 u - no jitter. Reversed infinite-Z handles the depth range;
    no second camera, no custom depth needed in bevy 0.19.
  - Cost: the planet render path (A3's mesh/material) + one anchor system +
    a config flag. ~0.5 day on top of A3.
- (b) Second camera + RenderLayers with its own depth range: the classic
  engine trick. The repo has RenderLayers precedent (nova_os_ui,
  render_scale.rs) and per-camera skyboxes, so it is FEASIBLE - but
  reversed infinite-Z makes it unnecessary machinery here (extra camera
  order, skybox ownership, clear-op wiring, editor confusion). REJECTED
  until something actually needs a separate world scale.
- Risks (both): a backdrop body must be excluded from targeting/radar/map
  by construction, or it becomes an unreachable objective bug.

### A3. Stylized miniature planets

Openly-fake scale: radius 200-500 u, reachable, orbitable. Matches the
flat-shaded, openly stylized art direction; the invulnerable "tutorial
planetoid" precedent already exists in asteroid.rs.

- Gameplay value: HIGHEST. Plugs into the existing well + ORBIT mechanic:
  a 300 u planet at the mu cap (10 * 300^2 = 9e5) has SOI = sqrt(9e5/0.25)
  = 1897 u (~6.3x radius); orbit at 450 u flies v = sqrt(9e5/450) = 45 u/s.
  Angular size sells it: 300 u radius seen from 3 km = 11.5 deg diameter
  (23x a full moon).
- Cost: new `planet` scenario object (~1-2 days code) + SBS texture import
  (0.5 day, credits entry). Mesh is a bevy UV sphere - already proven by
  compare_planets. Collider = `Collider::sphere(radius)` (cheap, exact; no
  trimesh needed - the render sphere IS the shape).
- Risks: scale honesty (owner already accepts); a landable-looking surface
  invites "can I land?" - out of scope, the surface clamp + collision
  answers it; Tundra-style saturated maps need a palette check (round 2).

### A4. Hybrid (RECOMMENDED)

Background impostors (A2a) + occasional miniature set-piece (A3), as ONE
object type with a mode flag. Backdrop and body planets share mesh,
material, texture set, and authoring; the flag decides physics presence.
This is strictly A3 + a flag, so the staged path is:

1. Planet body object (A3) - the gameplay payoff, reuses compare_planets
   verbatim (mesh params, poles-up rotation, SBS maps).
2. `backdrop: true` mode (A2a) - the anchor system + presence gating.
3. A1 skybox bake: PARKED. Only if a scenario wants a dense multi-planet
   vista beyond what 1-3 live impostors cost (which is ~nothing: 2 draw
   calls each).

### Mesh + UVs (the spherical-UV requirement)

- `Sphere::new(r).mesh().uv(64, 32)` = ~4k tris, indexed - fine. Poles sit
  on +Z; rotate poles onto +Y (`Quat::from_rotation_x(-FRAC_PI_2)`,
  compare_planets `poles_up()`). Equirect maps band along latitude.
- NOT `TriangleMeshBuilder`: planar per-triangle UVs cannot wrap equirect
  maps (round 2 finding, proven by the compare examples).
- Sampler: equirect UVs stay in [0,1]; default ClampToEdge is CORRECT here
  (no repeat trap). The u=0/1 wrap seam is handled by the sphere mesh's
  duplicated seam verts.
- Optional later: slow axial spin (rad/s about local Y) - one system, big
  liveliness win on gas giants.

### Lighting consistency with the skybox sun

- Scenes light via authored directional key lights; the skybox sun is
  painted pixels. They agree only by AUTHORING CONVENTION: when picking a
  skybox, aim the key light along the cubemap's brightest direction (the
  `aim` field exists for this). Planets then terminator-shade consistently
  with asteroids and ships for free (StandardMaterial).
- A planet's night side can go pitch black against a bright nebula: give
  the planet material a small emissive floor (e.g. base_color * 0.03) or
  rely on the scenario's fill light. Author-tunable, not engine work.
- Backdrop planets are far "away" but lit by the same directional lights
  (directional = position-independent), so they stay consistent
  automatically.

### Scenario grammar (new object type)

`PLANET_TYPE_NAME = "planet"`, config mirrors AsteroidConfig minus
mesh-noise and minus health - planets are invulnerable BY CONSTRUCTION (no
Health node; the asteroid.rs invulnerable rationale: a well dying
mid-scenario kills the orbit beat).

```ron
kind: Planet((
    radius: 300.0,                     // sphere + collider + mesh scale
    texture: "self://textures/planets/gaseous_01.png",  // equirect map
    mass: Some(900000.0),              // mu; None => asteroid radius rule
    spin: Some(0.02),                  // rad/s about local Y; None = static
    lock_signature: Some(300.0),       // None => radius, like asteroids
    backdrop: false,                   // true => sky-anchored impostor:
                                       //   no collider/well/lock/radar,
                                       //   position = offset from camera
))
```

- Body mode spawns: marker, `RigidBody::Static`, `Collider::sphere`,
  `BodyRadius(radius)` (exact - no derived factor needed), gravity well via
  the same `insert_asteroid_gravity_well` logic (crib, or generalize the
  observer to a shared `WellSource` component), LockSignature,
  InsetZoomable.
- Backdrop mode spawns: marker + mesh only + the anchor system; no physics,
  no HUD presence.
- Plumbing to crib from asteroid.rs: config -> bundle -> insert observers,
  render gating (`render: bool` plugin flag), content builders (RON stays
  generated).

## Topic B: asteroids

### B1. UV fix (near-term)

Options:

1. Triplanar projection in a custom material (RECOMMENDED). Sample the
   texture 3x along local-space axes, blend by |normal|^k (k ~ 4-8). No
   UVs consumed at all.
   - Fits the mesh: flat per-face normals make blend weights constant per
     facet - crisp faceted read, zero seams within a face, and blending is
     position-continuous across faces. Local-space projection (not world)
     so the texture rides a tumbling rock.
   - Implementation: `MaterialExtension` over StandardMaterial (keeps PBR
     lighting + the existing base_color_texture slot), one WGSL file in
     assets/shaders/ (~80 lines), a `texture_scale` uniform, swap in
     `insert_asteroid_render`. The thruster exhaust is the exact wiring
     precedent.
   - Requires REPEAT sampling (image .meta loader settings) - same fix the
     round-2 texture-swap task already carries.
   - WASM: web = WebGPU, so no WebGL2 shader constraints; 3 texture samples
     vs 1 is noise at this scene scale.
   - THE strategic reason: carved/remeshed geometry (B2) has no meaningful
     UVs either. Triplanar makes UV generation permanently irrelevant for
     asteroids - the fix is a prerequisite investment for carving, not a
     patch. Effort: 0.5-1 day incl. a compare_asteroids row to eyeball it.
2. Spherical per-vertex UVs at build (u = atan2, v = acos on the pre-noise
   direction): fixes macro continuity, but pole pinching + a u-wrap seam
   needing vertex splits, stretching on displaced blobs, and it DIES on
   carve remeshes. ~1 day for a worse result. Rejected.
3. Baked unwrap (xatlas-style) at mesh build: the mesh is generated at
   SPAWN TIME per seed - baking means shipping an unwrapper in the runtime
   (native lib, wasm pain). Rejected outright.

RECOMMENDATION: triplanar (option 1), folded into the round-2 texture-swap
escalation task (same files, same sampler fix, Rock030/Rock035 look even
better with unbroken macro features).

### B2. Destructible asteroids: carve-by-damage (future)

Goal: damage carves persistent craters; enough damage severs chunks that
drift off as pieces + small parts; kill still explodes.

#### Density/SDF representation

- Per-asteroid f32 SDF grid in the mesh's LOCAL unit space (child mesh is
  unit-scale, world = local * radius). Domain: [-6.4, 6.4]^3 (max surface
  radius 6.0 + margin).
- The pristine field is ANALYTIC: `d(p) = |p| - (1 + PlanetHeight(p/|p|))`
  (apply_noise displaces radially). Seed the grid by sampling that - no
  mesh voxelization step, exact same shape family.
- Resolution/memory (f32; u8-quantized in parens):

  | grid | cells   | memory        | cell size (unit) | world @ R=20 |
  |------|---------|---------------|------------------|--------------|
  | 32^3 | 32,768  | 131 KB (33)   | 0.40             | 8 m          |
  | 48^3 | 110,592 | 442 KB (111)  | 0.27             | 5.3 m        |
  | 64^3 | 262,144 | 1.05 MB (262) | 0.20             | 4 m          |

- Pick per asteroid by radius: 32^3 field rocks, 48^3 designated bodies,
  64^3 reserved for a set-piece. LAZY-ALLOCATE on first damage - an
  untouched field costs zero; 10 actively-carved rocks at 48^3 = 4.4 MB.
  wasm: cap at 32^3 (memory AND remesh time).

#### Remeshing

- Naive surface nets (RECOMMENDED): one vertex per sign-changing cell,
  quads across sign-changing edges. Simpler than MC (no 256-case table),
  no sliver triangles, and quads split into 2 tris read as clean facets.
  Dual contouring rejected: needs hermite data + QEF solves for sharp
  features a noise blob does not have. Naive MC workable but its
  near-degenerate slivers hurt both the look and the trimesh collider.
- Keeping the faceted look (the actual art risk): LOW-RES GRID + FLAT
  NORMALS does it. Emit unindexed triangles with per-face normals (exactly
  what TriangleMeshBuilder::build outputs today - the shading pipeline is
  already flat). Do NOT smooth vertex normals; do NOT raise grid res to
  "fix" blobbiness - coarseness IS the style. Optional: quantize vertex
  positions to 1/2 cell to harden facets further.
- Triangle budget: mean rock surface radius ~3.5 unit -> at 48^3 (~13
  cells) => ~4pi*13^2 ~ 2.1k surface cells -> ~4-5k tris; at 32^3 (~9
  cells) -> ~1k cells -> ~2k tris. vs 512 today: 4-10x, still trivial GPU
  load. First carve swaps the 512-tri builder mesh for the ~2-5k-tri
  voxel mesh - a one-time visible pop; acceptable (the rock was just hit),
  or later: generate ALL destructible-asteroid visuals from the field at
  spawn for uniformity.

#### Carving events

- Damage impact -> spherical carve: `d(p) = max(d(p), r_c - |p - hit|)`
  (SDF subtraction) in a local neighborhood; only cells within r_c +
  1 cell are touched. Carve radius from damage (e.g. r_c = k *
  sqrt(damage), clamped to 1-3 cells).
- Hook: the same collision/damage path that decrements Health on the
  collider node today; the carve consumes the hit point + damage the
  integrity glue already sees. Batch all carves on an asteroid within a
  fixed tick into one remesh.

#### Colliders (avian)

- Rebuild `Collider::trimesh_from_mesh` from the remeshed surface - the
  SAME call the spawn path uses; closed surface-nets output keeps
  mass-from-volume honest (ColliderDensity stays). Parry QBVH build for
  4-5k tris: ~1-5 ms native, order-of-magnitude.
- Convex decomposition (VHACD) per recarve: 50-500+ ms, parameter-
  sensitive. REJECTED (matches ship SPIKE R2.5 reasoning).
- Compound-of-convex-chunks: pays only if chunks pre-exist; a carved field
  changes shape arbitrarily, so maintaining a decomposition costs more
  than the trimesh rebuild. Rejected for the main body; severed chunks DO
  use `convex_hull_from_mesh` (like today's fragments).
- Well rocks are Static, small rocks Dynamic - both take trimesh swaps;
  BodyRadius shrinks only (carving removes material), so SOI/orbit bands
  stay valid without recomputation.

#### Spewing pieces and small parts

- Every carve: spawn 2-4 small shards (existing debris idiom: TempEntity'd
  convex fragments or the section-debris cubes) launched from the crater -
  the immediate "spew".
- Island detection: 6-connected flood fill over solid cells after each
  carve batch - 48^3 full pass ~110k cells, <1 ms native, few ms wasm;
  run only on carve frames. Same connected-component idea as the ship
  SPIKE's severing (R2.1), applied to the voxel graph instead of
  `ConnectedTo`.
- Severed island -> free-floating chunk, by size:
  - >= ~30 cells: mesh the island (same surface nets, island cells only),
    spawn a Dynamic body with `convex_hull_from_mesh`, inherit
    `v + omega x r` kinematics (ship SPIKE R2.2 rule). Conceptually a
    degenerate parts-object (no controller) - if `PartsBodyMarker` lands
    first, tag chunks with it and the aggregate-health backstop is free;
    otherwise a plain one-shot chunk with optional Health.
  - < ~30 cells: convert to shards + roll on an authored loot policy -
    spawn salvage crates ("small parts"). Salvage crates need scenario
    ids; spawned loot needs an id scheme (e.g. `<asteroid_id>_loot_<n>`)
    or an id-less pickup variant - flagged as a design point for the
    integration task, not solved here.
- Chunks are NOT carvable themselves (no grid) - they are debris. Keeps
  memory and code bounded.

#### Performance budget

- Per recarve (48^3, native): carve region update ~0, remesh 1-3 ms, CC
  <1 ms, trimesh 1-5 ms => ~5-10 ms. Run remesh + collider build on
  AsyncComputeTaskPool; swap mesh/collider handles on completion; at most
  1 job in flight per asteroid (a newer carve supersedes a queued one).
- Frequency: blaster fire ~2-5 hits/s on ONE rock -> coalesced to <= 5
  rebuilds/s on that rock only. Fine.
- wasm: single-threaded -> the job runs on the main thread; at 32^3 the
  whole recarve is ~3-8 ms - an acceptable hit-frame spike; defer the
  swap a frame if needed. This is the reason for the 32^3 wasm cap.

#### Health semantics + coexistence with slice.rs

- Keep Health as the kill gate UNCHANGED: carving is the damage
  VISUALIZATION and physical shape change; Health 0 still kills. Zero
  rebalance, scenario contracts (OnDestroyed counts) untouched. Later
  option: volume-based death (destroy when remaining volume < ~20%) as a
  deliberate balance change.
- What slice.rs still does BETTER and where it STAYS:
  - The death path: slicing the CURRENT (carved) mesh into 4 fragments is
    a cheap one-shot that already works on arbitrary meshes - the carved
    asteroid's finale routes through ExplodeMesh exactly as today.
  - Ships and props: section meshes, gltf content - no SDF exists and
    never will; slice.rs is the only fragmenter for them.
  - Low-budget contexts: one slice is far cheaper than voxelize+remesh;
    anything that just needs to "burst" keeps slicing.
  - slice.rs is also the fallback if a carve job fails (bad island, empty
    field): explode what remains.
- Carving REPLACES nothing; it inserts a persistent-damage stage between
  "hit" and "dead" for asteroids only.

#### Staged escalation (Topic B)

1. UV fix: triplanar MaterialExtension + repeat sampler + compare row
   (0.5-1 day; folds into the round-2 texture-swap task).
2. Carve prototype example: one asteroid, analytic field 48^3, click/hit
   carve, surface-nets remesh with flat normals, trimesh swap; eyeball the
   faceted look + measure remesh/collider times (2-4 days, example-only).
3. Island severing: flood fill, chunk meshing, kinematic inheritance,
   shard/salvage spew (2-3 days, prototype extension).
4. Full integration: damage-pipeline hookup, async jobs, wasm budget,
   scenario grammar (`carvable: bool`, loot policy on AsteroidConfig),
   husk/BodyRadius bookkeeping, slice.rs finale wiring (3-5 days).

Gate each stage on the previous one's look/perf verdict - stage 2 is the
cheap kill-switch if surface-nets output fights the art direction.

## Escalation plan v3 (supersedes round 2's plan)

1. Asteroid texture swap + TRIPLANAR fix (content + small code): Rock030
   default + Rock035 variant, credits, repeat sampler, triplanar
   MaterialExtension (B1).
2. Planet scenario object, body mode (code + content): `planet` type on a
   UV sphere, sphere collider, gravity well, spin; import 4-6 SBS maps +
   credits; author one orbit beat (A3).
3. Backdrop planets: `backdrop` flag + sky-anchor system, presence gating
   (A2a).
4. Debris/salvage/beacon props (unchanged from round 2): generic gltf-prop
   object; Kenney meteors/dishes/machines.
5. Station kitbash (unchanged): Fertile Soil derelict station.
6. Carve prototype example (B2 stage 2) - spike-grade, gates the rest.
7. Island severing + chunk/loot spawns (B2 stage 3).
8. Carve integration (B2 stage 4).
9. Distant traffic (unchanged, last; biggest style risk).

Parked: skybox planet bake (A1) - only for dense vistas; second-camera far
layer (A2b) - unnecessary under reversed infinite-Z.
