# Review: Remove the base demo scenario

- TASK: 20260716-155816
- BRANCH: refactor/drop-base-demo-scenario

## Round 1

- VERDICT: REQUEST_CHANGES

Verified independently: no references to the deleted scenario's ids
(demo, demo_asteroid_*, demo_beacon, demo_look_around, "Demo Scenario")
survive anywhere outside self-contained test fixtures;
webmods/gauntlet's `dependencies: ["base", "demo"]` names the demo MOD
id, which this branch does not touch; the Scenarios picker default was
already "Broadside" (sorts before "Demo Scenario"), so no default
change; demo_scenario 11/11, content_ron_parity 2/2, check+fmt green.
The repointed assertions still pin the base-scenarios-survive-overlay
property (via shakedown_run), so nothing was weakened.

- [x] R1.1 (MAJOR) CHANGELOG.md:13 (Unreleased) - removing the Demo
  Scenario from the base game deletes a player-visible Scenarios picker
  row, and the docs-sync map (web/src/wiki/dev/keeping-docs-in-sync.md:57)
  routes scenario content changes to the CHANGELOG; the Unreleased
  "Scenarios & Objectives" section exists and gets no line here. Add one
  line noting the base Demo Scenario was removed (superseded as the
  worked RON example by the demo mod's arena).
  - Response: fixed - one line added under Unreleased "Scenarios &
    Objectives" (CHANGELOG.md:21).
- [x] R1.2 (NIT) crates/nova_assets/tests/content_ron_parity.rs:19-21 -
  the new header claims "Every base content file is builder-backed and
  guarded here", which is true today but unenforced: a future hand-added
  RON listed in base.bundle.ron would not be caught. Not this branch's
  job to fix - route to task 20260716-155823 (the generator task) as a
  candidate assertion: the bundle's content list must equal the
  generated file map.
  - Response: agreed and routed - added to tasks/20260716-155823/TASK.md
    Notes as a required assertion for the generator task.

## Round 2

- VERDICT: APPROVE

R1.1 verified: CHANGELOG.md:21 carries the removal line under Unreleased
"Scenarios & Objectives", style matches the neighboring entries. R1.2
verified routed: tasks/20260716-155823/TASK.md Notes now requires the
bundle-list-equals-file-map assertion. No new findings.
