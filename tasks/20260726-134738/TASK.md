# Match NOVA OS drawer to terminal PoC

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.9.0,feature,ui,hud

## Story

As a player opening NOVA OS, I want the in-game drawer to visually match
`examples/ui/nova_os_terminal_poc.html`, so that the Bevy implementation feels
like the same cockpit terminal rather than an approximation of it.

The current code already has the right broad architecture from
`20260726-115320` and `20260726-115324`: one drawer-owned Bevy UI monitor under
`PauseStates::Drawer`, and a terminal registry with only `help` and `clear`.
This task is a UI-only fidelity pass. It must not implement `log`,
`objectives`, `ship`, `map`, app runtime, ship viewer, `exit`, `reload`, or
`repair`; those remain in `20260726-115330`, `20260726-115334` and
`20260726-115339` or their existing stretch tasks. Keep the existing objectives
and flight-log data plumbing in place for future commands, but do not render it
as permanent panels.

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Steps

- [x] Compare `examples/ui/nova_os_terminal_poc.html` against the current
      `crates/nova_gameplay/src/hud/drawer.rs` tree and list the concrete visual
      deltas in `tasks/20260726-134738/NOTES.md` before editing.
- [x] Rebuild the monitor body in `drawer.rs` to match the PoC structure:
      full-screen inset `.hud`, full-height `.bezel`, single `.screen`,
      `.screen-inner` with topbar, terminal area, prompt row, footer hints, and
      optional sleep/suspended overlay shape only if it maps cleanly to the real
      drawer state.
- [x] Remove the visible permanent `FLIGHT LOG` and objectives blocks from the
      open drawer UI; keep `DrawerFlightLog` and objective data/resources/tests
      as internal plumbing for the later command-output tasks.
- [x] Match PoC proportions and constants as closely as Bevy UI allows:
      viewport inset `clamp(14px, 3vw, 42px)` equivalent, bezel padding
      `clamp(18px, 2.6vw, 34px)` equivalent or tested responsive breakpoints,
      green phosphor screen, dark blue-black casing, amber prompt, dim status
      text, topbar lamp, right-side status spans, and footer hint row.
- [x] Add a CRT material path for fidelity where normal Bevy UI nodes are too
      weak: either a drawer-specific `UiMaterial`/WGSL overlay for scanlines,
      vignette/glass and phosphor tint, or a recorded fallback in `NOTES.md`
      with a concrete follow-up task if Bevy UI material constraints block it.
- [x] Keep the terminal command registry executable set to exactly `help` and
      `clear`; update tests so POC-only commands (`log`, `objectives`, `ship`,
      `map`, `ship viewer`, `exit`) are not accepted yet.
- [x] Add or update headless widget-tree tests that assert the PoC shape and
      absence of stale permanent panels: topbar brand/status, terminal
      scrollback, prompt row, footer hints, scanline/vignette/material overlay,
      and only `help`/`clear` commands.
- [x] Add a visual verification artifact or runnable capture path for the human
      comparison against `examples/ui/nova_os_terminal_poc.html`; read the
      produced artifact before close-out, per the repo visual-output lesson.
- [x] Update live player-facing docs (`CHANGELOG.md`, `web/src/wiki/hud.md`, and
      any other non-task surface found by grep) only if their current NOVA OS
      description becomes stale.
- [x] Add/update `tasks/20260726-134738/NOTES.md` with what changed, why this
      approach was chosen, shader/material tradeoffs, difficulties/bugs, and
      self-reflection.

## Definition of Done

- The in-game drawer spawns the same visual hierarchy as the HTML PoC: one inset
  monitor, physical bezel, single phosphor screen, topbar, terminal scrollback,
  prompt row, footer hints, and CRT overlay treatment. (test:
  `drawer_matches_nova_os_terminal_poc_structure`)
- The old permanent flight-log/objectives panes are not visible in the NOVA OS
  layout, while their backing logic remains available for future commands.
  (test: `nova_os_keeps_log_objective_state_without_visible_panes`)
- Only `help` and `clear` execute; planned commands from `20260726-115330`,
  `20260726-115334` and `20260726-115339` still return unknown-command behavior.
  (test: `nova_os_only_help_and_clear_are_registered`)
- The CRT treatment is implemented with a shader/material overlay or explicitly
  recorded as blocked/deferred with a follow-up task and a UI-node fallback.
  (cmd: `grep -n "CRT" tasks/20260726-134738/NOTES.md`)
- Touched drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)
- Formatting and build checks pass after the final edit. (cmd:
  `nix develop --command cargo fmt --check` and cmd:
  `nix develop --command cargo check`)
- manual: compare the running drawer or captured screenshot against
  `examples/ui/nova_os_terminal_poc.html` and confirm the remaining differences
  are either impossible in current Bevy UI or intentionally recorded.

## Notes

- Epic: `tasks/20260725-104330/TASK.md`.
- Builds after closed shell task `20260726-115320` and closed terminal input task
  `20260726-115324`.
- Blocks command/app tasks until the UI no longer needs to be reshaped under
  them: `20260726-115330`, `20260726-115334`, `20260726-115339`.
- Current code facts: `drawer.rs` already owns `NovaDrawerPlugin`,
  `NovaOsTerminal`, `TERMINAL_COMMANDS`, the monitor tree, CRT overlay markers,
  flight-log/objective state, and tests around monitor structure. The current
  layout still renders separate `FLIGHT LOG`, objectives and `TERMINAL` blocks
  inside the screen, unlike the PoC's single terminal surface.
- Use the default Bevy font for now; do not introduce the web font task here.
- Assumption for the plan gate: this task changes only the NOVA OS drawer UI and
  terminal executable command set. It does not implement any pending gameplay
  command output, app runtime, or app content.

## Work Record

- Rebuilt the NOVA OS monitor body in `crates/nova_gameplay/src/hud/drawer.rs`
  to match the HTML PoC structure: topbar with lamp/status, one terminal
  surface, scrollback, prompt row, footer hints, casing/bezel and CRT overlays.
- Removed visible permanent Flight Log and Objectives panes from the spawned
  monitor tree while keeping `DrawerFlightLog`, objective row derivation and
  teardown logic for future command-output tasks.
- Added `assets/shaders/nova_os_crt.wgsl` plus a drawer-specific
  `UiMaterial`/`MaterialNode` overlay. Minimal headless rigs still get the
  existing scanline/vignette UI-node fallback.
- Kept the command registry to exactly `help` and `clear`; planned commands
  still return unknown-command behavior.
- Updated `CHANGELOG.md` and `web/src/wiki/hud.md` so live docs no longer say
  Flight Log and Objectives are visible drawer panes.
- Verification:
  `grep -n "CRT" tasks/20260726-134738/NOTES.md`;
  `nix develop --command cargo fmt --check`;
  `nix develop --command cargo test -p nova_gameplay drawer`;
  `nix develop --command cargo check`;
  `cd web && npm ci && npm run ci`.
- The first web CI attempt failed before `npm ci` because `prettier` was absent
  in the fresh worktree. After `npm ci`, `npm run ci` passed. `npm ci` reported
  existing audit vulnerabilities unrelated to this task.
- No standalone `naga` WGSL validator is available in the devshell; `which naga`
  failed. The shader path is pinned by the material-node test and must still be
  visually checked in a real run.
- Manual visual comparison against `examples/ui/nova_os_terminal_poc.html`
  remains pending owner acceptance.
