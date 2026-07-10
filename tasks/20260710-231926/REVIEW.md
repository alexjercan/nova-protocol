# Review: Diegetic flight status v1

- TASK: 20260710-231926
- BRANCH: feature/diegetic-flight-status

## Round 1

- VERDICT: APPROVE (2 MINOR, fix at implementer's discretion before merge)

Verified: full diff read against master; `cargo fmt --check` clean;
`cargo check --workspace --examples` clean (work run, re-confirmed by the
fmt/test run); the 10 module tests pass. Spec check against TASK.md and
the spike doc: every fact of the old line is rehomed (speed chip, mode
chip, spoke) or deliberately dropped (GRAV cue, GOTO distance), and
`flight_status_line`/`GravStatus` are gone with their tests - grep
returns nothing. The destination marker and examples/12_hud_range are
untouched, as promised. Prose sweep done: no comment still claims the
corner line exists.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/flight_status.rs:214 -
  drive_speed_chip's Err arm clears the anchor, but the Ok arm only
  writes text and never restores it. If the target ever fails the query
  transiently (component churn rather than death), the chip goes dark
  permanently while its text keeps updating - text and anchor disagree.
  Set the anchor in the Ok arm too (mirror drive_mode_chip), making the
  chip self-healing and the two systems symmetric.
  - Response: fixed in-round - the Ok arm re-asserts the anchor every
    frame, mirroring drive_mode_chip.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/maneuver_instruments.rs
  radius spoke test - the well-death path (well despawned mid-orbit:
  `q_well.get` fails, spoke must sweep, chip must clear) is reachable in
  play (wells are destructible rocks) but untested; the maneuver-end path
  is. Add a well-despawn case to radius_spoke_and_chip_track_the_engaged_orbit.
  - Response: fixed in-round - the test re-engages after breakout and
    despawns the well; spoke count drops to zero and the chip clears.

Round 1 close-out: both fixes verified in the new diff (anchor
re-asserted in the Ok arm; well-despawn case asserts spoke death and
chip clear), 10/10 module tests green after the changes. APPROVE stands.
