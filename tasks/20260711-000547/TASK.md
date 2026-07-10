# Remove the redundant ORBIT ring chip (r | v_circ) from maneuver instruments

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.5.0,hud,ux


## Goal

Playtest (user, 2026-07-11, after 20260710-231926 landed): the ORBIT
ring chip reading `r 100 | 20 u/s` still floats near the ship while
orbiting and now duplicates the new instruments - the radius spoke chip
carries the current radius and the ship-anchored chips carry mode and
speed. Remove the ring chip entirely; the holo ring itself stays.

## Steps

- [x] Delete the OrbitChipUI child, OrbitChipUIMarker, and
      drive_orbit_chip from
      crates/nova_gameplay/src/hud/maneuver_instruments.rs (also unwire
      it from the plugin's system tuple and the module doc bullet); the
      layer drops from four chips to three.
- [x] Update the module tests: spawn_instruments returns three children
      again; orbit_ring_and_chip_live_and_die_with_the_plan loses its
      chip assertions (rename accordingly); the spoke test keeps its
      chip coverage.
- [x] Sweep: grep for OrbitChip and v_circ stragglers (symbols and
      prose); check whether circular_orbit_speed still has consumers
      outside the autopilot before touching it - if the chip was its
      only HUD consumer it simply loses that caller, nothing else.
- [x] cargo fmt + cargo check --workspace --examples; run the
      maneuver_instruments tests.
- [x] CHANGELOG.md [Unreleased]: fold into the existing diegetic-status
      Changed line (the ring chip is retired as redundant).

## Notes

- Follow-up to 20260710-231926; spike
  docs/spikes/20260710-234019-diegetic-flight-status.md (the spike had
  kept the ring chip "as-is" - the user's playtest overruled that once
  the spoke existed, since the two chips read as duplicates in the same
  screen area).

## Closing notes (2026-07-11)

Removed the OrbitChipUI child, its marker, and drive_orbit_chip; the
maneuver instruments layer is back to three chips (destination readout,
flip, radius spoke). flight::orbit_ring_point went too - the chip was
its only consumer (verified by grep before deleting); orbit_ring_offset
and orbit_ring_radial stay, the autopilot uses them. The ring lifecycle
test lost its chip assertions and was renamed to match. CHANGELOG's
diegetic-status line gained the retirement clause. No difficulties; the
sweep-then-delete order (find consumers before removing the symbol) made
this mechanical.
