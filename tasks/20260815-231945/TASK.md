# Market research: open-source prior art, technique, and licence positions

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,research,art

## Goal

A durable, browsable record of what already exists in the world that Nova
Protocol can REUSE or LEARN FROM. **The single point for market research** - a
new question gets a ROUND here rather than a task of its own, so the findings
accumulate in one place instead of scattering.

The record is reference material. Individual rounds are scheduled work.

Priority is OPEN SOURCE: other space games' code, art, data formats and design
decisions, plus the licence position on each. Second priority is reference
material ABOUT games - reviews, postmortems, dev blogs, GDC talks - where the
payload is design reasoning, not scores.

Immediate motivation: `20260815-225748` (ship skin styles) builds a
decoration/greeble system with moddable styles and a standard-library Python
art generator. Research that informs it is the most valuable.

## Contents

- `RESEARCH.md` - round 1. Fourteen sections plus a ranked recommendation
  section at the end; read that first.
- `PRIOR-POINT-DEFENCE.md` - a point-defence-versus-missile balance survey done
  by a SEPARATE lane, banked here rather than lost, credited to that lane, with
  my own Nova-specific reading kept separate from theirs. None of its figures
  could be re-verified from here; the file says so.
- `PLATING-AND-GREEBLES.md` - round 2, below.

## Round 2: hull plating shape and greeble placement

Scoped so the shell-shape work starts from a technique instead of inventing one.
Owner: "start the market research again and see if we find anything related to
this issue, such that we do not waste time with the research part".

Feeds task `20260816-112429` (shell shape and decoration placement), which holds
the full code reading. In short: every plate top is an eight-triangle fan off ONE
centre vertex at the mean of its eight boundary samples, so no plate has flat
area unless all eight agree; corners vote three ways only, so an alternating
corner pattern is a forced saddle; and the boundary polyline is the whole
contract with the neighbours, so the plate INTERIOR is free.

The questions:

1. **Plate top geometry.** How do other systems give procedural plating a flat,
   usable top while still matching neighbours exactly - inset plateau with a
   chamfer, bevelled tile edges, trim-sheet insets, something else? What breaks
   when they do it.
2. **Continuity over spikes.** How is a procedural surface biased to read as
   continuous plating rather than scattered studs and cones? What suppresses
   isolated protrusions WITHOUT flattening deliberate ones. Owner: "it should
   prefer having continous skin rather than spikes all over the place", while
   still liking ridges and spikes where they are meant.
3. **Greeble placement.** What do real systems key scatter on - flat AREA,
   contiguous run, normal, enclosure, curvature? Which decorations are worth
   placing on edges, corners and high points, and how are those made to look
   intentional rather than stuck on.
4. **Saddle avoidance.** Where a height field is sampled at corners and edge
   midpoints, how is the alternating-corner saddle avoided or made to read well.

Prior art to check: Townscaper's driver-layer / generated-layer split, Hardspace:
Shipbreaker, Lagae and Dutre 2006 corner tiles with Barrett's mid-edge fix,
Nebulous: Fleet Command, Children of a Dead Earth, marching-cubes and
dual-contouring work on flat-region preservation and sharp-feature
reconstruction, and open-source greebling or kitbash tooling.

A technique that needs a global relaxation pass or per-instance randomness is
OUT: Nova's derivation is deterministic and matches neighbours through shared
samples.

## Lane

Round 2 runs in sprout `shape-research`.

## Headline findings

- Author link points in Blender as glTF extras rather than typed coordinates.
  Naev and Pioneer both do it; `GltfExtras` is already a Bevy component the
  loader inserts on spawned entities. The data cannot desync from the mesh.
- Blue noise is the WRONG scatter for machined hardware. Grid-occupancy claiming
  keeps the alignment that makes greebles read as bolted-on.
- Greeble the seams. ILM's original functional reason, and the distances are
  already computed by the skin derivation.
- WebGL2 has no `BASE_VERTEX`, so distinct meshes can never share a batch set.
  Merging the generated skin is architectural, not an optimisation.
- Of eight open-source space games, only Naev offers any permissively-licensed
  3D ship art, and only a ~22-model slice of it.

## Cross-references it would be easy to miss

- Corrects the "never vertex colours" ruling on `20260815-190741`.
- Supports the generate-the-art decision on `20260815-225748`, and supplies the
  plate-sizing and scatter algorithms that task's Phase A needs.
- Reports one defect found in passing: `crates/nova_hud/src/target_inset.rs`
  sets both `emissive` and `unlit: true`, and the unlit branch never adds
  emissive. Not fixed here.

## Rules this record follows

- Every source carries its EXACT licence and attribution requirement.
- Share-alike (GPL, CC-BY-SA) is flagged loudly. Nova is MIT; share-alike art
  and GPL code are listed as UNUSABLE, not quietly borrowed.
- Commercial game screenshots, review text and marketing images are
  copyrighted. Nothing of that kind is committed - links and analysis only.
- Nothing is committed under `art/` unless its licence unambiguously permits
  redistribution and its attribution is recorded beside it.

## Not in scope

- Decoration continuity across tiles. `20260815-190741` NOTES.md already banks
  the corner-tile / Townscaper / Hardspace findings. This record complements
  them and does not repeat them.
