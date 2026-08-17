# Investigation: where the "no mass or inertia" warning comes from

Sprout `mass-nan` off master 3cc49bc9. Nothing fixed yet; this is the root
cause and the evidence for it.

## The task's premise is wrong

A `SpaceshipController::None` hull composes its mass exactly like a crewed
one. Proven twice:

- headless, through the production spawn path
  (`crates/nova_scenario/tests/massless_hull_probe.rs`,
  `probe_controller_less_hull_mass`): controller-less and AI hulls of the same
  four unit-density cells both settle at `mass=4.0`, same inertia, same COM.
- live, in `wfc_arena` with `freeze_junk` disabled: over five runs, not one of
  the twenty controller-less fragments ever entered the massless set.

Mass composition never looks at the controller. It comes from the child
sections' `Collider` + `ColliderDensity` (`base_section` ->
`destructible_body`), which every hull has, crewed or not.

## What the warning actually reports

`warn_invalid_mass` (avian 0.7, `dynamics/rigid_body/mass_properties/mod.rs`)
fires for a dynamic body whose `ComputedMass` is not FINITE. `ComputedMass`
stores the INVERSE mass, and `mass 0 -> inverse 0`, which is bit-identical to
avian's `INFINITY` constant. So the warning means "this body has no mass yet",
not "this body is NaN". The "can cause NaN" half of the message is avian's
generic caution, not a diagnosis.

## The mechanism: a one-frame window between the body and its collider

avian computes a body's mass twice at spawn:

1. an observer on `Add<RigidBody>` computes it immediately - at which point NO
   collider is linked, so the result is zero;
2. an observer on `Insert<RigidBodyColliders>` recomputes it once the collider
   link exists.

The link itself (`ColliderOf`) is inserted by ANOTHER observer, through
`Commands` - so it is deferred. Any dynamic body that meets a physics tick
between (1) and (2) spends that tick at zero mass and gets the warning.

Live evidence (`wfc_arena`, instrumented per frame): every massless body found
in five runs was massless for EXACTLY one frame and carried `children=0` at
that moment - the collider child had not landed yet. Two families showed up:

- arena rocks: 2-3 of ~18 per run, always in the same blob, at spawn;
- torpedoes: none in some runs, a dozen in others, at launch.

Which bodies land in the window is timing-dependent, which is why it is a
handful and not all of them: the scenario spawn queue drains in 3 ms chunks
(`SPAWN_DRAIN_BUDGET`), so where a spawn sits relative to the frame's physics
tick moves from run to run. A headless minimal app never reproduces it - every
flush completes before the next tick (`probe_asteroid_mass_over_frames`,
`probe_drain_path_mass` both settle with mass on the spawning frame).

## Zero mass is not NaN

`probe_zero_mass_contact`: two overlapping dynamic spheres, both at zero mass,
stepped eight ticks. Positions and velocities stay finite; avian treats
inverse mass 0 as INFINITE mass, so the pair simply passes through each other.
No division blows up.

Live: a per-frame NaN sweep over every `ColliderAabb`, `Position`,
`LinearVelocity` and `AngularVelocity` in the arena found zero NaN in the runs
it was armed for.

So the cost of the window is one tick of "this body ignores contacts", not
poisoned physics.

## The arena junk workaround does not hold up

`freeze_junk` was justified by "twenty controller-less fragments NaN-poisoned
the spatial queries". With freezing disabled: 4 of 5 runs completed the
autopilot at t=24.1s, exactly like the frozen control, and their guns fought
normally (one run: 137 kinetic + 36 pierce rounds, a clean kill).

One run out of five went bad - guns near-silent for 45 s, then both fleets
erased within 0.1 s of the first torpedo salvo. That run predates the NaN
sweep, so it has no NaN evidence either way, and it is NOT explained by
massless junk (the junk was never massless). Treat that bout as an open
question about the fight itself (approach, engagement gate, or the torpedo
alpha strike), not about mass.

## The variant that IS a real defect

A dynamic body that never gets a collider, or LOSES its only collider, stays
massless permanently rather than for a frame. `asteroid.rs`
(`on_asteroid_node_destroyed`) already documents and handles one instance: an
asteroid whose collider/health node explodes leaves an invisible dynamic husk,
which `despawn_asteroid_husk` reaps. A ship root that outlives its last
section is the same shape.

## What landed (option A1, owner's pick)

`asteroid_scenario_object` now takes `EntityCommands` and builds the root AND
the collider/health node in ONE batch, with the mesh generated inline;
`insert_asteroid_collider` is gone. The seed resolves at the call site
(authored wins, else `asteroid_seed_from_id`), because a command has no RNG -
so an unseeded rock is now stable per id instead of a fresh draw per spawn.

Verified live: two unfrozen arena runs after the change, ZERO rock warnings
(was 2-3 every run). Both runs completed the autopilot normally.

STILL WARNING: torpedoes, 14-16 per run, one frame each at launch, `children=0`
at that moment. They already spawn their collider children in the same
`children!` batch, so the remaining gap is the spawn-to-child-effect hop taken
inside the fixed-step loop, where the same step's physics prepare falls in the
window. Option A does not reach that; it needs either the collider on the
projectile root itself or an authored mass, and it is harmless per the NaN
evidence above.

## Fix options, for the discussion

- A. Close the window at the source: have the asteroid spawn its collider
  child in the SAME command batch as the root (move the mesh + collider build
  out of the `Add` observer into the spawn action), or hold `RigidBody` back
  until the observer inserts it alongside the collider. Kills the rock
  warnings outright. The torpedo case needs the same treatment for its
  `children!` batch.
- B. Author an explicit mass on the roots. Cheap, but in avian an explicit
  `Mass` OVERRIDES the composed collider mass, which breaks the ships'
  live COM and the "mass follows a destroyed section" invariant. Only viable
  for bodies whose mass is not composed.
- C. Accept the window and pin the invariant instead: a range that fails if any
  dynamic body is massless for MORE than one tick. That catches the permanent
  variant - the one that actually costs something - and leaves the benign
  one-frame case alone.
