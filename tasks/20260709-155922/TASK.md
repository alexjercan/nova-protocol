# Disabled-in-place controller still torques toward its frozen command

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.4.0, bug, handling

Pre-existing hole surfaced by review R1.6 of the flight-feel retune
(20260709-095043): `sync_controller_section_forces`
(sections/controller_section.rs) has no `Without<SectionInactiveMarker>`
filter, so a controller section that is disabled-in-place (zero health,
non-leaf, still attached) keeps applying PD torque toward its frozen command.
That contradicts the flight layer's "no live computer = adrift" semantics
(the autopilot disengages, the player command freezes - but the dead computer
keeps stabilizing the hull).

## Steps

- [x] Add the inactive filter so a disabled controller stops torquing. Added
      `Without<SectionInactiveMarker>` to `sync_controller_section_forces`
      (sections/controller_section.rs). nova's apply-torque system is the only
      seam: the bcs PD system still computes `PDControllerOutput`, but nothing
      else consumes it, so gating the apply is sufficient - no bcs change.
- [x] Physics test: `a_disabled_controller_leaves_the_spin_untouched`
      (flight.rs) disables a controller mid-hold and asserts the imposed spin
      is conserved; paired with `a_live_controller_damps_an_imposed_spin` as a
      control case so the regression cannot pass vacuously.
- [x] Feel check: covered by the deterministic physics test above rather than a
      manual 11_com_range run (per repo policy: automated tests over manual
      example runs; the example there destroys the controller, not disables it,
      so it would need a 2x2 ship to exercise the disable path). The example
      was left unchanged; the pipeline-level test is the stronger regression net.

## Notes

- Found while reviewing the no-computer freeze semantics in input/player.rs.
- The disable pipeline: IntegrityDisabledMarker -> SectionInactiveMarker
  (integrity/glue.rs on_section_disable), non-leaf sections only.
