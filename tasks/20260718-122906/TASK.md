# RCS core primitive: Rcs verb + RcsIntent + capped impulse burn system

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.7.0, feature, flight, spike

## Goal

The base RCS mechanic as a shared translational primitive, independent of any
player input. Deliver:

- `FlightVerb::Rcs` added to the verb enum, threaded through the grant checks,
  `WithheldVerbs`, the `SetControllerVerb` scenario action, and the `DisableVerb`
  section modification, so scenarios can grant/withhold RCS per controller. Only
  a ship whose live controller grants `Rcs` can fine-adjust.
- `RcsIntent` component on the ship root: a ship-local desired-direction command
  (per-axis or Vec3), written by whoever drives RCS (player input now, autopilot
  later). Absent/zero = no RCS.
- `RcsSpeedCap` for the small fine-adjust ceiling (start ~1-3 u/s; decide fixed
  `FlightSettings` constant vs authorable component during planning).
- `rcs_burn_system` in FixedUpdate (sibling of `manual_burn_system`): for each
  ship-local axis, apply a pure linear impulse at the center of mass toward the
  commanded sign ONLY while `sign * v_axis < cap`, tapering over the last band -
  the `manual_burn_system` speed-cap math (flight.rs:1899-1911) generalized to
  three signed axes. No torque, no dependence on physical thruster geometry.
  Residual velocity within the cap persists (Newtonian, no auto-null).

## Steps

- [x] Add the `Rcs` variant to the `FlightVerb` enum in
  `crates/nova_gameplay/src/sections/controller_section.rs:175` (doc it like the
  others: a computer-provided capability, "RCS: hold-to-nudge fine translation").
  `FlightVerb` derives serde, so RON scenarios can name `Rcs` with no parse
  table - confirmed no central match/lookup exists (grep found only per-call-site
  `FlightVerb::X` uses, all exhaustive-friendly). Build the workspace to surface
  any non-exhaustive `match FlightVerb` sites and handle them.
- [x] Verify the verb plumbing carries `Rcs` for free: `WithheldVerbs::granted`
  is generic over the variant, `SectionModification::DisableVerb(FlightVerb)` and
  the `SetControllerVerb` scenario action both take a `FlightVerb` by value
  (`crates/nova_scenario/src/objects/modification.rs`,
  `crates/nova_scenario/src/actions.rs`). Add a nova_scenario unit test asserting
  `DisableVerb(FlightVerb::Rcs)` withholds `Rcs` while leaving Stop/Goto/Orbit/Lock
  granted (mirror the existing `DisableVerb(FlightVerb::Orbit)` test at
  modification.rs:161).
- [x] Add tunables in `crates/nova_gameplay/src/flight.rs`: an `RcsSpeedCap(f32)`
  component on the ship root (fine-adjust ceiling, mirroring `FlightSpeedCap` at
  flight.rs:112) and, in `FlightSettings`, `rcs_speed_cap` (default ~2 u/s, used
  when the component is absent - RCS is ALWAYS capped, unlike the optional
  main-burn cap) and `rcs_accel` (the RCS thrust as an acceleration, the push
  magnitude). Reuse `SPEED_CAP_TAPER_FRACTION` for the taper band.
- [x] Add the `RcsIntent(Vec3)` component on the ship root in flight.rs: a
  ship-local desired-direction command (each component roughly -1..1, magnitude =
  how hard the nudge). Absent or zero = no RCS. Written by the input layer (task
  20260718-122912) or the autopilot (task 20260718-122932); this task only
  consumes it. Insert a default `RcsIntent` on player ships alongside
  `FlightIntent` in `insert_flight_control` (flight.rs:469).
- [x] Implement `rcs_burn_system` in flight.rs (FixedUpdate): query ship roots
  with `(&RcsIntent, Option<&RcsSpeedCap>, &Rotation, &LinearVelocity, Forces)`
  plus the ship's controller sections for the verb gate. Per the `two-clocks`
  lesson, read raw `Rotation`/`LinearVelocity` (NOT GlobalTransform). For each
  ship-local basis axis `a` in `[Vec3::X, Vec3::Y, Vec3::Z]`:
  - `cmd = intent.dot(a)`; skip if ~0.
  - `world_axis = rotation.0 * a`; `along = velocity.dot(world_axis)`.
  - taper gate `g = ((cap - cmd.signum()*along) / (cap*SPEED_CAP_TAPER_FRACTION).max(..)).clamp(0,1)`
    - so a push in the direction the ship already moves at the cap yields 0, the
    opposite direction yields full push (the "5u/s forward -> RCS forward does
    nothing, backward works to -5" rule).
  - accumulate `impulse += world_axis * cmd * rcs_accel * g * dt`.
  Apply the summed impulse once with `Forces::apply_linear_impulse(impulse)`
  (acts at COM, generates NO torque - confirmed avian3d-0.7 query_data.rs:388),
  so RCS never rotates the hull and needs no `ComputedCenterOfMass`.
- [x] Gate `rcs_burn_system` on the ship granting the `Rcs` verb: only apply
  impulse when the ship has a live controller section whose `WithheldVerbs`
  grants `Rcs` (same rule as `ship_grants_verb`, input/player.rs:775 - replicate
  the ChildOf + `WithheldVerbs::granted` check inside the system, or lift a
  shared helper). A ship without the verb gets no RCS even if something writes
  `RcsIntent`. Do NOT gate on `Without<Autopilot>` - the autopilot follow-up
  (task 20260718-122932) needs to drive RCS while engaged.
- [x] Register the systems and types: add `rcs_burn_system` to the `FixedUpdate`
  `NovaFlightSystems` set next to `manual_burn_system` (flight.rs:458-463); it
  writes `Forces`, not `ThrusterSectionInput`, so it does not conflict with
  `manual_burn_system`. `register_type::<RcsIntent>()` and
  `register_type::<RcsSpeedCap>()` in the flight plugin build (Reflect debug
  inspector, per the register_type lesson).
- [x] Headless unit tests in flight.rs (`#[cfg(test)]`, driver app + FixedUpdate,
  no render app - follow the existing flight tests): spawn a dynamic RigidBody
  ship root with `RcsIntent`, `RcsSpeedCap`, `Rotation`, `LinearVelocity` and a
  child controller section granting `Rcs`, step the schedule, and assert:
  (a) a forward `RcsIntent` builds `LinearVelocity` up toward `+cap` and then
  stops adding (levels at the cap, does not exceed); (b) at `+cap` a forward
  command adds nothing while a backward command still accelerates toward `-cap`;
  (c) a ship whose controller withholds `Rcs` gets zero velocity change;
  (d) `AngularVelocity` stays ~zero (no torque). Test frame composition with a
  non-identity ship `Rotation` (per `degenerate-inertia-frames`), so the
  world-axis math is actually exercised.
- [x] Record the design/fix notes in `tasks/20260718-122906/NOTES.md` (what
  shipped, the cap formula, any surprises) and append a short line to the spike's
  Fix record (`tasks/20260718-122508/SPIKE.md`).

## Notes

- Spike: tasks/20260718-122508/SPIKE.md (RECOMMENDED; forks Q1-Q4 resolved -
  held-direction, Newtonian residual, virtual impulse at COM, freeze heading).
- Scope guard: NO player input (task 20260718-122912) and NO HUD (task
  20260718-122923) here. This task delivers the primitive + verb + tests only.
- Reference points verified during planning:
  - `FlightVerb` / `WithheldVerbs` / `ship_grants_verb` semantics:
    sections/controller_section.rs:175, input/player.rs:775.
  - `FlightSpeedCap` + the taper this generalizes: flight.rs:112, 1899-1911.
  - System registration (FixedUpdate `NovaFlightSystems`, before
    `SpaceshipSectionSystems`): flight.rs:454-463.
  - `insert_flight_control` (where the default intent is inserted): flight.rs:469.
  - Thruster impulse-at-point precedent: sections/thruster_section.rs:301,327.
  - `Forces::apply_linear_impulse` acts at COM, no torque: avian3d-0.7
    dynamics/rigid_body/forces/query_data.rs:388.
  - `DisableVerb` / `SetControllerVerb` take `FlightVerb` by value (variant is
    transparent): nova_scenario objects/modification.rs, actions.rs.
- Lessons applied: `two-clocks` (FixedUpdate reads raw Rotation/LinearVelocity),
  `register_type` for Reflect components, `degenerate-inertia-frames` (test with a
  non-identity frame).
- Assumption to confirm at build: adding `FlightVerb::Rcs` produces no
  non-exhaustive `match` breakage beyond the call sites grepped; the workspace
  build will flag any.
