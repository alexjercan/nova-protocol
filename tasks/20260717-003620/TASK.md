# Numeric hull-integrity chip in the flight-status chip family (exact-value backstop)

- STATUS: OPEN
- PRIORITY: 29
- TAGS: v0.7.0,hud,ui,spike

Goal: keep an exact hull-integrity number available once the generic bar is
gone, as a subordinate backstop to the diegetic mesh tint (20260717-003613).
Add a compact numeric chip (e.g. `HULL 84%` or `672/800`) in the flight-status
chip family - `screen_indicator` anchored to the player ship, NAV_CYAN,
`HudTier::Instrument` - reading the root aggregate `Health`. Near-copy of
`drive_speed_chip` in hud/flight_status.rs.

Direction (from the spike - /plan owns the steps):
- The tint carries the gestalt/spatial read; this chip carries the precise
  value for docking/repair decisions (same split the flight-status spike used
  for speed).
- Coordinate rounding with the living-sliver fix 20260716-165617 so a barely
  alive ship never reads 0%.
- May ship in the same change as 20260717-003613 or as a fast follow.

Notes:
- Spike: tasks/20260711-202901/SPIKE.md (Option 2, subordinate role).
- Stepless: /plan this before /work.
- Related: hud/flight_status.rs (speed/mode chips), hud/screen_indicator.rs,
  sibling spike tasks/20260710-234019/SPIKE.md.
- Append a line to the spike's Fix record when this lands.

