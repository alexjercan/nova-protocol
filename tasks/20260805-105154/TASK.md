# Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- PRIORITY: 70
- TAGS: v0.10.0, web, assets, screenshot
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-093910

## Context

Replaces `20260724-082856`, which tried to refresh the shipped web imagery in
place. The shipped captures were deleted instead: every game-rendered PNG is
gone from `web/src/assets/` (only the authored `banner.png`, the five
`icon-*.png` and the font remain), so this task starts from an empty slate and
a full worklist rather than from "which of these 31 files is stale?".

The goal is not parity with what was deleted. The `screenshots/` examples are
rebuilt (`20260804-093910` reduced them to capture-only) and the shots they
pose are dull: default camera, no action, no lighting intent. Redo them so the
site's imagery reads as a game worth playing, one shot at a time.

The advisory coverage report is the input and the progress meter:

    python3 scripts/gen-web-screenshots.py --report

Its `capturable` class is this task's scope: 27 images. The `manual` (7 post-card
thumbnails) and `historical` (25 old-version news figures) classes are NOT in
scope and stay outstanding.

## Inputs

Every `capturable` gap, assigned to the example that will produce it after the
roster change settled in `DECISION.md` - one row is one child task.

| Producer | Was | Images |
| --- | --- | --- |
| `screenshot_scene` | `screenshot_reel` | `feature-gravity`, `wiki-gravity`, `wiki-sections` |
| `screenshot_flight` | `screenshot_orbit` + combat aliases | `feature-autopilot`, `wiki-flight`, `tutorial-orbit` |
| `screenshot_combat` | `screenshot_combat` + `screenshot_juice` | `feature-combat`, `feature-hud`, `wiki-combat`, `wiki-hud`, `tutorial-combat-lock`, `tutorial-radar-lock`, `wiki-radar`, `feature-juice`, `news-090-combat-readability`, `news-090-contextual-hud` |
| `screenshot_sections` | unchanged | the five `wiki-section-*` |
| `screenshot_ui` | `screenshot_ui` | `feature-editor`, `tutorial-menu`, `wiki-settings`, `news-090-scenario-campaigns` |
| `screenshot_nova_os` | `screenshot_nova_os` | `news-090-nova-os-terminal`, `news-090-nova-os-apps` |

Without a manifest slot today: `wiki-settings`, and all five `news-090-*`.
Currently ALIASES, and each becoming its own framed beat: `wiki-combat`,
`wiki-hud`, `wiki-flight`, `wiki-radar`.

Deleted and deliberately not replaced: `devlog5-target-viewfinder.png` and
`devlog5-radar-stance-slots.png` (the composite). The site references neither -
the report listed them "shipped but never referenced" - so the COMPOSITES entry
goes away with them unless a page asks for it again.

## Steps

This task is the container: the work is seven children, each independently
committable and each ending in an owner verdict. Scene tasks 2-6 depend on the
photo kit landing with scene 1; the capture round depends on all six.

- [ ] `20260805-112749` - photo kit + `screenshot_reel` -> `screenshot_scene`
      ("Drydock drift"). Sets the look every later scene inherits, so it is
      first.
- [ ] `20260805-112819` - `screenshot_combat` as a two-faction fight ("Rock
      hollow"), absorbing `screenshot_juice`. Proves AI-vs-AI first.
- [ ] `20260805-112841` - `screenshot_orbit` -> `screenshot_flight` ("The
      ring").
- [ ] `20260805-112903` - re-light and re-frame `screenshot_sections`.
- [ ] `20260805-112929` - extend `screenshot_ui` with settings and the
      Scenarios campaign picker.
- [ ] `20260805-112951` - point `screenshot_nova_os` at the two web names.
- [ ] `20260805-113033` - capture round: shoot all six, package, review at page
      crop.
- [ ] Close this container once the report is clean and the owner accepts the
      rendered site.

## Definition of Done

- The coverage report lists zero `capturable` gaps; what remains outstanding is
  `manual` or `historical`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Every shipped game-rendered image names one declared producer example in the
  manifest; no image is an alias of another (`ALIASES` is empty).
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The six producers are cataloged, smoked and reach `Playing` headless without a
  panic; `screenshot_reel`, `screenshot_orbit` and `screenshot_juice` are gone.
  (test: `catalog_matches_disk`)
- The owner accepted every scene before it was shot, running each example
  plainly with no `NOVA_REEL`.
  (manual: one verdict recorded per scene child task)
- The owner accepts the new shots at their actual page crop.
  (manual: inspect the locally rendered landing, tutorial, news and wiki pages)

## Notes

- Scope is the `capturable` class only. Post-card thumbnails (`manual`) and
  pre-v0.9.0 news figures (`historical`) stay outstanding on purpose.
- Framing fixes land in the example's scene/pose code, never on the PNG.
- Producers are capture-only (`20260804-093910`); `nova_probe` never enters this
  path (`20260802-120045` WONTDO).
- The packaging script and its `--report` flag already exist (`0ff077ff`); this
  task consumes them, it does not build them. Wiring the report into CI as a
  warning-only job is still unowned.
- Planning constraint, found in the code: `catalog_matches_disk`
  (`tests/examples_smoke.rs:120`) treats every `.rs` DIRECTLY under a category
  dir as an example and pins disk == the `[[example]]` catalog. The shared photo
  kit therefore CANNOT be `examples/screenshots/kit.rs`; it lives one level down
  and is pulled in with `#[path = ...] mod`, as
  `examples/sections/turret_section.rs:40` already does.
- Example-side lighting only. Authorable scenario lighting is `20260805-111534`
  and is not a dependency of any child.
