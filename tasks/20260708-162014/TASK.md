# Ship editor polish + save/load ship blueprints

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,editor

Spike: tasks/20260708-161726/SPIKE.md (roadmap)

The editor (`crates/nova_editor`) places sections on a fixed grid via click, with
no rotation, copy/paste, or persistence. Direction: editor UX polish (section
rotation, copy/paste, ship templates, clearer grid feedback) plus save/load of
ship blueprints to disk. Dovetails with the phase-1 asset format (133029): a saved
ship should be the same serialized `SpaceshipConfig`/section data a scenario uses,
so the editor and the modding format share one representation.
</content>
