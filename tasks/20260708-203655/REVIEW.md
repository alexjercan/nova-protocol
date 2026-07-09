# Review: Spaceship handling / Newtonian flight-feel overhaul

- TASK: 20260708-203655
- BRANCH: feature/flight-feel-overhaul

## Round 1

- VERDICT: APPROVE

Delivers the Goal as the spike specified it: an assisted-by-default
velocity-command layer (hold, brake latch, soft cap) over the untouched honest
thruster simulation, a Newtonian toggle, capability derived from live sections
(controller = computer + RCS, thrusters = main drive), spooled thruster input
feeding the existing plume/audio seams, and a minimal HUD readout. Checks
re-run by the reviewer at branch tip: fmt clean, clippy clean, `cargo test
--workspace` green (91 nova_gameplay tests incl. 15 new flight tests and the
examples smoke test), wasm32 check green. The physics-level tests genuinely
assert behavior (velocity nulled, station-keeping after a shove, exact
coasting, authority dying with sections, cap holding over a long burn) - not
just execution. The avian `Forces`-vs-`LinearVelocity` query conflict was
correctly anticipated with the same compute/apply split the PD controller
uses, and the AI/editor seams were checked (AI ships carry no `FlightIntent`;
editor-bound thrusters are excluded from FCS ownership; the editor preview
ship lacks physics components so the FCS query skips it - no PD-spam repeat).

No BLOCKER or MAJOR findings. MINORs below are worth doing before merge.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/sections/controller_section.rs
  (`ControllerSectionRcsMagnitude`) - the new component derives `Reflect` but
  is never registered, while the rest of the flight tree
  (`FlightSettings`/`FlightIntent`/`FlightCommand`/`FlightRcsImpulse`/mode) is
  registered in `NovaFlightPlugin`. The juice cycle's R1.1 lesson was
  "register the whole tree the inspector will traverse": the RCS authority is
  exactly the kind of per-ship tunable someone will want to poke in the
  inspector during the 20260709-095043 retune. Add a `register_type` (the
  controller plugin owning the type is the natural place).
  - Response: fixed in b839ac5 - `ControllerSectionPlugin::build` now
    registers `ControllerSectionRcsMagnitude` (and the other reflected
    controller-section components while at it, so the section's tree is
    traversable like flight's).
- [x] R1.2 (MINOR) crates/nova_gameplay/src/flight.rs (Newtonian branch) -
  `FlightIntent.brake` is silently ignored in Newtonian mode: X does nothing
  with FA off, and neither the HUD nor the design note says so. Either
  document the asymmetry explicitly or give brake a direct (non-servo)
  meaning in FA-off - a plain full-retro burn (equivalent to holding S) keeps
  the "no computer" purity while making the key never dead.
  - Response: fixed in b839ac5 - brake in Newtonian is now a direct full
    retro RCS burn (`Vec3::Z` local intent, no servo), covered by a new
    integration test (`newtonian_brake_is_a_direct_retro_burn`), and the
    design note documents the semantics in both modes.
- [x] R1.3 (NIT) crates/nova_gameplay/src/flight.rs
  (`flight_control_system`) - the assisted branch computes
  `needed = (cmd - velocity) * mass` for the main-drive target, drops it, and
  recomputes the same value after the spool loop for the RCS remainder
  (`command.velocity.unwrap_or(**velocity)` re-derivation). Compute it once
  and thread it through; the double derivation invites the two sites drifting
  apart under future edits.
  - Response: fixed in b839ac5 - the servo impulse is computed once into a
    local and reused for both the main-drive target and the RCS remainder.
