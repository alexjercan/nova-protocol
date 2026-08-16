# Thruster shells: sizes, side link points, and a showcase

- STATUS: IN_PROGRESS
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
