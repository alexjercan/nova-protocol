# Retro: cut Kenney .obj into modular hull cube tiles

- TASK: 20260717-221101
- LANDED: f80e0b25 (squash)

## What went well

- The stdlib-only glTF-binary writer worked first try and passed independent
  spec review (magics, alignment, accessors, min/max, byteLength). Hand-rolling
  it (vs pulling in trimesh) kept the script dependency-free per repo convention.
- Grounding the slicer on `bevy-common-systems` `triangle_slice` meant the cut
  logic matched an already-proven algorithm instead of being reinvented.
- Area-conservation as the loss-free check gives a cheap, exact invariant that
  will guard future changes.

## What went wrong

- I built the wrong cut method first (centroid bucketing, per the spike's A1
  recommendation) and the user corrected it mid-review to clipping (A2). The
  spike had named A2 and rejected it for a reason (hollow shell) that turned out
  not to matter, because the game backs tiles with a default-hull scaffold cube -
  context the spike did not have. Cost: one rework.
- Clipping surfaced a real bug the centroid version hid: flat walls lying exactly
  on a cube boundary + half-up rounding created phantom empty outer cells,
  asymmetrically (one side only). Fixed with ties-toward-zero rounding.

## What to improve next time

- When a spike rejects an option for a downside, check whether an existing game
  system already neutralizes that downside before treating the rejection as
  final. Ask the user about such mitigations at spike time.
- Any grid-bucketing of a mesh with axis-aligned faces will have geometry exactly
  on boundaries; pick a symmetric tie-break (toward zero) from the start.
