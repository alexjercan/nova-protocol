# Editor UI rework (baseline): wiki-style category rail + component drawer with icons/tooltips; player-only asteroid+planetoid scenario

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.6.0,editor,ui,spike

Spike: tasks/20260714-204059/SPIKE.md

Goal (BASELINE slice of the editor rework - see the spike for the full design and
the deferred "the rest"): re-work the sandbox editor UI to feel like the web
wiki's `/wiki/sections/` page.

- Restyle `nova_editor` to the wiki palette/typography (industrial HUD: bg
  #0b0f1c, panels #141a2e, borders #233052 / #3a4d7a, cyan #5cc8ff / amber
  #ffb877 accents, 2px radius, 1px borders, crisp hover) via a shared `theme`
  module inside the crate.
- Category RAIL on the left: `Components` active; `Ships`, `Objects`, `Events`,
  `Objectives` as greyed, non-clickable coming-soon rows (amber "soon" badge)
  that advertise "the rest".
- DRAWER with a CARD GRID for Components: one card per `GameSections` entry -
  placeholder hatched icon tinted by `SectionKind` (behind a `component_icon(kind)`
  helper so real art can drop in later) + name; hover shows a tooltip with name +
  HP + description (all read from `SectionConfig.base` + `SectionKind`; no new data
  model). Selecting a card arms placement, replacing today's flat tool-button list.
  Keep `Select/Rebind` + `Delete` tools and the per-section keybind chip (the one
  "simple modification").
- Placement mechanics unchanged (raycast + normal offset), driven from the card
  selection.
- Baseline `Play` SCENARIO: an asteroid field + a planetoid backdrop (reuse
  `nova_scenario::objects::asteroid` / `PlanetHeight`) with the PLAYER ship ONLY -
  drop the enemy ship and the destroy objective.
- Split the 2001-line `nova_editor/src/lib.rs` into modules (theme, rail, drawer,
  card, tooltip, placement, scenario) as part of the rework.

Verify via the `09_editor` example (BCS_AUTOPILOT headless + interactive) and the
menu -> sandbox path; keep the autopilot placement flow green.

Explicitly NOT in this task (deferred to "the rest", task 20260714-081703):
export/load `*.scenario.ron`, placing non-ship objects, events/objectives wiring,
factions, modifications beyond keybinds, real icons + an `icon` field in content
RON.

Still a `spike` tag: plan into Steps with `/plan` before building (per the spike's
Recommendation section).
