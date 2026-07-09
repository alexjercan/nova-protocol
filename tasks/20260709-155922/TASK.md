# Disabled-in-place controller still torques toward its frozen command

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.4.0,bug,handling

Pre-existing hole surfaced by review R1.6 of the flight-feel retune
(20260709-095043): `sync_controller_section_forces`
(sections/controller_section.rs) has no `Without<SectionInactiveMarker>`
filter, so a controller section that is disabled-in-place (zero health,
non-leaf, still attached) keeps applying PD torque toward its frozen command.
That contradicts the flight layer's "no live computer = adrift" semantics
(the autopilot disengages, the player command freezes - but the dead computer
keeps stabilizing the hull).

## Steps

- [ ] Add the inactive filter so a disabled controller stops torquing (check
      whether the PD systems in bcs also need input-gating or whether nova's
      copy system is the only seam).
- [ ] Physics test: disable a controller mid-hold; the hull stops being
      stabilized (a spun ship keeps spinning).
- [ ] Feel check: in 11_com_range, killing the controller should leave the
      spin untouched from that instant (it is destroyed there, not disabled,
      so extend the range or use a 2x2 ship where the controller is non-leaf).

## Notes

- Found while reviewing the no-computer freeze semantics in input/player.rs.
- The disable pipeline: IntegrityDisabledMarker -> SectionInactiveMarker
  (integrity/glue.rs on_section_disable), non-leaf sections only.
