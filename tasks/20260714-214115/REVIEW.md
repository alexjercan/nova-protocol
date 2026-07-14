# Review: restyle nova_menu to nova_ui theme

- TASK: 20260714-214115
- BRANCH: ui/menu-restyle

## Round 1

- VERDICT: APPROVE

A colour/metric-only restyle (no `Name`, layout-structure, or interaction
changes). Load-bearing claims re-verified:

- Interaction intact: the `12_menu_newgame` autopilot drove the restyled menu ->
  New Game -> the shakedown_run scenario loaded (player ship + beacons + crates
  spawned), cycle complete, no panic. The buttons still fire by `Name`.
- Semantics preserved, not flattened: the only meaningful menu colours were the
  enabled/base "on" greens; they map to `theme::CYAN`/`CYAN_BRIGHT` (the active
  accent) with disabled -> `theme::TEXT`, so the on/off distinction stays legible.
  The pause scrim (a dim, not chrome) is left as `srgba(0,0,0,0.6)`.
- Self-containment: kept the menu's own `update_button_colors` polling system
  (extended to set the border too) instead of `nova_ui`'s observers, so the global
  colour observers are NOT double-registered alongside the editor's. Correct call
  given both plugins load in the assembled app.
- `cargo check --workspace --all-targets --features debug` clean; 11 menu tests
  pass; no unused-import warnings.

No blocking or non-blocking findings. Residual (shared with the whole family): a
clean-machine visual capture of the menu would be nice but BCS_SHOT + the menu's
live ambience scene is finicky; the autopilot + tests carry the verdict.
