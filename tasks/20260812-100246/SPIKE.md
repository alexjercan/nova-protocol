# Spike: ship building from parts instead of cubes (2026-08-12)

Design + prototype for moving from 1-unit cube sections to semantic parts
(fuselage, wings, cockpits, thrusters, weapons). Reference pack: Fertile Soil
"Spaceship Blocks Collection" (CC0, verified on the itch.io page 2026-08-12;
the zip ships NO license file, so an import must add a name/URL/license/date
entry to `art/README.md` like the kenney-space-kit one). Prototype:
`scripts/cut-obj-into-parts.py` + `scripts/part-recipes/craft_racer.json`.

## Ground truth (measured, not assumed)

- Ship storage is ALREADY continuous. `SpaceshipSectionConfig` carries
  `position: Vec3`, `rotation: Quat`, `source: Prototype(id) | Inline`
  (`crates/nova_scenario/src/objects/spaceship.rs`). The 1-unit grid is a
  convention enforced in exactly three places:
  1. Editor placement: `position = hit.translation + normal * 1.0`
     (`crates/nova_editor/src/placement.rs`).
  2. Integrity adjacency: neighbours iff center distance == 1.0 +- 0.1
     (`crates/nova_gameplay/src/integrity/glue.rs`). The ONE hard engine
     dependency on the grid.
  3. Ship lint overlap check: AABB from `SectionCollider` half extents -
     already extent-aware, works for parts as-is.
- The runtime ALREADY has a ship graph. Every section carries
  `ConnectedTo(Vec<Entity>)` - node-local adjacency, deliberately not a
  central map (`crates/nova_gameplay/src/integrity/components.rs`). It is
  DERIVED from geometry at spawn, not authored. Destruction walks it:
  disabled interior node = deactivated but still load-bearing, disabled leaf
  = destroyed, destroy prunes neighbour lists and cascades.
- What does NOT exist: connected-component analysis. Destroying a cut vertex
  leaves the severed subgraph welded to the same rigid body - nothing
  detaches or drifts. `neutralize.rs` handles "powerless wreck" only at
  whole-ship granularity.
- `SectionCollider` is authorable per section (Cuboid/Sphere/Capsule/Cylinder)
  and is snapshotted onto the section entity; NOVA OS reads
  `aabb_half_extents` for schematics. Default = unit cube.
- avian3d 0.7.0 (pinned, default features incl. `collider-from-mesh`) has
  `Collider::convex_hull(points)`, `convex_hull_from_mesh`,
  `convex_decomposition` (VHACD), `compound`, `trimesh` - verified in the
  registry source of the exact pinned version.
- Blocks pack: 95 OBJ+MTL pieces, true flat Kd colours, no textures,
  multi-object files (greeble sub-objects), half-unit-friendly bboxes
  (wing 1.70 x 0.30 x 3.00, thruster cluster 1.00 x 0.70 x 1.68). Inspector
  verdict PARTIAL only because the CUBE cutter treats multi-object as one
  mesh; as PARTS they need no cutting at all.
- Racer cube library: 18 cubes, 152 KB. Same ship as 7 semantic parts: 95 KB.

## 1. Part data model

- A part IS a `SectionConfig`. No new top-level type: `base` (id, health,
  mass, sounds, collider) + `kind` already model everything a part needs.
  What changes is the CONTENT: real collider extents instead of the implicit
  unit cube, and a render mesh that is a whole part instead of a cube tile.
- Placement: half-unit grid anchors + free quarter-turn rotation. NOT a
  socket/mount-point graph: the pack's bboxes are half-unit friendly, so a
  0.5 snap plus face-offset placement gives most of the socket value at a
  fraction of the machinery. Storage stays continuous, so sockets can be
  added later as optional part metadata without a format change.
- New authoring surface per part (catalog side, not engine side): a category
  tag for the palette (Structure/Propulsion/Weapon/Misc - the pack's naming
  maps 1:1) and the footprint (= collider AABB, already derivable).
- Mount-like data that already exists stays where it is: thruster exhaust
  offsets, turret joint trees, torpedo spawn offsets are per-kind config.
- The graph view of the same data - parts as nodes, attachments as edges -
  is analysed in 3b: it is the RUNTIME representation (and already exists);
  the serialized form stays a flat list with edges derived.

## 2. Collider strategy

- Hybrid, primitives first:
  1. Default: primitive auto-fit per part. The cutter manifest already emits
     `collider_cuboid_size` (tight AABB). Wings/fuselage/engines fit a cuboid
     well; dishes fit spheres/cylinders. Deterministic, cheap, zero engine
     change (SectionCollider already authorable).
  2. Add `SectionCollider::ConvexHull { points: Vec<Vec3> }` for parts an
     AABB fits badly (angled wings, curved fuselage). Points are computed
     OFFLINE by the asset script and authored into the RON - runtime only
     calls `Collider::convex_hull(points)`. Deterministic (no runtime VHACD,
     no mesh dependency), and `aabb_half_extents` = bbox of points keeps the
     lint cheap.
  3. Rejected: runtime `convex_decomposition` (VHACD cost + parameter-
     sensitive output = determinism risk); trimesh (no interior volume, avian
     mass-from-volume breaks, poor for dynamic bodies).
- Physics cost goes DOWN, not up: 7 part colliders vs 18-45 cube colliders
  per shipped ship. Hull-vs-hull narrowphase at <100 verts/part is cheap in
  parry; fewer contact pairs dominate the win.
- Mass: avian derives mass from density x collider volume. Hull volume is
  more honest than bbox volume; retune per-part `mass` at import.

## 3. Part vs section semantics

- Every PLACED part is a section: own health, own damage class, own detach.
  Keeps the whole integrity/damage/explode/tint pipeline unchanged - it is
  all per-section already.
- Greebles do NOT become sections: multi-object sub-parts stay merged into
  their part's glb (visual only). The part is the damage unit. `--per-object`
  exists for when a pack file really contains several logical parts.
- Damage feel: fewer, bigger sections = chunkier destruction. Per-part HP
  scales with footprint (a wing is one 200 HP section, not 4 x 60 HP cubes);
  playtest owns the numbers.
- THE engine change: integrity glue adjacency. Replace the center-distance
  == 1.0 test with an AABB-touch test using each section's snapshotted
  `SectionCollider::aabb_half_extents` (expand by eps, overlap = connected).
  Unit cubes 1 apart touch exactly, so every existing ship builds the same
  graph - the parity is testable and must be pinned before anything else.

## 3b. Graph data model: parts as nodes, attachments as edges

Owner direction: represent the ship as a graph. Analysis against the current
flat list, layer by layer - the two answers differ.

Runtime layer - the graph already won:
- `ConnectedTo` IS parts-as-nodes / attachments-as-edges, and the whole
  destruction pipeline is graph algorithms over it (leaf derivation, prune,
  cascade). The parts work rides on this as-is; only edge DERIVATION changes
  (AABB touch instead of distance == 1.0).
- Representation choice inside the ECS, compared:
  1. Node-local adjacency component (current). Symmetric, cycle-friendly,
     observer-friendly (prune = mutate the neighbours you already hold), and
     uniform: a lone asteroid is the same shape as a 45-section ship. Keep.
  2. Edges as `ChildOf` parent/child: rejected. That makes structure a TREE -
     a 2x2 cube block or a wing attached at two ribs is a cycle and cannot be
     expressed; killing a parent implicitly orphans transforms of children;
     and it conflates the render/transform hierarchy with structural load.
  3. Central graph resource/component on the root (adjacency map or
     petgraph): easier whole-graph queries, but adds entity-lifecycle
     bookkeeping the node-local form gets free, serializes worse (Entity ids
     are not stable), and single-component mutation contention. Not worth it
     at N <= ~50 nodes; whole-graph passes can just walk `ConnectedTo`.
- The payoff feature the graph unlocks (new work, representation-independent):
  connected-component split on destroy. When a node dies, BFS from the
  controller (or root anchor) over `ConnectedTo`; every unreached component
  re-roots onto a fresh dynamic body inheriting linear + angular velocity -
  severed wings DETACH AND DRIFT instead of hanging in space welded to
  nothing. O(N+E) on destroy only; traversal cost is a non-issue at ship
  sizes. Detached components keep their sections, so they stay shootable
  debris; a detached component with a thruster but no controller is a
  spinning wreck, which `neutralize.rs` semantics already describe.

Authoring/save layer - flat list stays, edges become optional:
- Explicit edges in the save format are redundant with geometry: an
  attachment is derivable from touching authored colliders. Redundant data
  can disagree - an edge between non-touching parts is invisible glue, a
  touch without an edge is a phantom gap - so every authored edge adds lint
  surface (the derive-and-compare check) plus migration cost (existing cube
  ships and the editor's hand-off would need edge generation).
- What authored edges genuinely buy, when wanted later:
  1. Attachment INTENT: a part touching two parts records which one it is
     mounted ON - drives editor subassembly operations (move/mirror/delete a
     wing WITH its engine) and mount-point validation.
  2. Non-contact attachments (pylon-mounted pods, energy struts) - cannot be
     derived from touch.
  3. Robustness: derived adjacency depends on an epsilon; an authored edge
     does not. (With half-unit snapped anchors and authored extents, faces
     meet exactly by construction, so this is theoretical for now.)
- Recommendation: two layers. Serialized ships stay a flat section list
  (positions + rotations; no format break, mods unchanged); the runtime graph
  is derived at spawn exactly as today. Add an OPTIONAL serde-defaulted
  `attachments: [(id, id)]` field only when subassembly UX or non-contact
  mounts land - old files parse unchanged, and the runtime treats authored
  edges as EXTRA edges unioned with derived ones, so the two sources cannot
  fight over connectivity, only extend it.

## 4. Editor UX

- Palette: `GameSections` already IS the palette; parts are ordinary catalog
  entries (`hide_in_editor` keeps the cube tiles out today - parts stay
  visible). Add category grouping to the drawer UI from the new tag.
- Placement: raycast the hovered part face (unchanged); offset by the SUM of
  half extents along the face normal instead of `normal * 1.0`; snap the
  resulting anchor to the 0.5 grid. Preview ghost = the part's actual render
  mesh with a translucent material instead of the 1.01 cube.
- Rotation: quarter-turn hotkeys (yaw/pitch/roll). Mirroring: emit `_port` /
  `_starboard` part variants from the cutter (the racer recipe already cuts
  both wings) rather than runtime negative-scale tricks - colliders and
  integrity stay honest.
- Validation at Play hand-off: require >= 1 controller, >= 1 thruster, and a
  connected integrity graph (one BFS over the same derived adjacency as 3b),
  surfaced in the editor instead of a silent broken ship. Ship lint overlap
  check already works once colliders are authored per part.
- Save format: UNCHANGED. `PlayerSpaceshipConfig` -> `SpaceshipSectionConfig`
  list with continuous position + quat serializes parts today.

## 5. Migration

- No format break, no conversion pass, no compatibility layer beyond the
  glue generalization (which preserves the cube case by construction).
- Cube prototypes stay in the catalog untouched; racer/cargob/cargoa ships
  keep flying. Parts land as NEW prototypes via the content builders
  (`content -- gen`; the RON stays generated, never hand-edited).
- Ships convert one at a time by swapping their builder to part prototypes
  (`racer_part_nose`, ...). Mixed cube+part ships are legal - both are
  sections with colliders.

## 6. Asset pipeline

- "One dedicated script per ship" became recipe DATA + one engine:
  `scripts/cut-obj-into-parts.py` (stdlib only, mirrors the hulls cutter's
  clipping/caps/glb writer) + `scripts/part-recipes/<ship>.json`. A recipe is
  an ordered rule list; each rule claims fragments by box-plane clipping
  and/or material/object filters; the remainder is the `rest` part. Adding a
  ship = writing ~15 lines of JSON, not a script.
- Contrast with `cut-obj-into-hulls.py`: the grid cutter stays for cube
  ships; the parts cutter replaces it for new content. Same area-conservation
  invariant, same glb conventions, plus a manifest with ship-space origins
  (reassembly data) and collider suggestions.
- Blocks-pack import path: identity mode (1 obj file -> 1 part glb) or
  `--per-object`. No cutting, no bake - the pack is flat-Kd. At import:
  `art/README.md` entry (name, itch URL, CC0, verified 2026-08-12).
- Palette-atlas packs (Quaternius, newer Kenney) still need the
  bake-atlas-to-Kd pass from research round 1 before any of this applies;
  the writer drops UVs by design.
- WASM size: parts SHRINK the download. Racer: 95 KB as 7 parts vs 152 KB as
  18 cubes (cube libraries duplicate cap geometry per cell). A curated
  ~30-piece blocks import at ~10-30 KB/piece is ~0.5-1 MB against the
  current 15 MB `assets/base`. Import curated subsets, never all 95.

## 7. Verdict

GO. The storage format already supports parts; the engine change is one
adjacency function; colliders need one new variant at most; the asset
pipeline is proven end to end by the prototype. Ordered escalation
(SUPERSEDED by the round-2 plan below, owner direction 2026-08-12):

1. Integrity glue: AABB-touch adjacency with unit-cube parity tests
   (existing ships build identical graphs). Blocks everything else.
2. Connected-component split on destroy: severed subgraphs re-root onto new
   dynamic bodies and drift (see 3b). The graph payoff feature; also fixes
   the same latent bug for cube ships.
3. `SectionCollider::ConvexHull { points }`: serde round-trip, avian
   construction, `aabb_half_extents`, lint coverage.
4. Import a curated Fertile Soil subset (glb via the parts script), license
   entry in `art/README.md`, parts catalog builder + category tag,
   `content -- gen`.
5. Editor placement v2: footprint-aware offset + 0.5 snap, real-mesh ghost
   preview, quarter-turn rotation keys, palette categories.
6. Convert the racer to its 7 semantic parts as new prototypes; playtest
   damage feel and handling (mass shifts with collider volume).
7. Play hand-off validation: controller + thruster + connected graph.
8. Later: cargoA/B + Kenney speeder/miner recipes, mirrored-variant
   emission, palette-atlas bake pass, optional authored `attachments`
   edges + subassembly editor operations (3b), socket metadata.

## Prototype results (verified output, not exit status)

- `python3 scripts/cut-obj-into-parts.py --self-test` -> OK (box clip
  partition, material/object selectors, anchor snap, glb re-open).
- Racer, recipe mode (scale 2, yaw 180 - the shipped library's transform):
  280 tris -> 7 parts (engine_port/starboard, wing_port/starboard, nose,
  tail, fuselage), area conserved 27.893696 == input, every glb re-opened
  from disk with decoded POSITION bounds matching the manifest. Anchors land
  on the half grid: engines (+-0.5, 0.5, 1.5), wings (+-1.0, 0.5, 0.0),
  nose (0, 0.5, -1.5), tail (0, 1.0, 1.5).
- Independent re-check: union of (origin + local bbox) over all 7 parts
  reproduces the ship bbox exactly; Kd colours present in every glb's
  material table; 95,052 bytes total.
- Blocks pack: `Structure_Wing_Double` -> 1 part glb (identity mode);
  `Propulsion_Thruster_Triple_Small` -> 4 parts (`--per-object`: housing +
  3 nozzle objects), both area-conserved and re-verified.

## Not done (honest gaps)

- No in-game wiring: no part glb loaded into the editor or a probe run; glbs
  verified structurally and numerically, not rendered on screen.
- Cap quality on angled cuts not eyeballed; centroid-fan caps on non-planar
  loops can look rough (same caveat as the hulls cutter).
- Manifest suggests bbox cuboids only; no offline convex-hull points yet.
- Racer recipe only; no cargoA/B recipe.
- Physics cost argued from collider counts, not measured with the probe.
- Nothing imported into `assets/` - so no `art/README.md` license entry yet
  (required at actual import).
- Writer drops UVs (fine for flat-Kd packs, blocks palette-atlas packs).

# Round 2: owner direction (2026-08-12)

The owner read round 1 and gave a 100% GO plus decisions. Recorded verbatim
in intent, then deepened below:

1. Verdict accepted: parts route is a definite GO.
2. Integrity: move AWAY from distance-glue entirely - the GRAPH is the
   first-class integrity structure; severing by connected components is
   wanted.
3. Severed components: "dumb debris" for now, but generalize: ships and
   debris are both "objects made of parts"; a ship is a parts-object WITH a
   controller. Debris may keep parts, per-section health and integrity -
   just no control.
4. Player-follow on sever: design-only; hint "follow the one with the
   controller"; analyze the edge cases.
5. HP values: keep simple; no tuning machinery.
6. Link-points: parts carry authored sockets so parts know how to attach.
   Supersedes round-1's "defer authored edges" lean.
7. Colliders: configurable per part; ConvexHull as best-effort default where
   cubes were forced; primitives as explicit config; hull points computed
   OFFLINE (round-1 direction confirmed).
8. Recipes approved; "we just need good recipes" - at least one more recipe
   ship for the viewer.

## R2.1 Graph-first integrity (decision 2)

- `ConnectedTo` (node-local adjacency) STAYS the representation - the 3b
  analysis holds. What changes is its standing: edges become the truth about
  structure, geometry becomes one way to author them.
- Edge sources, unioned at spawn:
  1. Link-point MATES (R2.4) - the authored, primary source for parts.
  2. Derived AABB-touch (round-1 s.3) - the fallback for cube ships,
     link-point-less content, and the asteroid lone-body case. Unit cubes 1
     apart touch exactly, so every existing ship builds the identical graph;
     that parity is pinned by tests before anything else lands.
  The union rule keeps the two sources unable to fight over connectivity
  (round-1 3b): a mate without touch is a pylon, a touch without a mate on
  link-point-carrying parts is a lint warn.
- Severing by connected components (now REQUIRED, was the 3b payoff):
  - Trigger on the same seam the pipeline already exposes:
    `IntegrityDestroyMarker` -> after `prune_a_destroyed_node_from_its_
    neighbours`, BFS over `ConnectedTo` from the anchor (the controller
    section; root-anchor fallback below). Every unreached component severs.
  - Severed component -> fresh dynamic body (R2.2). Cross-fragment edges are
    already gone (the prune removed them); intra-fragment edges SURVIVE the
    reparent untouched, so no re-derivation runs - important, because
    `build_integrity_relations` is keyed on `Add<ColliderOf>` and a reparent
    is a replace, not a reliable re-add.
  - Cost O(N+E) on destroy frames only; N <= ~50.
- Leaf semantics change to note: today a severed-but-connected subgraph is
  impossible, so `derive_integrity_leaves` never sees one. After the split,
  each fragment is its own structure with its own leaves; the cascade
  keeps working per-fragment for free (node-local lists, no global state).

## R2.2 Parts-objects: ships and debris (decision 3)

A "parts-object" = one rigid body (root) + section children + the
`ConnectedTo` graph. A ship is a parts-object with a controller; debris is a
parts-object without one. What that costs, measured against the code:

- New marker: `PartsBodyMarker` (name TBD) on every parts-object root.
  `SpaceshipRootMarker` keeps ship-only concerns (HUD queries, camera,
  neutralize, AI); the health plumbing re-scopes to the new marker:
  - `aggregate_ship_health` (glue.rs) is `With<SpaceshipRootMarker>`-scoped
    today, and it OWNS the structural-death backstop. A fragment root
    outside that scope would be the exact 0-HP-ghost shape the ghost-ship
    tests killed. Re-scoping to `PartsBodyMarker` gives fragments the
    aggregate + backstop for free; the asteroid rationale (no meaningless
    Health on section-less roots) is preserved because lone bodies carry no
    section children and never get the marker.
- Sever spawn path (entities -> new root), a function not a config:
  - Spawn root: `RigidBody::Dynamic`, `Transform` at the fragment's world
    anchor, `TransformInterpolation`, `IntegrityRoot`, `PartsBodyMarker`.
  - Reparent the fragment's section entities preserving world transform.
  - Kinematics: new body inherits `omega` unchanged and
    `v_new = v_old + omega x (com_new - com_old)` - avian recomputes the
    COM from the moved colliders; without the cross term severed wings stop
    dead instead of carrying their tangential velocity.
  - No `SpaceshipController`, no `EntityId`/`EntityTypeName` on the root:
    fragment destruction stays scenario-silent (sections are silent today
    by design); scenario-visible salvage events are a later, deliberate add.
- What debris KEEPS, with zero new systems: section meshes + colliders,
  per-section `Health` and damage classes (the whole typed-damage path is
  per-section), impact/destroy sounds, explode/debris-shard pipeline,
  integrity cascade within the fragment. Shootable, breakable, honest.
- What debris LACKS, and why control dies naturally: input bindings live on
  thruster/turret/torpedo SECTIONS but are inserted per the root's
  `SpaceshipController` at spawn; severed sections KEEP old bindings.
  Sever must strip `Spaceship*InputBinding` from moved sections (one
  remove_bundle) - otherwise a severed thruster still answers the player's
  keys. AI directives live on the old root and do not follow. Attitude PD
  (controller section) follows its section; without a player/AI marker on
  the new root the input systems never address it.
- Salvage hook (design only): sections already carry `EntityTypeName` = the
  prototype id, so a future tractor/salvage verb reads the part id straight
  off the drifting fragment. No new storage needed now.
- Neutralize: unchanged, still `SpaceshipRootMarker`-scoped - a fragment is
  not a combatant. A SHIP that loses all weapons+thrusters to severing
  neutralizes exactly as today (its sections are gone from its children).

## R2.3 Player-follow on sever (decision 4, design only)

Ground truth: camera rig, HUDs and possession all key on
`PlayerSpaceshipMarker` (root entity); ship death removes the marker and the
camera reverts (glue.rs). Controller SECTIONS carry no input bindings -
bindings sit on thruster/weapon sections - so "the one with the controller"
must be defined, not assumed.

- Rule (recommended): on sever, `PlayerSpaceshipMarker` +
  `SpaceshipController::Player` stay with the fragment containing the
  player's CONTROLLER SECTION (the bridge). Possession, camera and HUD
  follow it automatically - no camera code changes.
- Controller on the smaller fragment: follow it anyway. The bridge is where
  the player sits; watching your own hull drift away is the point of the
  feature. Camera framing is extent-aware enough to close in.
- Multiple controller sections: they are interchangeable (no bindings).
  Tie-break deterministically: the fragment holding the controller that
  appears FIRST in the ship's authored section order; then most sections.
- Controller destroyed (no fragment has one): recommend SPECTATE THE DRIFT,
  not instant death. Keep the marker on the fragment with the most living
  sections; controls are dead (thrust/turret bindings can stay - with no
  working sections around the wreck they do nothing; strip them for
  cleanliness), and the existing end conditions decide the actual end:
  aggregate health zero -> death path, or the scenario outcome layer.
  Rationale: "bridge destroyed = instant death" is a real design option but
  a BALANCE change (today controller-section death does not end a ship),
  and it belongs to the damage/outcome layer as a "critical section" rule,
  not to the camera. Severing must not smuggle it in.
- Non-player ships: AI markers/directives stay with the controller fragment
  by the same rule; other fragments are debris. An AI wreck with no
  controller fragment is left as debris (no respawned AI).

## R2.4 Link-points (decision 6)

Where they live: CATALOG data (part asset metadata), not save data.

- Data model, on `BaseSectionConfig` (serde-defaulted empty, so all existing
  content is byte-identical):
  `link_points: Vec<LinkPoint>`,
  `LinkPoint { id: String, position: Vec3 (section-local), normal: Vec3
  (unit, outward) }`. A `class` tag (structural/hardpoint/rail) is a later,
  compatible add - start untyped.
- Seeding from the pack conventions: the Fertile Soil pieces have half-unit
  friendly bboxes, so AABB face centers land on the half grid by
  construction. Import-time generator: one link-point per collider-AABB face
  that is geometry-backed (mesh area within eps of the face plane >= ~40% of
  the face), id'd by face (`px`,`nx`,`py`,...). Naming seeds the filter:
  `Fuselage_*` keeps +-Z (spine flow) and +-X/-Y as configured,
  `Wing_*` keeps the root face only, `Thruster_*` the mount face opposite
  the nozzle, `Weapon_Modular_Gun_*` the documented stack faces. Generated
  points are a starting set; the content builders own the final say.
- Editor snapping (placement v2):
  1. Raycast the hovered part (unchanged); pick ITS nearest link-point to
     the hit.
  2. Orient the ghost so one of the ghost part's link-points mates: normals
     opposed, points coincident -
     `ghost_pos = target_wp - ghost_rot * ghost_lp.position`.
  3. Hotkeys cycle the ghost's candidate link-point and roll in
     quarter-turns about the mate normal.
  4. Fallback where either side lacks link-points: round-1 rule (sum of half
     extents along the face normal + 0.5 snap). Cubes never regress.
- Relation to derived adjacency: a MATE (coincident points, opposed normals,
  eps ~1e-3) is an authored edge - the primary source for parts. Derived
  AABB-touch is unioned in (R2.1). This buys the three round-1 authored-edge
  benefits without a save-format change: attachment intent (the mate SAYS
  which part is mounted on which), non-contact mounts (pylon tip carries the
  point), epsilon robustness (mates are exact by construction).
- Save format: still the flat section list, position + rotation. Mates are
  re-derived at spawn from catalog link-points + placed transforms, so saves
  stay mod-stable and the optional `attachments` field from 3b stays
  deferred - nothing needs it once mates exist.
- Lint: parts with link-points that touch without mating -> warn (kissing
  without a socket); mated pair whose sections' `ConnectedTo` would not
  contain each other -> impossible by construction (union rule).

## R2.5 Colliders (decision 7)

Audit - what content RON actually allows vs what the docs claim:

- `SectionCollider` (nova_ship base_section.rs): Cuboid{size} /
  Sphere{radius} / Capsule{radius,length} / Cylinder{radius,height};
  omitted field = unit cube; snapshotted onto the section entity;
  `aabb_half_extents` feeds NOVA OS schematics + ship lint.
  `web/src/wiki/modding/sections.md` documents exactly these four forms and
  the density note. Docs and code AGREE - no drift found.
- One real gap found (escalation item): the ship overlap lint
  (`check_section_overlaps`, nova_scenario lint/ship.rs) resolves colliders
  for INLINE sections only; `Prototype` refs fall back to the unit cube
  ("the catalog is not in scope here"). Harmless while prototypes are unit
  cubes; WRONG for part prototypes (a 4-unit pod linted as a 1-unit cube
  misses real overlaps). Fix: extend `KnownSections` (already threaded into
  the lint for mount kinds) with per-id `aabb_half_extents`.
- `SectionCollider::ConvexHull { points: Vec<Vec3> }` design:
  - Points are OFFLINE-computed (owner-confirmed): the parts cutter grows a
    stdlib quickhull and emits `collider_hull_points` (section-local,
    deduped, target <= ~48 points) next to `collider_cuboid_size` in the
    manifest; the import builder authors them into the RON. No runtime
    VHACD, no mesh dependency, deterministic.
  - `to_collider`: `Collider::convex_hull(points)` (avian 0.7, verified in
    round 1). It returns Option: content lint owns validation (>= 4
    non-coplanar points, count cap); runtime falls back to the AABB cuboid
    with an error log so a bad mod cannot panic a shipped build.
  - `aabb_half_extents`: conservative symmetric extents,
    `max(|min_k|, |max_k|)` per axis - the lint and NOVA OS assume extents
    centered on the section origin, and hull points need not be centered;
    conservative beats a silent off-center under-approximation.
  - Mass: avian derives mass from density x collider volume; hull volume is
    more honest than bbox volume. Re-check part `mass` values at import.
  - Default policy (owner): import builder defaults to ConvexHull where a
    box fits badly, primitive as explicit config. Concrete heuristic the
    script can emit: hull_volume / bbox_volume >= ~0.85 -> suggest Cuboid,
    else ConvexHull; dishes/domes suggest Sphere. The builder (a human
    decision point) can override per part.

## R2.6 HP values (decision 5)

Keep authored numbers simple: per-part `health` scaled by footprint at
import (a wing is one ~200 HP section, not 4x60 cubes), same for `mass`
density. No tuning machinery; playtest owns the numbers later.

## R2.7 Recipes (decision 8) + round-2 prototype results

- Second recipe ship: `scripts/part-recipes/craft_cargob.json` - the cargob
  hauler as 7 semantic parts (engine_port/starboard at the pod rears,
  pod_port/starboard full-length flanks, nose, tail, fuselage rest). Cut
  verified: area conserved 51.425131 == input; every glb re-opened and
  bounds-checked; independent union check: origin-placed part bboxes
  reproduce the ship bbox EXACTLY for both racer and cargob (script in the
  session scratchpad, method as round 1).
- Recipe authoring cost stays low: an axis-band area histogram over the
  transformed soup (10 lines of python) exposes the cut planes; the cargob
  recipe took one iteration.
- Imports landed this round (art/README.md updated):
  - `art/spaceship-blocks/`: full Fertile Soil pack, 95 OBJ+MTL (CC0,
    verified on the itch page 2026-08-12; zip ships no license file - the
    README entry is the record).
  - `art/kenney-space-kit/`: + craft_miner, craft_speederA sources.
  - `art/part-candidates/`: GENERATED viewer content, committed for
    reproducible judgement, never shipped (art/ is excluded from builds):
    racer (7 parts), cargob (7), 22 blocks pieces (identity mode, one dir
    each under `blocks/`, selection spans Structure/Propulsion/Weapon/
    Misc), miner + speederA cut whole (scale 2, yaw 180).
  - Regeneration: racer/cargob via their recipes; blocks via
    `cut-obj-into-parts.py art/spaceship-blocks/Spacestation_<P>.obj --out
    art/part-candidates/blocks/<p>`; craft via `--scale 2 --yaw 180`.

## R2.8 Parts viewer example (deliverable 2)

`examples/screenshots/parts_viewer.rs`, cataloged in Cargo.toml.

- Category reasoning: screenshots/ - the walk is graded like any autopilot
  script (probe: correctness passes only), but the FRAME is judged by human
  eyes, which is precisely this example's purpose (owner judges the part
  meshes). It asserts nothing about simulation; sections/ and systems/
  would be false claims, stress/ has no steady-state window.
- Loading: a dedicated `part-candidates://` bevy asset source
  (FileAssetReader rooted at `art/part-candidates`, registered before
  DefaultPlugins) - generated glbs stay OUT of assets/ per the art/README
  rule. The art-research worktree had no custom-source example to reuse;
  the mechanism follows nova_assets' `mods://` registration.
- Views: paged 4x3 gallery (name + pack labels, selection box, true scale
  capped at fit), focused turntable, recipe-ship view assembled/exploded
  (parts at manifest origins; explode pushes each origin out x2 from the
  ship centre). Editor code is NOT reused: the editor previews CATALOG
  sections via GameAssets/GameSections, and these candidates are
  deliberately not catalog content yet - a bare widget_zoo-style app is the
  honest fixture.
- Verification (all rendered output eyeballed, not exit-status):
  - Xvfb :99 run, NOVA_AUTOPILOT=1 NOVA_CAPTURE=1: 9 shots (4 gallery
    pages, focused cockpit, racer+cargob x assembled/exploded), labels
    legible, parts distinct, no black frames; cargob assembled reads as the
    catamaran hauler, exploded separates all 7 parts.
  - `probe run parts_viewer`: verdict OK - process_exit, run_completed,
    reached_playing, invariants_held, log_clean, artifacts_loadable all
    PASS; fps not claimed (no steady-state window, matching
    render_scale_shot's contract choice).
  - `cargo fmt --check` clean.
- In passing this closes round-1 gaps: part glbs now render in-engine
  (bevy gltf path), flat-Kd colours read correctly, caps read solid, and
  reassembly-at-origins is proven on screen for two ships.

## Escalation plan v2 (supersedes section 7)

Ordered follow-up tasks; none created in the tracker (owner-only backlog):

1. Derived AABB-touch adjacency + unit-cube parity tests (unchanged from
   round 1; still blocks everything).
2. Connected-component severing on destroy: `PartsBodyMarker`, the sever
   function (reparent + kinematics + binding strip), re-scoped aggregate
   health/backstop, player-follow rule (controller fragment keeps
   possession; spectate-the-wreck fallback). Fixes the same latent
   welded-wreck bug for cube ships.
3. `SectionCollider::ConvexHull { points }` + offline quickhull emission in
   the parts cutter + lint validation + the prototype-extent fix for
   `check_section_overlaps`.
4. Link-points: `LinkPoint` on `BaseSectionConfig`, import-time seeding from
   the blocks conventions, mate-derived edges unioned with touch, lint
   cross-checks.
5. Curated Fertile Soil subset into `assets/` via the content builders
   (category tag, `content -- gen`); viewer gallery is the selection tool.
6. Editor placement v2: link-point snapping with footprint-offset fallback,
   real-mesh ghost, quarter-turn keys, palette categories.
7. Racer + cargob as part prototypes; playtest damage feel and handling.
8. Play hand-off validation: controller + thruster + connected graph.
9. Later: cargoA/miner/speeder recipes, mirrored variants, palette-atlas
   bake pass, salvage verbs over fragment part ids, link classes.

## Round-2 honest gaps

- Design only: severing, PartsBodyMarker, link-points, ConvexHull variant -
  none implemented; the escalation order above is the implementation claim.
- Sever kinematics (omega x r term) and the Add<ColliderOf>-on-reparent
  behaviour are reasoned from avian docs/source, not prototyped.
- Blocks conversion: 22 of 95 pieces, identity mode only; multi-object
  pieces keep greebles merged (intended). `Glass` materials render opaque
  (Kd-only writer - fine for judging shape, not for final cockpits).
- Whole-craft gallery entries render scale-capped to fit their cell; the
  size label carries the truth.
- Exploded view is a uniform radial push from the ship centre, not
  link-aware.
- A recipe `rest` part can read disjoint out of context (cargob fuselage
  carries the full-width belly plate); recipes can claim such geometry
  explicitly when it matters.
- Probe verdict OK is against CURRENT master semantics; a pending
  probe-optout branch changes UNPROBEABLE handling but does not affect this
  example (it wires timeline + invariants).

# Round 3: solid cut faces (owner feedback, 2026-08-12)

Owner: "the Kenney ones - the insides do not have faces (because we slice the
mesh and the inside obviously has no faces) - think about how to fix that."
This round closes the round-1 gap "cap quality on angled cuts not eyeballed".

## Diagnosis (measured on the committed glbs)

- Caps WERE generated (the `--caps` default is on): every racer/cargob part
  glb carried a `_cap` primitive and near-zero boundary edges. The failure
  was GEOMETRIC, not missing caps.
- The old `cap_boundary` walked ALL boundary edges of a part into loops and
  fanned each loop to its 3D centroid. A part bounded by 2+ planes (wing =
  fuselage cut + engine cut) has ONE skin loop that wanders across all its
  cut planes, so the fan produced shards through the part interior with
  mixed winding. Measured: 558 of 615 cap triangles across racer+cargob
  (91%) were not flat on any cut plane. Reads as hollow/glitchy - the owner
  report.
- Non-convex sections (L-shaped wing cross-section) made even single-plane
  fans overlap; winding was uncontrolled (double-sided material hid the
  culling half of the problem, not the lighting half).
- Old cargob open-chain skips (3-9 boundary edges) were quantization breaks
  plus source-mesh T-junctions, dropped silently.

## Fix (`scripts/cut-obj-into-parts.py`, stdlib only)

- Plane-aware capping (`cap_cut_planes`): the cutter KNOWS every plane it
  clips at (`recipe_cut_planes` - all finite recipe box bounds; `claim_part`
  clips the whole pool at each, so any part can end at any plane). Per part,
  per plane: collect edges lying in the plane used by an ODD number of part
  triangles (surface-interior edges pair up and cancel), weld endpoints with
  tolerance (`_Welder`, 1e-5 spatial hash), chain into closed loops.
- Corner joints (`_synthesize_joint_segments`): where two openings meet, the
  joint line runs through the interior - no surface edge exists, each
  plane's outline stops there (the round-3 first-attempt failure). Odd-degree
  vertices on a plane-plane line are solid/void toggles; sort along the free
  axis, pair even-odd, add the synthetic segment to BOTH planes. The two
  caps meet flush; the joint edge ends up shared by exactly 2 cap triangles.
- Triangulation: 2D ear clipping in the plane frame (non-convex sections
  stay inside their outline; T-junction vertices stay real corners so cap
  edges match surface edges exactly). Centroid fan only as a noted fallback
  when no ear exists.
- Deterministic winding: loop side = sum of contributing triangles'
  off-plane offsets; cap faces AWAY from the material. Proven by self-test
  (cube cut: every cap normal outward) and by a post-run check on the real
  glbs (all cap normals face out of their cut planes).
- Cross-section HOLES (tube cut): nesting detected via point-in-polygon;
  the hole loop is bridged into its outer loop (nearest-vertex two-way
  bridge) and reported loudly. Deeper nesting is skipped with a note.
- Watertightness report: per part, `open cut edges N (pre-cap M,
  source-odd K)`. Odd-count edges ON a cut plane after capping fail the run
  (exit 1); off-plane odd edges are source-mesh T-junctions (a large face
  abutting two smaller ones - surface covered, no hole) and only reported.
- Old `cap_boundary` kept as FALLBACK for boundary loops on no cut plane
  (real source-mesh holes, e.g. the racer fuselage underside rectangle);
  winding there stays uncontrolled, documented.
- Cap material: dark neutral bulkhead Kd (0.16, 0.17, 0.20), roughness 0.9 -
  sits just below Kenney "dark" trim (0.27/0.30/0.34) so cut faces read as
  intentional interior, consistent across all parts of a ship. Per-part
  darkened dominant hull colour rejected: sibling caps would disagree and
  the writer's shared material table would need per-part forks.
- Area-conservation check unchanged: compares PRE-CAP surface only; caps are
  additional by design.
- Identity/per-object imports have no recipe rules, hence no cut planes and
  no plane caps by construction; self-test asserts a watertight cube gets
  zero caps. All 22 blocks pieces regenerated byte-comparable except the new
  `_cap` material colour and manifest fields (0 caps added everywhere).

## Round-3 verification

- `--self-test`: cube cut (watertight both halves, outward winding, cap
  area = cross-section), corner part (two orthogonal planes, joint
  synthesis, no fan), L-prism (non-convex; every cap centroid inside the L),
  square tube (hole bridged, bore left open, loud note), watertight-input
  no-op. All prior tests kept.
- Regenerated ALL of art/part-candidates: racer 7, cargob 7, blocks 22,
  miner + speederA. Racer+cargob: open cut edges 0 on every part; cap
  triangles 570 total, 100% flat on their cut planes (was 91% shards);
  cargob keeps 12/4/13 source-odd T-junction edges on nose/tail/fuselage
  (present in the source obj, surface covered).
- Viewer re-run under Xvfb (11 shots eyeballed): new deterministic cut-face
  close-ups `parts-viewer-racer-wing-cut.png` / `-fuselage-cut.png` (the
  autopilot stops the turntable and poses the part so the caps face the
  camera); wing's C-shaped section is one coherent dark bulkhead, engine
  notch capped, fuselage bulkheads flat; exploded racer/cargob: every part
  reads closed; assembled ships unchanged, no z-fighting, no cap
  bleed-through; blocks/craft pages unchanged.
- `probe run parts_viewer`: OK (process_exit, run_completed,
  reached_playing, invariants_held, log_clean, artifacts_loadable PASS; fps
  not claimed). `cargo fmt --check` clean.

## Round-3 limitations + port plan

- Joint synthesis assumes two-plane joints; a THREE-plane corner point
  (x, y and z bounds meeting inside a part) would need line segments split
  per octant - current recipes bound parts on x/z only. The odd-endpoint
  guard notes and drops instead of guessing.
- Hole bridging uses the nearest-vertex bridge without a visibility test;
  pathological outlines could self-intersect (ear clip then falls back to a
  noted fan). Not hit by any current content.
- Source-mesh T-junctions are reported, not healed (healing would re-split
  surface triangles; out of scope for a cutter).
- PORT PLAN (escalation, do not regenerate shipped assets this round): the
  SHIPPED cube libraries (assets/base/gltf/racer, cargob, cargoa via
  `cut-obj-into-hulls.py`) have the same hollow look in game - same
  centroid-fan `_cap_boundary`. The fix ports directly: grid cuts are
  axis-aligned planes at cell bounds, so `cap_cut_planes` +
  `_synthesize_joint_segments` + the watertightness gate drop in; each cell
  is bounded by up to 6 planes (three-plane corners DO occur at cube
  corners, so the joint synthesis needs the octant extension first, or
  cell-local capping per face rectangle which the grid makes trivial).
  Regenerating shipped cube libraries afterwards is a separate owner-visible
  change (game-visible meshes + WASM size), so it needs its own task/round.

# Round 4: recipe tuning per owner feedback (2026-08-12)

Owner: racer engines clipped at the back (expand left/right = thinner tail),
wings clipped (thinner fuselage); cargob same - thinner middle so the
thrusters and torpedo bays come back to the pods. Translated into plane
moves derived from the source geometry, not eyeballed.

## Measured seams

- Racer: x=+-0.4 is an exact seam - ZERO triangles straddle it anywhere in
  the ship. Nacelles span x 0.4..0.9 (nozzle interior 0.5..0.8), wings
  0.4..1.2, tail fin exactly -0.4..0.4, nose |x|<=0.4. The old +-0.55 planes
  sliced 21 wing-zone and 16 nacelle-zone triangles.
- Cargob: x=+-0.6 is the same kind of exact seam. Pod walls span x 0.6..1.4;
  thruster nozzles and torpedo bay mouths (x 0.8..1.2) hang off them. The
  old +-0.7 planes sliced 35 triangles of pod wall.
- BUT each seam plane contains coincident SKIN faces (racer: 25 per side -
  fuselage/nose side walls, fin sides; cargob: 31/27 - pod inner walls).
  Two consequences, both hit when cutting exactly ON the seam:
  1. The centroid-side claim was decided by the yaw-180 float residue
     (~1e-16, sign varies with z) - port/starboard came out asymmetric.
     Fixed in the cutter: box claims test the centroid STRICTLY inside
     finite bounds (eps 1e-9); coplanar skin stays with the remainder.
     Self-test pins it. No other output changes (no prior recipe cut
     through in-plane faces).
  2. Sheet faces in a cut plane defeat the round-3 capping model: the wing
     footprint is a real HOLE in the fuselage wall with a T-junctioned rim,
     and wall sheets split at z=1.2 leave odd 1D edges that are not holes -
     the watertight gate cannot hold. Cutting exactly on the seam needs a
     planar-arrangement capper; out of scope.
- Resolution: cut 0.01 OUTSIDE the seam. Racer +-0.41 (was 0.55), cargob
  +-0.61 (was 0.7). Wall skin stays with the central body (fuselage keeps
  its coloured walls; the 0.01 stub ring caps read as small dark mount
  plates), every cut section is a real 2D region, and the features keep all
  but a 0.01 sliver.

## Result (regenerated racer + cargob only)

- Racer: tail/fuselage width 1.10 -> 0.82, wings 0.65 -> 0.79 wide, engines
  0.35 -> 0.49 wide (full nacelle + nozzle frame). Cargob: middle parts
  1.40 -> 1.22 wide, pods 0.80 -> 0.89, engines 0.70 -> 0.79. Port ==
  starboard sizes on both ships. nose.glb (racer) byte-identical - the nose
  never touched the moved planes.
- Gates: area conserved exactly (27.893696 / 51.425131); 0 open cut edges
  on all 14 parts; ZERO cap-anomaly notes (round 3 had open-chain skips);
  anchors on the half grid; union of origin-placed part bboxes reproduces
  each ship bbox exactly.
- Viewer (Xvfb, 14 shots eyeballed): 3 new deterministic close-ups
  (racer engine, cargob engine, cargob pod bay) besides the round-3 wing/
  fuselage ones. Wings keep the full intake frame, nacelles the full nozzle
  recess, pod bays complete mouths; inboard cut faces are single solid
  bulkheads. Assembled before/after pixel diff: RMSE 0.02%/0.01%, 2-3
  pixels over a 3% threshold (seam AA flicker) - the partition
  re-arrangement left the union visually identical, as it must.
- Lighting caveat: in the racer-engine pose the inboard cap faces the key
  light head-on and reads lit gray rather than shadow-dark (every other
  shot's cap normal is perpendicular to the key). The mesh is right (9 cap
  tris, area 0.406 = full profile); it is a pose/lighting artifact only.
- `probe run parts_viewer` OK (correctness passes, fps not claimed);
  `cargo fmt --check` clean; cutter `--self-test` OK.

# Round 5: palette-atlas bake pass + Quaternius import (2026-08-12)

The "palette-atlas bake pass" deferred since round 1 (escalation item 9),
triggered by the owner-provided download of Quaternius "Ultimate Spaceships"
(May 2021 zip; CC0 1.0 per the pack's own License.txt, copied to
`credits/licenses/Quaternius_Ultimate_Spaceships_License.txt`; import record
in `art/README.md`).

## New: `scripts/bake-atlas-to-kd.py` (stdlib-only, --self-test)

Closes the palette-atlas trap from the research spike (one grey `Kd`,
colours in a UV atlas -> the cutter emits colourless parts):

- Samples the atlas on a fixed 7-point barycentric lattice per triangle;
  per-face colour is the per-channel MEDIAN, so thin painted panel lines and
  decals do not tint the face's fill colour.
- Area-weighted k-means over the face colours (deterministic seeding:
  coarse histogram + greedy farthest-point; <12 sRGB-distance clusters
  merge) -> a whole ship lands on <=14 flat materials, the same shape as a
  born-flat pack (Fertile Soil).
- `Kd` is written LINEAR (sRGB-decoded) because the cutter copies `Kd`
  floats into glTF `baseColorFactor` verbatim; material names carry the
  sRGB hex (`kd_8a3c2f`) for human reading. PNG decode is stdlib zlib
  (non-interlaced 8-bit RGB/RGBA); ~2.3 s per 2048px atlas.
- Verified per run: output OBJ re-parsed (face count preserved), every
  palette entry worn, palette printed with face counts + area shares.

## Import + gallery (all 11 ships)

- `art/quaternius-ultimate-spaceships/baked/` - flat-Kd OBJ+MTL bakes of
  all 11 ships, ONE colour variant each (variant recorded in each .mtl
  header). The 260 MB source zip (5 variants x 2048px atlases, FBX, blends)
  stays out of the repo; re-baking needs it, re-CUTTING does not.
- `art/part-candidates/quaternius/<ship>/` - 9 ships converted whole
  (identity mode, scale 0.5, yaw 180: pack scale is ~2x game scale and the
  pack noses point +Z) for gallery judgement, 2 ships recipe-cut (below).
- Facing measured, not assumed: width-vs-z profile on every baked mesh
  (narrow end = nose) agrees across the pack; confirmed visually in the
  gallery captures.

## Recipes: striker (5 parts) + spitfire (8 parts)

Seam data came from two measurements on the baked meshes (scratch
analysis): a plane scan (straddling-triangle counts per candidate axis
plane, the round-4 method) and a shell-component pass (union-find over
shared vertex positions) - the component bboxes turned out to be the
better recipe source, e.g. they exposed spitfire's four SEPARATE engine-pod
shells (two stacked per side, 0.06 y-overlap band between hulls) which the
x-only plane scan misread as an asymmetric ship.

- `quaternius_striker.json`: nacelle_port/starboard claim the whole
  outboard assemblies (inner rail + nacelle + gun prongs + thruster stacks)
  at the natural x=+-0.44 seam (16/4530 tris straddle); tail at z=1.26
  (keeps the canopy whole); nose bulkhead at z=-0.50; rest=fuselage.
- `quaternius_spitfire.json`: four engine pods split at x=+-1.85 (through
  the pod-free gap, only the 8-tri wing sheet is clipped) and y=-0.28
  (inside the shells' overlap band); wings at x=+-0.52 (outside the +-0.51
  tail block); nose at z=-0.35 (keeps the under-nose skids whole);
  rest=fuselage.
- Cut gates: all 13 parts `glb OK`, 0 open cut edges everywhere; a few
  ear-clip fallback/open-chain notes are source-mesh holes (the pack is not
  watertight), not cut regressions.

## Viewer

- `parts_viewer` rank list gains `quaternius`; the assembled/exploded ship
  capture walk is now DYNAMIC over every recipe ship (index re-resolved by
  name at runtime) instead of hardcoding racer/cargob.
- Captures eyeballed (Xvfb): gallery pages show all 11 ships in real baked
  colours (grey/gold/green striker, red/black executioner, purple pancake
  saucer, ring-ship zenith...); striker and spitfire assemble seamlessly
  and explode into sensible semantic parts.

## Honest gaps

- One colour variant per ship imported; other variants are a re-bake away.
- The bake keeps painted PANEL DETAIL only as much as face topology allows
  (per-face flat colour): decal-heavy faces flatten to their dominant fill.
  That matches the game's flat-shaded look; it does not reproduce the
  textured preview renders.
- Striker/spitfire recipes are first-cut; owner tuning (like round 4's
  racer pass) still expected before any promotion.
- The other 9 ships stay monolithic until someone wants them as parts.
