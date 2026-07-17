# Review: cut Kenney .obj into grid-aligned modular hull cubes

- TASK: 20260717-221101
- BRANCH: work/obj-cutter

## Round 1

- VERDICT: APPROVE

Reviewed by an independent out-of-context agent against the OBJ and the glTF
2.0 spec, plus a mid-review design pivot (see note below).

Findings on the first implementation (centroid bucketing):

- [x] R1.1 (MINOR) `write_glb` computed `buffers[0].byteLength` before the final
  BIN 4-byte padding loop. Benign today (the index buffer is always the last,
  4-aligned view) but fragile. FIXED: padding now runs before the gltf dict is
  built, and `self_test` asserts `byteLength == BIN chunk length`.
  - Response: fixed.
- [x] R1.2 (MINOR) `cell_of` docstring said "round" but implemented half-up,
  which is not Python `round` (banker's). ADDRESSED: rounding is now explicit
  `_round_cell` with ties-toward-zero and a docstring to match.
  - Response: fixed (and the rounding rule changed, see R1.4).
- [ ] R1.3 (NIT) `parse_mtl`/`parse_obj` index tokens without length guards.
  Left as-is: Kenney files are well-formed and a malformed line failing loudly
  is acceptable for an art-tooling script.
  - Response: wontfix (noise for this input).

Design pivot (user direction, mid-review): the cut method changed from
face-centroid bucketing (whole triangles, pieces bulge past the cube) to true
**grid clipping** - each triangle is sliced at the cube-boundary planes so every
fragment is strictly inside one cube ("cut the floor tile to fit"). This makes
the pieces reusable as a generic parts library, matching the mod's intent. The
split mirrors `bevy-common-systems` `triangle_slice` (signed-distance vertex
classification, lonely-vertex split, clamped edge-plane intersection); cut faces
are left open on purpose (the hollow backing is the game's default-hull scaffold
cube, not a generated cap).

New findings from the pivot, independently re-verified:

- [x] R1.4 (MAJOR) Flat port/starboard walls lie exactly on the `x=+-1.5` cube
  boundary. Half-up rounding sent the +x wall to a phantom empty outer cell
  (`i=2`) while leaving the -x wall in the real cell - asymmetric, invented
  shell cells. FIXED with ties-toward-zero rounding (`_round_cell`); phantom
  `i=+-2` cells gone (40 -> 38 cells), verified against the real cut.
  - Response: fixed.

Re-verified load-bearing claims (not from reading the diff alone):
- Area conservation: sum of fragment areas == original scaled area to 1e-6
  (51.425131 == 51.425131) - clipping is a true partition, no area lost/dupd.
- Partition: `self_test` asserts no fragment crosses the `x=0.5` plane after
  slicing, and a straddling triangle lands in both cells 0 and 1.
- glTF validity: all 38 emitted `.glb` parse - correct magics, `total`==file
  length, JSON chunk 4-aligned, `buffers[0].byteLength`==BIN chunk length,
  `scene==0`, POSITION accessors carry min/max.

Open NIT R1.3 is left to implementer discretion; no BLOCKER/MAJOR remains.
