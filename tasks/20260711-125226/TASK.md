# Remove the redundant closing-speed readout from the destination caption

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.5.0,hud,cleanup

## Goal

Playtest (user, 2026-07-11): during STOP and similar maneuvers the
destination caption shows ETA (fine), distance (fine), and closing speed -
but speed is already shown on the spaceship's own chip, so the caption's
copy is redundant. Drop it.

## Steps

- [x] Removed the closing-speed segment from the destination caption
      (drive_destination_readout); format is now `"ETA {n}s | {d}m"` with
      the redundancy rationale commented at the site.
- [x] Sweep-then-delete: `ManeuverTelemetry::closing_speed` has real
      consumers (the flight planner's flip/brake math in flight.rs; the
      torpedo HUD has its own separate closing_speed helper) - the FIELD
      stays, only the caption changed.
- [x] Caption test updated (`"ETA  18s |   300m"`).
- [x] fmt + full lib suite 358/358.

## Notes

- Filed from user feedback mid-flow (2026-07-11). Precedent for the
  shape of this change: docs/retros/20260711-000547-remove-orbit-ring-chip.md
  (same "two readouts of one number in the same screen area" call, same
  sweep-then-delete discipline).
- The ship speed chip is the diegetic flight status one
  (hud/flight_status.rs) - unchanged.

## Resolution

One format string, one comment, one test string. Sweep confirmed the
telemetry field is a planner input, not caption-only, so it stays.
