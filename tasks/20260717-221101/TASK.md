# Build cut-obj-into-hulls.py: bucket Kenney .obj into per-cell .glb pieces

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: spike, backlog, tooling, modding

## Goal

Build the stdlib-only Python cutter `scripts/cut-obj-into-hulls.py`. Parse
`art/kenney/craft_cargoB.obj` + `.mtl`, scale about the origin (default `2.0`,
mapping the 0.5 half-grid onto the 1.0 game grid), CLIP each triangle at the
cube-boundary planes so every fragment is strictly inside one cube, bucket the
fragments into integer cells `(i,j,k)`, recentre each cell to its own origin,
and write one `gltf/hull_i{i}_j{j}_k{k}.glb` per non-empty cell via a hand-rolled
glTF 2.0 binary writer (positions/normals/indices + base-color material from the
`.mtl` `Kd`). Reconstruction is loss-free: clipping partitions the surface, so
fragment area == original area and placing every cell back reproduces the ship.

## Steps

- [x] OBJ parser: read `v` and `f` (handle `v`, `v/vt`, `v//vn`, `v/vt/vn`), track `usemtl` per face; MTL parser for `Kd` per material.
- [x] Geometry transform: scale about origin by `--scale` (default 2.0), re-anchor grid by `--center`.
- [x] Clip each triangle at the cube-boundary planes so every fragment is strictly inside one cube (`--cell` default 1.0), then bucket fragments by cell (ties-toward-zero rounding). Changed from centroid bucketing to grid clipping per user direction - see REVIEW.md R1.4.
- [x] Recentre each cell's fragments to the cell centre (piece local origin = cell centre).
- [x] Per-fragment flat normals.
- [x] glTF 2.0 binary (`.glb`) writer: JSON chunk + BIN chunk, POSITION/NORMAL/indices accessors, one base-color material per `Kd` (stdlib only).
- [x] CLI via argparse: input obj, `--out` dir, `--scale`, `--cell`, `--center`.
- [x] Emit one `gltf/hull_i{i}_j{j}_k{k}.glb` per non-empty cell; print a cell manifest (cell -> fragment count, dominant material by area).
- [x] Loss-free check: fragment area == original area; `--self-test` covers slicing partition + glb structure.

## Notes

- Spike: tasks/20260717-220919/SPIKE.md (B1 stdlib glb; cut method changed A1->A2 clipping)
- Slicing mirrors `~/personal/bevy-common-systems` `src/mesh/builder.rs` `triangle_slice`.
- craft_cargoB.obj -> 38 cells, 1272 fragments, area 51.425131 conserved exactly.
