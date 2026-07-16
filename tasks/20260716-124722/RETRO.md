# Retro: Gauntlet Run 2.0 - a real parkour course

- TASK: 20260716-124722
- BRANCH: feature/gauntlet-2.0 (landed dfee3df0)
- REVIEW ROUNDS: 1 (APPROVE with 1 MINOR + 3 NITs, all addressed)

## What went well

- Geometry-invariant-as-computed-assertion. The course's two promises (gate
  areas never overlap; the racing line stays flyable past the 6x asteroid
  geometric factor) were encoded as rig assertions that MEASURE distances from
  the shipped positions, not eyeballed. I hand-derived every rock's clearance
  before authoring, then the rig proved it - so the geometry was correct on the
  first test run, and the fail-first proof (rock on the line -> -8.2u -> RED)
  was one edit away. This is `authored-vs-derived-values` applied to layout:
  the 6.0x factor came from the engine const, not folklore.
- Rig-before-content borrowed wholesale from broadside_assault.rs: the
  include_str + register-non-start-handlers + fire-event-infos shape
  transferred directly; the only new work was the geometry helpers and an
  OnEnter fire path.
- The spike front-loaded the five authoring hazards, so /work spent its time
  on the actual course, not on rediscovering the soft-lock risk. The
  no-overlap and geometric-factor traps were designed around, not tripped on.
- The out-of-context review (x25) again earned its tokens: it re-derived both
  invariants from the raw RON with its own script, re-ran the sabotage, and
  found a coupling a shared-session eye had no chance on (below).

## What went wrong

- R1.MINOR-1 (load-bearing dependency I did not audit): the mod declared
  `dependencies: ["base", "demo"]` (inherited from v1.0.0), and the demo mod
  silently OVERRIDES reinforced_hull_section by id (health 200 -> 400). So the
  whole "reinforced hull buys crash tolerance" premise rode on demo - which
  also force-enables the demo arena and is slated for removal. Root cause: I
  treated a declared dependency as "provides prototypes" and never asked what
  it OVERRIDES; my NOTES even claimed "base ... for the prototypes" while demo
  was quietly doing balance. Fixed by dropping demo to base-only (honest
  200-health hull, documented, future-proofed against demo's removal).
- First rig run panicked (actions.rs:288): the act-boundary SetSkybox actions
  resolve a cubemap off the AssetServer, which MinimalPlugins lacks.
  broadside's behavior walk never fired a skybox swap, so the borrowed harness
  hadn't needed an asset backend - a `production-faithful-rigs` gap that only
  showed when the new content exercised a new action.
- NIT-1/NIT-2: stale figures in the content header ("80-95u" for 81-102u;
  "crowd JUST off" for rocks 9-17u off the line) - prose written from the
  design intent, not re-checked against the final positions.

## What to improve next time

- When a mod/plugin declares a dependency, audit what that dependency
  OVERRIDES by id (section/prototype overlays), not just what it provides -
  a declared dep can be load-bearing for balance, and "for the prototypes" is
  an incomplete reason. Grep the dep's content for shared ids before writing
  the dependency rationale.
- A borrowed behavior rig inherits only the harness the SOURCE content needed;
  when the new content fires an action the source never did (SetSkybox here),
  expect a missing-resource panic and add the backend the action needs.
- Re-derive header/NOTES figures from the final positions at close-out, the
  same way the rig does - do not ship design-intent numbers as facts.

## Action items

- [x] Ledger: bumped `out-of-context-review-pass` (x25) and
      `authored-vs-derived-values` (x3, geometric-factor/computed-assertion
      variant); added domain lesson `mod-dependency-overrides-are-load-bearing`.
- [ ] Feel/balance playtest of the course is the user's (crash damage vs the
      200-health hull, gravity-well strength, rock tightness to the line);
      findings become `balance` tasks per the plan. Visible timer + clean-run
      bonus is the queued follow-up 20260716-174729 (backlog).
