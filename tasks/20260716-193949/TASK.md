# Runtime content gate: merge-time issue sweep + FAILED TO START overlay

- STATUS: CLOSED
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

## Steps

- [x] nova_scenario loader.rs: `ContentIssues` (scenario id -> lint
      issues, written by the merge) + `ScenarioStartFailure`
      (Option<report>) resources, plugin-inited, prelude-exported.
- [x] on_load_scenario: Error-level issues for the requested id ->
      refuse BEFORE teardown (previous scene stays), error!-log every
      issue, clear any stale CurrentOutcome (its overlay must not stack
      under the failure modal), set ScenarioStartFailure, return.
- [x] register_bundles (nova_assets): after the merge, lint every
      registered scenario against the MERGED registries (cross-mod
      correct) and insert ContentIssues.
- [x] nova_menu: FAILED TO START overlay (mirrors the outcome overlay:
      threat banner, per-issue lines, Main Menu button via
      on_back_to_menu), Playing-gated, cleared on MainMenu entry;
      load_menu_ambience filters Error-scenarios out of the backdrop
      draw (a broken backdrop must degrade to the bare-camera path, not
      refuse into a cameraless menu).
- [x] Tests: refusal (no scoped entities spawn, failure set, stale
      outcome cleared) + clean-load control; merged-registry issues via
      a synthetic bad bundle + clean-tree pin on the real catalog;
      overlay spawns on failure + backdrop filter degrades gracefully.
- [x] Docs: authoring guide runtime-gate note; CHANGELOG.
- [x] Verify: check --all-targets, fmt, touched suites.

## Notes

- Spike: tasks/20260716-193858/SPIKE.md
- Depends on: 20260716-191543 (the lint core this consumes).
- Stretch (decide at plan time): a warning badge on affected rows in
  the Scenarios picker details pane.

## Close notes (2026-07-16)

What changed: ContentIssues + ScenarioStartFailure resources
(nova_scenario loader, plugin-inited in BOTH the loader and menu
plugins - a menu-only rig panicked on the OnEnter clear until the
second init, the recorded resource-guard lesson class);
on_load_scenario refuses Error-flagged scenarios BEFORE teardown
(previous scene stays, stale outcome cleared so overlays cannot stack,
CurrentScenario untouched); register_bundles lints every merged
scenario against the MERGED registries (cross-mod correct) and warns
each finding; nova_menu renders the FAILED TO START modal
(threat banner, scenario name, one line per finding, Main Menu via the
existing on_back_to_menu) Playing-gated with menu-entry clearing, and
the backdrop draw filters Error-flagged backdrops (a refused menu load
would leave no camera - it degrades to other backdrops or the bare
camera instead).

Tests: loader refusal pin (clean control loads, flagged refuses,
report set, stale outcome cleared); merge sweep pin (real shipped
catalog merges issue-free + a synthetic broken bundle lands in
ContentIssues); menu pins (broken backdrop never draws over 6 seeded
entries; the modal spawns naming scenario + finding; menu entry clears
report + overlay). Suites: nova_menu 51/51, loader 17/17,
demo_scenario 14/14, content_lint_gate green, check --all-targets +
fmt clean. Full suite is CI's job per the standing instruction.

Difficulties: a text-anchored test insert stole the neighboring
skybox test's #[test] attribute (silently deregistering it) - caught
because the filtered run reported the new test TWICE; repaired and
both pass (anchor-edits-in-the-right-scope class, new variant: watch
the attribute line above the anchor).

Reflection: the two-plugin resource init should be the default recipe
for any resource one plugin writes and another consumes - init in
BOTH, they are idempotent.
