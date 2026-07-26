# rename 'drawer' related things to 'nova_os' + create a nova_os crate for the OS logic and refactor

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.9.0,refactoring

## Flow State

- FLOW STEP: PLANNING

## Problem

"drawer" is the ORIGINAL internal name for what the game now calls **NOVA OS**:
the Tab-opened ship-computer terminal monitor (CRT-cased cockpit screen that
freezes the sim, frees the cursor, and runs a command prompt with objectives +
flight-log feeds and launchable "apps"). The concept was renamed to NOVA OS in
the UI/casing/app-runtime work (d96dc54d, a5cc312d, 01cdd852, 05ba796e) but the
CODE is a half-finished rename: the visual marker structs already use `NovaOs*`
while the module, plugin, shared state, exempt component, consts and most
systems still say "drawer". The whole thing is also one ~5900-line file that
mixes OS logic (shell language, terminal model, app runtime) with bevy UI.

Two things to fix, in order:
1. Finish the rename: no `drawer`/`Drawer` left anywhere in the NOVA OS
   subsystem, with full cross-crate consistency.
2. Extract the OS LOGIC into a new `nova_os` crate and refactor the monolith
   into sensible modules; keep the bevy UI (which reads game data / the terminal
   model) in `nova_gameplay`.

## Scope

IN scope: the NOVA OS ship-computer in `nova_gameplay/src/hud/drawer.rs`
(~5900 lines) and its cross-crate surface (`PauseStates::Drawer`,
`DrawerTabAnchor`, `HudDrawerExempt`, refs in `nova_menu`, `nova_core`,
`input/player.rs`, `objective_hint.rs`, `objective_reveal.rs`).

OUT of scope: `crates/nova_editor/src/ui/drawer.rs` (the editor's component-card
"drawer" is a DIFFERENT concept - build-UI panel beside the rail - not NOVA OS).
The web easter-egg PoC (`examples/ui/nova_os_terminal_poc.html`, web route) is
standalone HTML/TS and not touched.

## Decisions (see DECISION.md)

- D1 Blast radius: FULL CONSISTENCY - rename `PauseStates::Drawer` ->
  `PauseStates::NovaOs`, `DrawerTabAnchor` -> `NovaOsTabAnchor`,
  `HudDrawerExempt` -> `HudNovaOsExempt`, updating every cross-crate call site.
- D2 Topology: new `nova_os` crate holds OS logic ONLY; the bevy UI + game-data
  bridges + plugin stay in `nova_gameplay` (which depends on `nova_os`).
  `nova_ui` MUST NOT depend on `nova_os`; only tiny model-independent visual
  helpers may move into `nova_ui`, and `nova_gameplay` then depends on both.
  Graph stays acyclic: `nova_gameplay -> nova_os`, `nova_gameplay -> nova_ui`.
- D3 `nova_os` MAY depend on `bevy` (consistent with `nova_ui`/`nova_events`);
  engine-free is NOT required. Needed for the app-runtime trait, `Key` input
  and `Handle<Font>` in row styling.

## Target crate boundary

`nova_os` (LOGIC): `NovaOsTerminal` model (+ `TerminalRow`/`TerminalRowKind`/
`TerminalParseStatus`/`TerminalMode`, `submit`/`refresh_parse`/`reset_*`/
`exit_app`/`replace_current_command`); shell command language (`parse_command`,
`TERMINAL_COMMANDS`, `TerminalCommand`/`TerminalCommandResult`/`...Snapshot`,
`nearest_command`, `current_command_prefix`, `command_has_arguments`, completion/
ghost/hint string fns); app runtime (`NovaOsAppRuntime` trait, `NovaOsAppRegistry`,
`NovaOsAppCommand`, `NovaOsAppInputOutcome`); content builders
(`nova_os_welcome_rows`, `terminal_help_rows`). Split across modules
(`terminal`, `shell`, `app`), NOT one file. Its logic unit tests move with it.

`nova_gameplay` (UI + WIRING, keeps): all `*Marker` structs + casing/CRT/bezel/
screw/vent/slide (`spawn_nova_os_*`, `drive_*_slide`, `DrawerOpenness`->`NovaOs*`,
`DrawerCloseTransition`->`NovaOs*`), `rebuild_terminal_ui`, `spawn_terminal_row`,
`spawn_nova_os_app`, `sync_nova_os_app_ui`, keyboard systems, the plugin
(`NovaDrawerPlugin`->`NovaOsPlugin`), `PauseStates`, HUD-visibility wiring
(`HudNovaOsExempt`, `lift_exempt_chrome_over_nova_os`, `*_EXEMPT_Z`), the public
`NovaOsTabAnchor`, and the game-data bridges (`terminal_snapshot_from_world`,
`terminal_log_rows`, `terminal_objective_rows`, `terminal_ship_rows`, the
`DrawerFlightLog*`/`DrawerObjective*` data+UI, renamed `NovaOs*`).

`nova_ui`: optional tiny model-independent visual helpers only (e.g. a
screw/vent/recessed-plate builder) if cleanly separable; skip if not worth it.

## Steps

### Phase A - finish the rename (in place, nova_gameplay + cross-crate)

- [ ] Rename module `hud/drawer.rs` -> `hud/nova_os.rs`; update
      `hud/mod.rs` (`pub mod drawer` -> `nova_os`, plugin add, doc prose).
- [ ] Rename plugin `NovaDrawerPlugin` -> `NovaOsPlugin`; all `DRAWER_*` consts
      -> `NOVA_OS_*`; systems (`setup_drawer`, `toggle_drawer`,
      `drive_drawer_slide`, `spawn_drawer_*`, `rebuild_drawer_*`,
      `sync_drawer_logs`, `scroll_drawer_panels`, `remove_drawer`,
      `close_drawer_from_menu_keys`) and run-conditions (`drawer_active`,
      `drawer_open`, `in_drawer`, `drawer_prompt_active`) -> `*nova_os*`.
- [ ] Rename types `DrawerFlightLog*`/`DrawerObjective*`/`DrawerRootMarker`/
      `DrawerBackdropMarker`/`DrawerOpenness`/`DrawerCloseTransition`/
      `DrawerScrollViewportMarker` -> `NovaOs*`.
- [ ] D1 cross-crate: `PauseStates::Drawer` -> `PauseStates::NovaOs`
      (`nova_gameplay/lib.rs` + doc), update `nova_menu` (OnEnter/OnExit +
      menu-key handling), `input/player.rs`, `hud/mod.rs`.
- [ ] D1 public: `DrawerTabAnchor` -> `NovaOsTabAnchor` (update
      `objective_hint.rs`, `objective_reveal.rs`); `HudDrawerExempt` ->
      `HudNovaOsExempt`, `DRAWER_EXEMPT_Z` -> `NOVA_OS_EXEMPT_Z`,
      `lift_exempt_chrome_over_drawer` -> `..._over_nova_os`.
- [ ] Rename `drawer_*` test fn names -> `nova_os_*`; update prelude exports
      and any remaining "drawer" prose in nova_gameplay/nova_menu/nova_core
      (do NOT touch `tasks/` history or the editor's `ui/drawer.rs`).
- [ ] `cargo check` + `cargo fmt`; `cargo test -p nova_gameplay nova_os` green
      (the 56 renamed tests), plus `-p nova_menu` for the moved wiring tests.

### Phase B - extract the nova_os crate + refactor

- [ ] Create `crates/nova_os` (Cargo.toml with bevy per D3, `src/lib.rs` with
      `prelude`, `#![warn(missing_docs)]` as it comes clean); add to workspace
      `members` and the crate table in AGENTS.md.
- [ ] Move the LOGIC surface (see Target crate boundary) into `nova_os`, split
      into `terminal` / `shell` / `app` modules; move its unit tests with it.
- [ ] Add `nova_os = { path = "../nova_os" }` to `nova_gameplay`; import via
      `nova_os::prelude`. Leave game-data bridges + all bevy UI + plugin in
      `nova_gameplay`.
- [ ] Opportunistic per D2: extract any tiny nova_os-independent visual helper
      into `nova_ui`; verify `nova_ui` still has NO nova_os dep.
- [ ] Crate-level `//!` rustdoc for `nova_os`; keep
      `cargo doc --workspace --no-deps` warning-free.
- [ ] `cargo check --workspace` + `cargo fmt`; `cargo test -p nova_os` and
      `-p nova_gameplay` green.

### Phase C - verify behavior unchanged

- [ ] Prove the graph: `nova_ui` does not depend on `nova_os`; `nova_gameplay`
      depends on both (cargo tree).
- [ ] Prove the rename: no `drawer`/`Drawer` token remains outside
      `nova_editor/ui/` and `tasks/` (grep).
- [ ] Probe / run the game, open NOVA OS with Tab, and SEE it render + a
      command + an app launch behave exactly as before (screenshot in shots/).

## Definition of Done

1. Rename complete: `grep -rInE '\b[Dd]rawer\b' crates/ src/ --include=*.rs`
   returns nothing outside `crates/nova_editor/src/ui/` (cmd).
2. New crate builds standalone: `cargo check -p nova_os` (cmd).
3. Topology holds: `cargo tree -p nova_ui` shows NO `nova_os`; `cargo tree -p
   nova_gameplay` shows both `nova_os` and `nova_ui` (cmd).
4. Behavior preserved: `cargo test -p nova_os` and `cargo test -p nova_gameplay
   nova_os` green; `cargo check --workspace` + `cargo doc --workspace --no-deps`
   clean (cmd).
5. NOVA OS still opens, renders and runs commands/apps identically, confirmed by
   a probe run or a running-game screenshot (manual: owner eyeballs shots/).
