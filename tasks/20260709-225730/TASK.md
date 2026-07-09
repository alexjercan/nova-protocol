# AI patrol and idle flight states

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.4.0,ai,spike,handling


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 2)

Goal: make AI ships placeable in scenarios before combat starts. Patrol state:
fly a waypoint loop, reusing the GOTO autopilot / FlightIntent machinery
(flight.rs) where possible instead of a parallel steering path; a
hostile-detection range transitions Patrol -> Engage. Idle state:
station-keeping drift (kill velocity, hold position loosely).

Blocked on: 20260709-155921 (AI rotation path). Depends on:
20260709-225726 (skeleton).
