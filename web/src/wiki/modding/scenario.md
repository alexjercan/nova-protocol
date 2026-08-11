# The scenario file

The top level of every piece of authored content. A `*.content.ron` file is a
RON LIST of content items - `[ ... ]` - and each item is one of three kinds,
externally tagged by name:

| item | payload | registry | documented |
|---|---|---|---|
| `Scenario((..))` | a whole playable scenario or backdrop | `GameScenarios`, keyed by `id` | this page |
| `Section((..))` | a ship-part prototype | `GameSections`, keyed by `base.id` | [Author a section](../../dev/guide-author-section/) |
| `Campaign((..))` | an ordered scenario grouping for the picker | `GameCampaigns`, keyed by `id` | this page, below |

One file may mix kinds. Overlay across bundles is last-wins by id: the same id
replaces the earlier item wholesale, a fresh id adds (see the
[base content catalog](../base-content/#the-overlay-rule) for the exact merge
rules). The minimal valid file:

```ron
[
    Scenario((
        id: "my_scenario",
        name: "My Scenario",
        description: "One rock.",
        cubemap: "self://textures/sky.png",
        events: [],
    )),
]
```

## Scenario

`Scenario((..))` holds a `ScenarioConfig`
(`crates/nova_scenario/src/loader/mod.rs`). Fields, mandatory first:

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | unique key in the scenario registry; snake_case slug by convention. What `NextScenario` and campaign member lists target. |
| `name` | string | required | display name (Scenarios picker, outcome overlay). |
| `description` | string | required | the picker's details blurb. |
| `cubemap` | asset ref | required | the skybox image, a SCHEMED path string (`dep://base/textures/cubemap.png` or `self://...`; never bare). |
| `thumbnail` | `Option` asset ref | `None` | picker details image; strict RON `Some("self://thumbnails/x.png")`. Must be a plain 2D image, never a cubemap (the picker skips a non-2D thumbnail with a warning). |
| `hidden` | bool | `false` | `true` keeps the scenario out of the flat Scenarios picker (menu backdrops, mid-story continuations reached by `NextScenario`). Still launchable by id and still listed under its campaign header. |
| `menu_backdrop` | bool | `false` | `true` opts INTO the main-menu backdrop rotation (the menu picks one flagged scenario at random on each entry). Orthogonal to `hidden`; backdrops normally set both. |
| `events` | list of handlers | `[]` | the scenario's entire script (below). Empty is legal - nothing happens. |

Backdrop convention: a `menu_backdrop` scenario should contain a gravity-well
object with entity id `menu_planetoid` for the cinematic menu framing;
without one the menu falls back to the scenario's own camera pose after a
short grace.

## Handlers: the event entry

Each entry in `events` is one handler (`ScenarioEventConfig`): an event name,
optional filters, optional actions.

| field | type | default | meaning |
|---|---|---|---|
| `name` | event kind | required | which event fires this handler - one of the nine [event kinds](../events/), written bare: `OnStart`, `OnDestroyed`, ... |
| `filters` | list | `[]` | ALL listed [filters](../filters/) must pass (logical AND) before the actions run. |
| `actions` | list | `[]` | the [actions](../actions/) to run, in listed order. |

```ron
(
    name: OnDestroyed,
    filters: [ Entity((type_name: Some("asteroid"))) ],
    actions: [ DebugMessage((message: "rock down")) ],
),
```

Dispatch rules an author can rely on:

- Handlers for the same event run in AUTHORED order; actions within a handler
  run in authored order.
- Do not make cross-EVENT logic depend on handler position, though - gate on
  variables instead (the [count-gate idiom](../expressions/#recipes)).
- Every filter in the list must pass; an empty list always passes. An
  `OnStart` handler with no filters just runs once on load.

## Campaign

`Campaign((..))` holds a `CampaignConfig` - the ordered mapping from a
campaign to its member scenarios. The Scenarios picker groups and launches a
campaign's chapters as a unit; `hidden` members stay replayable under the
campaign header.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | stable campaign key, e.g. `"nova_protocol"`. |
| `name` | string | required | the picker's group header. |
| `scenarios` | list of scenario ids | required | members in play order. A dangling id is a lint Error; a duplicate is a Warn. |

```ron
Campaign((
    id: "nova_protocol",
    name: "Nova Protocol",
    scenarios: ["shakedown_run", "broadside", "broadside_gunship", "lifeline", "final_tally"],
))
```

## Lifecycle: load, teardown, retry

What the engine does with this file, in order:

1. **Load** - a scenario is looked up by id and loaded: the previous scenario
   is torn down, the camera and input context spawn, one handler entity is
   created per `events` entry, and `OnStart` fires (after every handler
   exists, so an `OnStart` handler can never miss it).
2. **Live** - events fire, filters gate, actions run. Everything a scenario
   spawns is scenario-SCOPED: tagged so teardown can find it.
3. **Teardown** - switching scenarios (or retrying) despawns every scoped
   entity and clears the whole event world: variables, objectives, story log,
   HUD readouts, pending switches. NOTHING persists across scenarios or a
   retry - a retry is a fresh load.

## The lint gate

`cargo run -p nova_authoring --bin content -- lint --target <dir or id>`
checks what the RON parser cannot: dangling `NextScenario` / filter / action
target ids, duplicate spawn ids, section-prototype ids against the catalog,
ship-section geometry, reserved-variable writes, pacing ranges, plus the
combat balance audit. The SAME checks run in-game when mods merge: a scenario
with Error-level findings REFUSES to start and the player sees a FAILED TO
START report naming each problem. Lint early, lint often - a typo'd id loads
green and misbehaves at spawn time otherwise.

## Traps for the unwary

- The loader uses STRICT RON. An unknown or misspelled field is a hard parse
  error; enum variants use newtype double parens (`Scenario((...))`); every
  `Option` keeps its variant (`thumbnail: Some("x.png")`, never a bare
  string). The full gotcha list is in the
  [RON format reference](../../dev/modding-ron/).
- A scenario with no `Light` object renders BLACK - the engine spawns no
  light of its own. See [Light](../objects/#light).
- `hidden` and `menu_backdrop` are orthogonal flags, not a mode enum: a menu
  backdrop that should not appear in the picker sets both.
- New Game is base-owned: a mod bundle declaring `new_game_scenario` is
  warned and ignored. Mods reach players through the picker (leave `hidden`
  off) or a `NextScenario` chain.
