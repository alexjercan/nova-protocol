# Spike: cut monolithic Kenney .obj into grid-aligned modular hull cubes

- DATE: 20260717-220919
- STATUS: RECOMMENDED
- TAGS: spike, tooling, modding

## Question

We have a full-ship Kenney model, `art/kenney/craft_cargoB.obj` (+ `.mtl`), that
is one solid mesh of an entire cargo spaceship. Our game builds ships from
modular 1x1x1 grid **sections** (Hull / Thruster / Controller / Turret /
Torpedo) that snap together on a unit grid. The uncertainty: **how do we cut
this monolithic mesh into grid-aligned "cube" pieces, and emit them as a loadable
mod, so the game can reconstruct the Kenney ship out of our own section
primitives?** A good answer is a concrete, runnable Python pipeline
(`obj -> per-cell meshes -> a mod bundle`) plus the exact grid, scale, and output
format decisions it must make, grounded in what the game actually loads.

## Context

Two facts from the code pin the whole design (see the exploration findings baked
in below):

1. **The game grid unit is 1.0 world unit.** Sections are unit cubes centred on
   their grid position; two sections are glued when their centres are exactly
   `1.0` apart on an axis (`crates/nova_gameplay/src/integrity/glue.rs:95`). Ship
   assembly places each section at an integer `Vec3` position with a `source`
   (`SectionSource::Prototype("<id>")`) - see `examples/03_hull_section.rs:68`.

2. **The engine loads glTF binary (`.glb`), not `.obj`.** Every part mesh in
   `assets/base/gltf/*.glb` is referenced as `"self://gltf/hull-01.glb#Scene0"`.
   Raw/scheme-less asset paths are rejected by the content lint. So the cutter
   must emit `.glb`, and a mod bundle of the standard shape:

   ```
   mods/<id>/
     <id>.bundle.ron      # manifest: content + resources + meta
     <id>.content.ron     # a [Content] list of Section((base, kind)) items
     gltf/<piece>.glb     # one mesh per cut cell
   ```

   A `Section` is `Section(( base: (id, name, description, mass, health, ...),
   kind: Hull((render_mesh: Some("self://gltf/<piece>.glb#Scene0"))) ))`.
   Thruster/Controller kinds carry extra stat fields
   (`crates/nova_gameplay/src/sections/*_section.rs`).

**The mesh geometry.** `craft_cargoB.obj`: 192 verts, 380 tris, one group, four
materials (`metal`, `metalDark`, `dark`, `metalRed`). Bounding box:

| axis | range          | span | cells @ 0.5 |
|------|----------------|------|-------------|
| x    | [-0.75, +0.75] | 1.5  | 3           |
| y    | [ 0.00, +0.90] | 0.9  | ~2          |
| z    | [-1.25, +1.25] | 2.5  | 5           |

The vertices cluster on a clean **0.5-unit half-grid** in x/z (peaks at
0, +-0.3, +-0.5, +-0.7). It is NOT a pure voxel model - the cockpit and engine
cowls have beveled/angled faces - but it is grid-aligned enough that cutting on
0.5-unit planes lands on natural seams. `metalRed` is the engine/thruster accent
material and marks the rear cells.

## Options considered

Two independent decisions: **(A) how to partition the geometry into cells**, and
**(B) how to produce the `.glb` files**.

### A. Partitioning the mesh into grid cells

- **A1. Face-centroid bucketing (no clipping).** Scale the ship so its 0.5 grid
  becomes the game's 1.0 grid (x2), then assign each whole triangle to the cell
  containing its centroid. Re-centre each cell's geometry to the cell origin and
  emit it as one mesh. **Pros:** trivial, robust, no new geometry, preserves
  Kenney's exact shapes and materials; crucially, since no triangle is split or
  dropped, re-placing every cell at its grid position **reproduces the original
  mesh exactly** - the pieces snap back perfectly because the seams are the
  original edges. **Cons:** a triangle straddling a boundary sticks out slightly
  past its nominal cube; a piece is not strictly bounded by its 1x1x1 box. For
  visual reconstruction that is invisible (neighbours overlap where they always
  did); only matters if something relies on strict per-cube AABBs.

- **A2. True plane-clipping (Sutherland-Hodgman per cell).** Clip every triangle
  against the 6 planes of each cell, re-triangulating the polygon fragments.
  **Pros:** each piece is exactly bounded by its cube; clean, watertight-ish cube
  faces. **Cons:** materially more code; introduces T-junctions / new verts on
  cut faces; open cross-sections where the hull interior was never modelled (the
  ship is a shell, so clipped cells show hollow gaps). Higher risk, worse-looking
  seams than A1 for this shell mesh.

- **A3. Voxelize + re-mesh.** Rasterize to a 0.5 voxel grid, emit a cuboid per
  filled voxel. **Pros:** dead-simple uniform cubes, trivially grid-bounded.
  **Cons:** throws away all of Kenney's silhouette (cockpit, cowls, fins) - the
  reconstructed ship is a blocky Minecraft approximation, not the Kenney ship.
  Defeats the point of using a nice source model.

### B. Producing the `.glb` output

- **B1. Pure-stdlib minimal glb writer.** glTF 2.0 binary is a small, fully
  specified container: one JSON chunk + one BIN chunk, positions/normals/indices
  as accessors, `KHR_materials` base-color from the `.mtl` `Kd`. Writable in
  ~150 lines of stdlib `struct`/`json`. **Pros:** matches the repo convention -
  both existing scripts (`scripts/gen-placeholder-sounds.py`,
  `scripts/gen-web-screenshots.py`) are deliberately **stdlib-only** (they even
  hand-roll a WAV and a PNG codec); no new dependency; deterministic bytes.
  **Cons:** we write and own the glb encoder (bounded, one-time cost).

- **B2. Depend on `trimesh` / `pygltflib`.** Load obj+mtl and export glb in a few
  calls. **Pros:** least code. **Cons:** breaks the established stdlib-only
  convention; adds a heavy dep (numpy etc.) to an art-tooling script for a task
  the repo's own precedent says to hand-roll; non-deterministic across versions.

- **B3. Headless Blender.** `blender --background --python cut.py`. **Pros:**
  industrial-strength geometry ops. **Cons:** enormous external dependency, not
  in the toolchain, overkill; CI/agents would need Blender installed.

## Recommendation

**A1 (face-centroid bucketing) + B1 (stdlib glb writer).**

> UPDATE (during implementation, task 20260717-221101): the user chose **A2
> (grid clipping)** over A1 - each triangle is sliced at the cube-boundary
> planes so every piece is strictly self-contained in its 1x1x1 slot, making the
> cut pieces reusable as a generic parts library (not just this ship). The
> "hollow shell" downside of A2 is handled not by capping cut faces but by
> backing each tile with the game's existing default-hull scaffold cube. The
> slicing mirrors `bevy-common-systems` `src/mesh/builder.rs` `triangle_slice`.
> B1 (stdlib glb writer) stands. Loss-free is now area-conservation, not
> triangle-count.

Rationale:

- A1 is the only option that gives **exact, loss-free reconstruction** of the
  Kenney ship from the pieces - the whole point of the task - while keeping the
  cutter simple and the source silhouette intact. A2's strict cubes buy us
  nothing here (the game glues by centre distance, not by face contact) and cost
  us hollow clipped faces on a shell mesh. A3 discards the art.
- B1 matches the repo's own strong precedent (stdlib-only asset scripts) and the
  global guideline to choose the correct, maintainable design over the
  quickest-to-type one. glb-writing is well-specified and bounded; owning ~150
  lines beats importing numpy into art tooling.

**Concrete pipeline** (`scripts/cut-obj-into-hulls.py`, argparse, stdlib only):

1. Parse obj verts + faces + per-face material; parse mtl `Kd` colours.
2. Scale by a `--scale` factor (default `2.0`, mapping the 0.5 half-grid to the
   1.0 game grid) about the mesh centre; `--cell 1.0` grid size configurable.
3. Bucket each triangle by centroid into integer cell `(i,j,k)`; recentre each
   cell's verts to the cell centre so each piece's local origin is its own
   centre (so the game can place it at the grid position unshifted).
4. Compute flat normals; write one `gltf/hull_i{ i }_j{ j }_k{ k }.glb` per
   non-empty cell (base-color material from the dominant `Kd`).
5. **Classify** each cell into a section kind (heuristic, `--classify`):
   - cells containing `metalRed` faces at the extreme -z (rear) -> **Thruster**
   - the single most central occupied cell -> **Controller**
   - everything else -> **Hull**
   Emit the matching `kind: Hull/Thruster/Controller` with sensible base stats.
6. Emit `<id>.content.ron` (the Section list) and `<id>.bundle.ron` (manifest
   listing every glb in `resources` + `meta`).
7. Also emit a ready-to-run **assembly**: the list of
   `SpaceshipSectionConfig { position: Vec3(i,j,k), source: Prototype("<id>") }`
   so a scenario / example can spawn the reconstructed ship and prove the pieces
   fit. Wiring this into a scenario or example is the acceptance test.

Validate the emitted mod with the existing gate:
`cargo run -p nova_assets --bin content -- lint --target <mod-path>`.

## Open questions

- **Front/back and axis orientation.** Which world axis is the game's "forward",
  and is Kenney +z the nose or the tail? Needs a 30-second look in the editor /
  an example once a first cut is placed; resolved empirically, not blocking.
- **Thruster/Controller heuristic quality.** The material+position heuristic is a
  first cut; the author may want to hand-tag which cells are thrusters vs hull.
  Cheap follow-up: a `--overrides cell=kind` flag.
- **Cell count vs playability.** 3x2x5 is up to 30 cells but most are empty; the
  real occupied count (likely ~12-18) sets how many sections the reconstructed
  ship has. Confirm it is a sane ship size after the first run.
- **Do clipped/strict cubes ever matter?** Only if a future feature needs strict
  per-section AABBs (e.g. collision per cube). If so, revisit A2 then; not now.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps:

- tatr 20260717-221101: build the `cut-obj-into-hulls.py` cutter (A1 bucketing +
  B1 stdlib glb writer) that emits per-cell `.glb` pieces from `craft_cargoB.obj`
- tatr 20260717-221106: emit the mod scaffold (`.bundle.ron` + `.content.ron`
  with Hull/Thruster/Controller classification) and a reconstruction assembly
- tatr 20260717-221110: wire a demo scenario/example that spawns the
  reconstructed Kenney ship from the cut sections, and lint the mod as acceptance

## Fix record

(Appended by each implementing task as it lands.)
