# CATALOG - Scenario/Modding Authoring Constructs

Raw, mechanical inventory of the RON authoring vocabulary, extracted from the
Rust source of truth. Every enum variant listed; no samples. Seed data for the
Wesnoth-style modding reference.

Sources of truth (all paths repo-relative to /home/alex/personal/nova-protocol):

- crates/nova_events/src/lib.rs - event kinds + payloads
- crates/nova_events/src/engine.rs - dispatch machinery
- crates/nova_scenario/src/events.rs - EventConfig (handler trigger names)
- crates/nova_scenario/src/filters.rs - filter vocabulary
- crates/nova_scenario/src/actions/{mod,flow,mission,ship,spawn,view}.rs - actions
- crates/nova_scenario/src/objects/*.rs - spawnable object kinds
- crates/nova_scenario/src/variables.rs - expression AST
- crates/nova_scenario/src/loader/{mod,clock,lifecycle,trackers}.rs - top-level
  scenario shape, reserved variables, event bridges
- crates/nova_scenario/src/world.rs - NovaEventWorld (variable store, sync)
- crates/nova_modding/src/lib.rs + crates/nova_mod_format/src/lib.rs - content
  file / bundle / catalog shapes
- crates/nova_ship/src/sections/*.rs - ship-section prototype vocabulary
- Shipped usage confirmed against assets/base/scenarios/*.content.ron,
  assets/base/base.bundle.ron, assets/mods/example/*.

Counts: 9 event kinds, 3 filter kinds (5 counting the Conditional combinators),
22 action kinds, 5 spawnable object kinds (+ the CreateScenarioArea trigger
volume), 5 ship-section kinds, 15 expression-AST variants across 5 node enums.

RON notation conventions used throughout (derived from the serde derives):

- All config enums are externally tagged: the variant name is written in the
  data, e.g. `OnStart`, `Entity((..))`, `SpawnScenarioObject((..))`.
- Newtype/struct payloads inside a variant take double parens:
  `Variant((field: value, ...))`. Enum struct-variants take single parens with
  named fields: `Directional(illuminance: 11000.0, ...)`,
  `Box(min: (..), max: (..))`.
- Strict RON keeps Option variants explicit: `thumbnail: Some("x.png")`,
  `dwell: Some(12.0)` - never a bare value. Omitting a serde-defaulted field is
  always legal.
- Vec3 is written as a tuple `(x, y, z)`; Quat as `(x, y, z, w)`; Color as
  e.g. `Srgba((red: 1.0, green: 0.96, blue: 0.9, alpha: 1.0))` (bevy serde).
- Asset references are bare path strings (AssetRef, section 0.3):
  `cubemap: "self://textures/cubemap.png"`.
- `ron::de` runs with the default recursion limit (128), so over-deep authored
  nesting is a parse error, not a stack overflow
  (crates/nova_scenario/src/variables.rs:318-341).

--------------------------------------------------------------------------------

## 0. File-level shapes: content files, bundles, catalog

### 0.1 Content item - `Content` enum

Source: crates/nova_modding/src/lib.rs:68

A `*.content.ron` file is a RON `Vec<Content>` (a top-level `[ ... ]` list).
One file may mix kinds. Externally tagged; one loader reads any content file
and the bundle merge routes each item into its id-keyed registry. Overlay is
last-wins by id (base loads first, then mods in catalog order): same id
replaces, fresh id adds (assets/mods/example/example.content.ron header).

| RON variant | payload | registry | meaning |
|---|---|---|---|
| `Section((..))` | `SectionConfig` (boxed in Rust; RON shape unchanged) | `GameSections` | ship-section prototype, referenced by id from ships |
| `Scenario((..))` | `ScenarioConfig` | `GameScenarios` | a playable/backdrop scenario |
| `Campaign((..))` | `CampaignConfig` | `GameCampaigns` | ordered scenario-id grouping for the picker |

Minimal file:

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

### 0.2 Bundle manifest - `BundleManifest` (`*.bundle.ron`)

Source: crates/nova_mod_format/src/lib.rs:78 (re-exported by nova_modding)

A mod (the base game included - `assets/base/base.bundle.ron`) is a directory
plus this manifest. The manifest, not directory listing, defines the bundle
(wasm-safe: `load_folder` is broken on web).

| field | type | default | meaning |
|---|---|---|---|
| `content` | `Vec<String>` | required | content-file paths relative to the manifest's directory |
| `resources` | `Vec<String>` | `[]` | binary files the bundle ships (PNG/GLB/WAV...), manifest-dir-relative; every `self://` content ref must name one (portal generator, static lint and runtime gate all enforce); sidecar `.meta` files ride along unlisted |
| `meta` | `ModMeta` | all-default | the mod's self-description (below) |
| `new_game_scenario` | `Option<String>` | `None` | scenario id New Game launches; honored ONLY from the catalog's `base: true` bundle (merge warns and ignores elsewhere); strict RON `Some("shakedown_run")` |

`ModMeta` (crates/nova_mod_format/src/lib.rs:44) - every field serde-defaulted:

| field | type | default | meaning |
|---|---|---|---|
| `name` | `String` | `""` | display name (falls back to catalog id) |
| `description` | `String` | `""` | one-liner for the mods list |
| `author` | `String` | `""` | credit line |
| `version` | `String` | `""` | opaque semver-ish; empty = unversioned (base leaves it empty; the portal requires non-empty) |
| `dependencies` | `Vec<String>` | `[]` | mod ids; `base` is implicit and never listed; resolved topologically |
| `icon` | `Option<String>` | `None` | bundle-dir-relative path (reserved) |
| `screenshots` | `Vec<String>` | `[]` | bundle-dir-relative paths (reserved) |

### 0.3 Asset references - `AssetRef<A>` and the path schemes

Source: crates/nova_gameplay/src/asset_ref.rs:29

`AssetRef<A>` is `Path(String) | Handle(Handle<A>)`; it (de)serializes as the
bare path string and resolves lazily via the AssetServer at spawn/load time.
Serializing a handle-backed ref errors (why ScenarioConfig has no Default).

Path schemes (crates/nova_mod_format/src/lib.rs:83-92, base.bundle.ron):

- `self://<path>` - resolves against the bundle's OWN folder (rewritten at
  merge time to `mods/<id>/...` or `mods://<id>/...`; base -> `base/...`).
- `dep://<modid>/<path>` - another bundle's resource, e.g.
  `dep://base/icons/comms.png`.
- plain `path` - asset-root-relative (engine assets).

### 0.4 Installed catalog - `CatalogManifest` (`mods.catalog.ron`)

Source: crates/nova_mod_format/src/lib.rs:139 (entries: `ModEntry`, :116)

| field (ModEntry) | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | stable id; enable/disable key and overlay namespace |
| `bundle` | `String` | required | the mod's `*.bundle.ron` path, asset-root-relative |
| `base` | `bool` | `false` | marks the base game entry (enabled + locked on) |
| `hidden` | `bool` | `false` | dev/tooling mod: loads, omitted from the player-facing list |

--------------------------------------------------------------------------------

## 1. Scenario top level

### 1.1 `ScenarioConfig`

Source: crates/nova_scenario/src/loader/mod.rs:156

The payload of `Scenario((..))`. RON writes the struct body directly inside
the variant's parens. No `Default` (a defaulted cubemap cannot serialize);
in-repo Rust builders use `ScenarioConfig::new(id, name, cubemap)`
(loader/mod.rs:218; debug-asserts that `id` is a snake_case slug).

| field | type | serde default | meaning |
|---|---|---|---|
| `id` | `ScenarioId` (= `String`) | required | unique key in `GameScenarios`; snake_case slug by convention |
| `name` | `String` | required | display name (picker, failure overlay) |
| `description` | `String` | required | picker details blurb |
| `cubemap` | `AssetRef<Image>` | required | skybox cubemap path; resolved at load (loader/lifecycle.rs:198 spawns the camera with a `PendingSkyboxSwap` at brightness 1000) |
| `thumbnail` | `Option<AssetRef<Image>>` | `None` | picker details image; strict RON `Some("self://thumbnails/x.png")` |
| `hidden` | `bool` | `false` | hides from the flat Scenarios picker (backdrops, mid-story chapters reached via NextScenario); still launchable via campaign rows |
| `menu_backdrop` | `bool` | `false` | opts INTO the main-menu ambience rotation (menu picks one flagged scenario at random). Orthogonal to `hidden`. Convention: a backdrop contains a gravity well with entity id `menu_planetoid` for the cinematic framing; without one the menu falls back to the scenario camera pose after a grace |
| `events` | `Vec<ScenarioEventConfig>` | `[]` | the scenario's handlers (the entire script) |

```ron
Scenario((
    id: "example_arena",
    name: "Example Arena",
    description: "A mod-shipped playground.",
    cubemap: "self://textures/nebula.png",
    thumbnail: Some("self://thumb.png"),
    hidden: false,
    menu_backdrop: false,
    events: [ /* handlers */ ],
))
```

### 1.2 `ScenarioEventConfig` - one handler

Source: crates/nova_scenario/src/loader/mod.rs:254

| field | type | serde default | meaning |
|---|---|---|---|
| `name` | `EventConfig` | required | which event kind this handler reacts to (section 2) |
| `filters` | `Vec<EventFilterConfig>` | `[]` | ALL must pass (logical AND) for the actions to run (engine.rs:155-158) |
| `actions` | `Vec<EventActionConfig>` | `[]` | run in listed order after the filters pass |

```ron
(
    name: OnDestroyed,
    filters: [ Entity((id: Some("gate_1"))) ],
    actions: [ DebugMessage((message: "gate down")) ],
)
```

At load, `on_load_scenario` (loader/lifecycle.rs:222-236) turns each entry into
an `EventHandler<NovaEventWorld>` entity tagged `ScenarioScopedMarker`, then
fires `OnStart` (lifecycle.rs:244). One handler entity per event entry;
handlers dispatch in spawn order = authored order (engine.rs:364-370).

### 1.3 `CampaignConfig`

Source: crates/nova_scenario/src/loader/mod.rs:135

The payload of `Campaign((..))`. First-class content entity; owns its ordered
membership (hidden members included, so they list for replay under the
campaign header).

| field | type | default | meaning |
|---|---|---|---|
| `id` | `CampaignId` (= `String`) | required | stable key, e.g. `"nova_protocol"` |
| `name` | `String` | required | display name, e.g. `"Nova Protocol"` |
| `scenarios` | `Vec<ScenarioId>` | required | member ids in play order; dangling id = lint Error, duplicate = Warn (lint/scenario.rs:20-31) |

```ron
Campaign((
    id: "nova_protocol",
    name: "Nova Protocol",
    scenarios: ["shakedown_run", "broadside", "broadside_gunship", "lifeline", "final_tally"],
))
```

Shipped example: assets/base/campaigns/nova_protocol.content.ron.

### 1.4 Related runtime resources (not authored, but the registry surface)

- `GameScenarios` (loader/mod.rs:55) - `HashMap<ScenarioId, ScenarioConfig>`.
- `GameCampaigns` (loader/mod.rs:66) - `HashMap<CampaignId, CampaignConfig>`.
- `NewGameStart` (loader/mod.rs:76) - base bundle's `new_game_scenario`.
- `CurrentScenario` (loader/mod.rs:315), `LoadScenario` (:274),
  `UnloadScenario` (:279), `ScenarioLoaded` (:285 - carries `scenario_id`,
  `handler_count`, `object_count`).
- `ContentIssues` (loader/mod.rs:84) + `ScenarioStartFailure` (:105) - the
  runtime lint gate; Error-level findings make the load REFUSE and drive the
  Wesnoth-style "Failed to start" overlay.
- `ScenarioScopedMarker` (loader/mod.rs:320) - every scenario-scoped entity;
  all of them despawn at unload.

--------------------------------------------------------------------------------

## 2. Event kinds

### 2.1 `EventConfig` - the handler trigger enum (RON `name:` values)

Source: crates/nova_scenario/src/events.rs:21 (lowering: :50-64).
Underlying event kinds + payloads: crates/nova_events/src/lib.rs.

All 9 variants are unit variants in RON: `name: OnStart` etc. Payload fields
below are the JSON keys exposed to `Entity` filters via `GameEventInfo.data`
(serialized `On*EventInfo`; serde renames pin the keys - nova_events/src/lib.rs:55-61:
`id`, `type_name`, `other_id`, `other_type_name`).

| RON name | engine name | payload fields | fired when | fired by |
|---|---|---|---|---|
| `OnStart` | `onstart` | (none) | once, right after a scenario loads (after handler spawn) | loader/lifecycle.rs:244 |
| `OnUpdate` | `onupdate` | (none) | every frame while a scenario is live AND unpaused | pulse, loader/clock.rs:107 (`register_clock_and_pulse` :95 chains clock -> player-speed -> pulse under `scenario_is_live && Unpaused`) |
| `OnDestroyed` | `ondestroyed` | `id`, `type_name` | a scenario object is destroyed (despawn class) | asteroid destruction (objects/asteroid.rs:243), ship-section explosion pipeline (nova_gameplay/src/integrity/explode.rs:179) |
| `OnNeutralized` | `onneutralized` | `id`, `type_name` | an armed combatant loses ALL working weapons AND ALL working thrusters; hull may be intact, ship stays in the world (distinct from OnDestroyed) | nova_gameplay/src/integrity/neutralize.rs:134 |
| `OnEnter` | `onenter` | `id` (area), `other_id`, `other_type_name` (entering body) | a body's FIRST collider contact with a trigger area (refcounted 0 -> 1) | objects/area.rs:91 (also fired by salvage crates and area-radius beacons, which ARE areas) |
| `OnExit` | `onexit` | `id` (area), `other_id`, `other_type_name` (leaving body) | a body's LAST collider leaves the area (refcounted 1 -> 0) | objects/area.rs:139 |
| `OnOrbit` | `onorbit` | `id` (well), `other_id`, `other_type_name` (ship) | a ship has HELD an engaged autopilot ORBIT around one well for the hold window; RECURS every window while the hold continues | orbit-hold tracker, loader/trackers.rs:55; window = ship's `OrbitHoldSecs` override else `ORBIT_HOLD_SECS` = 5.0 s (trackers.rs:18) |
| `OnTravelLock` | `ontravellock` | `id` (locked target), `other_id`, `other_type_name` (player ship) | the PLAYER's travel (white/nav) lock lands on a scenario object; fires on acquisition, then RECURS every re-fire period while held | player-lock bridge, loader/trackers.rs:186; period = `LockRefireSecs` override else `LOCK_REFIRE_SECS` = 5.0 s (trackers.rs:131) |
| `OnCombatLock` | `oncombatlock` | same as OnTravelLock | the PLAYER's combat (red) lock lands on a scenario object; same acquisition + re-fire contract | same bridge, separate slot (trackers.rs:214-219) |

Notes:

- OnEnter/OnExit/OnOrbit/OnTravelLock/OnCombatLock all share the
  (id, other_id, other_type_name) shape ON PURPOSE so entity filters compose
  identically (nova_events/src/lib.rs:170-198).
- OnOrbit / the lock pair recur BY DESIGN: a one-shot event consumed while a
  beat guard rejects it would be gone for good and soft-lock the scenario;
  beat-gated handlers make repeats no-ops (trackers.rs:13-18, :126-131).
- The lock bridges and orbit tracker measure windows against the scenario
  clock (`scenario_elapsed`), so they freeze under pause and reset on
  teardown/retry (trackers.rs:50-54).
- OnDestroyed/OnNeutralized carry the DESTROYED/NEUTRALIZED entity in
  `id`/`type_name`; there is no `other_*` on them.
- A non-positive/non-finite `orbit_hold_secs`/`lock_refire_secs` override is a
  content_lint error and falls back to the default at runtime
  (trackers.rs:24-29, `resolve_window_secs`).

Example handler heads (from shipped content):

```ron
(name: OnEnter, filters: [Entity((id: Some("asteroid_zone"), other_id: Some("player_spaceship")))], actions: [...])
(name: OnDestroyed, filters: [Entity((type_name: Some("asteroid")))], actions: [...])
```

### 2.2 Dispatch semantics (what "fires" means)

Source: crates/nova_events/src/engine.rs.

- `commands.fire::<E>(info)` triggers a `GameEvent` (engine.rs:240-244); an
  observer pushes it onto `GameEventQueue` (:309).
- Each frame's PostUpdate chain: `world_to_state` -> `queue_system` ->
  `state_to_world`, gated on queue-non-empty OR event-world-changed
  (engine.rs:286-297).
- `queue_system` (engine.rs:404) drains the queue FIFO; for each event it walks
  the handlers registered for that name IN SPAWN ORDER (EventHandlerIndex,
  :346), runs `filter` (ALL filters ANDed, :155), then each action in order.
- Actions mutate only `NovaEventWorld`; world effects apply when
  `state_to_world_system` drains the queued commands the same frame
  (world.rs:237-249). So: within one handler, actions order-execute against the
  event world immediately, but their WORLD effects (spawns, despawns) land
  together at the drain; an attach ordered after a spawn in the same handler
  does see the fresh entity (mission.rs:227-231).
- Payload serialization failure => `data: None` => every Entity filter reads
  "no match" permanently (loud error; engine.rs:174-187).

--------------------------------------------------------------------------------

## 3. Filter kinds

Source: crates/nova_scenario/src/filters.rs. Top enum `EventFilterConfig`
(:26). A handler's `filters` list is ANDed (engine.rs:155-158). An empty list
always passes.

### 3.1 `Entity` - `EntityFilterConfig`

Source: filters.rs:49 (semantics :77-140).

Match the event payload's identity fields. Every SET field must match; unset
fields match anything. Matching is exact string equality against the payload
keys (`id`, `type_name`, `other_id`, `other_type_name`).

| field | type | serde default | matches payload key |
|---|---|---|---|
| `id` | `Option<String>` | `None` | `id` |
| `type_name` | `Option<String>` | `None` | `type_name` |
| `other_id` | `Option<String>` | `None` | `other_id` |
| `other_type_name` | `Option<String>` | `None` | `other_type_name` |

Fail-closed behaviors (filters.rs:78-137): payload `data: None` => false; a
set field whose key is ABSENT from the payload => false (so `other_id` on an
OnDestroyed handler can never pass - OnDestroyed has no `other_id`).

```ron
Entity((id: Some("beacon_1"), other_id: Some("player_spaceship")))
Entity((type_name: Some("asteroid")))
```

Type-name values are the object-kind constants (section 5): `"asteroid"`,
`"spaceship"`, `"beacon"`, `"salvage_crate"`, `"light"`.

### 3.2 `Conditional` - `ConditionalFilterConfig`

Source: filters.rs:145 (semantics :171-183). Boolean combinators over other
filters; nestable to any depth (subject to RON's 128 recursion limit). Tuple
variants in RON - inner filters are written positionally.

| RON variant | arity | passes when |
|---|---|---|
| `Not(<filter>)` | 1 | the inner filter does NOT pass |
| `Or(<filter>, <filter>)` | 2 | either inner filter passes |
| `And(<filter>, <filter>)` | 2 | both inner filters pass |

(`And` is rarely needed at the top level - the `filters` list is already an
AND; it exists for composing under `Or`/`Not`.)

Shipped example (assets/base/scenarios/final_tally.content.ron:1355):

```ron
Conditional(Or(
    Expression((Equal(
        Term(Factor(Name("picket_a_down"))),
        Term(Factor(Literal(Number(0.0)))),
    ))),
    Expression((Equal(
        Term(Factor(Name("picket_b_down"))),
        Term(Factor(Literal(Number(0.0)))),
    ))),
))
```

### 3.3 `Expression` - `ExpressionFilterConfig`

Source: filters.rs:189 (semantics :191-204). Newtype over a
`VariableConditionNode` (section 6): `Expression((<condition>))`. Evaluates
against the scenario variable store; ignores the event payload entirely.

FAIL-CLOSED: any evaluation error (undefined variable, type mismatch, division
by zero) logs an error and returns false (filters.rs:195-201). Consequence:
shipped content must seed its variables in OnStart; a missing seed soft-locks
instead of misfiring (pinned by filters.rs:372-396).

```ron
Expression((GreaterThan(
    Term(Factor(Name("asteroids_destroyed"))),
    Term(Factor(Literal(Number(4.0)))),
)))
```

--------------------------------------------------------------------------------

## 4. Action kinds

Source: crates/nova_scenario/src/actions/mod.rs:43 (`EventActionConfig`, 22
variants). All variants are newtype wrappers: RON `Variant((fields...))`.
Actions run in authored order after all filters pass. Common failure mode is
warn-and-continue, never panic (each queued command guards its lookups).

"Scoped-only lookup" below = the target is found by `EntityId` among entities
carrying `ScenarioScopedMarker` only. Rationale: ship SECTIONS carry EntityId
too (per-ship ids like `"controller"`), and an unscoped match would hit every
ship's section (spawn.rs:33-38, pinned :560-596).

### 4.1 `DebugMessage` - `DebugMessageActionConfig`

Source: actions/mod.rs:194.

| field | type | default | meaning |
|---|---|---|---|
| `message` | `String` | required | text logged at debug level; no game effect |

```ron
DebugMessage((message: "gate reached"))
```

### 4.2 `VariableSet` - `VariableSetActionConfig`

Source: actions/mod.rs:168.

| field | type | default | meaning |
|---|---|---|---|
| `key` | `String` | required | scenario variable to write (overwrites) |
| `expression` | `VariableExpressionNode` | required | evaluated against CURRENT variables; result stored under `key` |

Evaluation error => logged, variable NOT written (actions/mod.rs:177-187).
Writing a reserved engine variable (`scenario_elapsed`, `player_speed`) is a
content_lint ERROR (clock.rs:43-45). Re-evaluated per event - `n = n + 1`
accumulates (the kill-counter pattern, filters.rs:399-424).

```ron
VariableSet((
    key: "crates_recovered",
    expression: Add(Factor(Name("crates_recovered")), Term(Factor(Literal(Number(1.0))))),
))
```

### 4.3 `Objective` - `ObjectiveActionConfig`

Source: actions/mission.rs:19.

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | opaque handle used by ObjectiveComplete |
| `message` | `String` | required | text shown in the objectives HUD panel |

Adds to the HUD objective list (world.rs:329-339; duplicate id warns).
"Update" an objective by `ObjectiveComplete` + re-`Objective` with new text.

```ron
Objective((id: "obj_beacons", message: "Reach Beacon 1"))
```

### 4.4 `ObjectiveComplete` - `ObjectiveCompleteActionConfig`

Source: actions/mission.rs:184.

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | removes (completes) the HUD objective with this id; missing id warns (world.rs:343-355) |

```ron
ObjectiveComplete((id: "obj_beacons"))
```

### 4.5 `ObjectiveMarkerAttach` - `ObjectiveMarkerAttachActionConfig`

Source: actions/mission.rs:202.

| field | type | default | meaning |
|---|---|---|---|
| `target_id` | `String` | required | `EntityId` of the scoped object to mark (scoped-only lookup) |
| `label` | `String` | required | short name on the gold HUD marker chip |

Inserts `ObjectiveMarkerTarget` on the object; the HUD grows a gold chip with
label + live distance. Re-attach updates the label in place. Missing id warns.

```ron
ObjectiveMarkerAttach((target_id: "beacon_1", label: "BEACON 1"))
```

### 4.6 `ObjectiveMarkerDetach` - `ObjectiveMarkerDetachActionConfig`

Source: actions/mission.rs:261.

| field | type | default | meaning |
|---|---|---|---|
| `target_id` | `String` | required | scoped object to strip the marker from; missing id is a quiet debug (detach-after-despawn is legitimate) |

```ron
ObjectiveMarkerDetach((target_id: "beacon_1"))
```

### 4.7 `HintEmphasisSet` - `HintEmphasisSetActionConfig`

Source: actions/mission.rs:317.

| field | type | default | meaning |
|---|---|---|---|
| `verb` | `String` | required | keybind-dock chip to pulse gold; must be one of `DOCK_VERBS` |

`DOCK_VERBS` (crates/nova_hud/src/keybind_dock.rs:67):
`"STOP"`, `"GOTO"`, `"ORBIT"`, `"CANCEL"`, `"RADAR"`, `"COMPONENT"`, `"RCS"`.
Unknown verbs are refused with a warning. Emphasizing an unavailable verb
REVEALS its chip dimmed and pulses it (tutorial pointer); emphasis never
grants the verb. Cleared by HintEmphasisClear or scenario teardown.

```ron
HintEmphasisSet((verb: "GOTO"))
```

### 4.8 `HintEmphasisClear` - `HintEmphasisClearActionConfig`

Source: actions/mission.rs:354.

| field | type | default | meaning |
|---|---|---|---|
| `verb` | `String` | required | drop the gold emphasis on one chip |

```ron
HintEmphasisClear((verb: "GOTO"))
```

### 4.9 `SpawnScenarioObject` - `ScenarioObjectConfig`

Source: actions/spawn.rs:72 (base :83, kinds :112). Spawns one scenario object
(section 5). The object gets `ScenarioScopedMarker`, `Name`, `EntityId`,
Transform and visibility from `base` (spawn.rs:99-107), plus the kind bundle.

| field | type | default | meaning |
|---|---|---|---|
| `base` | `BaseScenarioObjectConfig` | required | identity + pose (below) |
| `kind` | `ScenarioObjectKind` | required | which object + per-kind config (section 5) |

`BaseScenarioObjectConfig` (spawn.rs:83):

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | the object's scenario `EntityId` (event/filter/action address) |
| `name` | `String` | required | display `Name` |
| `position` | `Vec3` | required | initial world position |
| `rotation` | `Quat` | required | initial world rotation |

```ron
SpawnScenarioObject((
    base: (id: "rock_1", name: "Rock", position: (10.0, 0.0, -40.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Asteroid((radius: 5.0, texture: "self://textures/asteroid.png", health: 100.0, invulnerable: false)),
))
```

Duplicate spawn ids inside one handler = lint Error; the same id across two
handlers (choice fork) = Warn (lint/scenario.rs tests :688-710).

### 4.10 `ScatterObjects` - `ScatterObjectsConfig`

Source: actions/spawn.rs:248 (region :168, cap :239). Declarative procedural
field: spawns `count` clones of `template` across `region`, deterministic per
`seed`. Copy i gets `base.id = "{id_prefix}{i}"` and `base.name =
"{template name} {i}"`.

| field | type | serde default | meaning |
|---|---|---|---|
| `id_prefix` | `String` | required | id prefix for the copies; also satisfies entity-filter id lint (a filter id starting with a scatter prefix lints clean, lint/scenario.rs:748) |
| `count` | `u32` | required | copies; clamped at runtime to `MAX_SCATTER_COUNT` = 4096 (spawn.rs:239; absurd counts are also a lint Error) |
| `seed` | `u64` | required | StdRng seed; same seed = same layout every load |
| `region` | `ScatterRegion` | required | sampling volume (below) |
| `template` | `ScenarioObjectConfig` | required | object each copy clones (any kind) |
| `asteroid_radius` | `Option<(f32, f32)>` | `None` | when set and the template is an Asteroid, randomize each rock's radius in `[lo, hi)` |
| `min_separation` | `Option<f32>` | `None` | minimum centre distance (world units) between a copy and EVERY body scattered so far THIS SCENARIO (across sibling scatters, world.rs:44-50); rejection-sampled 64 tries per copy (`SEPARATION_ATTEMPTS`, spawn.rs:290), unplaceable copies are DROPPED, never overlapped |

`ScatterRegion` (spawn.rs:168) - enum with struct variants:

| RON variant | fields | meaning |
|---|---|---|
| `Box(min: Vec3, max: Vec3)` | `min`, `max` (both required) | uniform per-axis in `[min, max]`; degenerate axis pins to `min` |
| `Ring(center: Vec3 = (0,0,0), inner: f32, outer: f32, y_min: f32, y_max: f32)` | `center` serde-defaulted to origin | horizontal annulus around `center`: uniform angle, radius in `[inner, outer]`, y offset in `[y_min, y_max]` |

Scatter is gameplay content: NEVER thinned by graphics tier (spawn.rs:314-318,
pinned :991).

```ron
ScatterObjects((
    id_prefix: "asteroid_",
    count: 20,
    seed: 433757350076153856,
    region: Box(min: (-100.0, -20.0, -100.0), max: (100.0, 20.0, 100.0)),
    template: ( base: (...), kind: Asteroid((...)) ),
    asteroid_radius: Some((1.0, 3.0)),
    min_separation: Some(40.0),
))
```

### 4.11 `DespawnScenarioObject` - `DespawnScenarioObjectActionConfig`

Source: actions/spawn.rs:15.

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | scoped-only lookup; recursive despawn (whole child hierarchy); missing id warns |

The complement of SpawnScenarioObject - e.g. remove a salvage crate on pickup.

```ron
DespawnScenarioObject((id: "crate_1"))
```

### 4.12 `SetSpeedCap` - `SetSpeedCapActionConfig`

Source: actions/ship.rs:16.

| field | type | serde default | meaning |
|---|---|---|---|
| `id` | `String` | required | scoped SHIP root (`SpaceshipRootMarker` required) |
| `cap` | `Option<f32>` | `None` | `Some(c)` installs/updates `FlightSpeedCap` (u/s, soft manual-burn taper); `None`/omitted REMOVES the cap |

```ron
SetSpeedCap((id: "player_spaceship", cap: Some(25.0)))
SetSpeedCap((id: "player_spaceship"))            // release the governor
```

### 4.13 `SetControllerVerb` - `SetControllerVerbActionConfig`

Source: actions/ship.rs:108.

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | scoped ship whose controller sections to edit |
| `verb` | `FlightVerb` | required | `Stop` / `Goto` / `Orbit` / `Lock` / `Rcs` (nova_ship controller_section.rs:209) |
| `enabled` | `bool` | required | `true` grants, `false` withholds |

Writes EVERY controller section on the ship (the input layer reads the union).
Materializes `WithheldVerbs` on disable; enable on an absent component is a
no-op. Runtime mirror of the `DisableVerb` section modification (4.24/5.2.6).

```ron
SetControllerVerb((id: "player_spaceship", verb: Goto, enabled: true))
```

### 4.14 `SetAllegiance` - `SetAllegianceActionConfig`

Source: actions/ship.rs:67.

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | scoped ship root to re-align |
| `allegiance` | `Allegiance` | required | `Player` / `Enemy` / `Neutral` (nova_gameplay/src/relations.rs:26) |

The neutral-until-provoked primitive: a Neutral bystander flips Enemy when a
trigger fires this. Dangling id = lint Error (lint/scenario.rs:668).

```ron
SetAllegiance((id: "magpie", allegiance: Enemy))
```

### 4.15 `CreateScenarioArea` - `ScenarioAreaConfig`

Source: actions/spawn.rs:373. Spawns a spherical SENSOR zone that drives
OnEnter/OnExit (section 7.4). Static rigid body + sphere collider + Sensor +
`ScenarioAreaMarker`. Works even when spawned AROUND a body already inside
(the fresh pair still starts, pinned area.rs:209).

| field | type | default | meaning |
|---|---|---|---|
| `id` | `String` | required | the `id` reported by OnEnter/OnExit |
| `name` | `String` | required | display name |
| `position` | `Vec3` | required | sphere centre |
| `rotation` | `Quat` | required | world rotation (cosmetic for a sphere) |
| `radius` | `f32` | required | sensor sphere radius (world units) |

```ron
CreateScenarioArea((id: "asteroid_zone", name: "Asteroid Zone",
    position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), radius: 120.0))
```

### 4.16 `NextScenario` - `NextScenarioActionConfig`

Source: actions/flow.rs:98.

| field | type | serde default | meaning |
|---|---|---|---|
| `scenario_id` | `String` | required | scenario to switch to; dangling id = lint Error; unknown at runtime = error + unload |
| `linger` | `bool` | required | `true` defers the switch until released - the scenario-advance input (Enter / DPadDown) or the outcome overlay's Continue/Retry (world.rs:361-374); `false` switches on the next state sync |
| `delay` | `Option<f32>` | `None` | delayed NON-lingering cut: hold the switch this many seconds while the world keeps playing (virtual, pause-frozen time). Meaningless with `linger: true`. Non-positive/non-finite arms nothing; runtime cap `NEXT_SCENARIO_DELAY_MAX_SECS` = 300 s, lint warns above 60 s (flow.rs:89-91). Strict RON `delay: Some(4.0)` |

Last request wins wholesale (a fresh queue resets the clock, flow.rs:125-135).
Enter during a delay window jumps the beat (release skips the delay,
world.rs:366-370). Composing `Outcome` + non-lingering NextScenario in one
handler is a lint Warn (overlay gets swallowed / frozen; lint tests :811).

```ron
NextScenario((scenario_id: "broadside", linger: true))
NextScenario((scenario_id: "x", linger: false, delay: Some(4.0)))
```

### 4.17 `SetCamera` - `SetCameraActionConfig`

Source: actions/view.rs:29. Photo-mode: pins `ScriptedCameraPose` on the
`ScenarioCameraMarker` camera and drops the free-fly `WASDCameraController`;
the pose is re-enforced every frame in the camera-authority Override phase
(loader/mod.rs:438-441, :461). No camera = warn no-op.

| field | type | default | meaning |
|---|---|---|---|
| `position` | `Vec3` | required | world-space camera position |
| `look_at` | `Vec3` | required | world point to look at (up is +Y) |

```ron
SetCamera((position: (0.0, 30.0, 80.0), look_at: (0.0, 0.0, 0.0)))
```

### 4.18 `Screenshot` - `ScreenshotActionConfig`

Source: actions/view.rs:96. Captures the primary window to PNG. Relative paths
join under the `NOVA_SHOT_DIR` env var when set (view.rs:70-85). Parent dirs
are created. Without a render backend the capture never lands (dev tool).

| field | type | default | meaning |
|---|---|---|---|
| `path` | `String` | required | output PNG path |

```ron
Screenshot((path: "shots/feature-gravity.png"))
```

### 4.19 `SetSkybox` - `SetSkyboxActionConfig`

Source: actions/view.rs:154. Swaps the scenario skybox mid-scenario. Two-step:
tags the camera with `PendingSkyboxSwap` (view.rs:217); the applier
(`apply_pending_skybox_swaps`, view.rs:247) installs `SkyboxConfig` once the
image is loaded, sets the Cube texture view if needed, and drops the swap on a
failed load (sky unchanged). No camera = warn no-op.

| field | type | serde default | meaning |
|---|---|---|---|
| `cubemap` | `AssetRef<Image>` | required | new cubemap path, e.g. `"self://textures/nebula.png"` |
| `brightness` | `Option<f32>` | `None` | multiplier; `None` inherits the camera's current skybox brightness (initial load default 1000.0, view.rs:139) |

```ron
SetSkybox((cubemap: "self://textures/nebula.cube.png", brightness: Some(700.0)))
```

### 4.20 `Outcome` - `OutcomeActionConfig`

Source: actions/flow.rs:28 (kind :13). Declares win/lose; drives the outcome
overlay (writes `CurrentOutcome`, flow.rs:65). Presentation only - what
happens next is composed: pair with `NextScenario(linger: true)` so Enter
continues (Victory) or retries (Defeat); with nothing queued Enter returns to
the menu.

| field | type | serde default | meaning |
|---|---|---|---|
| `outcome` | `ScenarioOutcomeKind` | required | `Victory` (gold banner) or `Defeat` (red banner) |
| `message` | `Option<String>` | `None` | flavor line under the banner; strict RON `Some("...")` |
| `auto_advance_secs` | `Option<f64>` | `None` | timed overlay: after this many REAL seconds (the overlay pauses virtual time) advance the queued LINGERING chain as if Continue were pressed; runtime cap `OUTCOME_AUTO_ADVANCE_MAX_SECS` = 300 s (flow.rs:93); meaningless without a queued lingering NextScenario |

```ron
Outcome((outcome: Defeat, message: Some("The convoy is lost.")))
Outcome((outcome: Victory, auto_advance_secs: Some(6.0)))
```

### 4.21 `StoryMessage` - `StoryMessageActionConfig`

Source: actions/mission.rs:51. Speaker-attributed line for the HUD comms
panel; appends to the scenario-scoped story log (cleared at teardown, no
leakage).

| field | type | serde default | meaning |
|---|---|---|---|
| `speaker` | `String` | required | line prefix in the panel |
| `text` | `String` | required | the line |
| `dwell` | `Option<f32>` | `None` | on-screen hold override, seconds; default hold 8 s; panel clamps to [3, 30]; lint warns outside that range; strict RON `Some(12.0)` |
| `icon` | `Option<AssetRef<Image>>` | `None` | speaker image (`Some("self://icons/okono.png")` or `dep://...`); omitted = HUD fallback tile |

Lint pacing arms: two StoryMessages in one handler warn; story beside Outcome
in one handler warns (lint tests :848).

```ron
StoryMessage((speaker: "Foreman Okono", text: "Strip it clean.", dwell: Some(12.0),
    icon: Some("self://icons/okono.png")))
```

### 4.22 `HudReadout` - `HudReadoutActionConfig`

Source: actions/mission.rs:133 (format enum :90). Shows/updates/clears a named
HUD readout bound to a scenario variable - the display half of the variable
vocabulary. One fire is enough: the HUD tracks the variable's CURRENT value
every frame at sync time (world.rs:129-159). Value freezes under pause /
outcome overlay (final time holds through the Victory banner). All readouts
clear at teardown.

| field | type | serde default | meaning |
|---|---|---|---|
| `slot` | `String` | required | stable readout id; several slots can coexist; addressed for update/clear |
| `variable` | `String` | required | scenario variable displayed (e.g. `"scenario_elapsed"`); undefined/non-numeric reads 0.0 |
| `format` | `HudReadoutFormatConfig` | `Number` | `Number` (one decimal, `12.3`) / `Integer` (rounded, `12`) / `Time` (`mm:ss.s`, `01:23.4`) |
| `label` | `Option<String>` | `None` | caption before the value (e.g. `"TIME"`) |
| `visible` | `bool` | `true` | `true` shows/updates the slot; `false` clears exactly that slot (clear of an unknown slot is a quiet no-op, world.rs:310-326) |

```ron
HudReadout((slot: "timer", variable: "scenario_elapsed", format: Time, label: Some("TIME")))
HudReadout((slot: "timer", variable: "scenario_elapsed", visible: false))
```

--------------------------------------------------------------------------------

## 5. Scenario object kinds

Source: `ScenarioObjectKind` (actions/spawn.rs:112). Five variants, all
newtype: `Asteroid((..))`, `Spaceship((..))`, `Beacon((..))`,
`SalvageCrate((..))`, `Light(<variant>(..))`. Type-name constants tag each
spawned object's `EntityTypeName` for `type_name` filters.

Every object also carries the shared base bundle (spawn.rs:99):
`ScenarioScopedMarker` + `Name` + `EntityId` + Transform + Visibility. The
body (RigidBody) is per-kind: Asteroid and Spaceship are Dynamic, Beacon /
SalvageCrate / Light are Static.

### 5.1 `Asteroid` - `AsteroidConfig`

Source: objects/asteroid.rs:38; type name `"asteroid"` (:30). Noise-generated
destructible rock; radius drives mesh scale, collider, mass and well
qualification together.

| field | type | serde default | meaning |
|---|---|---|---|
| `radius` | `f32` | required | nominal radius (world units); lock signature default; the true geometric extent (`BodyRadius`) is derived from the generated collider (mesh reaches up to `radius * ASTEROID_GEOMETRIC_FACTOR_MAX`) |
| `texture` | `AssetRef<Image>` | required | surface texture path |
| `health` | `f32` | required | hit points; ignored when `invulnerable` |
| `impact_sound` | `Option<AssetRef<AudioSource>>` | `None` | played on hit; authored-or-silent |
| `destroy_sound` | `Option<AssetRef<AudioSource>>` | `None` | played on destruction; authored-or-silent |
| `mass` | `Option<f32>` | `None` | gravity parameter mu (u^3/s^2): `Some` always makes this a well (`a = mu / r^2`, SOI where accel decays to the cutoff; tune as `mu = soi_cutoff_accel * soi^2`); `None` = global rule (default mass iff `radius >= min_well_radius`) |
| `invulnerable` | `bool` | required | collider WITHOUT a health node - can never be destroyed or disabled (its well can never die mid-scenario); `health` ignored |
| `lock_signature` | `Option<f32>` | `None` | radar signature override; `None` = the radius (rocks lock in proportion to size) |

```ron
Asteroid((
    radius: 60.0,
    texture: "self://textures/asteroid.png",
    health: 100.0,
    mass: Some(12000.0),
    invulnerable: true,
))
```

Destruction fires `OnDestroyed` with the asteroid's id / `"asteroid"`
(asteroid.rs:243).

### 5.2 `Spaceship` - `SpaceshipConfig`

Source: objects/spaceship.rs:232; type name `"spaceship"` (:30). A ship is a
root plus a tree of section children.

| field | type | serde default | meaning |
|---|---|---|---|
| `controller` | `SpaceshipController` | required | who flies it (5.2.1) |
| `allegiance` | `Option<Allegiance>` | `None` | side override; omitted keeps the controller default: Player ships -> `Player`, AI ships -> `Enemy`. `Some(Neutral)` authors a bystander; the explicit insert wins over the marker's requirement default (spawn.rs:140-150, pinned :424) |
| `sections` | `Vec<SpaceshipSectionConfig>` | `[]` | the hull/thruster/weapon/controller layout (5.2.3) |

#### 5.2.1 `SpaceshipController`

Source: spaceship.rs:38.

| RON variant | payload | meaning |
|---|---|---|
| `None` | - | nobody drives; the ship station-keeps |
| `Player((..))` | `PlayerControllerConfig` | human-driven |
| `AI((..))` | `AIControllerConfig` | bot-driven |

`PlayerControllerConfig` (spaceship.rs:54):

| field | type | serde default | meaning |
|---|---|---|---|
| `input_mapping` | `HashMap<SectionId, Vec<Binding>>` | `{}` | per-section input bindings, keyed by section id; RON values are `BindingInput` forms (5.2.5), bridged by `binding_map_serde` (binding_input.rs:70); serialized in BTreeMap order for stable generated files |
| `speed_cap` | `Option<f32>` | `None` | soft manual-speed cap (u/s) as `FlightSpeedCap`; `None` = unbounded Newtonian burn |
| `infinite_ammo` | `bool` | required (base RON authors it explicitly; Rust Default = false) | weapon sections built with no magazine - guns never run dry; player-scoped |
| `lock_refire_secs` | `Option<f64>` | `None` | re-fire period for held travel/combat locks (OnTravelLock/OnCombatLock recurrence); `None` = 5 s default; non-positive/non-finite = lint error, runtime falls back |

`AIControllerConfig` (spaceship.rs:103):

| field | type | serde default | meaning |
|---|---|---|---|
| `patrol` | `Vec<Vec3>` | `[]` | waypoint loop while nothing hostile is detected; empty = station-keep |
| `orbit` | `Option<String>` | `None` | scenario id of a gravity-well entity to orbit passively; precedence orbit > patrol > idle |
| `leash` | `Option<f32>` | `None` | territorial tether radius from patrol centroid (or spawn); combat breaks off beyond it; `None` = chases freely |
| `engage_delay` | `Option<f32>` | `None` | arrival grace (s): refuses to engage until elapsed; being shot ends it immediately and permanently; strict RON `Some(8.0)` |
| `orbit_hold_secs` | `Option<f64>` | `None` | seconds of HELD engaged orbit before `OnOrbit` fires (and its re-fire period); `None` = 5 s default; only meaningful with `orbit` |

#### 5.2.2 `Allegiance` (nova_gameplay/src/relations.rs:26)

`Player` | `Enemy` | `Neutral`. Player/Enemy are mutually Hostile; Neutral (or
no allegiance) relates Neutral to everyone.

#### 5.2.3 `SpaceshipSectionConfig`

Source: spaceship.rs:200.

| field | type | serde default | meaning |
|---|---|---|---|
| `id` | `SectionId` (= `String`) | required | scenario-local section id; keys `input_mapping` and scripts |
| `position` | `Vec3` | required | offset from the ship root |
| `rotation` | `Quat` | required | rotation relative to the root |
| `source` | `SectionSource` | required | where the section config comes from (5.2.4) |
| `modifications` | `Vec<SectionModification>` | `[]` | spawn-time data deltas (5.2.6) |

#### 5.2.4 `SectionSource`

Source: spaceship.rs:188.

| RON variant | payload | meaning |
|---|---|---|
| `Inline((..))` | full `SectionConfig` (section 5.6) | authored in place |
| `Prototype("id")` | catalog id | resolved against `GameSections` at spawn; unknown prototype = lint Error |

```ron
(id: "cube_i0_j0_k0", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0),
 source: Prototype("racer_cube_i0_j0_k0"))
```

#### 5.2.5 `BindingInput` (input_mapping values)

Source: objects/binding_input.rs:25. Only modifier-free button forms are
authorable; modifiers, mouse motion/wheel, gamepad axes, AnyKey, Custom are
rejected at serialize.

| RON variant | payload | example |
|---|---|---|
| `Keyboard(<KeyCode>)` | bevy `KeyCode` | `Keyboard(KeyW)` |
| `Mouse(<MouseButton>)` | bevy `MouseButton` | `Mouse(Left)` |
| `Gamepad(<GamepadButton>)` | bevy `GamepadButton` | `Gamepad(RightTrigger2)` |

```ron
input_mapping: { "turret_a": [ Mouse(Left), Gamepad(RightTrigger2) ] },
```

#### 5.2.6 `SectionModification`

Source: objects/modification.rs:33. Closed, data-only deltas applied to the
resolved section at spawn (inserted as components; per-variant observers apply
where relevant, inert elsewhere).

| RON variant | payload | meaning |
|---|---|---|
| `DisableVerb(<FlightVerb>)` | `Stop`/`Goto`/`Orbit`/`Lock`/`Rcs` | withhold a flight verb on this controller section from birth; multiple DisableVerbs ACCUMULATE into one `WithheldVerbs` set (modification.rs:52-77); inert on non-controller sections; runtime mirror: `SetControllerVerb` action |
| `SetHealth(<f32>)` | starting health | overrides the section's `Health` (current and max); inert with no Health |
| `Rename(<String>)` | new name | renames the section entity's `Name` |

```ron
modifications: [ DisableVerb(Goto), DisableVerb(Lock), SetHealth(40.0), Rename("Scarred Hull") ]
```

#### 5.2.7 Spawn-derived components (authored indirectly)

- `OrbitHoldSecs` (spaceship.rs:159) from `orbit_hold_secs`.
- `LockRefireSecs` (spaceship.rs:168) from `lock_refire_secs`.
- `FlightSpeedCap` from `speed_cap`.

### 5.3 `Beacon` - `BeaconConfig`

Source: objects/beacon.rs:43; type name `"beacon"` (:22). Static, lockable,
blinking nav marker with an automatic HUD chip (label + distance +
edge-clamped direction cue).

| field | type | serde default | meaning |
|---|---|---|---|
| `label` | `String` | required | HUD chip text ("BEACON 1") |
| `radius` | `f32` | required | visual orb radius (world units) |
| `color` | `Color` | required | base + emissive color (blink sweeps luminance 8..60, period 1.2 s) |
| `area_radius` | `Option<f32>` | `None` | when set, the beacon IS its own trigger area of this radius: OnEnter/OnExit fire under the beacon's id - no separate CreateScenarioArea needed |
| `lock_signature` | `Option<f32>` | `None` | radar signature override; default 20.0 (= 600 u lock range at default settings, beacon.rs:24-28); author bigger for longer GOTO legs |

```ron
Beacon((
    label: "BEACON 1",
    radius: 2.0,
    color: Srgba((red: 0.3, green: 0.9, blue: 1.0, alpha: 1.0)),
    area_radius: Some(40.0),
))
```

### 5.4 `SalvageCrate` - `SalvageCrateConfig`

Source: objects/salvage.rs:48; type name `"salvage_crate"` (:26). Minimal
proximity pickup: a static tumbling prop that IS its own trigger area - flying
through fires OnEnter under the crate's id. No inventory system: "collected"
is scenario state (pair the OnEnter handler with DespawnScenarioObject + a
VariableSet counter). Gets an intrinsic HUD `ItemHighlight` bracket.

| field | type | serde default | meaning |
|---|---|---|---|
| `size` | `f32` | required | visible box edge length (world units) |
| `area_radius` | `f32` | required | pickup sensor sphere radius ("collected" distance) |
| `pickup_sound` | `Option<AssetRef<AudioSource>>` | `None` | pickup ding (player pickups only, deduped per crate); authored-or-silent; base authors `self://sounds/salvage_pickup.wav` |

```ron
SalvageCrate((size: 1.5, area_radius: 8.0, pickup_sound: Some("self://sounds/salvage_pickup.wav")))
```

### 5.5 `Light` - `LightConfig`

Source: objects/light.rs:32; type name `"light"` (:21). Scenes light
THEMSELVES: the engine supplies no default light; a scenario that spawns no
Light objects renders black (lifecycle.rs:204-206). Position/rotation come
from the object base; this enum picks the method. Enum with struct variants -
single parens, named fields.

`Directional` - a sun (parallel rays; only direction matters):

| field | type | serde default | meaning |
|---|---|---|---|
| `illuminance` | `f32` | required | lux (the old engine key light was 10000) |
| `color` | `Color` | required | light color |
| `shadows` | `bool` | required | shadow casting; convention: ONE caster per scene (key light) |
| `aim` | `Option<Vec3>` | `None` | aim the light at this world point, ignoring the base `rotation` (hand-authoring a quaternion is impractical); `None` uses `rotation` |

`Point` - a positional lamp (a star, floodlight, nebula glow):

| field | type | default | meaning |
|---|---|---|---|
| `intensity` | `f32` | required | lumens |
| `range` | `f32` | required | contribution cutoff distance (world units) |
| `radius` | `f32` | required | source radius (softens the terminator) |
| `color` | `Color` | required | light color |
| `shadows` | `bool` | required | shadow casting |

```ron
Light(Directional(
    illuminance: 11000.0,
    color: Srgba((red: 1.0, green: 0.96, blue: 0.9, alpha: 1.0)),
    shadows: true,
    aim: Some((0.0, 0.0, 0.0)),
))
```

Rust-side helper (not RON): `ThreePointRig::around(prefix, target, scale)`
emits the standard key/rim/fill trio (light.rs:157-200).

### 5.6 Ship-section prototype vocabulary (`Section((..))` content items)

Source: crates/nova_ship/src/sections/base_section.rs. Used both as content
prototypes (`Content::Section`) and inline in ships (`SectionSource::Inline`).

`SectionConfig` (base_section.rs:278): `base` + `kind`.

`BaseSectionConfig` (base_section.rs:177):

| field | type | serde default | meaning |
|---|---|---|---|
| `id` | `String` | required (Rust Default: `""`) | catalog id (`GameSections` lookup, `Prototype` target) |
| `name` | `String` | required | editor palette / HUD display name |
| `description` | `String` | required | editor tooltip |
| `mass` | `f32` | required | fed to avian as DENSITY: real mass = `mass * collider_volume` |
| `health` | `f32` | required | section hit points |
| `impact_sound` | `Option<AssetRef<AudioSource>>` | `None` | per-target hit sound (the target is the material) |
| `destroy_sound` | `Option<AssetRef<AudioSource>>` | `None` | destruction sound |
| `collider` | `Option<SectionCollider>` | `None` (= unit cube) | authored physics shape |
| `hide_in_editor` | `bool` | `false` | hidden from the editor palette (cut-cube ship tiles); still spawnable |

`SectionCollider` (base_section.rs:51) - struct variants, avian units
(Cuboid `size` = FULL side lengths; Capsule/Cylinder along local Y):

| RON variant | fields |
|---|---|
| `Cuboid(size: Vec3)` | full side lengths |
| `Sphere(radius: f32)` | - |
| `Capsule(radius: f32, length: f32)` | length = cylindrical segment |
| `Cylinder(radius: f32, height: f32)` | - |

`SectionKind` (base_section.rs:260) - 5 variants, newtype:

| RON variant | config | role |
|---|---|---|
| `Hull((..))` | `HullSectionConfig` (hull_section.rs:16) | passive structure. Fields: `render_mesh: Option<AssetRef<WorldAsset>> = None`, `render_mesh_transform: Option<RenderMeshTransform> = None` |
| `Thruster((..))` | `ThrusterSectionConfig` (thruster_section.rs:31) | directional thrust. Fields: `magnitude: f32` (default 1.0), `render_mesh = None`, `render_mesh_transform = None`, `loop_sound: Option<AssetRef<AudioSource>> = None` (engine hum), `exhaust: Option<ThrusterExhaust> = None` (cone placement/shape) |
| `Controller((..))` | `ControllerSectionConfig` (controller_section.rs:27) | attitude PD + the ship's computer (grants flight verbs). Fields: `frequency: f32` (Hz), `damping_ratio: f32`, `max_torque: f32`, `render_mesh = None`, `render_mesh_transform = None`, plus authored-or-silent cue sounds: `lock_on_sound`, `lock_off_sound`, `radar_deny_sound`, `radar_retarget_sound`, `safety_on_sound`, `rcs_loop_sound` (all `Option<AssetRef<AudioSource>> = None`) |
| `Turret((..))` | `TurretSectionConfig` (turret_section/config.rs:110) | aimed gun. Fields: `root: TurretJoint` (kinematic tree), `muzzle_speed: f32` (u/s), `projectile_lifetime: f32` (s), `bullet_damage: f32` (authored kinetic per hit), `bullet_kind: DamageType`, `projectile_render_mesh = None`, `fire_sound = None`, `dry_fire_sound = None`, `ammo_capacity: Option<u32> = None` (None = unlimited), `reload: Option<SectionReloadConfig> = None` |
| `Torpedo((..))` | `TorpedoSectionConfig` (torpedo_section/mod.rs:52) | guided-torpedo bay. Fields: `render_mesh = None`, `render_mesh_transform = None`, `projectile_render_mesh = None`, `spawn_offset: Vec3`, `spawn_rotation: Quat`, `fire_rate: f32` (rounds/s), `spawner_speed: f32` (muzzle u/s), `projectile_lifetime: f32`, `arm_time: f32`, `arm_distance: f32` (armed at time OR distance), `nav_constant: f32` (PN constant, typical 3-5), `max_speed: f32`, `linear_damping: f32`, `blast_radius: f32`, `blast_damage: f32`, `blast_effect: Option<AssetRef<EffectAsset>> = None`, `launch_effect = None`, plus launch sound field |

`TurretJoint` (turret_section/config.rs:47) - recursive tree node:
`offset: Vec3`; `axis: Option<Vec3> = None` (None = fixed node);
`speed: f32` (default PI rad/s, omitted when default);
`min`/`max: Option<f32> = None` (radian limits);
`render_mesh = None`; `render_mesh_transform = None`;
`muzzle: Option<MuzzleConfig> = None` (`fire_rate: f32`,
`muzzle_effect: Option<AssetRef<EffectAsset>> = None`; config.rs:13);
`children: Vec<TurretJoint> = []`.

`SectionReloadConfig` (ammo.rs:98): `reload_time: f32` (s, must be > 0),
`rounds_per_cycle: u32`, `only_when_empty: bool` (true = discrete reload,
false = regen).

`RenderMeshTransform` (base_section.rs:135): `position: Vec3 = (0,0,0)`,
`rotation: Quat = identity` - moves the render mesh only, never the collider.

--------------------------------------------------------------------------------

## 6. Expression / value AST

Source: crates/nova_scenario/src/variables.rs. Hand-written precedence chain:
condition > expression (add/sub) > term (mul/div) > factor (atom). All
variants are tuple variants in RON (positional, boxed operands written
inline). Evaluation is against `NovaEventWorld`'s variable map.

### 6.1 `VariableLiteral` (variables.rs:36) - the value types

| RON variant | payload | example |
|---|---|---|
| `String(..)` | `String` | `String("act_two")` |
| `Number(..)` | `f64` | `Number(4.0)` |
| `Boolean(..)` | `bool` | `Boolean(true)` |

### 6.2 `VariableFactorNode` (variables.rs:49) - atoms

| RON variant | payload | meaning |
|---|---|---|
| `Parens(<expression>)` | boxed `VariableExpressionNode` | parenthesized subexpression |
| `Literal(<literal>)` | `VariableLiteral` | constant |
| `Name("var")` | `String` | variable lookup; UNDEFINED name = `VariableError::UndefinedVariable` (fails the enclosing filter closed) |

### 6.3 `VariableTermNode` (variables.rs:91) - multiply/divide level

| RON variant | operands | semantics |
|---|---|---|
| `Multiply(<factor>, <term>)` | factor x term | Number*Number = product; Boolean*Boolean = AND; anything else = TypeMismatch |
| `Divide(<factor>, <term>)` | factor / term | Numbers only; divisor 0.0 = DivisionByZero error |
| `Factor(<factor>)` | - | a bare factor |

### 6.4 `VariableExpressionNode` (variables.rs:160) - add/subtract level (the value root)

| RON variant | operands | semantics |
|---|---|---|
| `Add(<term>, <expression>)` | term + expression | Number+Number = sum; Boolean+Boolean = OR; String+String = concat; mixed = TypeMismatch |
| `Subtract(<term>, <expression>)` | term - expression | Numbers only |
| `Term(<term>)` | - | a bare term |

Right recursion: chains associate rightward (`a - b - c` authored as
`Subtract(a, Subtract(b, Term(c)))` means a - (b - c); shipped content only
chains Add, where it does not matter).

### 6.5 `VariableConditionNode` (variables.rs:235) - the boolean root (filters)

| RON variant | operands | semantics |
|---|---|---|
| `LessThan(<expr>, <expr>)` | numeric | `l < r`; non-numbers = TypeMismatch |
| `GreaterThan(<expr>, <expr>)` | numeric | `l > r`; non-numbers = TypeMismatch |
| `Equal(<expr>, <expr>)` | same-type | Numbers: `abs(l - r) <= EQUAL_EPSILON` (1e-6, variables.rs:229 - exact float equality burned an author once); Booleans/Strings: exact; mixed types = TypeMismatch |

There is no NotEqual / LessOrEqual / GreaterOrEqual variant - compose with the
filter-level `Conditional(Not(..))`, or flip the comparison.

### 6.6 Canonical spellings

Read a variable as a bare value expression:
`Term(Factor(Name("scenario_elapsed")))`.
A number: `Term(Factor(Literal(Number(1.0))))`.

Full examples (shipped):

```ron
// filter: asteroids_destroyed > 4.0
Expression((GreaterThan(
    Term(Factor(Name("asteroids_destroyed"))),
    Term(Factor(Literal(Number(4.0)))),
)))

// action: counter += 1
VariableSet((
    key: "asteroids_destroyed",
    expression: Add(
        Factor(Name("asteroids_destroyed")),
        Term(Factor(Literal(Number(1.0)))),
    ),
))

// action: flag := true
VariableSet((
    key: "objective_destroy_asteroids",
    expression: Term(Factor(Literal(Boolean(true)))),
))
```

--------------------------------------------------------------------------------

## 7. Cross-cutting mechanics

### 7.1 The variable store and sync loop (`NovaEventWorld`)

Source: crates/nova_scenario/src/world.rs:32.

- One flat `HashMap<String, VariableLiteral>`; `VariableSet` overwrites,
  `Expression` filters read. No namespaces, no scoping - the whole scenario
  shares one store.
- Cleared WHOLESALE at scenario teardown (`clear`, world.rs:256-275) along
  with objectives, story log, HUD readouts, scatter placements, queued
  commands, and any pending scenario switch. Nothing persists across
  scenarios or a retry.
- Actions never touch the bevy world directly: they mutate the event world or
  `push_command` closures; `state_to_world_system` (world.rs:72) drains
  everything each PostUpdate - objectives (write-on-diff), story feed,
  HUD readouts (live variable values re-read per frame), the queued
  NextScenario switch (with delay ticking), then the command queue in push
  order (:237-249).
- Undrained commands die with the scenario at teardown (world.rs:260-267) -
  which is how an `Outcome` composed with an INSTANT (`linger: false`) switch
  gets swallowed; hence the lint warn (4.16).

### 7.2 Reserved engine variables

Source: crates/nova_scenario/src/loader/clock.rs.

| name | type | written by | meaning |
|---|---|---|---|
| `scenario_elapsed` | Number | `tick_scenario_clock` (clock.rs:51), every live+unpaused frame | seconds of live, unpaused scenario time; freezes under pause; clears at teardown (a retry restarts it); read before first tick fails closed (undefined) |
| `player_speed` | Number | `track_player_speed` (clock.rs:67) | player ship's live speed (u/s, LinearVelocity magnitude); 0.0 when no player ship exists |

Authors GATE on these, never write them: a `VariableSet` on either key is a
content_lint ERROR; reading them needs no seeding (exempt from the
unset-variable warn). Single source of truth: `is_reserved_engine_var`
(clock.rs:43).

Clock idioms documented in source (clock.rs:20-23):
- one-shot at time T: `Expression((GreaterThan(elapsed, N)))` + act-flag gate,
  then advance the flag;
- repeating waves: gate on `elapsed > next_at`, then
  `VariableSet(next_at, Add(next_at, interval))`.

### 7.3 The count-gate idiom (and act/flag gates)

The composition every shipped scenario uses (pinned in filters.rs:314-424 and
visible in assets/base/scenarios/asteroid_field.content.ron:473-505):

1. Counter: an `OnDestroyed`/`OnEnter` handler with an Entity filter runs
   `VariableSet(n, Add(n, 1))`.
2. Gate: a second handler (often `OnUpdate`, so it does not depend on handler
   order within another event) filters
   `Expression((GreaterThan(n, K)))` AND a boolean act flag
   `Expression((Equal(flag, Boolean(false))))`, then sets the flag true and
   fires the beat's actions.

The flag is what makes the recurring pulse a one-shot. Variables MUST be
seeded (usually in OnStart) because expression filters fail closed on
undefined names. Warn-level lint flags reads of never-set variables and
ObjectiveComplete of never-posted ids (lint/scenario.rs:786).

### 7.4 Areas, occupancy refcount, spawn-inside-area

Source: crates/nova_scenario/src/objects/area.rs.

- A trigger area = `ScenarioAreaMarker` (+ Collider + Sensor, required
  components, area.rs:23) on an entity with an `EntityId`. Three authored
  producers: `CreateScenarioArea`, a Beacon with `area_radius`, every
  SalvageCrate.
- The other party must carry `EntityId` + `EntityTypeName` (i.e. be a scenario
  object) to be reported.
- Occupancy is refcounted per (area, body) pair (AreaOccupancy, area.rs:37):
  a compound ship fires one avian collision per section collider, so OnEnter
  fires only on the 0 -> 1 transition and OnExit on 1 -> 0.
- An area spawned AROUND a body already inside still fires OnEnter (the fresh
  overlapping pair starts; requires the RigidBody::Static the spawn bundle
  includes; pinned area.rs:209).
- A body (or area) despawned mid-overlap: its rows are silently pruned
  (`forget_body_occupancy`, area.rs:78) - no OnExit fires for the despawned
  entity itself; the pruning exists so the NEXT body can reach zero.

### 7.5 Event ordering summary

- Within a frame: gameplay fires events (observers push to the queue);
  PostUpdate runs `world_to_state` -> `queue_system` (drains ALL queued events
  FIFO; per event, handlers in spawn = authored order; per handler, actions in
  authored order) -> `state_to_world` (drains commands, syncs HUD, executes
  scenario switches). engine.rs:286-297, :404-427.
- `OnUpdate` is fired in Update, chained AFTER the clock tick and speed track
  (clock.rs:95-102), so time/speed gates always see this frame's values; the
  whole chain is gated live + Unpaused.
- Handler index maintenance runs every frame ungated, before dispatch
  (engine.rs:283-287), so handlers spawned at load are live before OnStart is
  processed (OnStart is fired via the same command flow after handler spawn,
  lifecycle.rs:222-244).
- Trackers (orbit/lock) run in Update after the clock tick, ungated by pause
  themselves but frozen via the frozen clock (loader/mod.rs:369-409).

### 7.6 Scoped-despawn / scoped-target rule

Every by-id action (Despawn, marker attach/detach, SetSpeedCap,
SetControllerVerb, SetAllegiance) resolves its id ONLY among
`ScenarioScopedMarker` entities; ship actions additionally require
`SpaceshipRootMarker`. Ship sections carry EntityId with per-ship ids
("controller"), so an unscoped despawn would rip that section out of every
ship (spawn.rs:33-38). Scoping also guarantees teardown removes everything a
scenario made. Anything spawned mid-scenario by gameplay (fragments,
projectiles) is auto-scoped while a scenario is live (lifecycle.rs
`on_add_entity_with`, loader/mod.rs:361-363).

### 7.7 Pacing vocabulary (three gears)

- Hard cut: `NextScenario(linger: false)` - switch on next sync.
- Delayed cut: `+ delay: Some(secs)` - world keeps playing, cut at expiry;
  Enter jumps it.
- Modal overlay: `Outcome((..))` + `NextScenario(linger: true)` - banner
  pauses the world; Continue/Retry (or `auto_advance_secs`) releases the
  lingering switch. With nothing queued, Enter returns to the menu.

### 7.8 Lint gate (author feedback loop)

Source: crates/nova_scenario/src/lint/. `LintSeverity::Error` refuses the
scenario start at runtime (`ContentIssues` -> failed-to-start overlay) and
fails the `content` CLI gate; `Warn` reports only. Notable rules
(lint/scenario.rs): dangling NextScenario / SetAllegiance / filter ids
(scatter prefixes satisfy id checks), duplicate spawn ids (Error in one
handler, Warn across), absurd scatter counts, reserved-variable writes,
unset-variable reads (Warn), unmatched ObjectiveComplete (Warn), Outcome +
hard-switch in one handler (Warn), double story lines per handler (Warn),
pacing-field ranges (delay > 60 s, dwell outside [3, 30], non-positive
orbit_hold/lock_refire), campaign dangling/duplicate members.

### 7.9 Generated content

assets/base/**/*.content.ron files are GENERATED from Rust builders
(crates/nova_authoring) via `content -- gen`; never hand-edit them (repo
memory rule). Mods hand-author the same format - the example mod
(assets/mods/example/) is the hand-written reference.

--------------------------------------------------------------------------------

## 8. Cross-reference: producers -> payloads -> consumers

| producer | event | `id` is | `other_id`/`other_type_name` is | typical filter |
|---|---|---|---|---|
| scenario load | OnStart | - | - | (none / expression seeds) |
| per-frame pulse | OnUpdate | - | - | Expression gates (clock, counters, flags) |
| asteroid destruction, ship-section explosion | OnDestroyed | destroyed object | - | `Entity((id:))` or `Entity((type_name:))` + count-gate |
| weapons+thrusters all dead | OnNeutralized | neutralized ship | - | `Entity((id:))` |
| area sensors (CreateScenarioArea, beacon area_radius, salvage crate) | OnEnter / OnExit | the AREA/beacon/crate | entering/leaving body (usually `player_spaceship` / `spaceship`) | `Entity((id: Some(area), other_id: Some(body)))` |
| orbit-hold tracker (5 s window, recurs) | OnOrbit | the WELL | orbiting ship | same shape as OnEnter |
| player lock bridge (acquire + 5 s re-fire) | OnTravelLock / OnCombatLock | locked target | player ship | `Entity((id: Some(target)))` |

Action -> event feedback edges:

- `SpawnScenarioObject`/`ScatterObjects` create the entities whose ids later
  appear in OnDestroyed / OnEnter / lock events.
- `CreateScenarioArea` (and Beacon.area_radius, SalvageCrate) create OnEnter /
  OnExit sources.
- `DespawnScenarioObject` silences an object (prunes occupancy, no OnExit for
  itself).
- `SetAllegiance` can flip a Neutral ship into the combat loop, eventually
  producing OnNeutralized / OnDestroyed.
- `VariableSet` feeds `Expression` filters; `HudReadout` displays the same
  variables; the engine feeds `scenario_elapsed` / `player_speed` into both.
- `NextScenario` + `Outcome` end the loop; the next scenario's OnStart begins
  a fresh (cleared) event world.

END OF CATALOG
