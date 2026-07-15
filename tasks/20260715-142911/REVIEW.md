# Review: two-pane mods screen - tabs, quiet checkboxes, details panel

- TASK: 20260715-142911
- BRANCH: feature/mods-screen

## Round 1

- VERDICT: APPROVE (one MINOR + three NITs; MINOR + one NIT fixed before
  landing, two NITs deferred with reasons)

Out-of-context review pass over the full diff (e4da1309), INCLUDING a visual
check: the reviewer built a throwaway autopilot capture harness (patterned on
14_screenshot_ui, deleted after), screenshotted the panel on Xvfb, and
eyeballed all three states; the orchestrator re-eyeballed the capture. Layout
renders cleanly - panes, tabs, rows, quiet checkboxes, details, default
selection all correct. Verified independently: bevy_ui_widgets Button stops
click propagation (source-checked - the checkbox cannot select its row);
observer cleanup on rebuild (ObservedBy despawn semantics source-checked); the
refresh run-condition chain has no stale-pane hole incl. panel re-entry and
stale-selection repair; base is locked at all three layers; hidden filtering
unaffected (upstream); the coming-soon sweep is complete; test census 13 -> 18
with zero deletions and the adapted test strengthened. Sabotage re-run by the
reviewer: no-op'd tab switching -> exactly the tab test failed, revert green.
Counts reproduced: nova_menu 18, demo_scenario 11, fmt/check clean.

- [x] R1.1 (MINOR, visually confirmed) the main-menu card painted OVER the
  open mods panel's corner with z-order decided by recycled entity ids
  (nondeterministic by construction; Settings had the same latent bug).
  - Response: fixed in 9daa45b8 - explicit `GlobalZIndex(1)` on both overlay
    roots, constraint-comment on each, pinned by a component-presence test
    asserting z > 0 (the doc states render order itself is only visually
    verifiable). Verified by reviewer role.
- [x] R1.2 (NIT) the Explore tab kept showing the last installed mod's details
  with a live Enable/Disable button next to the portal placeholder.
  - Response: fixed - the Explore branch clears SelectedModId (fallback text
    renders same-frame); switch-back re-runs default selection; the tab test
    extended to the exact scenario. Verified by reviewer role.
- [x] R1.3 (NIT) details pane has no scroll for long descriptions. DEFERRED -
  fine at three mods; becomes real when long portal descriptions arrive
  (noted for the future).
- [x] R1.4 (NIT) `Selected` marker sets only BackgroundColor until the next
  interaction event. DEFERRED - pre-existing nova_ui widget behavior, cosmetic.

## Round 2

- VERDICT: APPROVE

Fix commit 9daa45b8 verified: nova_menu 19 passed (z-index pin added,
tab-switch test extended in place), fmt clean, worktree clean. The z-index
test's `> 0` judgment call is right - it pins direction without pretending to
verify render order. No new findings.
