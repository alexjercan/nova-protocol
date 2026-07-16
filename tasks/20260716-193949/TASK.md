# Runtime content gate: merge-time issue sweep + FAILED TO START overlay

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.7.0, modding, feature, ui

## Goal

Wesnoth-style runtime reporting for broken content (user request
2026-07-16): after the bundle merge, sweep every registered scenario
with the shared lint core (tasks/20260716-193858/SPIKE.md) against the
MERGED registries into a `ContentIssues` resource; a scenario with
Error-level issues REFUSES to start - `on_load_scenario` builds no
scene, logs every issue, and the player sees a FAILED TO START modal
("Failed to start '<name>': unknown section prototype '<id>'.") with a
Main Menu button, riding the outcome-overlay path. The spawn-time
error-and-skip stays as the last-ditch backstop.

Direction-level; /plan breaks it into steps when picked up.

## Notes

- Spike: tasks/20260716-193858/SPIKE.md
- Depends on: 20260716-191543 (the lint core this consumes).
- Stretch (decide at plan time): a warning badge on affected rows in
  the Scenarios picker details pane.
