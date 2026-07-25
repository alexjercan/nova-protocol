# Retro: Drawer LEFT panel combined Flight Log

- TASK: 20260724-102309
- BRANCH: feature/drawer-combined-flight-log
- REVIEW ROUNDS: 1

## What went well

- The plan gate caught the core UX shape before implementation. The task moved
  from separate `COMMS` / `FLIGHT LOG` sections to one chronological stream
  while the cost was still only task-record edits.
- The tests hit the real risky edges: comms/objective interleaving, objective
  text updates without duplicate rows, final completion, current-only right
  panel behavior, and teardown clearing.
- Keeping the right panel as a direct `GameObjectives` render made the new
  information model simple: active state on the right, retained history on the
  left.

## What went wrong

- The first plan mapped existing resources directly to UI sections, which was a
  plausible implementation shape but not the desired reading experience. Root
  cause: the word "logs" was treated as a data source question before pinning
  whether the UI should be grouped or chronological.
- The out-of-context review default could not be followed because subagent
  spawning is tool-restricted unless the user explicitly asks for it. The review
  recorded an in-session exception instead.

## What to improve next time

- For log-style UI, ask or infer the reader shape first: one chronological
  stream, grouped categories, or separate panes. Then map data sources into that
  shape.
- When a review process wants out-of-context review but tool policy blocks
  subagents, record the exception directly in `REVIEW.md` and compensate with a
  more explicit verification list.

## Action items

- [x] Added `log-ui-shape-before-plan` to `LESSONS.md`.
