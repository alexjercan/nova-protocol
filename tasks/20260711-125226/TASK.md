# Remove the redundant closing-speed readout from the destination caption

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.5.0,hud,cleanup

## Goal

Playtest (user, 2026-07-11): during STOP and similar maneuvers the
destination caption shows ETA (fine), distance (fine), and closing speed -
but speed is already shown on the spaceship's own chip, so the caption's
copy is redundant. Drop it.

## Steps

- [ ] In `drive_destination_readout`
      (crates/nova_gameplay/src/hud/maneuver_instruments.rs, the
      `"{eta}{:4.1} u/s | {:5.0}m"` format around line 210): remove the
      closing-speed segment, keeping ETA and distance.
- [ ] Sweep-then-delete (orbit-ring-chip retro rule): grep for consumers
      of `ManeuverTelemetry::closing_speed` before touching the FIELD -
      only the caption format changes unless the field is caption-only;
      if other consumers exist (autopilot planning does use closing
      speeds internally), leave the telemetry field alone.
- [ ] Update the maneuver_instruments caption tests that assert the old
      format string.
- [ ] check + fmt + affected tests.

## Notes

- Filed from user feedback mid-flow (2026-07-11). Precedent for the
  shape of this change: docs/retros/20260711-000547-remove-orbit-ring-chip.md
  (same "two readouts of one number in the same screen area" call, same
  sweep-then-delete discipline).
- The ship speed chip is the diegetic flight status one
  (hud/flight_status.rs) - unchanged.
