# RCS core primitive - design / fix record

Task: 20260718-122906. Spike: tasks/20260718-122508/SPIKE.md.

## What shipped

The base RCS fine-adjustment mechanic as a shared translational primitive, with
no player input and no HUD (those are tasks 20260718-122912 / -122923).

- `FlightVerb::Rcs` (crates/nova_gameplay/src/sections/controller_section.rs):
  a new computer-provided capability, threaded through the existing verb model
  for free. `WithheldVerbs::granted`, `SectionModification::DisableVerb` and the
  `SetControllerVerb` scenario action all take a `FlightVerb` by value, and the
  enum derives serde, so RON scenarios name `Rcs` with no parse table. A
  workspace build confirmed no non-exhaustive `match FlightVerb` sites broke.
- `RcsIntent(Vec3)` and `RcsSpeedCap(f32)` components on the ship root, plus
  `FlightSettings::rcs_speed_cap` (default 2.0 u/s) and `rcs_accel` (1.5 u/s^2)
  (crates/nova_gameplay/src/flight.rs). `RcsIntent` is a ship-LOCAL desired
  direction; a default one is inserted next to `FlightIntent` on player ships.
- `rcs_burn_system` (FixedUpdate, in `NovaFlightSystems` next to
  `manual_burn_system`): the primitive. Registered the two new reflected types.

## The cap formula (the heart of it)

Generalizes `manual_burn_system`'s speed-cap taper to three signed ship-local
axes. For each axis `a` in `[X, Y, Z]` with command `cmd = intent.dot(a)`:

    world_axis = rotation * a
    along      = velocity.dot(world_axis)
    gate       = clamp((cap - sign(cmd) * along) / (cap * 0.2), 0, 1)
    impulse   += world_axis * (sign_and_mag(cmd) * rcs_accel * gate * dt * mass)

- `gate` is the whole trick: a push in the direction the hull already moves at
  the cap has `sign(cmd)*along = cap` -> gate 0 (does nothing), while the
  opposite direction has `sign(cmd)*along = -cap` -> gate saturates at 1 (full
  push). So velocity can only ever be reshuffled within `+/-cap` per axis, never
  accumulated past it. Exactly the user's "moving 5 u/s forward, RCS forward
  does nothing but backward works to -5" rule.
- The summed impulse is applied ONCE with `Forces::apply_linear_impulse`, which
  acts at the center of mass and generates NO torque (avian3d-0.7
  query_data.rs:388), so RCS never rotates the hull and needs no physical
  side/vertical thrusters - the `Rcs` verb is the fiction that the computer has
  cold-gas quads (spike fork 2A).
- `* dt * mass`: `apply_linear_impulse` divides by mass, so scaling the impulse
  by mass makes `rcs_accel` a true acceleration and the feel mass-independent -
  a heavy hull and a light one both close a docking gap at the same rate.
- Residual velocity persists (pure Newtonian, no auto-null) - spike fork Q2.
- `RcsSpeedCap` exists as a per-hull override but is deliberately NOT
  scenario-authorable yet (unlike `FlightSpeedCap`, which has spawn-config and
  runtime-action wiring): the global `FlightSettings::rcs_speed_cap` default is
  intentional for the base primitive. Wiring per-hull authoring is a clean
  follow-up if a scenario ever needs a distinct RCS ceiling (review R1.2).
- Intent magnitude sets the ACCELERATION, not the terminal speed - any held
  deflection asymptotes to the full `cap`. Whether a partial mouse deflection
  should instead target a lower speed (scale the cap by `|cmd|`) is a feel
  decision left to the player-input task 20260718-122912 (review R1.1).
- Gated on the ship granting `Rcs` (a controller with a live PD not withholding
  it, mirroring `ship_grants_verb`). Deliberately NOT gated on
  `Without<Autopilot>`, so the autopilot follow-up (task 20260718-122932) can
  drive the same primitive.

## Difficulties / surprises

- `Forces` and `&ComputedMass` in one query: `Forces` already `Read`s
  `ComputedMass`, and a second immutable `&ComputedMass` in the same query
  tuple is a shared read, so no conflict - it let the system stay a single
  query and read mass for the acceleration scaling.
- `Forces` exposes `rotation()` and `linear_velocity()` directly (the thruster
  system reads pose the same way), so the pose is read THROUGH the mutable
  `Forces` item rather than via a separate `&Rotation`/`&LinearVelocity` query,
  which would have aliased the velocity `Forces` writes.
- The manual-burn taper floors the band at `.max(1.0)` (sized for the main
  drive's tens-of-u/s caps); at a 2 u/s RCS cap that floor would make the taper
  start at 1.0 u/s (half the cap). Used `.max(1e-3)` instead - floor only
  against division blow-up, not a feel constant.
- A duplicate `velocity_of` test helper already existed in the flight test
  module; removed the one I added and reused the existing one.
- Running the nova_scenario verb test standalone needs `--features serde` (the
  `ScenarioConfig` serde derives are gated; workspace feature unification
  enables them in CI's `cargo test --workspace --features debug`).

## Tests (all green)

- flight.rs: `rcs_builds_to_the_cap_then_levels_off_without_torque`,
  `rcs_holds_the_cap_forward_but_reverses_freely`,
  `rcs_does_nothing_without_the_verb`,
  `rcs_pushes_along_the_ship_local_axis_in_a_rotated_frame` (non-identity hull
  frame, per the `degenerate-inertia-frames` lesson).
- nova_scenario: `disable_verb_clears_rcs_on_a_controller`.

Per repo policy the full suite / clippy run in CI, not locally; ran check, fmt,
and these five new tests.
