# A disabled controller stops torquing the hull (task 20260709-155922)

## What changed

`sync_controller_section_forces` (sections/controller_section.rs) applied each
controller section's `PDControllerOutput` as torque to its target hull with no
`SectionInactiveMarker` filter. A controller disabled in place - zero health,
non-leaf, still attached, which the integrity pipeline marks
`SectionInactiveMarker` via `on_section_disable` - therefore kept stabilizing
the hull toward its frozen command. That contradicted the flight layer's "no
live computer = adrift" semantics: the autopilot disengages and the player
command freezes, but the dead computer went on holding attitude.

The fix adds `Without<SectionInactiveMarker>` to that system's query, mirroring
the filter already carried by the sibling
`update_controller_section_rotation_input` and the flight systems
(`autopilot_system`, `manual_burn_system`).

## Why nova-side only, and why this one system

The bcs PD system still computes `PDControllerOutput` for a disabled controller,
but `sync_controller_section_forces` is the *only* consumer of that output in
nova - nothing else reads it - so gating the apply is the whole fix; bcs needs
no input-gating. Both disable paths are covered: a non-leaf controller gets
`SectionInactiveMarker` and is filtered here, and a leaf controller is despawned
by the integrity core, so it has no output to apply in the first place.

## Verification

Two flight physics tests in `flight.rs`:

- `a_disabled_controller_leaves_the_spin_untouched` - imposes a spin, disables
  the controller, and asserts the spin is conserved. Confirmed to FAIL with the
  filter reverted, so it genuinely catches the regression.
- `a_live_controller_damps_an_imposed_spin` - the control case: the same spin is
  damped when the controller is live, so the regression cannot pass vacuously.

The imposed spin is about a transverse (Y) axis; the ship is a symmetric top
about its long z-axis (three unit cubes on z), so a torque-free spin about Y is
exactly constant with no tumbling - the tolerance is physically justified, not
tuned. The manual 11_com_range feel check in the task was substituted with this
deterministic test (the example destroys its controller rather than disabling
it, so exercising the disable path would need a 2x2 ship).
