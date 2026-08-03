# Notes: Generated placeholder thumbnails for the Scenarios picker

## What changes

Before: all 13 picker-visible scenarios show one of two shared images - the six
base scenarios (`shakedown_run`, `asteroid_field`, `broadside`,
`broadside_gunship`, `lifeline`, `final_tally`) use
`thumbnail: Some("self://banner.png")` with a `TODO(20260715-220011)` beside
each, and the seven webmod scenarios (`gauntlet`, `ledger_ch1..ch5`) use
`dep://base/textures/asteroid.png`. Selecting a different scenario changes the
text but not the picture.

After: each scenario resolves to its own deterministic 320x180 PNG, generated
by a stdlib-only python script in the phosphor look (glitched title text on a
dark scanlined field). Real art remains owner work and overwrites the same
paths later with no code change.

Scope change 2026-08-04: in-game capture of scenario art is OUT. A scenario
image may want a drawn/composed look rather than a gameplay still, and mod-owned
scenarios would each need an example. If a specific scenario later wants a
real in-game still, it gets its own task with an explicit view/vibe brief.

## Surfaces

| File | Why |
| --- | --- |
| `scripts/gen-scenario-thumbnails.py` | New. The generator. |
| `scripts/gen-web-screenshots.py` | Holds the stdlib PNG encoder/decoder to reuse (`write_png`, `Canvas`, `draw_icon` precedent) and the coverage report that lists outstanding real art. |
| `scripts/gen-placeholder-sounds.py` | The precedent to follow: deterministic stdlib generator, overwrite-with-real-assets contract. |
| `crates/nova_assets/src/scenario/*.rs` | Six builders holding the `TODO` placeholders; source of truth for generated RON. |
| `assets/base/scenarios/*.content.ron` | Generated output; `content_ron_parity` keeps it honest. |
| `webmods/the-ledger/*.content.ron`, `webmods/gauntlet/gauntlet.content.ron` | Seven hand-authored placeholders. |
| `crates/nova_menu/src/scenarios.rs` | `refresh_scenario_details` mounts the thumbnail; `poll_scenario_thumbnail` re-arms on load; non-2D images are skipped with a warning. |
| `crates/nova_scenario/src/loader/mod.rs` | `ScenarioConfig::thumbnail: Option<AssetRef<Image>>`. |

## Data and interfaces

```python
# scripts/gen-scenario-thumbnails.py  (stdlib only, deterministic)
THUMB_SIZE = (320, 180)
SCENARIOS = [
    # scenario id            title shown        destination
    ("shakedown_run",        "SHAKEDOWN RUN",   "assets/base/scenarios/thumb-shakedown-run.png"),
    ("gauntlet",             "GAUNTLET",        "webmods/gauntlet/thumb-gauntlet.png"),
    ...
]
def render(title: str, seed: int) -> bytes:  # phosphor field + scanlines + glitched title
def main(): ...   # --check re-renders and diffs instead of writing
```

Seed derives from the scenario id, so the glitch pattern is stable per
scenario and identical across machines - `--check` is a byte comparison.

## Sketches

Illustrative only.

```diff
-        // TODO(20260715-220011): placeholder thumbnail; real per-scenario art pending.
-        thumbnail: Some(AssetRef::from("self://banner.png")),
+        thumbnail: Some(AssetRef::from("self://scenarios/thumb-broadside.png")),
```

```
+---------------------------------------+
|  ..... scanlines .....                |   dark field, phosphor green
|        B R O A D S I D E              |   title, offset/torn glitch rows
|        ::broadside::                  |   dim id line
+---------------------------------------+   320x180
```

## Shape

```
scenario id/title --> gen-scenario-thumbnails.py --> 320x180 PNG
                          (deterministic, stdlib)        |
                                                         v
                        assets/base/scenarios/*.png   webmods/<mod>/*.png
                              ^                            ^
   nova_assets builders (self://)                          |
   webmod .content.ron (dep:// or self://) -----------------
                              |
                              v
              nova_menu refresh_scenario_details -> details pane
                              |
   gen-web-screenshots.py --report: "still generated, real art pending" (manual)
```

## Consequences and open questions

- Cost: one script plus 13 path edits and a `content -- gen` round trip. Far
  cheaper than the capture pipeline this replaced.
- 13 more PNGs in the asset tree (WASM payload), but tiny at 320x180 and they
  are the files real art will overwrite, not extra ones.
- Open: whether webmod thumbnails should switch from `dep://base/...` to
  `self://` so a mod owns its own art. `self://` is the honest shape; it means
  the generator writes into each mod dir.
- Open: font. There is no stdlib text renderer, so the title is either a tiny
  hand-drawn block-glyph set (the `draw_icon` approach, already in the repo) or
  heavily stylized shapes that only suggest letters. A 5x7 block font is enough
  at 320x180 and stays stdlib-only.
- Open: whether the picker assertion lives in a `ui/` example or a menu unit
  test. The `ui/` example proves it in the live tree, which is the category's
  contract.
