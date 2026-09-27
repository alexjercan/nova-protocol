# Replace TAB NOVA OS with the themed interface and a command-only CRT

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.15.0,ui,novaos,migration

Approved plan, implementation ACTIVE. The owner approved the corrected plan on
2026-09-26 and resumed implementation on `master` at `ab614a686`. Line numbers
below are from `6eaf99673`.

## User facts
- The owner approves the visual direction of the example-only
  `ui_app_variants` sketch (PR #77). Ship it as the real TAB interface. TAB
  never opens the old NOVA OS computer.
- This is a breaking replacement. No old NOVA OS screen, parallel app host,
  legacy default, adapter, alias, or fallback survives.
- Station, cargo, credit, repair-economy, and persistence mechanics belong to
  `20260926-174806`. The sketch's stock and transactions are not game state.
- Owner feedback 2026-09-26 (supersedes the first visual pass): the TAB
  interface does not resemble the PR #77 example. The dark gray-blue
  full-screen background must go. The top-level tab strip must go; the Map and
  Ship buttons go inside the panels. Match the example structure and geometry
  closely (`git show 6eaf99673:examples/playable/ui_app_variants.rs`,
  `top_bar` 785-845, `rebuild_body` 886-935). Build from the sketch, not from
  the old NOVA OS app or CRT host. Do not only change colors.
- Owner feedback 2026-09-26: the TAB interface is extremely laggy. Reproduce
  and profile it, keep baseline artifacts, inspect frames before and after, and
  compare matched repeat sets. No timing assertions.
- The `:` CRT stays separate unless the owner changes that decision.

## Decisions (owner-approved)
- TAB opens a themed interface with two panes, Map and Ship. The Map and Ship
  buttons (in the card's title row since the 2026-09-26 feedback below),
  keyboard M, and gamepad Y switch between them. Inventory and station panes
  are deferred.
- `:` keeps the existing CRT: CRT material, casing, pointer rig, monitor
  settings (BRIGHT, SCAN, SND), and sounds. It becomes a command-only modal
  titled `NOVA COMMANDS`. It opens from the main menu, the editor, flight, the
  pause menu, and over the TAB pane.
- Closing the command modal returns to the previous pane or state. There is no
  input gap and no freeze gap: no frame runs the world, and no key reaches
  flight or the pane, between the modal and the state it returns to.
- Delete the `nova>` prompt, the NOVA OS shell, the CRT app host, and every
  app-specific text verb (`map`, `map view`, `map goto`, `ship`, `ship view`,
  `ship section`, `ship reload`, `ship repair`, and the NOVA OS-only copies of
  `help`, `log`, `objectives`, `clear`, `version`, `commands`, `exit`). The generic
  command parser's existing `version` response remains. No aliases.
- The generic command-shell verbs stay: `ships`, `ship <id>`, `sections`,
  `section`, `objectives`, and the rest of `COMMAND_CATALOG`.
- Add one new generic verb, `log`. It prints comms, objective updates, and
  labeled combat lock-drop events.
- Live Map GOTO and Ship Repair, Reload, Rebind, and the mates overlay stay as
  pane actions (keys and buttons), not text verbs.
- Rename crates: `nova_os` -> `nova_command`, `nova_os_ui` -> `nova_interface`.
- Rename every surviving `novaos_*` action ID. Migrate the shipped saved-settings
  keybinds that use the old IDs (v0.14.0 shipped these IDs).
- The old `NovaOs` lesson category fails at lint and at load. No alias.
- Rewrite the eight NOVA OS lessons as six smaller new lessons with new media.
  Old viewed lesson IDs in saved progress stay inert.

## Approved names (owner-approved 2026-09-26)
- `PauseStates::NovaOs` -> `PauseStates::Interface` (TAB pane) plus
  `PauseStates::Commands` (CRT modal).
- `FreezeOwner::Terminal` -> `FreezeOwner::Interface`, held for both states.
- Action IDs: `novaos_toggle` -> `interface_toggle`; `novaos_orbit_*`,
  `novaos_pan_*`, `novaos_reframe`, `novaos_next`, `novaos_prev` ->
  `viewer_*`; one new `interface_next_tab` action (keyboard M, gamepad Y).
- Plugins and sets: `InterfacePlugin` / `InterfaceSystems`,
  `CommandsPlugin` / `CommandsSystems` (replacing `NovaOsUiPlugin`,
  `NovaOsPlugin`, `NovaOsSystems`, `MonitorFrame`).
- Lesson category `LessonCategory::Interface` (label `INTERFACE`) and the six
  lesson IDs `interface_open`, `interface_map`, `interface_ship_service`,
  `interface_rebind_section`, `command_open`, `command_prompt`.
- Pane state (owner-approved 2026-09-26): `nova_interface::pane::InterfacePaneType`
  `{ Map, Ship }`, a `Resource`, not `SubStates`. The first open shows Map; TAB
  reopens the last pane, and closing the `:` modal over the pane returns to
  it with its scene, camera, and selection. Systems: `toggle_interface`,
  `next_interface_pane`, `spawn_interface_root`, `rebuild_interface_body`.
- Pane helpers (owner-approved 2026-09-26): private `InterfaceRootMarker`,
  `interface_shown(pause, close)`, `themed_label`, and
  `InterfacePaneType::context_id`. `interface_shown` (Interface, or Commands
  returning to Interface) governs only root and scene lifetime and visuals.
  Pane input, GOTO, repair, reload, and rebind stay gated on
  `pause == Interface && pane == <pane>`, so nothing fires under the modal. A
  duplicate interface root fails loudly.
- Rebind gate (owner-approved 2026-09-26): `nova_interface::ship::ShipRuntime`
  is public with crate-private fields and a documented
  `pub fn rebind_armed(&self) -> bool`, exported from the prelude.
  `nova_menu::open_command_shell` refuses `:` while it or `PendingRebind` is
  armed. Proof: Shift+; during a ship rebind neither opens `Commands` nor binds
  the colon unexpectedly, and `:` over Ship still opens afterward.
- Pane picking proof (owner-approved 2026-09-26): one test-only
  `pointer_rig::pane_pointer_rig()` drives real window-space `ui_picking` with
  no CRT forwarding. The map label/dot, straddle, and topmost tests and the
  ship label/dot test run on it. `map_contacts_select_where_the_crt_shows_them`
  is deleted. The CRT rig stays for the command CRT mapping tests.
- Defaults (owner-approved 2026-09-26): pad Y is `interface_next_tab`, so
  `ship_reload` loses pad North and keeps keyboard L. Keybind labels read
  `Open Interface` and category `INTERFACE`.
- Persisted monitor fields `nova_os_bright_detent`, `nova_os_scan_detent`,
  `nova_os_sound_enabled`: keep the shipped keys unchanged, so no migration is
  needed.
- Examples, media, and wiki (owner-approved 2026-09-26):
  - `system_nova_os` -> `system_interface`: TAB opens the pane, M switches,
    `:` over the pane, Escape returns to the same pane, clocks frozen, and no
    pane or flight action fires on transition frames. This is the modal-return
    proof.
  - `system_headless_novaos` is deleted; `system_command_shell` covers the
    headless modal.
  - `system_headless_crt` -> `system_headless_map_goto`: TAB, click a blip in
    window space, G, assert `Autopilot` GOTO on that contact.
  - The rebind-gate proof is one beat group in `system_command_shell`: over
    Ship, arm a section rebind; Shift+; opens no `Commands`, and the armed
    section takes the physical `;` key, which spends the capture; then `:`
    opens `Commands` over Ship.
  - `screenshot_nova_os_terminal` -> `screenshot_command_shell`
    (`wiki-command-shell.png`); `screenshot_nova_os_apps` ->
    `screenshot_interface` (`wiki-interface-map.png`,
    `wiki-interface-ship.png`). Old PNGs are deleted.
  - `lesson_novaos*` producers are deleted. New producers: `lesson_interface`
    (`interface_open`, `interface_map`), `lesson_interface_ship`
    (`interface_ship_service`, `interface_rebind_section`), `lesson_command`
    (`command_open`, `command_prompt`); all loops. Media files use lesson IDs;
    old `novaos_*.webp` are deleted.
  - `web/src/wiki/nova-os.md` -> `web/src/wiki/interface.md` (opening, map,
    ship, rebinding). Lessons link `wiki/interface#...`.
  - `ui_app_variants` is deleted. PR #77 history keeps the sketch for
    `20260926-174806`.
- Internal names (owner-approved 2026-09-26, option A):
  `ActionContext::ViewerApp` -> `ActionContext::InterfacePane`;
  `novaos_bindings` -> `interface_bindings`; `HudNovaOsExempt` ->
  `HudInterfaceExempt`; `NovaOsMapSystems`, `NovaOsShipSystems`,
  `NovaOsMapPlugin`, `NovaOsShipPlugin` -> `MapPaneSystems`,
  `ShipPaneSystems`, `MapPanePlugin`, `ShipPanePlugin`;
  `close_nova_os_from_menu_keys` -> `close_surface_from_menu_keys`; field note
  `note_novaos` -> `note_interface`. Doc comments that describe deleted
  behavior are fixed. The retained CRT monitor keeps its `NovaOs*` and
  `NOVA_OS_*` names as the monitor brand.

## Agent findings (code evidence at 6eaf99673)

### TAB and the NOVA OS shell
- `crates/nova_os_ui/src/bindings.rs:43-117` `novaos_bindings()`:
  `novaos_toggle` (TAB, pad RightThumb, `Always`) at 48-51; viewer actions
  `novaos_orbit_{left,right,up,down}`, `novaos_pan_{forward,back,left,right}`,
  `novaos_reframe`, `novaos_next`, `novaos_prev` (`ActionContext::Viewer`) at
  54-97; `map_goto` (G), `ship_mates` (G), `ship_reload` (L), `ship_repair` (P),
  `ship_rebind` (B) (`ActionContext::ViewerApp`) at 99-116. M and Y are unbound.
- `crates/nova_os_ui/src/terminal/input.rs:53-89` `toggle_nova_os` opens
  `ShellKind::NovaOs` and sets `PauseStates::NovaOs`; 95-175
  `close_nova_os_from_menu_keys` owns Escape, Start, and Ctrl+C back-out.
- `crates/nova_gameplay/src/lib.rs:156-166` `PauseStates { Unpaused, Paused,
  NovaOs }`. `crates/nova_gameplay/src/freeze.rs:27-39` `FreezeOwner::Terminal`
  already covers both shells.
- `crates/nova_menu/src/lib.rs:330-346` holds clocks and frees the cursor on
  `OnEnter/OnExit(PauseStates::NovaOs)`; `crates/nova_menu/src/pause.rs:253-263`
  `hold_clocks_for_terminal`, `release_clocks_for_terminal`.
- `crates/nova_menu/src/pause.rs:127-172` `open_command_shell` reads typed `:`,
  opens `ShellKind::Commands`, and stores `NovaOsCloseTransition::return_to`.
  It returns early inside the CRT (147) and during a rebind capture (150).
  `crates/nova_os_ui/src/terminal/components.rs:280`
  `NovaOsCloseTransition { closing, return_to }`.
- `crates/nova_os/src/terminal/state.rs:42-67` `ShellKind { NovaOs, Commands }`,
  prompt `nova> ` at 56. `nova>` also at
  `crates/nova_os_ui/src/terminal/spawn.rs:540` and `components.rs:75`.
- `crates/nova_os/src/command.rs:24-285` `CommandBody`, `TerminalCommand`,
  `core_terminal_commands` (188-204), `NovaOsCommandRegistry` (220-257),
  `nova_os_footer_hints` (261-276). `crates/nova_os/src/app.rs:41-130`
  `command_shell_hints`, `terminal_hints`, `NovaOsAppRuntime`,
  `NovaOsAppInputOutcome`. `crates/nova_os/src/terminal/edit.rs:175,291`
  `submit_nova_os` and `CliOutput::EnterCommands`.
- `crates/nova_os_ui/src/terminal/mod.rs:134-155` `NovaOsSystems { Toggle,
  Input, Simulate, Paint }`; 160-190 `sync_nova_os_contexts`; 193-378
  `NovaOsPlugin`. `crates/nova_os_ui/src/lib.rs:74-110` `NovaOsUiPlugin`,
  `MonitorFrame`. `crates/nova_core/src/lib.rs:444-478`: HUD, then
  `NovaOsUiPlugin` (445), then menu (469), then console (478).
- Map app: `crates/nova_os_ui/src/map/mod.rs:179-235` `NovaOsMapPlugin`
  registers `MapApp` with `map view` and `map goto`; `map/app.rs:18,37`
  `sync_map_arg_completions`, `apply_map_cli_commands`; `map/scene.rs:275`
  `map_input` (live GOTO key).
- Ship app: `crates/nova_os_ui/src/ship/mod.rs:122-159` `NovaOsShipPlugin`,
  165 `ship_command_tree`; `ship/app.rs:160,186,317`
  `sync_ship_arg_completions`, `apply_ship_cli_commands`,
  `apply_ship_section_commands`; `ship/rebind.rs:24` `apply_ship_rebind`;
  `ship/scene.rs:753-798` repair, rebind, reload button observers; 888
  `mate_edges_mesh`.

### Command shell (`:`) and log sources
- `crates/nova_os/src/commands.rs:172-455` `COMMAND_CATALOG` (`ships` 246,
  `ship` 256, `sections` 266, `section` 276, `objectives` 291); 460
  `command_shell_specs`; 742 `resolve_command_line`.
- `crates/nova_console/src/lib.rs:46-90` `ConsoleSystems::Dispatch`, ordered
  after `NovaOsSystems::Input` and before `NovaOsSystems::Simulate`;
  `crates/nova_console/src/dispatch.rs:18-41` `execute` routes catalog verbs to
  `inspect::*`.
- `crates/nova_os_ui/src/terminal/flight_log.rs:23` `sync_nova_os_logs`
  (`StoryFeed`, `GameObjectives`); 104-130 `log_combat_lock_drops` and
  `combat_lock_drop_line` (`CombatLockDropped`); 138
  `announce_objectives_in_terminal`. Model `NovaOsFlightLog` at
  `terminal/components.rs:310-335`.

### Settings, lessons, and consumers
- `crates/nova_menu/src/settings_store.rs:33-92` `PersistedSettings`; keybinds
  at 91 keyed by action name; 202 `nova_os_monitor`; 429-462
  `load_persisted_settings` calls `apply_overrides`.
  `crates/nova_input/src/registry.rs:640-647` `apply_overrides` warns and skips
  unknown names, so without migration a renamed ID silently drops the player's
  rebind.
- `crates/nova_training/src/catalog.rs:31-64` `LessonCategory::NovaOs` (41, 53,
  64). Lessons `novaos_open`, `novaos_terminal`, `novaos_view`,
  `novaos_commands`, `novaos_contacts`, `novaos_service`,
  `novaos_rebind_section`, `novaos_shell` at
  `crates/nova_authoring/src/base_content/lessons.rs:1001-1155`; media
  `assets/base/training/novaos_*.webp` (8 files); generated
  `assets/base/training/base.content.ron`. Lint:
  `crates/nova_authoring/src/lint_walk.rs:310`; load:
  `crates/nova_assets/src/merge.rs:392`; lesson lint
  `crates/nova_training/src/validate.rs:88`.
- `crates/nova_training/src/progress.rs:24-40,98-115` keeps unknown viewed IDs
  and counts only catalog lessons, so old IDs are already inert.
- `crates/nova_training/src/facts.rs:255-259` field note cites `novaos_open`.
  `crates/nova_hud/src/objective_stack.rs:354-387` marks notifications read on
  NOVA OS open and labels the TAB affordance; `crates/nova_hud/src/lib.rs:446,507`
  hide HUD parts in `PauseStates::NovaOs`.
- Crate consumers: `Cargo.toml` and `crates/{nova_core,nova_menu,nova_console,
  nova_channel,nova_debug,nova_probe}/Cargo.toml`; code in `nova_channel`
  (`apply.rs`), `nova_console` (7 files), `nova_core/src/lib.rs`,
  `nova_debug/src/harness.rs`, `nova_menu` (`lib.rs`, `pause.rs`,
  `settings_store.rs`, tests), `nova_probe/src/capabilities/snapshot.rs:786`.
- `examples/playable/ui_app_variants.rs`: `SketchView` 237, `top_bar` 785,
  `rebuild_body` 886. It reuses production map and ship models but owns fixture
  inventory, station, credits, and repair pricing. It is not a production screen.

## Delivery

### End state
- TAB (pad RightThumb) opens and closes the themed interface in flight.
  Keyboard M, gamepad Y, and the Map and Ship buttons in the card's title row
  switch Map and Ship. The world is
  frozen while it is open.
- `:` opens `NOVA COMMANDS` on the retained CRT over any surface except
  Loading and an armed rebind capture. Escape or `close` returns to the stored
  state. TAB inside the modal completes. The modal has no app host.
- `log` prints the flight log. Generic inspection verbs keep their output.

### Before
```
TAB -> toggle_nova_os -> PauseStates::NovaOs + ShellKind::NovaOs
  -> nova> prompt -> NovaOsCommandRegistry -> MapApp / ShipApp (CRT app host)
':' -> open_command_shell -> PauseStates::NovaOs + ShellKind::Commands
  -> cmd> -> nova_console::execute -> COMMAND_CATALOG
```

### After
```
TAB -> interface toggle -> PauseStates::Interface -> Map | Ship pane (M/Y, buttons)
  pane actions: GOTO, Repair, Reload, Rebind, mates
':' -> open_command_shell -> PauseStates::Commands (return_to = previous state)
  -> NOVA COMMANDS CRT -> nova_console::execute -> COMMAND_CATALOG (+ log)
Escape in modal -> return_to (Interface | Paused | Unpaused | menu/editor)
```

### Owner and ordering
- `nova_command` owns the command language: catalog, parser, terminal model,
  and one shell. `nova_interface` owns the TAB pane, the CRT modal surface,
  bindings, and the flight-log model. `nova_console` keeps dispatch.
- Plugin order in `AppBuilder` stays: HUD -> `nova_interface` -> menu ->
  console. `ConsoleSystems::Dispatch` stays after the modal input set and
  before its simulate set.
- `open_command_shell` stays the one `:` owner in `nova_menu`. The single back-out
  owner (today `close_nova_os_from_menu_keys`) handles Escape for both the
  modal and the pane. Precedence: rebind capture, then modal, then pane, then
  pause menu.
- The clock freeze owner stays held across `Interface <-> Commands`, so the
  transition never releases the clocks.

### Proposed signature changes (names pending, see Open names)
- `nova_gameplay::PauseStates { Unpaused, Paused, Interface, Commands }`.
- `nova_command`: delete `ShellKind`; the terminal holds one shell session.
  `COMMAND_CATALOG` gains `CommandSpec { name: "log", usage: "log", class:
  ReadOnly, arity: None, .. }`.
- `nova_console::inspect`: `pub(crate) fn log(world: &World) -> CommandResult`,
  routed from `dispatch::execute`.
- `nova_menu`: a fixed old-to-new action-ID table applied to
  `PersistedSettings::keybinds` inside `load_persisted_settings` before
  `apply_overrides`; the next save writes only new IDs.
- `nova_interface`: the TAB pane state (Map or Ship) and its toggle and switch
  systems, built from the production map and ship scenes. Exact types need
  owner approval before code.

### Deletion list
- `crates/nova_os/src/command.rs` (whole registry), `crates/nova_os/src/app.rs`
  app host, `ShellKind`, `submit_nova_os`, NOVA OS `CliOutput` variants and
  `CommandDispatch::{App, Cli, Gameplay}`, the NOVA OS boot banner, unread
  event count, and objective announcements into the prompt.
- `toggle_nova_os`, `handle_nova_os_app_keyboard`, `sync_nova_os_commands`,
  `sync_nova_os_app_ui`, the NOVA OS shell topbar (`nova_os_topbar_head`) and
  app footer hints, `MapApp`, `ShipApp`, `ship_command_tree`,
  `apply_map_cli_commands`, `apply_ship_cli_commands`, and the two
  `*_arg_completions`. Separate per-pane input contexts remain necessary
  because `map_goto` and `ship_mates` share G; the app-host `ViewerApp` name
  does not survive.
- `LessonCategory::NovaOs`, the eight `novaos_*` lessons and media, and the
  `lesson_novaos*` capture examples.
- The `nova>` prompt strings, and tests that assert deleted behavior.
- Docs for removed unshipped behavior.

### Affected examples, docs, content, and CI
- Examples (`Cargo.toml` entries): `ui_app_variants` (167, becomes redundant
  or trimmed), `system_nova_os` (463), `system_command_shell` (467),
  `system_headless_novaos` (475), `system_headless_drag` (487),
  `system_headless_crt` (491), `screenshot_nova_os_terminal` (634),
  `screenshot_nova_os_apps` (638), `lesson_novaos` (837),
  `lesson_novaos_contacts` (841), `lesson_novaos_prompt` (845),
  `lesson_novaos_ship` (849), `loop_command_shell` (923);
  `examples/screenshots/shared/computer.rs`.
- CI: `.github/workflows/ci.yaml:275`, `.github/workflows/probe-full.yaml:85-97,
  171-172`.
- Docs: `web/src/wiki/nova-os.md`, `commands.md`, `keybinds.md`, `hud.md`,
  `glossary.md`, `settings.md`; `web/src/create/lessons.md`;
  `web/src/docs-manifest.js`, `web/src/site.ts`; `docs/architecture.md`,
  `docs/project-tour.md`, `docs/concept-index.md`, `docs/development.md`.
  Old news posts stay as history.
- Scripts: `scripts/capture-lesson-media.sh`, `scripts/gen-lesson-media.py`.
- Content: regenerate `assets/base/training/base.content.ron` from the builder.
- `CHANGELOG.md` `[Unreleased]`: Interface & HUD entries, marked
  `**(breaking)**` for the removed NOVA OS verbs, crate rename, and the lesson
  category.

### Failure policy
- Content with `category: NovaOs` fails RON parse at lint and at load.
- Old `novaos_*` keybind names in a saved store are renamed on load. Other
  unknown names keep the existing warn-and-skip.
- Typing a deleted app or NOVA OS-only verb (`map`, `exit`) at `NOVA COMMANDS`
  returns the normal unknown-command error. The generic parser's existing
  `version` response (`nova_os/src/commands.rs:773-776`) remains available.
- Old viewed lesson IDs load without error and count toward nothing.

### Work sequence
1. Get owner approval for the open names. Done 2026-09-26.
2. Rename crates; then change `PauseStates` and the terminal model; fix every
   caller through compiler errors.
3. Delete the NOVA OS shell and app host; add `log`.
4. Build the TAB pane from the production map and ship scenes.
5. Rename action IDs and add the keybind migration.
6. Replace lessons, media, capture examples, docs, CI lists, and changelog.

## Verification
- Rendered player flow (lavapipe or hardware, frames inspected): TAB opens the
  interface; keyboard M, gamepad Y, and the title-row Map and Ship buttons
  switch panes; `:` over the pane shows the retained CRT with `NOVA COMMANDS`;
  Escape returns to the same pane.
- Clocks and input: across pane -> modal -> pane and flight -> modal -> flight,
  `Time<Virtual>` does not advance while either surface is open, and no flight
  or pane action fires on the frames of each transition. Same from main menu,
  editor, and pause menu.
- Typing in the modal (letters, TAB completion, Escape) triggers no pane or
  flight action.
- Commands: `ships`, `ship <id>`, `sections`, `section`, `objectives`, and
  `log` return live rows; `log` shows a comms line, an objective update, and a
  labeled lock-drop line.
- Pane actions: GOTO, Repair, Reload, Rebind, and mates keep their game effects.
- Bindings: a v0.14.0-shaped store with a moved `novaos_*` key loads onto the
  renamed action and saves under the new ID.
- Content: a lesson with `category: NovaOs` fails lint and load; old viewed IDs
  load inert.
- Searches find no `nova>`, `NovaOsAppRuntime`, `NovaOsCommandRegistry`,
  `novaos_toggle`, or `nova_os_ui` outside history.
- Permanent tests only for the named stable behaviors above (keybind migration,
  modal return with no gap, `log` rows). Each new test needs owner approval.
- Perf result (2026-09-27): the owner's lag does not reproduce. Real game
  through New Game (seed 7), hardware GPU on Xvfb, 1920x1080, debug+trace
  build, three matched repeat sets of flight, Map and Ship, each with a 10 s
  pointer sweep and a 5 s drag (`target/perf-baseline/drive-ng-sweep.sh`,
  `phase.py`, 2 s settle skipped). Median wall ms per phase (main-thread
  median in brackets):

  | Run | Flight | Map | Ship |
  | --- | --- | --- | --- |
  | `ng-h2`, before the sketch rewrite | 40.8-42.2 (8.8-8.9) | 40.8-41.5 (5.0-5.1) | 41.5-41.7 (5.0-5.2) |
  | `ng-a1`, after the sketch rewrite | 41.0-42.4 (8.6-8.9) | 41.1-42.1 (4.8-4.9) | 41.1-42.1 (5.0-5.1) |
  | `ng-a2`, after the Map label fix | 40.8-41.4 (9.8-10.2) | 41.2-43.2 (5.3-5.5) | 38.9-42.3 (5.8-6.0) |

  Every phase sits at the same ~41 ms wall, flight included, so the frame is
  bound outside the interface systems, and the open interface costs less main
  thread than flight. The `ng-a2` main-thread rise is in flight too, which the
  label change cannot touch, so it is host noise. Not measured: a native
  display, a release build, or the owner's own hardware. Frames and traces
  are in the sprout at `target/perf-baseline/ng-*`.

## Worker defaults (2026-09-27, owner said no questions; pending review)
- Map label legibility: the open-world Map plotted about 200 `AST-n` labels
  in piles (`target/perf-baseline/ng-a1/map1.png`). An asteroid's code label
  now shows only while it is selected, the rule the Ship pane already uses
  for sections. Ship, planet and objective labels stay. `MapBlipLabel` marks
  the pill; `project_map_blips` sets its visibility and a `ZIndex` of 0
  (unlabelled rock), 1 (labelled) or 2 (selected), so a quiet rock never
  covers a label you read. Every blip stays a clickable `Button`. Frames:
  `ng-a2/map1.png`, `ng-l1/map-open.png`, `ng-l1/map-rock.png` (a clicked
  rock shows `AST-154`, its ring and readout).
- `MapContactKind` gains `Neutral` (`NEU-n`, `NEUTRAL`, `Ship on no side.`)
  and `Planet` (`PLN-n`, `PLANET`, `Planetary body.`). `Terrain` is only an
  asteroid. v0.14.0 coded both as `AST-n` with `Asteroid mass.`; changelog
  Fixes entry added. Colours are unchanged (secondary).
- Wiki rebind example reads `Bound engine_port to K`; LMB is refused.
- `system_headless_map_goto` read the selection ring as an `Outline`, but
  the pane draws it as a `ThemedBorder`. Reproduced: the old read stalls 30 s
  at `viewer_next cycles the ring onto a contact`
  (`target/probe-map-goto-outline`). The read is now `ThemedBorder.alpha`;
  `probe run system_headless_map_goto --correctness-only` passes and clicks
  `PLN-1` (`target/probe-map-goto`).

- Rebind gate for TAB and M (review finding 1): an armed section capture now
  also holds back `interface_toggle` and `interface_next_tab`
  (`toggle_interface`, `next_interface_pane`). Reproduced first: TAB closed
  the interface and M flipped to Map, each dropping the capture
  (`target/probe-rebind-gate-before`, `target/probe-rebind-gate-m-before`).
  TAB is now refused by the capture and M binds like any key. Proof: three
  beats in the approved `system_command_shell` rebind group
  (`target/probe-rebind-gate-after`, OK).
- Ship Reset (review finding 6): Reset and `T` now restore the opening zoom
  as well as the angles and centre, as the sketch's `reset_ship` does. The
  existing reset test now nudges the radius; it failed before the fix
  (`target/ship-reset-before.log`, 8.45 vs 16.9) and passes after.
- Ship open zoom: the sketch opened, fit and reset the Ship view at
  `SHIP_PANE_ZOOM` 0.72 of the framing radius. Tried at 0.72 for open and
  Reset, with Fit at 1.0, then reverted. On lavapipe at 1920x1080 the
  2081-section carrier clipped on the left and bottom viewport edges, and the
  3-section range hull clipped its outline at the bottom. At 1.0 both fit
  with margin (`target/zoom-proof/{small,large}-{open,fit,reset}.png`,
  `REPORT.md`). Open, Fit and Reset stay at the full framing radius.
- Escape during an armed section rebind: a disposable beat in
  `system_command_shell` showed Escape cancels the capture with
  `Rebind cancelled` and keeps the Ship pane (`target/zoom-proof/run-escape.log`).
  Not kept: no failure reproduced, and `close_surface_from_menu_keys` runs in
  `CommandsSystems::Toggle`, before `ShipPaneSystems`, and yields to an armed capture.
- The six lesson loops were missing from `assets/base/training/`; captured
  with `scripts/capture-lesson-media.sh` on lavapipe and inspected. The wiki
  shots still showed the rejected first pass; recaptured. The orphan
  `nova-os-open.webm` loop (only consumer was the deleted page) is removed.

## Finishing verification (2026-09-27)
- Rendered: `wiki-interface-map.png`, `wiki-interface-ship.png`,
  `wiki-command-shell.png` and the six lesson sheets, lavapipe on Xvfb,
  frames inspected. Pad Y and the title-row buttons: disposable test, both
  pass, not kept (`target/finish-artifacts/pane-switch-proof`).
- Probes `--correctness-only`: `system_interface` OK, `system_command_shell`
  OK, `system_headless_map_goto` PASS line (UNPROBEABLE verdict, headless).
- Unit tests `--lib`: `nova_interface`, `nova_command`, `nova_console`,
  `nova_menu`, `nova_training` pass, including keybind migration and `log`
  rows. `catalog_drift` passes. `content gen` leaves the training RON
  unchanged; `content lint` 0 errors.
- `content lint --target` on a disposable mod with `category: NovaOs` fails:
  "Unexpected variant named `NovaOs` in enum `LessonCategory`"; the same mod
  with `Interface` lints clean (`target/finish-artifacts/novaos-category`).
- Not run: the `NovaOs` category at game load (same `Lesson` type, not run),
  the editor `:` path in a live app, and a native-display run.

## PR audit (2026-09-27)
- Two living loops still showed NOVA OS: `landing-cockpit.webm` typed
  `nova> map`, and `command-shell-open.webm` had the old header. Both were
  recaptured on lavapipe and frames inspected (`target/pr-audit/`).
- After the last Ship pane and `system_command_shell` edits: `nova_interface`
  `--lib` passes 94 tests, and the `system_command_shell` probe is OK with
  the rebind beats (`target/pr-audit/probe-command-shell`).
- Kept as approved: the `NovaOs*` names in `nova_interface::terminal`, which
  are the retained CRT monitor brand.

## Done when
- The names are approved, the end state above works in the real app, the
  deletion list is gone, docs and changelog are current, and the affected
  checks plus the rendered flow pass. PR #77 alone does not complete this task.

## Origin
- Split from `20260824-125943` after owner acceptance of PR #77's UI look on
  2026-09-26. Coordinate station and inventory panes with `20260926-174806`.
