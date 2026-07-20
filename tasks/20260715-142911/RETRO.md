# Retro: two-pane mods screen

- TASK: 20260715-142911
- BRANCH: feature/mods-screen (landed on master as fd4c86af)
- REVIEW ROUNDS: 2 (R1 APPROVE with one MINOR fixed, R2 APPROVE)

## What went well

- The review's VISUAL capture (throwaway autopilot harness -> Xvfb screenshots
  -> eyeball by both reviewer and orchestrator) caught the one real defect -
  the menu card painting over the panel with entity-id-recycled z-order - that
  no headless assertion could see. render-output-eyeball generalizes from
  generated images to UI: a layout task is not verified until someone SEES it.
- Descoping search at plan time (no text-input widget exists; three mods) kept
  the diff focused; the plan's marker/contract sketch (ModDetailsActions as a
  stable container for 142916) survived implementation intact.
- The implementer verified engine claims in vendored source unprompted (Button
  click propagation, ObservedBy cleanup semantics) - the
  verify-engine-guarantees-in-source habit has propagated into delegated work.

## What went wrong

- The z-order bug is the 174126 review's "modal-overlap UX nit deferred"
  matured into a real defect when the panel grew from 460px to 85% - a
  deferred cosmetic nit whose precondition (panel size) changed without the
  deferral being revisited. Deferrals age; a scope change that touches a
  deferral's premise should re-open it.
- Nondeterministic z-order existed silently all along (sibling roots without
  GlobalZIndex, ordered by recycled entity ids) - it just happened to render
  correctly until now.

## What to improve next time

- When a task grows a UI element that a past review deferred a finding
  against, re-read that deferral (grep prior REVIEW.md files for the panel
  being reworked).
- For any UI-layout task: schedule the visual capture as a first-class verify
  step, not a reviewer improvisation.

## Action items

- [x] LESSONS.md: sharpened `render-output-eyeball` (UI variant + the
  aging-deferral corollary); bumped `out-of-context-review-pass`.
