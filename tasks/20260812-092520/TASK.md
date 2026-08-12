# Make lock events edge-triggered

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, modding, scenario

## Goal

Make player lock events report acquisition once instead of recurring on a hidden interval.

## Scope

- Fire travel and combat lock events once on acquisition.
- Add events for lock loss or target changes.
- Remove lock recurrence and lock_refire_secs.
- Migrate shipped scenarios and update modding docs and player-path coverage.
