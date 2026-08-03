# Generated placeholder thumbnails for the Scenarios picker

- PRIORITY: 68
- TAGS: v0.10.0,menu,scenario,art,tooling
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955

## Story

Every picker-visible scenario shows the SAME image: the six base scenarios use
`self://banner.png` (each carrying a `TODO(20260715-220011)` in its Rust
builder) and the seven webmod scenarios use
`dep://base/textures/asteroid.png`. The details pane therefore tells the player
nothing about what they are about to fly.

Real scenario art is owner work and is NOT generated from the game: a
scenario image may want a drawn or composed look, not a gameplay still, and
in-game capture would mean building an example per scenario (including
mod-owned ones). Owner call 2026-08-04.

So this task ships GOOD PLACEHOLDERS instead: a deterministic python generator
that renders a distinct, on-theme 320x180 image per scenario - phosphor-styled
glitched title text on a dark field, in the NOVA OS look - the same way
`scripts/gen-placeholder-sounds.py` fills the audio gap. The picker stops
looking broken, each scenario is visually distinct, and real art later
overwrites the same paths with no code change.

## Steps

- [ ] Inventory every picker-visible scenario and its current thumbnail source
      (6 base builders + 7 webmod `.content.ron`), incl. hidden/campaign
      members and how the picker indents them.
- [ ] Write `scripts/gen-scenario-thumbnails.py`: stdlib-only, deterministic,
      one 320x180 PNG per scenario from its title/id, in the phosphor look
      (glitched/offset title text, scanlines, dark field). Reuse the PNG
      encoder already in `scripts/gen-web-screenshots.py` rather than
      writing a second one.
- [ ] Write the generated PNGs into the owning asset trees - `self://` for
      base, the mod's own dir for webmods - and point each builder / RON at
      its own file.
- [ ] Regenerate base RON (`cargo run -p nova_assets --bin content -- gen`)
      and keep `content_ron_parity` green.
- [ ] Add a picker assertion that no listed scenario resolves to a shared
      placeholder path, so a new scenario that forgets art is caught.
- [ ] Report outstanding real art: the coverage report in `20260724-082856`
      lists every scenario still on a generated placeholder, classed `manual`.
      Advisory, never a failure.
- [ ] Inspect the picker at its shipped size and confirm each image is legible
      and distinct.

## Definition of Done

- Every picker-visible scenario resolves to its OWN image; no two scenarios
  share one, and none uses `banner.png` or `textures/asteroid.png`.
  (test: `scenario_picker_thumbnails_are_distinct_and_not_shared_placeholders`)
- The generator is deterministic: a second run reproduces every file byte for
  byte. (cmd: `nix develop --command python3 scripts/gen-scenario-thumbnails.py --check`)
- Generated base scenario RON matches its Rust builders.
  (test: `content_ron_parity`)
- The coverage report lists the scenarios still awaiting real art as `manual`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner accepts the generated placeholders in the rendered picker.
  (manual: inspect the Scenarios picker at its shipped layout size)

## Notes

- Real per-scenario art stays owner-owned. If a specific scenario later wants
  an in-game still, that gets its own task with an explicit brief ("this view,
  these objects, this vibe"), not a blanket "capture them all".
- Website post-card thumbnails (`thumb-news-*`) are also owner art and are
  NOT this task.
- Overwriting a generated file with real art at the same path must need no
  code change - the same contract `gen-placeholder-sounds.py` has.
- Picker rendering: `crates/nova_menu/src/scenarios.rs`
  (`refresh_scenario_details`, `poll_scenario_thumbnail`); a thumbnail must be
  a plain 2D image (a cubemap is skipped with a warning).
- Schema: `ScenarioConfig::thumbnail: Option<AssetRef<Image>>` in
  `nova_scenario`.
- No longer depends on the example fleet: nothing here is captured in-game.
