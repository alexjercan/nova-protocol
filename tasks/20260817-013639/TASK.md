# Thruster art candidates in the mechanical style

- STATUS: CLOSED
- PRIORITY: 49
- TAGS: v0.11.0,art,ship

## Goal

Owner-approved, deliberately relaxed: ART ONLY thruster candidates. "a few
candidate models and them working in the [gallery] example is good progress".

- A few (3-5) recipe-generated thruster shell candidates in a MECHANICAL /
  engineering look that fits ALL factions (like PDCs share one style) - the
  owner's stated direction. Square shell variants (cladding-friendly) and at
  least one cylinder bell. 1x1 scale only - multi-cell stays parked in
  THRUSTERS.md.
- Shown in the existing thruster_gallery example (it has named rows and the
  idle orbit); the owner judges from the render or the raw glb files.
- No content prototypes, no wfc changes, no engine changes.

## Done when

- candidates render in thruster_gallery with names; a capture lands in this
  folder; recipes are deterministic under the generator's --check idiom

## Closure

Landed 2026-08-17, lane thruster-art. Owner feedback folded in mid-lane:
bell + vector kept, the three rejected candidates replaced by gimbal, twin
and paddle in the same anatomy (plate, drum, cone, heat ring, dark throat),
plus two large formats - shell_bank (3x3x1 nine-bell lattice) and
shell_capital (5x5x3 vectoring drive). Judging render in this folder.

Generator: sibling gen-thruster-shells.py importing gen-greebles' primitive
and verify layer (one vocabulary, two frames); cell-frame budgets, exhaust
check, 450 tris/cell, --check byte-compare. Candidates under
art/part-candidates/shells/, art only.

Noted, unfixed: GREEBLES.md claims the generator --check "fails CI on a
stale commit" but ci.yaml runs no python step - both generators could be
gated with one CI line; owner's call.
