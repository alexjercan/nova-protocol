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

## Steps

- [ ] Replace the current left/right drawer panel shell in
      `crates/nova_gameplay/src/hud/drawer.rs` with a single full-main/inset
      monitor surface that still lives under `PauseStates::Drawer`.
- [ ] Keep the accepted freeze/cursor behavior from
      `tasks/20260724-102304/DECISION.md`; do not introduce a second pause or
      drawer state.
- [ ] Port the PoC's visual structure into Bevy UI: dark blue-black outer casing,
      physical bezel, inset green phosphor screen, orange/yellow accent slots,
      and dense monospace terminal typography.
- [ ] Add a scanline/vignette/screen-glass treatment using the simplest Bevy UI
      path that renders reliably; defer a custom shader if the UI/render path is
      not clean.
- [ ] Hide ordinary flight HUD and key hints while NOVA OS is open, preserving
      only diagnostic screenshot chrome such as FPS/version.
- [ ] Update drawer-focused tests for the single monitor structure, z-order, and
      HUD suppression behavior.
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
- Visual reference: `examples/ui/nova_os_terminal_poc.html`.
- This task unblocks the terminal input task `20260726-115324`.
