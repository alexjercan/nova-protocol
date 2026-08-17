# Controller-less hulls spawn massless and NaN-poison physics

- STATUS: CLOSED
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

## Closure (2026-08-17)

Both families of the warning are closed, and neither was the bug this task was
filed for - controller-less hulls compose their mass like any other hull, and
zero mass is not NaN (avian reads inverse mass 0 as INFINITE mass; the contact
solve stays finite). NOTES.md carries the evidence.

- BIRTH: a rock's collider node came from a later observer, one command hop
  after its body, so a physics tick could land in between. Asteroids now build
  root and node in one batch (`asteroid_scenario_object` takes
  `EntityCommands`), with the silhouette seed resolved by the caller. Zero rock
  warnings after, was 2-3 every run.
- DEATH: a shot-down torpedo and a broken rock are marked dead one pass before
  their reaper runs, and by then their sections - every collider they had -
  are gone. Both now take `RigidBody::Static` in the same insert as the death
  marker. An arena fight that shot down 20 torpedoes logged zero warnings,
  where it logged 14-16 before.

Both fixes carry fail-first tests. The arena's `freeze_junk` pin stays as a
FRAMING choice and its comment no longer repeats the disproven NaN story.

Not taken, on purpose:

- the invariant range (a dynamic body massless for more than one tick is a
  defect). With both families closed the warning itself now carries that
  meaning; worth revisiting only if a third family shows up.
- one unfrozen arena run in five went bad (guns silent 45 s, both fleets
  erased within 0.1 s of the first torpedo salvo) with no NaN in the physics
  state. Not a mass problem. Left for whoever looks at the fight itself.
