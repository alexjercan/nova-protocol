# Keep final completed objectives in drawer log

- STATUS: CLOSED
- PRIORITY: 53
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Goal

Owner feedback after task 20260724-134350: completed objectives should remain
in the drawer's Objectives list and be marked done, for example with
strike-through. The landed implementation already retained completed objectives
while another objective stayed active, but completing the final active objective
emptied `GameObjectives` and cleared the drawer log.

## Steps

- [x] Change drawer objective-log sync so an empty `GameObjectives` list after
  prior active objectives marks those prior objectives completed instead of
  clearing the log.
- [x] Clear the drawer objective log on drawer/player teardown, where stale
  scenario state actually needs to die.
- [x] Update drawer tests for final-completion retention and teardown clearing.

## Definition of Done

- Completing the last active objective leaves it in the drawer as a completed
  struck-through row. (test:
  `drawer_objectives_keep_final_completed_row_with_strike`)
- Drawer/player teardown clears the derived objective log. (test:
  `drawer_objective_log_clears_on_drawer_teardown`)
- The touched drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)

## Notes

- Follow-up to 20260724-134350.
- Implementation: empty `GameObjectives` now completes any previously active
  drawer-log entries instead of clearing the log; `remove_drawer` clears the
  derived log when the player/drawer teardown happens.
- Verification: `nix develop --command cargo test -p nova_gameplay drawer`
  passed with 17 drawer-filtered tests.
