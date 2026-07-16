# Review: runtime content gate + FAILED TO START overlay

- TASK: 20260716-193949
- BRANCH: feature/runtime-content-gate

## Round 1

- VERDICT: APPROVE

Verified independently: the pin chain covers every link - synthetic bad
bundle -> ContentIssues (merge sweep test, plus the shipped-tree clean
pin), ContentIssues -> refusal (loader test, sabotage-proven by this
review: removing the early return turned "a refused start must spawn
nothing" red; restored, green), report -> modal (menu test asserting
scenario name + finding text) and modal lifecycle (dies with Playing,
resource cleared on menu entry). The refusal happens BEFORE teardown
(previous scene survives) and clears the stale outcome so overlays
cannot stack. The backdrop-draw filter closes the one path where a
refusal would have cost the menu its camera - and the seeded 6-entry
test pins that the broken backdrop never draws. Both writer and
consumer plugins init the shared resources (the rig-panic class caught
in-cycle). nova_menu 51/51, loader 17/17, demo_scenario 14/14,
content_lint_gate green, check --all-targets + fmt clean.

No findings.
