# Retro: drawer tabs scroll instead of overflowing

- TASK: 20260725-163835
- BRANCH: fix/drawer-scroll-tabs
- REVIEW ROUNDS: 1

## What went well

- Reading the editor/menu scroll implementations before editing kept the drawer
  fix aligned with existing Bevy UI practice.
- The widget-tree tests used the real drawer spawn path, so they pinned the
  actual left/right panel structure without needing a flaky render capture.
- Out-of-context review approved the branch after re-running the key checks.

## What went wrong

- The initial task checklist mixed automated verification and manual visual
  acceptance into one step. Root cause: the plan treated "verify the layout"
  as one activity even though only the automated checks belong to implementation
  close-out.

## What to improve next time

- Split manual acceptance out of implementation checklists during planning, and
  leave it as a `manual:` DoD item for review/user acceptance.

## Action items

- [x] Added a lessons-ledger entry for separating manual acceptance from
  implementation checklist items.
