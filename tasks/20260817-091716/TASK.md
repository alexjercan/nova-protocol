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
