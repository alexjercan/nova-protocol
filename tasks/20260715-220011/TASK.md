# Real per-scenario thumbnail art for the Scenarios picker (replace placeholder textures)

- STATUS: OPEN
- PRIORITY: 28
- TAGS: v0.7.0,feature,menu,scenario,art

## Goal

The Scenarios picker (task 20260715-200828) added `thumbnail: Option<AssetRef<Image>>`
to `ScenarioConfig` and renders it in the details pane. Phase 1 pointed the
listed scenarios at PLACEHOLDER images that already ship (`banner.png` for
shakedown_run, `textures/asteroid.png` for demo, `textures/cubemap.png` for
gauntlet_run) just to exercise the rendering path. Replace these with real,
representative per-scenario thumbnails.

## Steps

- [ ] Decide the source: hand-authored art, or generate them via the existing
  screenshot infrastructure (examples/13_screenshot_reel / 15/16/17/18 capture
  a scenario to PNG under Xvfb). Generation is attractive - a scenario's own
  loaded view IS its thumbnail - but needs a per-scenario capture + crop to the
  16:9 box the details pane draws.
- [ ] Add the thumbnails under `assets/` (and the mod's own dir for
  gauntlet_run), sized ~320x180 (the details pane box).
- [ ] Point each listed scenario's `thumbnail` at its real image: the base
  builders (crates/nova_assets/src/scenario.rs + scenario/shakedown.rs) then
  regenerate the `*.content.ron` via the content_ron_parity test; demo and
  gauntlet content.ron by hand.
- [ ] Eyeball the picker (Xvfb screenshot) to confirm the thumbnails read well
  at the rendered size (render-output-eyeball).

## Notes

- Relevant: crates/nova_menu/src/lib.rs `refresh_scenario_details` (renders the
  ImageNode at 16:9), crates/nova_scenario/src/loader.rs `ScenarioConfig`.
- Depends on: 20260715-200828 (the picker + schema field).
- Deferred from that task deliberately: the field + rendering shipped, only the
  art is a placeholder.
