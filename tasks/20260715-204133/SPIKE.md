# Spike: refactor the developer wiki toward how-to / extension guides

- DATE: 20260715-204133
- STATUS: PROPOSED
- TAGS: spike, docs, web

## Question

The developer wiki (`/wiki/dev/`, task 20260715-195621) moved the durable docs
in as reference pages. The user wants the documentation "refactored to make more
sense and cover other important parts", specifically modding, sections and
scenarios - "mainly on how to ADD NEW THINGS and how the COMPLEX things work".
What information architecture, and what new pages, best serve that?

## Current state

Six dev pages, almost all **reference** ("what exists"):

| Page | Kind today |
|---|---|
| development | reference (build/run/release) |
| architecture | reference (crate map, states, frame flow) |
| sections | reference (components, integrity, damage, ammo) |
| scenario-system | reference (events/filters/actions, variables, objects) |
| modding-ron | reference (RON format, catalog, bundles, cache) |
| mod-portal | reference (portal layout, generator, wire schema) |

Categories: Contributing, Architecture, Modding (technical). This is a flat
"here is each subsystem" layout. It answers "what is X" but never "how do I
add/author X", which is exactly what the user is asking for.

## What the code actually affords (grounded sweep)

Three subagent sweeps mapped the real extension points (file:line anchors in the
task's exploration notes). The finding is a consistent, teachable pattern:

- **Ship sections are a CLOSED enum.** Adding a kind touches ~10 core sites in a
  fixed order: `SectionKind` (base_section.rs) + `SectionDamageClass` and
  `resistance()` (damage.rs) + spawn arm (spaceship.rs) + section Plugin (mod.rs)
  + editor placement (placement.rs) + card tint/glyph (card.rs) + an asset
  prototype (nova_assets/sections.rs) + an example (`examples/0X_*_section.rs`).
  A perfect checklist-style how-to.
- **The scenario engine is CONFIG-OPEN, CODE-CLOSED.** Authors write scenarios
  freely in RON with existing primitives; adding a new event kind / filter /
  action / scenario-object kind is one repeated recipe: add the enum variant
  (`EventConfig` / `EventFilterConfig` / `EventActionConfig` / `ScenarioObjectKind`),
  implement the `EventAction`/`EventFilter` trait (or `EventHandler::new::<T>()`),
  and export it from the module prelude. `NovaEventWorld` is the state/command
  seam an action mutates.
- **Modding has a full lifecycle but no author guide.** author bundle (stemmed
  `*.bundle.ron` + `*.content.ron`) -> local test (add to `mods.catalog.ron` or
  the cache) -> `nova_portal_gen` validate -> publish to the static portal ->
  player fetch/install over the wire. Sharp edges: asset refs are hand-typed
  path strings, no scenario editor, no in-game schema reference, version is an
  opaque string.

The common thread: the "how to extend" knowledge exists in the code as a stable
pattern but is undocumented. That is the gap.

## Proposed information architecture (Diataxis-lite)

Split the dev wiki by READER INTENT, not just by subsystem. Three groups:

1. **Get started** (orientation + build)
   - `development` (keep) - build, run, test, release.
   - NEW `project-tour` - "you have 20 minutes": the crate map at a glance, where
     each kind of thing lives, the app boot path, and a "if you want to change X,
     start in file Y" table. The fast-onboarding page the user asked for.

2. **Understand** (how the complex parts work - reference + explanation)
   - `architecture` (keep) - crates, states, frame flow (already has the diagrams).
   - `sections` (keep) - the integrity/damage model.
   - `scenario-system` (keep) - the event/filter/action + variables/world model.
   - `modding-ron` + `mod-portal` (keep) - the data format and the portal/merge
     pipeline.
   These stay as the "what/why" reference, lightly sharpened with a one-line
   "to extend this, see <guide>" pointer at the top.

3. **Extend the game** (how-to guides - the NEW value)
   - NEW `guide-add-section` - the ~10-step checklist to add a ship-section kind,
     grounded in the real touch-points, ending at a runnable example.
   - NEW `guide-extend-scenarios` - add an event kind / filter / action / object
     kind (the enum+trait+prelude recipe, one worked example each).
   - NEW `guide-author-scenario` - write a scenario in RON end to end, built up
     from the shipped `asteroid_field` / `shakedown_run` examples (objectives via
     variables + filters + actions).
   - NEW `guide-make-a-mod` - the mod author lifecycle end to end: bundle layout,
     local testing, `nova_portal_gen` publish, what the player sees. Call out the
     sharp edges honestly.

Sidebar categories become: **Get started**, **Understand**, **Extend the game**
(replacing Contributing / Architecture / Modding (technical)). Each guide
cross-links to its reference page and vice versa.

Bevy migration notes stay in `docs/` (user decision, done in 195621).

## Options considered

- **A. Add guides, keep the flat reference (recommended).** Keep the six pages,
  regroup into intent-based categories, add the 4-5 guides + the project tour.
  Highest value for least churn; the reference pages are already good.
- **B. Full Diataxis rewrite** (tutorial/how-to/reference/explanation as four
  hard-separated trees). More "correct" but heavy, and splits content that reads
  fine together at this size. Overkill for ~11 pages.
- **C. One giant "extending" page.** Rejected - the three domains are big enough
  that a single page buries each; separate guides are searchable and linkable.

## Recommendation

Option A: intent-based regroup + new "Extend the game" guides + a project-tour
onboarding page, grounded in the extension maps from this spike. The wiki
pipeline (from 195621) already renders markdown with mermaid + code highlighting,
so each guide is just a new `.md` + manifest entry.

## Seeded tasks (for /plan to break into steps)

- Project tour / fast-onboarding page (`guide` category "Get started").
- Guide: add a ship-section kind (checklist + example).
- Guide: extend the scenario engine (event/filter/action/object recipes).
- Guide: author a scenario in RON (worked from the built-ins).
- Guide: make and publish a mod (end-to-end lifecycle + sharp edges).
- Regroup categories + add "to extend this, see <guide>" pointers to the
  reference pages.

## Open questions for the user

- Scope/priority: all four guides + tour, or start with the one or two that
  matter most (modding? sections? scenarios?).
- Audience: the mod-author guide can stay "how to author with existing
  primitives" (data authors) vs the extend-the-engine guides (Rust contributors)
  - keep both, but which is the priority reader?
