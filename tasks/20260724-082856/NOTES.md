# Notes: Refresh frontend app images

## What changes

Before: `web/src/assets/` holds 31 files. Roughly 45 referenced image names do
not resolve - every `news-0X0-*.png` post figure (36), all seven
`thumb-news-0.X.0.png` post cards, and `wiki-settings.png`. The site renders
those as styled placeholder boxes (`figure__placeholder-name`), 40 of them
across `index.html` (6), `tutorial.html` (4), the nine news posts, and several
wiki pages. Four shipped wiki figures are not captures at all but ALIASES of
other shots (`wiki-radar` = `tutorial-radar-lock`, `wiki-hud` =
`feature-hud`, ...), and the packaging script's thumbnail slots are named
`thumb-devlog-3/4/5.png` while the site asks for `thumb-news-0.X.0.png` - a
naming scheme that never agreed. `gen-web-screenshots.py` reports pending shots
and skips them, so nothing fails; the gap is silent.

After: one naming scheme, one declared producer per game-rendered shot, an
ADVISORY coverage report (`--report`, exit 0, CI warning) that classes every
gap `capturable` / `manual` / `historical`, and a refreshed capture set
produced by the rebuilt `screenshots/` examples.

## Surfaces

| File | Why |
| --- | --- |
| `scripts/gen-web-screenshots.py` | The producer manifest (`FIGURES`, `THUMBNAILS`, `COMPOSITES`, `ALIASES`, `ICONS`) and the copy/validate step. Gains `--report`. |
| `web/src/index.html`, `tutorial.html`, `news.html` | Placeholder figures and post cards; the thumbnail names live here. |
| `web/src/news/*.md` (9) | ~36 placeholder post figures, historical. |
| `web/src/wiki/*.md` | Wiki figures, incl. the aliased four and missing `wiki-settings.png`. |
| `web/src/assets/` | Destination. 31 files today. |
| `examples/screenshots/*.rs` | The producers, rebuilt by `20260802-120029`. Framing fixes land here, never on the PNG. |

## Data and interfaces

```python
# gen-web-screenshots.py, today
FIGURES    = [(web_name, producer_example_or_None), ...]   # 19 entries
THUMBNAILS = [("thumb-devlog-3.png", None), ...]           # 3, all producerless
COMPOSITES = [(out, left_src, right_src)]                  # 1
ALIASES    = {web_name: source_web_name}                   # 4
ICONS      = [(name, section, rgb)]                        # 5, authored

# added - ADVISORY, always exits 0
OWNER = ("capturable", "manual", "historical")

def report(stage_dir) -> None:
    """Scan web/src/** for `assets/<name>`, diff against the manifest and
    web/src/assets/, and print each gap with its owner class. Wrong
    dimensions and undeclared staging files print the same way. Runs no
    capture and never fails: it is a worklist, not a gate."""
```

The reference set must come from the site, not a hand list: scan `web/src/**`
for `assets/<name>` and diff against the manifest, so a new page reference
shows up on the worklist instead of silently 404ing.

## Sketches

Illustrative only.

```diff
 THUMBNAILS = [
-    ("thumb-devlog-3.png", None),
-    ("thumb-devlog-4.png", None),
-    ("thumb-devlog-5.png", None),
+    ("thumb-news-0.7.0.png", "screenshot_combat"),
+    ("thumb-news-0.8.0.png", "screenshot_reel"),
+    ("thumb-news-0.9.0.png", "screenshot_nova_os"),
 ]
```

```
$ python3 scripts/gen-web-screenshots.py --report
capturable  wiki-settings.png          screenshot_ui        MISSING - no capture staged
capturable  news-090-nova-os-apps.png  screenshot_nova_os   MISSING - no producer step yet
manual      thumb-news-0.9.0.png       (hand-made art)      MISSING - owner
historical  news-030-torpedo-blast.png (v0.3.0 build)       MISSING - approximate ok
36 outstanding: 21 capturable, 8 manual, 7 historical            (exit 0)
```

## Shape

```
web/src/**  --scan--> referenced names ----+
                                           |  diff (--report, advisory)
gen-web-screenshots.py manifest -----------+
                                           +--> CI: warning-only worklist
    FIGURES / THUMBNAILS / COMPOSITES / ALIASES / ICONS
          |
          | validate + copy
          v
target/reel/ (staging)  <---- examples/screenshots/* (NOVA_SHOT_DIR, NOVA_REEL)
          |
          v
web/src/assets/   ---> site build (landing, tutorial, news, wiki)
```

## Consequences and open questions

- Cost: mostly inventory and judgment, not code. The script stays stdlib-only
  python; `--report` is a diff plus the existing dimension validation.
- The historical news figures are the hard part: `news-010-*` through
  `news-080-*` depict builds that no longer exist. Re-capturing them with
  current visuals would make old posts lie about what shipped. With an
  advisory report this stops being a gating decision - they simply carry the
  `historical` class. Owner call 2026-08-04: capture what the current build can
  plausibly stand in for, accept that those shots are not exact, and leave the
  rest outstanding. More images beats more empty placeholder boxes.
- The report is also the automation's own input: "which of the outstanding
  images could a `screenshots/` producer make?" is the `capturable` column, so
  it feeds `20260802-120029` producer work directly.
- The four ALIASES are honest reuse but mean four wiki pages show a shot framed
  for something else. Turning each into its own capture step is cheap once the
  producers are on the predicate driver.
- `wiki-settings.png` has no producer at all - the settings pane is a `ui/`
  surface, so its capture belongs in `screenshot_ui`.
- Depends on `20260802-120029` only for the producers' shape; the manifest and
  `--report` work could land first if the sprint order slips.
