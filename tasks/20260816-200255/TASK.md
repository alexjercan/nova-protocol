# Thruster shells: sizes, side link points, and a showcase

- STATUS: CLOSED
- PRIORITY: 61
- TAGS: v0.11.0,research,art,ship

## Goal

Research + design spike WITH a built showcase example, owner-requested,
sibling to the greeble spike (20260816-194637).

## The owner's brief

The default thruster model "is kind of simple". Wanted:

- New thruster models: "similar either to the ones from kenney or something
  custom made up (even similar to the other assets we have imported but not
  used yet - quaternius-ultimate-spaceships have some interesting designs)".
- "create an example that showcases them, similar to how the greeble research
  will do some design and showcase them in examples".
- Sizes: "we can even have big thrusters like 3x3x1 or 5x5x3 or x1 or x5".
- The core design idea, in the owner's words: "I should create just the SHELL
  of the thruster with different sizes and then let the style shell actually
  make it look good" - thrusters with LINK POINTS on the sides so the skin /
  style can dress them, "let the thrusters connect on sides too and leave
  only the exhaust to void".

## Deliverables

1. THRUSTERS.md in this folder:
   - audit: the current drive prototype (model, sockets, clearance) and why
     it reads simple
   - candidate looks: kenney + quaternius (art/quaternius-ultimate-spaceships)
     thruster/engine designs, with licence status
   - the size family: proposed grid of shells (1x1x1 up through 5x5x3), mass
     and thrust scaling stance
   - the side-link-point design and its CONSEQUENCES: wfc mating (a drive
     deliberately carries ONE socket today - shared/wfc.rs documents why),
     editor placement, the exhaust clearance lane, and whether the skin can
     clad thruster flanks so the shell idea works
   - follow-up task breakdown; the engine link-point change is a follow-up,
     not this lane
2. A showcase example: candidate thruster models and size variants in a named
   row, idle orbit, in the fleet capture idiom.

## Constraints

- Visual-only: no changes to section sockets, clearance, wfc or editor
  behavior in this lane.
- New meshes follow the primitives-first route where custom: JSON recipes via
  the generator scripts, not hand-authored binaries (see the greeble spike's
  sourcing plan).

## Closure

Landed as 38114fff (2026-08-16), lane thruster-shells. THRUSTERS.md plus the
thruster_gallery example (16 candidate looks, named, idle orbit, capture
idiom) and two renders in this folder.

Headline answers:
- THE SHELL IDEA WORKS. The skin is a pure function of link-point normals:
  stands()/walls() in shell_skin.rs read sockets, so flank-socketed drives
  get clad automatically with zero skin-code change, and the socket-free
  exhaust face keeps its lane bare via the tested exit_pocket behaviour.
- Priced-in consequences of flank sockets: mixed fitting banks become illegal
  in wfc unless bays get shells too; staggered drive banks become
  clearance-illegal; and fitting_distance walks only in-plane, so shroud
  plates need a one-line near_fitting repair.
- Size grid: 1x1x1, 2x2x1, 3x3x1, 5x5x1, 5x5x3, 1x1x5. Mass and thrust
  proportional to cell volume - the family is a geometry choice, not a power
  ladder.
- THE REAL PREREQUISITE is multi-cell sections: a section occupies one cell
  today, so every size above 1x1x1 waits on that (follow-up 3, size L).
- Licences: Quaternius/Kenney CC0 (per GREEBLES.md), Fertile Soil CC0 (per
  art/README.md), proposed shells are recipe-generated originals.

Follow-ups are enumerated in THRUSTERS.md section 6; opening them is the
owner's call. The flank-socket engine change is the decision point.
