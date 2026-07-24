# Retro: Collapsible campaign-header picker + per-chapter replay

- TASK: 20260723-095951
- BRANCH: feature/collapsible-campaign-picker
- REVIEW ROUNDS: 1 (out-of-context APPROVE, no findings)

Process notes only; what/why/evidence is in TASK.md close-out.

## What went well

- Retiring the DECISION's carried risk FIRST: before writing any UI, I grepped
  the two hidden members' builders for a player spawn in OnStart. Both spawn
  their own `player_ship()`, so cold replay was provably sound and the whole UI
  could be built without a mid-flow stop-and-ask. A "verify during work" caveat
  in a decision is cheapest to discharge before you build on it.
- Keeping `listed_scenarios` (the flat, non-hidden order) UNCHANGED as the source
  for the New Game fallback and default selection meant every pre-existing
  selection/fallback test passed untouched - the collapsible grouping was layered
  purely as rendering + a separate `selectable_scenario_ids` for repair. Isolating
  the new concern from the load-bearing selection semantics kept the blast radius
  to zero on the existing contract.
- Widget-tree assertions (per `widget-tree-eyeball-for-logical-layout`) gave a
  deterministic, headless eyeball of the grouped/collapsed layout - header + row
  labels in child order - without the flaky llvmpipe pixel-capture the sibling
  task's lesson warned about. One clean round, APPROVE.

## What went wrong

- Nothing material. One-round APPROVE. The design question that could have gone
  wrong (how default selection interacts with collapsible groups) was resolved by
  decoupling flat-selection-order from grouped-rendering up front, so it never
  became a review finding.

## What to improve next time

- When a task DEPENDS on a decision that carries a "verify X during work" caveat,
  discharge that verification as the FIRST work step (a grep or a tiny probe),
  not after building - it either unblocks the build or triggers the stop-and-ask
  while it is still cheap. Generalized from this cycle's cold-launch check.

## Action items

- [x] Manual DoD items moved to GOAL.md Manual acceptance (browse/expand a
  campaign; replay a hidden chapter) for the flow Finish checkpoint.
- No follow-up code tasks. (Real per-scenario thumbnail art, 20260715-220011,
  shares the picker surface but is independent and already queued.)
