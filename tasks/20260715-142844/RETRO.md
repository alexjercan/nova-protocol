# Retro: hidden catalog flag keeps dev/tooling mods out of the Mods menu

- TASK: 20260715-142844
- BRANCH: feature/hidden-dev-mods (landed on master as 4a6d2615)
- REVIEW ROUNDS: 1 (APPROVE, one MINOR fixed on-branch)

## What went well

- Plan-time CI check: before writing steps, a look at master CI caught that the
  exact assertion this task touches was already red (stale 2-entry expectation
  from 92aaf8da). That turned a would-be surprise into a plan step AND supplied
  the fail-first evidence for free - CI history on pre-change code IS the
  failing run, no local sabotage needed.
- Both new tests drive the production path (real catalog asset, real systems)
  and are genuinely falsifiable: the boundary pin fails without the filter
  (proven by master CI), the strip pin fails without the strip (the pre-fix
  seed only ever inserts).
- The out-of-context review pass found the one real defect (R1.1) that
  shared-session eyes missed, again.

## What went wrong

- R1.1 (persisted hidden enablement had no UI exit): the plan swept the READERS
  of the resource I changed (`ModCatalog`) but not the WRITERS/persisters of
  the adjacent state the semantic change touched (`EnabledMods` via
  `save_enabled_mods` + example runs). Hiding the row removed the only UI
  affordance that could correct a persisted enablement. Root cause: the
  consumer sweep was scoped to the changed resource, not to the state whose
  control surface the change removed.

## What to improve next time

- When a change removes or hides a control (row, toggle, button), sweep every
  writer/persister of the state that control managed and ask how that state
  gets corrected without it. Readers-of-the-changed-resource is not enough.

## Action items

- [x] LESSONS.md: new lesson `removed-control-orphans-persisted-state`;
  bumped `out-of-context-review-pass` and sharpened `fail-first-regression-ab`
  with the CI-history variant.
- [x] Spike fix record appended (tasks/20260714-202515/SPIKE.md).
