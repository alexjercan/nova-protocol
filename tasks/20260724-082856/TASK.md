# Refresh frontend app images: fill missing + re-capture stale screenshots across web/

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.10.0, web, assets, screenshot
- KIND: STORY
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955
- DEPENDS ON: 20260802-120045

## Story

Use the automated showcase pipeline to replace every missing, placeholder, or
stale website capture with current v0.10.0 output. This task owns the shipped
asset refresh, not the capture mechanism built by `20260802-120045`.

## Steps

- [ ] Inventory all image references under `web/src/` and classify current,
      missing, placeholder, stale UI, stale version chrome, alias, or authored
      non-game art.
- [ ] Reconcile thumbnail naming between the website and shared capture manifest;
      retain one version-based scheme and remove obsolete devlog aliases.
- [ ] Run the canonical showcase capture. Refresh HUD, radar, flight, combat,
      editor, NOVA OS, gravity, sections, tutorial, feature, and news images that
      the inventory marks stale or missing.
- [ ] Review every generated image at its actual page crop. Adjust source scene
      checkpoints or camera framing, then recapture; do not hand-fix generated
      screenshots.
- [ ] Run the strict asset check and website CI. Open the rendered landing,
      tutorial, news, and affected wiki pages.

## Definition of Done

- Every game screenshot reference resolves to current generated output with one
  declared producer. (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --check`)
- The site build has no missing asset reference or obsolete screenshot name.
  (test: `web_asset_manifest_covers_every_game_capture`)
- HUD/radar captures show the current instruments and no pre-v0.10.0 version
  chrome. (manual: inspect rendered HUD and radar wiki figures)
- Landing, tutorial, news, and affected wiki pages use intentional crops with no
  placeholder fallback. (manual: inspect the locally rendered website pages)

## Notes

- Known drift: site version thumbnails and `thumb-devlog-*` generator names do
  not currently agree; `wiki-hud.png` and `wiki-radar.png` reuse older captures.
- Authored diagrams/icons may remain authored. Only game-rendered imagery needs
  an automation producer.
