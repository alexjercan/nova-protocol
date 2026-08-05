# Decision: Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- DATE: 20260805-112222
- STATUS: ACCEPTED
- TASK: 20260805-105154
- TAGS: screenshots, examples, scene-design, web

## Context

The 27 `capturable` web images are missing, and the scenes that would produce
them are not worth photographing: one hardcoded top-down `DirectionalLight` per
scenario (`crates/nova_scenario/src/loader/lifecycle.rs:203`), a showcase ship
made of engine primitives, an asteroid ring scattered 90-180 units out of frame,
and a combat range holding two ships and nothing else. Capture plumbing is not
the problem - `--report`, the manifest and the packaging script landed in
`0ff077ff`.

The tree already holds what the scenes need: 123 section prototypes including
the cut-up Kenney hulls (`racer_cube_*`, `cargoa_*`, `cargob_*`), whole ships
already assembled from them in the shipped menu backdrops, an allegiance model
that lets two AI flights be hostile to each other with no player present, and
`Bloom::NATURAL` + `TonyMcMapface` on every scenario camera.

See `NOTES.md` for the full inventory, the four ranked ideas and the per-scene
map designs.

## Decision

Six screenshot examples, each owning its scene data and beats, over one bounded
shared kit:

| Example | Shots | Scene |
| --- | ---: | --- |
| `screenshot_scene` | 3 | "Drydock drift" - planetoid, near-field rocks, a hero Kenney racer, drifting hulls on AI orbit |
| `screenshot_flight` | 3 | "The ring" - player racer on the ORBIT autopilot, HUD on |
| `screenshot_combat` | 10 | "Rock hollow" - two Kenney flights split by allegiance in a dense field, plus a player ship for the lock/HUD beats |
| `screenshot_sections` | 5 | frozen five-section ship, clean backdrop, three-point rig |
| `screenshot_ui` | 4 | the shipped app: menu, editor, settings, campaigns picker |
| `screenshot_nova_os` | 2 | existing Tab-computer range, web names wired |

Settled with the owner 2026-08-05:

1. **Six examples**, not four - `sections` needs a frozen ship on a clean
   backdrop and `flight` needs a player ship and HUD, both of which fight the
   "alive beauty scene" the other shots want.
2. **A shared kit** under `examples/screenshots/`, bounded to three things: the
   three-light photo rig, the Kenney hull section lists lifted from the menu
   backdrops, and a near-field dressing helper. Scene RON and beats stay
   per-example.
3. **Example-side lighting only.** The engine and the scenario RON are not
   touched; authorable scenario lighting is filed as `20260805-111534` (v0.10.0)
   and does not block this.
4. **Scene by scene.** One scene is built, the owner runs it plainly (free-fly
   WASD, no `NOVA_REEL`) and gives a verdict, then the next - so the look
   converges on scene one instead of all six needing the same correction.
   `screenshot_scene` goes first; the AI-vs-AI faction fight is proven inside
   the `screenshot_combat` turn, before that scene is built out.
5. **Round one captures nothing.** No PNG ships until the owner accepts every
   scene. No new art assets.
6. Scope is the 27 `capturable` images. The 7 `manual` thumbnails and 25
   `historical` news figures stay outstanding.

The four `ALIASES` each become their own framed beat, so `ALIASES` empties out
of the manifest. `devlog5-target-viewfinder` and the `devlog5-radar-stance-slots`
composite are dropped - the site references neither.

## Alternatives considered

- **Six self-contained examples, no shared kit.** Rejected: a Kenney racer hull
  is ~100 lines of RON section entries and four examples want one, and a
  lighting tweak becomes six edits. Reconsider only if the scenes turn out to
  want genuinely different rigs.
- **Four examples** (`scene`, `combat`, `ui`, `nova_os`), folding the section
  closeups into `scene`. Rejected: one example would serve a frozen macro rig
  and an alive drifting scene at once, which is a mode switch per shot.
- **Build the scenes as shipped scenario content** under `assets/base/scenarios/`
  so players could fly them. Rejected: shipped content carries balance, lint and
  mod-format obligations, and a scene built for one camera angle fails them. The
  example-owned-data convention exists for exactly this
  (`examples/screenshots/data/reel.content.ron`).
- **One big showcase scene reused by all six examples.** Rejected: the examples
  need contradictory states - frozen vs alive, HUD vs none, player vs no player,
  clean vs dense.
- **Add authorable lights to the scenario RON now**, so the shipped menu
  backdrops improve too. Deferred to `20260805-111534`: it widens this task
  across the loader, lint, mod format and docs, and the capture examples can
  light themselves today.
- **Build all six scenes, then review.** Rejected by the owner in favour of
  scene-by-scene review, so the look is settled once.

## Consequences

- The shipped menu backdrops keep the flat single light until `20260805-111534`
  lands. First impression of the game is unchanged by this task.
- Six review rounds instead of one. Wall-clock is longer; rework is smaller.
- The AI-vs-AI faction fight is inferred from the relation model and unproven -
  if two AI flights will not engage each other, the `screenshot_combat` scene
  design changes, and that is found during its turn rather than at the end.
- `wiki-settings` and the three `news-090-*` shots get manifest slots when their
  producer is settled, not before, so `--report` keeps listing them as
  `capturable` with no producer in the meantime.
- The packaging script, `--report` and the manifest shape are inputs, not
  deliverables. Wiring `--report` into CI as a warning-only job stays unowned.
