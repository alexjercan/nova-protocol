# Nav beacon and salvage crate scenario objects

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,scenario,content,spike

Goal: two reusable scenario-object primitives the Shakedown Run starter
scenario needs, built as general content pieces, not tutorial hacks:

- Nav beacon: small emissive mesh + screen-indicator chip (label +
  distance via the existing indicator substrate) + optional trigger area.
  The game's first player-facing waypoint marker.
- Salvage crate: small prop that despawns on player proximity (its
  OnEnter area) and fires its handler; "collected" is a scenario
  variable, no inventory. Check whether a despawn-scenario-object action
  exists; add one EventActionConfig variant if not.

Notes:
- Spike: docs/spikes/20260712-092926-starter-scenario.md
- Blocks: 20260711-180506 (Shakedown Run scenario uses both objects)
