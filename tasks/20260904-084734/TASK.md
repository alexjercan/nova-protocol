# Enforce vector speed caps for diagonal flight

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog

## Goal

Make manual and RCS speed limits constrain total velocity-vector magnitude. Diagonal input must not exceed the stated limit. Braking and recovery must remain possible when a ship is already at or above the cap.

## Confirmed problem

- `rcs_burn_system` gates each ship-local velocity axis independently. Two-axis input can approach `sqrt(2) * cap`; three-axis input can approach `sqrt(3) * cap`.
- RCS acceleration is already clamped by total vector magnitude. Its velocity cap is not.
- `manual_burn_system` gates only the velocity component along the current burn direction. A ship can turn and add another component while already near the displayed `FlightSpeedCap`.
- Existing documentation describes the manual directional behavior as deliberate, but it does not match the expected meaning of a player-facing speed limiter.

## Required behavior

- At a 100 m/s RCS cap, straight and diagonal translation share one 100 m/s vector budget relative to `RcsReference`.
- At a 150 m/s manual cap, changing attitude and burning cannot increase total speed beyond 150 m/s.
- Input that reduces total speed remains available at and above the cap.
- A ship already overspeed can brake back below the cap.
- RCS orbit trim continues to measure the residual velocity relative to `RcsReference`, not absolute orbital speed.
- The cap remains soft enough to avoid oscillation at its boundary and handles finite-step overshoot explicitly.
- Straight-line behavior and mass-independent 5 g RCS authority remain unchanged below the cap.

## Investigation and implementation notes

1. Review the cap semantics in `flight/state.rs`, `flight/manual.rs`, autopilot RCS use, and all focused tests.
2. Define one tested vector-budget helper if manual and RCS can share the same rule without obscuring their different reference velocities and taper behavior.
3. Handle acceleration that is inward, tangential, or outward relative to the current velocity vector.
4. Do not independently clamp world axes or ship-local axes.
5. Preserve vector-magnitude acceleration clamping for diagonal RCS input.
6. Update player-facing and creator-facing documentation that currently promises directional/per-axis limits.

## Proof

Add focused tests for:

- One-, two-, and three-axis RCS reaching the same speed ceiling.
- RCS reference-relative diagonal trim during orbit.
- Manual burn after turning near the cap.
- Braking from exactly at and above the cap.
- No artificial rotation or main-drive use during low-speed RCS STOP.
- No regression to existing RCS acceleration-budget tests.

Run only affected `nova_ship` tests, formatting, documentation checks, and `git diff --check` unless the review finds a broader dependency.

