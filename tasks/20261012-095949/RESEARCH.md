# World first: the scenario declares its world

Research note for decision 14 of the design page
(`tasks/20261012-095526/DESIGN.html`, Part 9). Read-only; master `decf3581d`,
2026-10-12. Two code surveys back every `path:line` here. No code was
written.

Owner direction: the open world is the main mode and the first citizen.
Scenarios and `nova_events` are a harness put on top of it so the world
works in bounded form. Story mode stays as it is.

## 1. How it stands

The dependency points the other way today. The scenario is the root and
the world is a plugin that watches the scenario's role.

- New Game inserts `OpenWorldSession { seed }`, clears the picker override,
  sets `GameMode::NewGame` and enters Playing
  (`crates/nova_menu/src/world_setup.rs:355-365`). On entering Playing the
  menu loads the base-declared start (`NewGameStart`, from
  `new_game_scenario: Some("open_world")` in `assets/base/base.bundle.ron:304`)
  through `start_new_game_scenario`
  (`crates/nova_menu/src/menu_ui.rs:567-637`).
- The loader clones the config into `CurrentScenario`
  (`crates/nova_scenario/src/loader/lifecycle.rs:418`). `ScenarioConfig`
  carries no seed and no world field; it has `id, name, description,
  cubemap, skybox_brightness, thumbnail, role, watches, events`
  (`crates/nova_scenario/src/loader/mod.rs:271-323`).
- `sync_open_world` arms the generator only while
  `CurrentScenario.role == ScenarioRole::OpenWorld`; any other role or no
  scenario disarms (`crates/nova_world_base/src/lib.rs:249-256`). It panics
  on an OpenWorld scenario with no session (`:258-263`), on two player
  ships (`:275-280`), and on content that does not build (`:295-332`). The
  `WorldConfig` it writes is `seed` from the session plus two crate
  constants and the generator (`:333-338`).
- `nova_world` itself is already world-first in shape: `WorldConfig<G>` is
  the arming switch and nothing streams without it
  (`crates/nova_world/src/lib.rs:498-521`); `Cleanup` runs on config
  insert, replace or removal and on a scenario change (`:849-854`); a swap is
  a clear session (`:143-153`). Streamed bodies are scenario-scoped so an
  `UnloadScenario` sweeps them, but not addressable
  (`crates/nova_world/src/streaming.rs:175-186`).
- Chapters, backdrops, lessons and every webmod scenario run in a void:
  authored objects only, no `WorldConfig`. Base catalog: 4 Backdrop,
  4 Lesson, 2 Chapter, 1 OpenWorld
  (`crates/nova_authoring/src/base_content/scenarios/mod.rs:29-43`);
  7 webmod scenarios, all implicit Chapter, no `role:` line.
- Readers of `ScenarioRole::OpenWorld`: the arming gate (`lib.rs:252`); the
  campaign lint message (`crates/nova_scenario/src/lint/scenario.rs:88`);
  the probe `world_seed` gate
  (`crates/nova_probe/src/capabilities/snapshot.rs:434-442`) and its
  fixture (`:2200-2210`); the arming tests
  (`crates/nova_world_base/src/tests.rs:347-495`); the base builder
  (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:47`);
  the generated RON (`assets/base/scenarios/open_world.content.ron:7`).
  `picker_lists()` hides it from the picker by listing Chapter only
  (`crates/nova_scenario/src/loader/mod.rs:236-240`).
- `GameMode::NewGame` does not mean open world. It means "the menu triggers
  `LoadScenario` on entering Playing"; the picker, training, Load and the
  `--scenario` flag all set it (`crates/nova_gameplay/src/lib.rs:210-224`).
  Only Create and Load insert `OpenWorldSession`
  (`world_setup.rs:362`, `crates/nova_world_base/src/save/session.rs:261`).
- The `--scenario <id>` boot path lands on the same loader with no session
  (`crates/nova_core/src/lib.rs:548-595`), so `--scenario open_world` would
  hit the no-session panic. Inferred from code, not run.
- Retry re-triggers `LoadScenario(current)` without leaving Playing; Load
  last save runs `resume_world` first, which removes `WorldConfig`, inserts
  the session from the header and holds the ledger until the world re-arms
  (`crates/nova_menu/src/leave.rs:128-183`, `save/session.rs:234-266`).
- Areas are spheres at absolute world positions with an addressable id;
  `OnEnter`/`OnExit` report area id and the other body
  (`crates/nova_scenario/src/actions/spawn.rs:701-731`,
  `crates/nova_events/src/lib.rs:279-311`).
- `docs/scenario-system.md:833-835` still says the tutorial builds the New
  Game starter. Stale since the base bundle pointed New Game at `open_world`.

## 2. The three shapes

1. **A `world` block on the scenario.** `Void` or `Generated { seed, bounds }`.
   The role stops gating; the generator arms from the block; the open world
   is the New Game harness. Consequence: one field, the four readers move
   to it, chapters say `Void` and keep working, New Game and a picked
   scenario share one path. Recommended.
2. **Keep `role: OpenWorld` as the gate** and give bounded scenarios a fixed
   seed through the role. Consequence: no new field, but role and world
   become one axis (a Backdrop on a generated world is impossible, a
   Lesson on one too), the picker rule stays role-based, and "void" stays
   implicit.
3. **A separate harness content kind** beside `Content::Scenario`.
   Consequence: the cleanest split on paper, but a second vocabulary for the
   same handlers with its own loader, lint, picker rows and docs.

Shape 1 is the one the rest of this note details.

## 3. Types and fields

In `crates/nova_scenario/src/loader/mod.rs`, beside `ScenarioRole`.

```rust
/// Which world a scenario runs on. ONE field, independent of `role`: a
/// chapter, a backdrop or a lesson may sit on a generated world or on none.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScenarioWorldType {
    /// No streamed world: the scenario's authored objects are the whole
    /// scene. Every shipped chapter, backdrop and lesson today.
    #[default]
    Void,
    /// The base generator streams sectors around the one player ship the
    /// scenario spawns.
    Generated(GeneratedWorldConfig),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct GeneratedWorldConfig {
    /// `None`: the seed the player typed on New Game, carried by
    /// `OpenWorldSession`; only New Game and Load may start such a
    /// scenario. `Some`: a fixed seed, so the scenario is a bounded world
    /// the picker may launch with no session.
    pub seed: Option<u32>,
    /// A sphere the player is meant to stay in. Spawned at load as an
    /// addressable area named `bounds`, so leaving it raises `OnExit` with
    /// `id: "bounds"` like any area and the outcome is authored.
    pub bounds: Option<ScenarioBoundsConfig>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ScenarioBoundsConfig {
    /// Sphere centre in metres, world space, like `ScenarioAreaConfig`.
    pub position: Meters3,
    pub radius: Meters,
}
```

`ScenarioConfig` gains one field after `role`:

```rust
    /// Which world the scenario runs on. Default `Void`, and a `Void`
    /// scenario writes no `world:` line, as `role` does for `Chapter`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "ScenarioWorldType::is_void"))]
    pub world: ScenarioWorldType,
```

`ScenarioRole` loses `OpenWorld`. `Chapter`, `Backdrop`, `Lesson` stay as
they are. RON that still says `role: OpenWorld` fails to parse (unknown
variant), which is the loud failure the owner's rules want.

Helpers on `ScenarioWorldType`: `is_void()`, `is_generated()`,
`needs_session()` (`Generated` with `seed: None`), `fixed_seed()`.

`picker_lists()` moves from `ScenarioRole` to `ScenarioConfig`:
`role == Chapter && !world.needs_session()`. A fixed-seed generated chapter
is a picker row; the New Game harness is not.

`OpenWorldSession { seed }` stays as the carrier of the typed seed. Its doc
changes from "before the open-world bootstrap loads" to "for a scenario
whose world is `Generated { seed: None }`".

## 4. Where it is read

- **Arming** (`crates/nova_world_base/src/lib.rs:248-342`): replace the
  role test with a match on `CurrentScenario.world`:
  `Void` disarms; `Generated { seed: Some(s), .. }` arms with `s`;
  `Generated { seed: None, .. }` arms with `OpenWorldSession.seed` and
  panics with the existing message when there is none. Everything after
  the seed (player, packs, styles, items, catalog pin, config write) is
  unchanged.
- **Probe** (`snapshot.rs:434-442`): `world_seed` is reported when the live
  scenario's world is generated, from the armed `WorldConfig.seed` rather
  than the session, so a fixed-seed chapter reports its seed too.
- **Lint** (`crates/nova_scenario/src/lint/scenario.rs`): the
  `OpenWorld` arm of the campaign message goes; the new rules are in
  section 7.
- **Picker** (`crates/nova_menu/src/scenarios.rs:109-118`): calls the new
  `ScenarioConfig::picker_lists()`.
- **Bootstrap builder** (`open_world.rs:38-68`): `role` line removed,
  `world: ScenarioWorldType::Generated(GeneratedWorldConfig { seed: None,
  bounds: None })`. Regenerate RON with `content -- gen`; `content_ron_parity`
  pins it.
- **Bounds** (new, in the loader's `configure_scenario_shape` or the
  `OnStart` injection path, `lifecycle.rs:345-420`): a `Generated` world
  with `bounds` spawns one `ScenarioAreaConfig { id: "bounds", .. }`
  through the existing area path, so no new sensor code.
- **New Game modal** (`world_setup.rs`): unchanged in this strand. The
  wizard strand replaces its body later; what it inserts stays
  `OpenWorldSession` plus the save session.
- **Tests** (`crates/nova_world_base/src/tests.rs:61-99`, `:347-495`): the
  `world(role)` fixture becomes `world(ScenarioWorldType)`; the
  "another role disarms" test (`:446`) becomes "a Void scenario disarms";
  one new test: a fixed-seed scenario arms with its own seed and no
  session.

## 5. What every shipped scenario declares

| Scenario | Role | World | RON change |
|---|---|---|---|
| open_world | Chapter (hidden by `needs_session`) | `Generated((seed: None))` | `role: OpenWorld` line becomes `world: Generated((seed: None))` |
| tutorial, season_one_chapter_one | Chapter | Void | none |
| menu_duel, menu_gauntlet, menu_waystation, menu_weave | Backdrop | Void | none |
| drill_autopilot, drill_gunnery, drill_momentum, drill_stop | Lesson | Void | none |
| 7 webmod scenarios (gauntlet, the-ledger) | Chapter | Void | none |

So one generated file changes and nothing in a campaign or a mod moves.
The `open_world` scenario keeps its id (`OPEN_WORLD_SCENARIO_ID`,
`crates/nova_world_base/src/lib.rs:116`) because the bench referee keys on
it (`crates/nova_bench/src/referee.rs:375`).

## 6. Call graphs

Before:

```
New Game: Create -> OpenWorldSession(seed) + WorldSaveSession -> Playing
          -> start_new_game_scenario -> LoadScenario(open_world)
          -> CurrentScenario(role OpenWorld) -> sync_open_world: role? -> arm(seed from session)
Play (picker): NewGameScenario(id) -> Playing -> LoadScenario(id)
          -> CurrentScenario(role Chapter) -> sync_open_world: role? -> disarm
Retry:    LoadScenario(current) -> teardown + OnStart again -> role? -> arm(same session)
```

After:

```
New Game: Create -> OpenWorldSession(seed) + WorldSaveSession -> Playing
          -> start_new_game_scenario -> LoadScenario(open_world)
          -> CurrentScenario(world Generated{None}) -> sync_open_world: world? -> arm(seed from session)
Play (picker): NewGameScenario(id) -> Playing -> LoadScenario(id)
          -> world Void -> disarm | world Generated{Some(s)} -> arm(s), no session, no save
Retry:    LoadScenario(current) -> world? -> arm(same seed source)
Load:     resume_world (removes WorldConfig, inserts session from header) -> LoadScenario(open_world) -> arm
--scenario open_world: lint-equivalent refusal at boot: a needs_session scenario cannot start without New Game
```

The only node that changes is the arming question. Teardown, save
moments and the frozen ledger are untouched because they key on
`WorldConfig` and `CurrentScenario` change, not on the role
(`save/session.rs:305-326`, `nova_world/src/lib.rs:849-854`).

## 7. Lint and load rules

All at lint (content walk) then at load (`start_errors` gate,
`lifecycle.rs:345-402`), never a fallback.

1. A `Generated` scenario must spawn exactly one player ship in an
   `OnStart` handler. Today a missing player silently disarms
   (`lib.rs:269-273`); after this it is an Error. How the lint recognises
   the player spawn (the spawn config's controller field) is verified at
   implementation.
2. A `Void` scenario with `bounds` is an Error.
3. `bounds.radius` above the active window's half edge is an Error. The
   constants are private to `nova_world_base` (`lib.rs:139-145`); the rule
   needs `pub const OPEN_WORLD_BOUNDS_RADIUS_MAX: Meters` there and the
   check in `nova_authoring::lint_walk`, which already sees both crates.
4. `NewGameStart` must name a scenario whose world `needs_session()`; the
   base bundle naming a `Void` or fixed-seed start is an Error.
5. A campaign member whose world `needs_session()` is an Error (a campaign
   cannot supply a typed seed). Fixed-seed members are allowed.
6. `--scenario <id>` naming a `needs_session` scenario refuses at boot with
   the unknown-startup-scenario path (`nova_core/src/lib.rs:548-595`), not
   the session panic.

## 8. What dies, what may break, what must fail loudly

Dies: `ScenarioRole::OpenWorld` and its three match arms; the
`ScenarioRole::picker_lists` helper (moves to the config); the "the world
is the scenario" doc in `open_world.rs:1-9`; the void as an unstated
default (it is now a named variant); the stale New Game line in
`docs/scenario-system.md:833-835`.

May break: the four readers (arming, lint, probe, tests); the picker's
assumption that a listed scenario runs with no world; `nova_bench` and
`nova-probe` fixtures that build a `ScenarioConfig` with `role: OpenWorld`
(`snapshot.rs:2200-2210`); every `ScenarioConfig { .. }` literal in tests
(struct update syntax from `ScenarioConfig::new` keeps most compiling).

Must fail loudly: the six rules above; a `Generated { seed: None }`
scenario live with no session (existing panic, kept); `role: OpenWorld` in
any RON (parse error at load, lint error in the walk).

## 9. Out of this strand's first tasks

These are later children of the epic and are only named here so the
block's shape leaves room for them: open-world events (`OnSectorEntered`,
`OnMined`, `OnPickedUp`, `OnTraded`, `OnCreditsChanged`), addressable
generated bodies (a typed generated id a handler may name), and scenario
state in the world save (`WorldSaveState` holds none,
`crates/nova_world_base/src/save/mod.rs:103-117`). None of them changes the
`world` block.

## 10. Changelog and docs

- `CHANGELOG.md` under `[Unreleased]`, scenario subsystem, **(breaking)**:
  `role: OpenWorld` is gone; a scenario declares `world: Generated((seed:
  None))` or nothing for a void.
- `docs/scenario-system.md`: a "Which world a scenario runs on" subsection
  under Scenario structure; fix lines 833-835.
- The example mod needs no change (all Void); add one fixed-seed bounded
  scenario to it as the living example once bounds land.

## 11. Decisions for the owner

1. Shape: 1 (world block, recommended), 2 (role stays the gate) or 3
   (harness content kind).
2. `world` default: `Void` with no line, mirroring `role: Chapter`
   (recommended, since a scenario that says nothing is a plain scene), or
   required on every scenario (every base and webmod file gains a line).
3. Picker: list fixed-seed generated chapters as rows (recommended) or keep
   the picker void-only until a bounded scenario exists.

Assumed unless the owner objects: bounds are a sphere at an absolute world
position with id `bounds`; `OpenWorldSession` keeps its name; a
picker-launched fixed-seed world is session-only (no save folder).
