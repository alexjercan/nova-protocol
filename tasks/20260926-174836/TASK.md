# Replace TAB NOVA OS with the themed interface and a command-only CRT

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.15.0,ui,novaos,migration

Approved plan, implementation PAUSED by owner. The owner approved the corrected
plan on 2026-09-26 and asked to record it here before any code change. Do not
start implementation until the owner resumes it. Base: `master` at `6eaf99673`.
All line numbers below are from that commit.

## User facts
- The owner approves the visual direction of the example-only
  `ui_app_variants` sketch (PR #77). Ship it as the real TAB interface. TAB
  never opens the old NOVA OS computer.
- This is a breaking replacement. No old NOVA OS screen, parallel app host,
  legacy default, adapter, alias, or fallback survives.
- Station, cargo, credit, repair-economy, and persistence mechanics belong to
  `20260926-174806`. The sketch's stock and transactions are not game state.

## Decisions (owner-approved)
- TAB opens a themed interface with two panes, Map and Ship. Top buttons,
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

## Open names (agent proposals, confirm at the implementation gate)
The approved decisions above do not fix these names. Each needs owner approval
before code uses it.
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
  lesson IDs.
- Persisted monitor fields `nova_os_bright_detent`, `nova_os_scan_detent`,
  `nova_os_sound_enabled`: recommend keeping the shipped keys unchanged, so no
  migration is needed.

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
  Keyboard M, gamepad Y, and the top buttons switch Map and Ship. The world is
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
1. Get owner approval for the open names.
2. Rename crates; then change `PauseStates` and the terminal model; fix every
   caller through compiler errors.
3. Delete the NOVA OS shell and app host; add `log`.
4. Build the TAB pane from the production map and ship scenes.
5. Rename action IDs and add the keybind migration.
6. Replace lessons, media, capture examples, docs, CI lists, and changelog.

## Verification
- Rendered player flow (lavapipe or hardware, frames inspected): TAB opens the
  interface; keyboard M, gamepad Y, and the top buttons switch panes; `:` over the pane shows
  the retained CRT with `NOVA COMMANDS`; Escape returns to the same pane.
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

## Done when
- The open names are approved, the end state above works in the real app, the
  deletion list is gone, docs and changelog are current, and the affected
  checks plus the rendered flow pass. PR #77 alone does not complete this task.

## Origin
- Split from `20260824-125943` after owner acceptance of PR #77's UI look on
  2026-09-26. Coordinate station and inventory panes with `20260926-174806`.
