# Make orbit events edge-triggered

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, modding, scenario

## Goal

Make orbit events report state transitions once instead of recurring on a hidden hold window.

## Scope

- Replace OnOrbit with start, stable, unstable, and end lifecycle edges.
- Define cancellation, destruction, and well-switch semantics.
- Remove orbit recurrence and orbit_hold_secs.
- Migrate shipped scenarios to scenario timers where a held orbit is required.
- Update modding docs and player-path coverage.
