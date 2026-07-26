# NOVA OS monitor shell and visual treatment

- STATUS: OPEN
- PRIORITY: 49
- TAGS: v0.9.0,feature,ui,hud

## Story

As a player opening Tab, I want the old drawer panels to be replaced by one
inset NOVA OS cockpit monitor, so that the ship computer feels like a physical
terminal screen rather than a floating UI overlay. This is the first build task
from the feedback epic `20260725-104330`; use
`examples/ui/nova_os_terminal_poc.html` as the visual reference, not as
production code.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Steps

- [ ] In `crates/nova_gameplay/src/hud/drawer.rs`, replace the two
      `DrawerRootMarker` side panels and `DrawerSide` slide model with one
      drawer-owned `NovaOsMonitor` root under the existing
      `PauseStates::Drawer` toggle.
- [ ] Keep the accepted freeze/cursor behavior from
      `tasks/20260724-102304/DECISION.md`; do not introduce a second pause state,
      drawer state, or a new route around `PauseStates::Drawer`.
- [ ] Rebuild the drawer child tree as a physical monitor: dark blue-black
      casing root, hard bezel, inset green phosphor screen, NOVA OS top bar,
      terminal-like scrollable body area, prompt/status row placeholder, and
      orange/yellow accent slots derived from the PoC.
- [ ] Preserve the current `DrawerFlightLog` and objectives data plumbing as
      internal content for the monitor placeholder, so later terminal-output
      tasks can reuse the live feed instead of re-deriving it.
- [ ] Add scanline, vignette, and screen-glass layers using normal Bevy UI nodes
      and translucent backgrounds/borders. Record any shader deferral in
      `NOTES.md` if a custom material path is avoided.
- [ ] Change drawer visibility and z-order tests so they assert one inset monitor
      above the backdrop and no permanent left/right panels.
- [ ] Change HUD suppression in `crates/nova_gameplay/src/hud/mod.rs` so ordinary
      flight HUD and lower-left key hints hide behind NOVA OS while diagnostic
      status chrome such as FPS/version remains visible by the chosen rule.
- [ ] Update drawer-focused tests for monitor structure, monitor z-order, scroll
      viewport behavior, freeze/cursor preservation, and HUD suppression.
- [ ] Add/update `tasks/20260726-115320/NOTES.md` with what changed, why the PoC
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
