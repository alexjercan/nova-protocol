# Extend screenshot_ui: settings pane and Scenarios campaign picker beats

- PRIORITY: 68
- TAGS: v0.10.0,screenshot,examples
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154
- DEPENDS ON: 20260805-112749

## Context

The four UI-state shots of the refresh (`20260805-105154`). `screenshot_ui`
already drives the shipped app (`editor_app`) through the autopilot for
`tutorial-menu` (main menu over the ambience backdrop) and `feature-editor` (the
sandbox editor). Two shots have no producer at all and belong here:

- `wiki-settings.png` - the settings pane. `crates/nova_menu/src/settings.rs`
  exists and `crates/nova_menu/src/tests/settings.rs` drives it.
- `news-090-scenario-campaigns.png` - the Scenarios picker with its collapsible
  campaign headers (`crates/nova_menu/src/scenarios.rs`). The
  `examples/ui/menu_scenarios.rs` example already clicks through that picker
  with real synthesized pointer input - copy its gesture, do not reinvent it.

This is the one scene task with no 3D set to design: the "look" is the real UI
in a real state. The kit's photo rig is irrelevant here except for the menu
backdrop behind `tutorial-menu`.

Depends on the photo kit only for the menu backdrop shot; sequenced after
`20260805-112749` so the look is settled first.

## Steps

- [ ] Add a settings beat: reach the settings pane the way a player does and
      settle before framing. Reuse the pointer gesture from
      `examples/ui/menu_scenarios.rs` (pointer moved to the widget's resolved
      screen position, pressed and released there - nothing triggered through
      its observer).
- [ ] Add a Scenarios-picker beat with a campaign expanded, so the shot shows
      the campaign grouping the v0.9.0 post is about.
- [ ] Re-frame `tutorial-menu` and `feature-editor`: the editor shot needs a
      ship built from sections on screen, not an empty grid.
- [ ] Give `wiki-settings.png` and `news-090-scenario-campaigns.png` FIGURES
      slots in `scripts/gen-web-screenshots.py` naming `screenshot_ui` - they
      have no manifest slot today.
- [ ] Check what the menu backdrop scenario gives the menu shot
      (`menu_ambience` / `menu_scrapyard` / `menu_waystation`) and pick the one
      that frames best behind the menu chrome.
- [ ] Hand it to the owner: run plainly, step the four states, verdict.

## Definition of Done

- The example builds and the catalog agrees with disk.
  (cmd: `nix develop --command cargo check --examples --features debug`)
- The run reaches every UI state and exits clean headless, naming any step that
  stalls. (test: `screenshots_reach_playing_without_panic`)
- The report names `screenshot_ui` for all four shots, including the two that
  have no slot today.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner inspects the four states and accepts them as good enough to shoot.
  (manual: `cargo run --example screenshot_ui --features debug`, no NOVA_REEL)

## Notes

- No PNG is captured or committed in this task.
- Driven clicks pin the pointer for the press/release pair
  (`crates/nova_autopilot::input`, guarded by `tests/pointer_pin.rs`) - a stray
  cursor event mid-click silently cancels the click otherwise
  (`20260805-091151`).
- The menu backdrops keep the flat single scenario light until
  `20260805-111534`; an example cannot re-light the shipped menu scene without
  it.
