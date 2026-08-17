# Controller-less hulls spawn massless and NaN-poison physics

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.11.0,bug,physics,ship

## The bug

A hull spawned with `SpaceshipController::None` becomes a DYNAMIC rigid
body with no mass or inertia. avian3d warns exactly that ("can cause NaN")
and it did: in the arena-polish lane, twenty controller-less wreckage
fragments NaN-poisoned the spatial queries combat aims through - gun output
fell from ~700 rounds to 0-177 per bout until the lane pinned its junk
`RigidBody::Static` as a workaround (landed in 71f519cc). The pre-existing
`mass: None` warning noise from scenery rocks is the same family.

Sections author `mass: 1.0` each, so the composed body should weigh
plenty - the mass composition path is skipping these hulls for a reason
nobody has found yet.

## What it needs

- root-cause where root mass/inertia composition happens and why a
  controller-less spawn misses it
- the invariant, enforced: EVERY dynamic hull carries its composed mass,
  controller or not (or such hulls are made static by explicit policy, not
  accident)
- per the bug-to-range doctrine (CONVENTIONS.md): a reproducible check
  first - a lib test or systems range that spawns a controller-less dynamic
  hull and asserts nonzero mass/inertia and NaN-free queries - then the fix
  turns it green
- revisit the arena's freeze_junk workaround once fixed (it can stay for
  determinism, but it should be a choice, not a shield)

## Done when

- the repro exists and is green post-fix; the avian massless warning is
  gone from arena runs; workaround status decided and documented

## Progress (2026-08-17)

Investigated; see NOTES.md for the evidence. The premise above is WRONG - a
controller-less hull composes its mass like any other, proven headless and
across five instrumented arena runs, and zero mass is not NaN (avian reads
inverse mass 0 as INFINITE mass and the contact solve stays finite). The
warning marks the window between `Add<RigidBody>`, where avian computes mass
with no collider linked, and the deferred `ColliderOf` link.

Landed (option A1): asteroids build their collider node in the SAME command
batch as the body, so the window closes for rocks - zero rock warnings in two
unfrozen arena runs, was 2-3 every run. The arena's `freeze_junk` keeps its
pin as a FRAMING choice and its comment no longer carries the disproven NaN
story.

Left open:

- torpedoes still take the window at launch (14-16 one-frame warnings per
  arena run). They already batch their collider children, so closing it needs
  the collider on the projectile root or an authored mass.
- the PERMANENT variant is the one worth an invariant: a dynamic body that
  never gets a collider or loses its last one (the asteroid husk, already
  reaped; a ship root outliving its sections). Owner has not decided whether
  to pin it.
- one unfrozen arena run in five went bad (guns silent 45 s, both fleets
  erased within 0.1 s of the first torpedo salvo) with no NaN in the physics
  state. Not a mass problem; worth its own look at the fight itself.
