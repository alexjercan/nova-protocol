# Make orbit events edge-triggered

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, modding, scenario

## Goal

Make orbit events report state transitions once instead of recurring on a hidden hold window.

## Scope

- Fire OnOrbit once when orbit engagement starts.
- Add an event for orbit engagement ending or changing target.
- Remove orbit recurrence and orbit_hold_secs.
- Migrate shipped scenarios to scenario timers where a held orbit is required.
- Update modding docs and player-path coverage.
