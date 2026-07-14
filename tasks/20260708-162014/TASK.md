# Ship editor polish + save/load ship blueprints

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: wontdo,editor,redundant

Spike: tasks/20260708-161726/SPIKE.md (roadmap)

The editor (`crates/nova_editor`) places sections on a fixed grid via click, with
no rotation, copy/paste, or persistence. Direction: editor UX polish (section
rotation, copy/paste, ship templates, clearer grid feedback) plus save/load of
ship blueprints to disk. Dovetails with the phase-1 asset format (133029): a saved
ship should be the same serialized `SpaceshipConfig`/section data a scenario uses,
so the editor and the modding format share one representation.

CLOSED (wontdo, 20260714): redundant. A spaceship blueprint is just a scenario
containing a single spaceship, so ship save/load falls out of the in-editor
scenario builder (20260714-081703) + the RON format (20260525-133029) for free -
export an "empty" scenario with only the ship. No separate ship-blueprint path
needed. The section-placement UX polish that lived here is folded into the
scenario-builder task's scope. See tasks/20260714-081636/SPIKE.md.
