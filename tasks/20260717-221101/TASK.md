# Build cut-obj-into-hulls.py: bucket Kenney .obj into per-cell .glb pieces

- STATUS: OPEN
- PRIORITY: 0
- TAGS: spike,backlog,tooling,modding

## Goal

Build the stdlib-only Python cutter `scripts/cut-obj-into-hulls.py`. Parse
`art/kenney/craft_cargoB.obj` + `.mtl`, scale about the mesh centre (default
`2.0`, mapping the 0.5 half-grid onto the 1.0 game grid), bucket each triangle
by centroid into integer cells `(i,j,k)`, recentre each cell to its own origin,
and write one `gltf/hull_i{i}_j{j}_k{k}.glb` per non-empty cell via a hand-rolled
glTF 2.0 binary writer (positions/normals/indices + base-color material from the
`.mtl` `Kd`). Reconstruction must be loss-free: no triangle split or dropped, so
placing every cell back at its grid position reproduces the original ship.

## Notes

- Spike: tasks/20260717-220919/SPIKE.md (approach A1 bucketing + B1 stdlib glb)
