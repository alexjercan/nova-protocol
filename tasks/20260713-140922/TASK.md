# OnLock scenario event: bridge the lock components into the event vocabulary

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.5.0,scenario,events,spike

## Goal

The beat-sheet-v2 spike needs a "player locked X" completion signal so the
split radar lessons TICK the instant a lock lands (three consumers: the
first-lock lesson, the waypoint re-designation leg, the combat-lock
rehearsal). Add ONE event: an OnLock bridge in the scenario loader (the
`track_orbit_holds` shape, loader.rs:195) watching the player's
TravelLock/CombatLock by change detection, resolving the target's scenario
EntityId, firing `{ id, other_id, other_type_name, combat }` - filterable
by target id like OnEnter; the travel/combat discrimination encoding
(info-field filter vs two variants) is /plan's call.

## Notes

- Spike: docs/spikes/20260713-140742-shakedown-beat-sheet-v2.md (option C;
  option D's wider vocabulary deliberately rejected).
- Firing semantics are an open question the plan must settle: once per
  ACQUISITION onto the filtered id (the live radar retargets under the
  sweep; a leftover lock must not self-complete the next lock lesson).
- Surface: nova_events (event + info), nova_scenario events.rs (EventConfig
  variant) + loader.rs (bridge system + tests); nova_editor untouched
  (verified - it does not enumerate event variants).
- /plan before implementation.
