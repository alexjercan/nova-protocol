# Market research: hull plating shape and greeble placement prior art

- STATUS: IN_PROGRESS
- PRIORITY: 67
- TAGS: v0.11.0,research,art,ship,skin

## Goal

Scoped second round of market research, aimed at ONE question so the shell-shape
work does not re-derive technique from scratch. Owner: "start the market research
again and see if we find anything related to this issue, such that we do not
waste time with the research part".

## The questions

1. **Plate top geometry.** How do other systems give procedural hull plating a
   FLAT usable top while still matching neighbours exactly? Inset plateau with a
   chamfer, bevelled tile edges, trim-sheet insets, something else. What breaks.

2. **Continuity over spikes.** How is a procedural surface biased toward reading
   as continuous plating rather than as scattered studs and cones? What rules
   suppress isolated protrusions without flattening deliberate ones.

3. **Greeble placement.** What do systems actually key scatter on - available
   flat AREA, contiguous run, surface normal, enclosure, curvature? Which
   decorations are worth placing on edges, corners and high points, and how are
   those made to look intentional rather than stuck on.

4. **Saddle avoidance.** Where a height field is sampled on corners and edge
   midpoints, how is the alternating-corner saddle avoided or made to read well.

## Prior art worth checking

Townscaper (the drive-layer / generated-layer split and its prop scatter),
Hardspace: Shipbreaker (panel lines that ARE module boundaries), Lagae and Dutre
2006 corner tiles with Barrett's mid-edge fix, Nebulous: Fleet Command,
Children of a Dead Earth, marching-cubes and dual-contouring literature on flat
region preservation, and any open-source greebling or kitbash tooling.

The first round's findings are banked in `tasks/20260815-190741/NOTES.md` and
`tasks/20260815-231945/`. Do not re-run them.

## Constraint

Open source and game sources - reviews, images, documents, code - as the owner
scoped the first round. Record licence positions for anything reusable.

## Definition of done

Findings that let the shell-shape task START from a technique rather than invent
one, with the counter-evidence recorded too.
