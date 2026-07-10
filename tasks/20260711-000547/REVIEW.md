# Review: Remove the redundant ORBIT ring chip

- TASK: 20260711-000547
- BRANCH: fix/remove-orbit-ring-chip

## Round 1

- VERDICT: APPROVE

Pure removal, verified against the spec (the user's playtest call): the
chip, its marker, its drive system, and its only helper
(flight::orbit_ring_point) are gone; the holo ring and its lifecycle are
untouched, and circular_orbit_speed keeps its autopilot consumers
(checked before the helper deletion). Tests were updated, not weakened -
the deleted assertions covered the deleted chip, the renamed ring test
still covers spawn/replan/breakout, and the spoke chip coverage is
intact at its new child index. grep for OrbitChip/orbit_ring_point/
"v_circ chip" is clean in code and prose. cargo fmt and
`cargo check --workspace --examples` clean; the 10 module tests pass.
No findings.
