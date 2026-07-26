# Decision: build NOVA OS as a drawer-owned monitor tree

- DATE: 20260726-115320
- STATUS: ACCEPTED
- TASK: 20260726-115320
- TAGS: decision, ui, hud, drawer

## Context

The parent decision in `tasks/20260725-104330/DECISION.md` chooses one inset
NOVA OS monitor with app takeover instead of permanent drawer panels. This child
task needs the concrete implementation artifact for the first slice: the current
`crates/nova_gameplay/src/hud/drawer.rs` still owns two `DrawerRootMarker`
sliding side panels, while `crates/nova_gameplay/src/hud/mod.rs` exempts both
status chrome and lower-left key hints from drawer hiding.

## Decision

Build NOVA OS as a bespoke Bevy UI monitor tree owned by `hud/drawer.rs`. The
tree replaces the two side-panel roots with one inset monitor root under the
existing `PauseStates::Drawer` state. The monitor contains its own physical
casing, bezel, phosphor screen, scanline/vignette/screen-glass overlays, and
placeholder terminal content fed by the existing drawer log/objective data.

## Alternatives considered

- **Restyle the existing left/right panels** - rejected because the accepted
  product shape is one screen with future app takeover, not permanent panes.
- **Put NOVA OS in shared status-bar or HUD chrome infrastructure** - rejected
  because the monitor must be a modal full-main cockpit surface, while the
  status strip remains diagnostic chrome above it.
- **Create a reusable menu/window component first** - rejected for this slice
  because the drawer is the pilot for the style and needs drawer-specific state,
  z-order, scroll, and HUD-suppression behavior before wider UI reuse is known.
- **Use a custom shader for CRT treatment immediately** - deferred unless normal
  Bevy UI layering cannot produce reliable scanlines/vignette/glass. The first
  implementation should prefer deterministic UI nodes and record the limitation
  in `NOTES.md`.

## Consequences

The task can preserve the existing drawer state and live data plumbing while
removing the stale side-panel structure. Tests should assert the spawned Bevy UI
tree rather than pixel-perfect rendering, with manual screenshot comparison
against `examples/ui/nova_os_terminal_poc.html` covering the human visual call.
The owner approved this gate on 2026-07-26 before implementation started.
