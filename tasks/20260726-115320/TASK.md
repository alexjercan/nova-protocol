# NOVA OS monitor shell and visual treatment

- PRIORITY: 49
- TAGS: v0.9.0, feature, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player opening Tab, I want the old drawer panels to be replaced by one
inset NOVA OS cockpit monitor, so that the ship computer feels like a physical
terminal screen rather than a floating UI overlay. This is the first build task
from the feedback epic `20260725-104330`; use
`examples/ui/nova_os_terminal_poc.html` as the visual reference, not as
production code.

## Steps

- [x] In `crates/nova_gameplay/src/hud/drawer.rs`, replace the two
      `DrawerRootMarker` side panels and `DrawerSide` slide model with one
      drawer-owned `NovaOsMonitor` root under the existing
      `PauseStates::Drawer` toggle.
- [x] Keep the accepted freeze/cursor behavior from
      `tasks/20260724-102304/DECISION.md`; do not introduce a second pause state,
      drawer state, or a new route around `PauseStates::Drawer`.
- [x] Rebuild the drawer child tree as a physical monitor: dark blue-black
      casing root, hard bezel, inset green phosphor screen, NOVA OS top bar,
      terminal-like scrollable body area, prompt/status row placeholder, and
      orange/yellow accent slots derived from the PoC.
- [x] Preserve the current `DrawerFlightLog` and objectives data plumbing as
      internal content for the monitor placeholder, so later terminal-output
      tasks can reuse the live feed instead of re-deriving it.
- [x] Add scanline, vignette, and screen-glass layers using normal Bevy UI nodes
      and translucent backgrounds/borders. Record any shader deferral in
      `NOTES.md` if a custom material path is avoided.
- [x] Change drawer visibility and z-order tests so they assert one inset monitor
      above the backdrop and no permanent left/right panels.
- [x] Change HUD suppression in `crates/nova_gameplay/src/hud/mod.rs` so ordinary
      flight HUD and lower-left key hints hide behind NOVA OS while diagnostic
      status chrome such as FPS/version remains visible by the chosen rule.
- [x] Update drawer-focused tests for monitor structure, monitor z-order, scroll
      viewport behavior, freeze/cursor preservation, and HUD suppression.
- [x] Add/update `tasks/20260726-115320/NOTES.md` with what changed, why the PoC
      was adapted this way, rendering difficulties, and self-reflection.

## Definition of Done

- Opening Tab shows one inset NOVA OS monitor, not separate left/right drawer
  panels. (test: `drawer_spawns_single_nova_os_monitor`)
- The monitor uses dark casing, green phosphor screen, orange/yellow accents,
  scanline/vignette treatment, and no edge-to-edge floating panel. (manual:
  compare a real run or screenshot against `examples/ui/nova_os_terminal_poc.html`)
- The drawer still freezes gameplay and frees the cursor through
  `PauseStates::Drawer`. (test: existing drawer freeze/cursor tests still pass)
- Ordinary flight HUD and lower-left key hints are hidden behind NOVA OS while
  diagnostic FPS/version remains visible according to the chosen diagnostic
  chrome rule. (test: `nova_os_hides_flight_hud_but_keeps_diagnostics`)
- Touched drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)

## Notes

- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- Decision: `tasks/20260725-104330/DECISION.md`.
- Local decision: `tasks/20260726-115320/DECISION.md`.
- Visual reference: `examples/ui/nova_os_terminal_poc.html`.
- This task unblocks the terminal input task `20260726-115324`.
- Assumption for the plan gate: this task builds the concrete artifact as a
  bespoke Bevy UI monitor tree owned by `hud/drawer.rs`, not as a shared status
  bar item, not as reusable menu chrome, and not as two restyled side panels.
- Current code facts: `hud/drawer.rs` owns the Tab toggle, backdrop, real-time
  slide animation, `DrawerFlightLog`, objective rows, and scroll viewports.
  `hud/mod.rs` hides tiered HUD while `PauseStates::Drawer` is active, but the
  current exemption includes the status strip and lower-left key hints. This
  task narrows that exemption so key hints no longer sit over NOVA OS.

## Work Record

- Replaced the two drawer side-panel roots with one `NovaOsMonitor` Bevy UI tree
  under the existing `PauseStates::Drawer` toggle and real-time openness driver.
- Adapted the PoC as normal Bevy UI nodes: casing, bezel, phosphor screen,
  scanline layer, vignette/glass layer, accent slots, terminal top bar,
  scrollable flight log, objectives block and prompt placeholder.
- Kept `DrawerFlightLog`, objective rebuilding, wheel scrolling, Tab/gamepad
  toggle, freeze/cursor behavior and drawer z-order on the existing state path.
- Narrowed drawer chrome exemption so lower-left key hints hide with ordinary
  flight HUD while diagnostic/status chrome can remain visible above NOVA OS.
- Updated `CHANGELOG.md` and `web/src/wiki/hud.md` because player-facing drawer
  behavior changed.
- Verification:
  `nix develop --command cargo test -p nova_gameplay drawer`;
  `nix develop --command cargo test -p nova_gameplay nova_os_hides_flight_hud_but_keeps_diagnostics`;
  `nix develop --command cargo check`;
  `cd web && npm ci && npm run ci`.
- Manual visual comparison against `examples/ui/nova_os_terminal_poc.html`
  remains a human acceptance item.
