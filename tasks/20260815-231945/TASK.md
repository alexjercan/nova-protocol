# Market research: open-source prior art, technique, and licence positions

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, research, art

## Goal

A durable, browsable record of what already exists in the world that Nova
Protocol can REUSE or LEARN FROM. Reference material, not scheduled work.

Priority is OPEN SOURCE: other space games' code, art, data formats and design
decisions, plus the licence position on each. Second priority is reference
material ABOUT games - reviews, postmortems, dev blogs, GDC talks - where the
payload is design reasoning, not scores.

Immediate motivation: `20260815-225748` (ship skin styles) builds a
decoration/greeble system with moddable styles and a standard-library Python
art generator. Research that informs it is the most valuable.

## Contents

- `RESEARCH.md` - the findings. Fourteen sections plus a ranked recommendation
  section at the end; read that first.
- `PRIOR-POINT-DEFENCE.md` - a point-defence-versus-missile balance survey done
  by a SEPARATE lane, banked here rather than lost, credited to that lane, with
  my own Nova-specific reading kept separate from theirs. None of its figures
  could be re-verified from here; the file says so.

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
