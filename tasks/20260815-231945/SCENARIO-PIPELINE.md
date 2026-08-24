# Scenario pipeline audit (round 4 research for v0.12.0 planning, written 2026-08-24)

Scope: the flat ScenarioConfig format, editor-lowering feasibility, the
one-shot/Sequence claims of task 20260820-223059, the autopilot chain, and the
process-channel reuse surface of task 20260820-174148. All file:line references
verified against the working tree at 947b228a.

## 1. The flat ScenarioConfig RON format

### The struct

`ScenarioConfig` is defined at `crates/nova_scenario/src/loader/mod.rs:157-207`:

- `id: ScenarioId` (:159) - a `String` alias (:49).
- `name`, `description` (:161, :163).
- `cubemap: AssetRef<Image>` (:166), `thumbnail: Option<AssetRef<Image>>` (:176).
- `hidden: bool` (:183), `menu_backdrop: bool` (:194) - serde-defaulted,
  skipped when false.
- `watches: Vec<WatchConfig>` (:200) - read-only typed queries published as
  variables (`crates/nova_scenario/src/queries.rs:64-71`).
- `events: Vec<ScenarioEventConfig>` (:206) - EVERYTHING ELSE. There is no
  other container.

`ScenarioEventConfig` (`loader/mod.rs:308-323`) is `{ name: EventConfig,
filters: Vec<EventFilterConfig>, actions: Vec<EventActionConfig> }`. Handlers
are anonymous: no id field; the spawned entity's `Name` is just
`"Event Handler: {:?}"` (`loader/lifecycle.rs:241`).

- Events: 16 variants, `crates/nova_scenario/src/events.rs:21-58`
  (OnStart, OnDefeated, OnDestroyed, OnNeutralized, OnUpdate, OnTimerEnd,
  OnEnter, OnExit, 4 orbit, 4 lock).
- Filters: 4 kinds, `crates/nova_scenario/src/filters.rs:27-36`
  (Entity, Conditional, Expression, Timer).
- Actions: 25 variants, `crates/nova_scenario/src/actions/mod.rs:45-98`.

### What a scenario contains, concretely

There is no top-level ships/objects/factions/objectives/variables section.
Everything is actions inside handlers, almost always OnStart:

- Objects: `SpawnScenarioObject(ScenarioObjectConfig)`
  (`actions/spawn.rs:72-77`), base = `{id, name, position, rotation}`
  (:83-92), kind = one of 6 (`ScenarioObjectKind`, :112-127): Anchor,
  Asteroid, Spaceship, Beacon, SalvageCrate, Light. Plus
  `ScatterObjects` (:265-301, seeded fields) and `CreateScenarioArea`.
- Ships: `SpaceshipConfig` (`objects/spaceship.rs:250` on) carries
  `hull: ShipSource`, `controller: SpaceshipController` (None/Player/AI),
  and an optional `allegiance` override. `ShipSource`
  (`objects/ship.rs:118-125`) is `Inline(ShipHull)` OR `Prototype(ShipId)`
  resolved against the `GameShips` catalog (:166) at spawn. One level down,
  each section is `SectionSource::Inline | Prototype`
  (`objects/spaceship.rs:195-201`), with per-spawn deltas via
  `ShipSectionModification` (`objects/ship.rs:154-160`). So prototype refs
  and inline section lists both exist; base content uses prototypes for the
  shared hulls.
- Factions: there is no faction system. Sides are `Allegiance`
  (Player/Enemy default from the controller, overridable per spawn at
  `actions/spawn.rs:164-166`, mutable at runtime via `SetAllegiance`).
- Objectives: not declared, POSTED - `Objective` / `ObjectiveComplete` /
  marker attach-detach actions (`actions/mod.rs:55-61`), HUD-side state.
- Variables: created by `VariableSet` actions (`actions/mod.rs:186-207`),
  untyped names; `watches` are the read-only engine-owned complement
  (writes refused at `world.rs:509-515`).

### Load path (shared by everything)

One representation, three producers, one consumer:

- Hand-written mods: `*.content.ron` = RON `Vec<Content>`;
  `Content::Scenario(ScenarioConfig)` at
  `crates/nova_modding/src/lib.rs:71-92`. Parsed and merged into the
  `GameScenarios` registry (`loader/mod.rs:56`) by
  `register_bundles` (`crates/nova_assets/src/merge.rs:49`).
- Base content: same files, but GENERATED - Rust builders under
  `crates/nova_authoring/src/base_content/`, serialized by
  `content_files()` (`crates/nova_authoring/src/generation.rs:162-190`)
  when `content gen` runs (`crates/nova_authoring/src/cli.rs:104`).
- The editor: builds a `ScenarioConfig` IN RUST at runtime
  (`crates/nova_editor/src/scenario.rs:247-268` `sandbox_scenario`),
  inserts it into `GameScenarios`
  (:236-243 `register_sandbox_scenario`, overwritten on Play at :198-217
  `setup_scenario`), and triggers `LoadScenario`. The editor writes NO
  file today (no fs writes anywhere in `nova_editor/src` outside tests).

Runtime load: `on_load_scenario` (`loader/lifecycle.rs:126-254`) - lint gate
(:145-169), teardown, camera + input context, then ONE `EventHandler` entity
per `ScenarioEventConfig` (:231-244), then `OnStart` (:253). Save: only
`content gen` writes scenario files; there is no runtime save path at all.

## 2. Lowering feasibility for an editor node graph

### Identity constraints

- Every id is a runtime string: `ScenarioId` (`loader/mod.rs:49`),
  object ids become `EntityId(String)` (`crates/nova_events/src/lib.rs:57`,
  inserted at `actions/spawn.rs:103`). Nothing type-checks them.
- Scenario ids must be snake_case slugs - debug assert in
  `ScenarioConfig::new` (`loader/mod.rs:230-237`), builders only; RON
  deserializes straight into the struct and bypasses it.
- The lint walks literals: `lint_scenario`
  (`crates/nova_scenario/src/lint/scenario.rs:64`) collects declared spawn
  ids and scatter prefixes (:116-135), then requires every filter/action
  target to be satisfiable against them (:168-174, `check_target` :577).
  Reserved names `scenario_elapsed` / `player_speed` must be declared as
  typed watches (:286-296).

### Duplicate instances of one prefab

- Duplicate spawned id WITHIN one handler is a lint Error (:142-152);
  across handlers a Warn (:153-166). A world node lowered to one OnStart
  handler therefore MUST uniquify: two corvette instances need two literal
  ids.
- The sanctioned derived-id family already exists: `ScatterObjects` clones
  a template with `base.id = format!("{id_prefix}{i}")`
  (`actions/spawn.rs:352-354`), and the lint accepts prefix matches
  (:170-174, test :1042). For hand-placed prefab instances the editor
  should mint the literal id at INSTANCE CREATION (corvette_1, corvette_2),
  store it in the graph, and never re-derive it at save - that keeps ids
  stable across saves, satisfies the "every id stays a literal" constraint
  of 20260820-223059, and keeps the lint decidable.

### Round-trip (save -> load -> identical editor state)

Nothing in the format structurally blocks it:

- The whole config tree round-trips through RON under the `serde` feature -
  proven by `a_scenario_config_round_trips_through_ron`
  (`loader/mod.rs:579-724`, includes inline ship + bindings) and the
  defaulted-fields test (:731-776).
- Serde-defaulted fields are skipped on serialize, so defaults are stable.

Real hazards, all fixable:

- Ordering nondeterminism. The editor already lowers its live ship with
  `player_config.sections.values().cloned().collect()`
  (`nova_editor/src/scenario.rs:478`) - HashMap value order. Fine for a
  transient sandbox, WRONG for a saved file (spurious diffs every save).
  Same for `PlayerControllerConfig::input_mapping: HashMap<SectionId,
  Vec<Binding>>` (`objects/spaceship.rs:58-68`). Sort before serialising.
- Comments. Generated base RON is comment-free
  (`generation.rs:151` `to_string_pretty`); webmods are hand-written WITH
  comments (e.g. `webmods/the-ledger/ledger_ch3.content.ron:1-24`).
  A serde round-trip destroys them. Editor-owned files are fine; opening a
  hand-written mod in the editor is lossy and should be treated as such.
- Editor-only metadata (graph layout, prefab grouping, which handler is
  the world node) has NO home in the format. No `deny_unknown_fields`
  anywhere in the workspace (grep), so extra RON fields are silently
  ignored on load and silently dropped on the next save by any other
  writer. Use a sidecar file, or a new serde-defaulted field on
  `ScenarioConfig` - the latter is a format addition, not a break
  (`thumbnail` at `loader/mod.rs:172-176` is the precedent).
- Reconstruction convention: the world node lowers to
  `SpawnScenarioObject` actions in OnStart handler(s); spawns in NON-start
  handlers are logic, not layout. The editor needs a fixed convention
  (e.g. first OnStart handler = the world node) to rebuild the graph
  without a sidecar.

## 3. Task 20260820-223059's claims, verified

### EventHandlerIndex and the invariant

`crates/nova_events/src/engine.rs:371-395`. The index stores CLONES of
handlers grouped by event name; the invariant is stated twice: "a handler is
built, spawned once, and never mutated in place" (:367-369) and "spawned once
and never changes its event name" (:402-404). Maintained every frame by
`maintain_handler_index` (:405-426).

Two facts the task's wording under-sells:

- DESPAWN is already supported and tested: removed handlers are pruned
  (:410-416, test `despawned_handler_is_pruned_from_the_index` :643-661).
  Retirement-by-despawn does not break the index. What breaks is
  same-frame semantics: `queue_system` (:429-462) drains the WHOLE queue
  against one `Res<EventHandlerIndex>` snapshot, and a despawn issued via
  commands lands next frame - so a `once` handler must also be latched
  where dispatch can see it (in `W`, which `queue_system` holds as
  `ResMut`), or two queued events of the same name fire it twice.
- STATE CANNOT LIVE IN THE ACTION. `EventAction::action` takes `&self`
  (:68-70), actions are `Arc`-shared, and the index holds a clone of the
  handler (:100-108) - a cursor inside a `Sequence` action would diverge
  between the ECS copy and the index copy. The engine-held cursor must
  live in `NovaEventWorld`, exactly as timers do
  (`world.rs:504` `timer_is_running`; tick at
  `loader/clock.rs:66-71`). Since handlers are anonymous, a `Sequence`
  needs an authored literal key to index that state (lintable, and it
  serialises - the mid-scenario-save argument in the task).

### Handler counts and the builders

- `shakedown_run`: 19 OnUpdate of 42 handlers, 2246 lines
  (`assets/base/scenarios/shakedown_run.content.ron`). `scenario_elapsed`
  appears 27 times; `open_step` 12 times; the :882-1046 passage cited by
  the task is real (open_step/opened/beat_gate cascade). GENERATED from
  `crates/nova_authoring/src/base_content/scenarios/nova_protocol/shakedown/mod.rs:622`
  (`shakedown_run(...)`, id at :33).
- `lifeline`: 13 of 27 (`assets/base/scenarios/lifeline.content.ron`),
  builder `.../nova_protocol/lifeline.rs`.
- `ledger_ch3`: 15 of 27 - CORRECTION: this is NOT base content and NOT
  generated. It is hand-written RON in
  `webmods/the-ledger/ledger_ch3.content.ron` (portal mod, published by
  `scripts/gen-portal.py` per its bundle header). A Sequence rewrite of it
  is a direct RON edit, not a builder change; it is still linted by the
  repo walk (`crates/nova_authoring/src/lint_walk.rs:173`).

### The 25-action enum and the lint

- `EventActionConfig`: exactly 25 variants,
  `crates/nova_scenario/src/actions/mod.rs:45-98`. Confirmed.
- `lint/scenario.rs` is 1391 lines TODAY, not the task's 1,452 (stale by a
  few edits, same order of magnitude). It is whole-program over one
  scenario: watches (:90-114), declared pass (:116-135), duplicate ids
  (:137-166), query targets (:176-181), filters+actions (:183-209),
  objective pairing (:211-220), beat-sheet pacing (:222-257), outcome
  traps (:259-284), variable read/write hygiene (:286-313).

### What `once` + `Sequence` would actually touch

`once` (stage 1):
- `ScenarioEventConfig` gains a serde-defaulted `once: bool`
  (`loader/mod.rs:308-323`) - no format break, old files parse.
- Engine: a retirement latch in `W` consulted by `queue_system`
  (engine.rs:441-450), plus despawn of the handler entity (path exists).
- `/create` reference and a new lint (flag OnUpdate handlers whose filters
  read only time/variables once better spellings exist).

`Sequence` (stage 2), the parts nobody has listed yet - every one of these
walks `event.actions` at TOP LEVEL ONLY today and must recurse into steps:
- `ScenarioConfig::inline_queries` (`loader/mod.rs:259-280`) - misses
  expressions inside nested VariableSet/TimerStart, which silently
  disables the entity sampler for them (:289-295 gates on it).
- `ScenarioLoaded::from_config` object_count (`loader/mod.rs:352-364`) -
  harness assertions read it.
- Lint: `collect_declared` (`lint/scenario.rs:318`), `check_action`
  (:346), and the per-event spawn-id pass (:120-135).
Plus: the new variant + dispatch arm in `actions/mod.rs`, cursor state in
`NovaEventWorld` keyed by an authored literal key, `after:` on the existing
keyed-timer machinery (`clock.rs:66-71` + OnTimerEnd), and `deadline:` as a
loud failure mirroring the autopilot's (see 4).

## 4. The autopilot chain to copy

Task 20260802-120019 moved it: the plugin lives in
`crates/nova_autopilot/src/autopilot.rs` (`AutopilotPlugin` :143), and
`nova_debug::harness::nova_autopilot()` (`harness.rs:163`) wraps it with
`GameStates` + completion wiring.

Exact shape (`autopilot.rs:100-107`, doc table :7-14): a `Step` is
`{ name, enter: Option<S>, on_enter: Vec<EnterFn>, each: Option<EachFn>,
until: Arc<Predicate>, deadline: Option<f32> }`. Builder methods:
`enter` :268, `on_enter` :276, `each` :284, `until` :291, `deadline` :299,
`add` :305, sugar `hold` :204, `input` :228, `loop_from` :243. A missed
deadline ABORTS the run naming the step (doc :14; test
`completion.rs:260`). Input synthesis: `press_key` / `release_key` /
`press_mouse` / `release_mouse` (`nova_autopilot/src/input.rs:74,81,124,129`).

Usage: 23 of the 25 `examples/systems/*.rs` files drive on `.step(...)`.
Representative chain: `system_borrowed_battery.rs:114-118` -
`.step("open the tubes").on_enter(open_the_tubes)
.until(a_mount_is_claimed()).deadline(90.0).add()` (the task's quoted
`the_envelope_is_full()` is a paraphrase; the real gates are Nova-typed
closures per example - same shape).

### Predicate vocabulary today (`nova_autopilot/src/predicate.rs`)

`elapsed` :58, `frames` :72, `state_is` :86, `resource_where` :98,
`any_entity` :110, `shot_written` :129, `loop_written` :147, combinators
`and` :156 and `not` :162, plus arbitrary `Arc<Predicate>` closures
(doc :31-34).

What is missing (the 20260824-011329 gap):
- No `or` combinator, no changed/edge predicates.
- No editor-state predicates POSSIBLE from outside the crate:
  `SectionGhost` (`nova_editor/src/config.rs:96`), `PlacementStatus`
  (:105) and `Placement` (`nova_editor/src/snap.rs:69`) are all
  `pub(crate)`. `system_ship_editor.rs:161-168` says exactly this in the
  `SETTLE` doc comment and cites the task.
- So the ranges hand-roll frame sleeps: `SETTLE = 10`
  (`system_ship_editor.rs:170`), `SHIP_SETTLE = 40` (:175); same pattern
  in `bug_sandbox_soak.rs:96-100`, `system_menu_boot.rs:81`, and
  `examples/screenshots/shared/ui_walk.rs:32` (`STEP_DEADLINE_SECS` with
  its own settles).

## 5. The process-channel reuse surface (20260820-174148)

- `capture_snapshot(world: &mut World, reason: &str) -> serde_json::Value`
  at `crates/nova_probe/src/capabilities/snapshot.rs:364`. Confirmed pure:
  "no IO, no world mutation beyond the query state bevy builds to read it"
  (:359-360); the module doc (:5-9) already promises the stdin/stdout mode.
- `EventActionConfig` serde: derived behind the `serde` feature
  (`actions/mod.rs:43-44`) - the channel can parse the same enum the RON
  does, as the task assumes.
- `AppBuilder::headless()` at `crates/nova_core/src/lib.rs:182-184`;
  `NORENDER_ENV` turns every `new()` headless (:162-164); the binary flag
  is `--norender` (`src/main.rs:35`, wired :78). Landed in a47c6247
  ("Make --norender actually skip the renderer").
- `ScheduleRunnerPlugin::default()` is added on the headless path at
  `nova_core/src/lib.rs:211` - free-running; a step mode hooks here.
- The SyncWorldPlugin leak: CONFIRMED and quantified twice.
  `nova_core/src/lib.rs:212-229` (the workaround and why): the queue is
  never drained because `entity_sync_system` lives in the render sub-app;
  ~24 bytes leak per synced spawn AND per synced component removal,
  ~2.4 MB per 100k, linear in run length.
  `tasks/20260819-173219/notes-render-off.md:137-144` adds the blocker:
  `PendingSyncEntity` is `pub(crate)` in bevy, so it CANNOT be drained
  from outside - the options are an upstream fix, vendoring, or bounding
  session length. Step mode makes long sessions likely, so this is
  load-bearing exactly as the task says.

## What this means for v0.12.0

1. Ship `once` first and alone (20260820-223059 stage 1). It is a
   serde-defaulted field plus a latch in `NovaEventWorld` plus the
   already-tested despawn path. Guard the same-pass double-fire in
   `queue_system`, not only via despawn.
2. Design `Sequence` around an authored literal sequence key with cursor
   state in `NovaEventWorld` (timers are the template). Do NOT put state in
   the action - the index clones it. Budget for the four recursion sites
   (inline_queries, object_count, lint collect/check, per-event spawn
   pass); they are the hidden half of the work.
3. For the editor's lowering: mint literal instance ids at instance
   creation and store them in the graph; lower the world node to OnStart
   `SpawnScenarioObject` lists; sort every map and the section list before
   serialising. This satisfies the existing lint with zero lint changes.
4. Decide the editor-metadata home early: sidecar file (safe) or one new
   serde-defaulted `ScenarioConfig` field (format addition, thumbnail
   precedent). Do not rely on unknown-field tolerance - other writers drop
   it silently.
5. Treat hand-written mods as read-only in the editor for v0.12.0:
   round-trip destroys comments, and the ledger campaign is the best
   authored content the project has.
6. Do 20260824-011329 (editor wait predicates) BEFORE building more editor
   harness coverage: expose a read-only placement status from nova_editor,
   add `or()`, retire `SETTLE`. Sequence `until:`/`deadline:` and the
   harness gates are the same idea; keep their vocabularies consciously
   parallel.
7. For 20260820-174148: put the step-mode hook at the
   `ScheduleRunnerPlugin` site (`nova_core/src/lib.rs:211`), and resolve
   the SyncWorldPlugin leak strategy (upstream issue or bounded sessions)
   before promising indefinite driven sessions.

Not confirmed / honest gaps: I did not measure the 150.6 us / 2.7 us D19
numbers (out of scope, measurement task's evidence); I did not re-count the
16 expression-grammar nodes the review cites (the grammar enums are at
`variables.rs:39-58,105-111,174-180,249-255` and are plausibly 16 with
condition variants); `lint/scenario.rs` is 1391 lines, not the cited 1,452;
and `ledger_ch3` is a hand-written webmod, not a generated base file - the
one factual correction to this lane's briefing.
