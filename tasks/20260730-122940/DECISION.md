# DECISION: keycaps are trimmed at load, and pinned by HEIGHT

- STATUS: ACCEPTED
- DATE: 2026-07-30
- TASK: 20260730-122940

## The fork

Two load-bearing choices had to be made before writing the fix, and they are
independent.

### 1. Where the cap's shape comes from

- **A hand-maintained aspect table** next to `KEY_GLYPH_FILES`: one literal per
  keycap file. Trivial, no runtime work, no image reads - and silently wrong the
  first time an artist replaces a PNG or a new key is mapped, which is exactly
  how the current bug was born (the `GLYPH_PX` doc asserted the art was square).
- **CHOSEN: trim at load.** Scan each preloaded glyph's alpha channel once, in
  `OnEnter(GameAssetsStates::Processing)` where the whole collection is already
  loaded, and store the cap rect beside the handle (`KeyCap`). A new or replaced
  glyph then just works.

Cost is one 128x128 alpha scan per distinct glyph (13 today), once per run. It
needs no new dependency and no filesystem access, so it is wasm-safe: the pixels
are already in the main world because the image loader defaults to
`RenderAssetUsages::default()` (`MAIN_WORLD | RENDER_WORLD`).

An unreadable or fully transparent glyph resolves to `None`, and `KeyCap`'s
aspect falls back to 1.0 - the old square box. That keeps bare-app rigs (which
never load pixels) rendering exactly as they did, instead of dividing by a
missing measurement.

### 2. Which dimension is pinned

The art draws wide keys SHORTER as well as wider (Tab is 112x74 where X is
96x104), so "preserve the aspect" alone does not say what to preserve it
against:

- **Pin the canvas scale** (cap = bbox * height/128): art-faithful - Tab would
  still render shorter than X, just without the empty bands. It does NOT make
  the wide legends bigger, which was the entire complaint.
- **CHOSEN: pin the HEIGHT** (`height = <site constant>`, `width = height *
  aspect`), which is the owner's stated rule: "keep the width:height ratio, and
  constrain the height only".

The consequence, accepted deliberately: every cap now renders the same height,
so Tab/Shift/Ctrl legends read LARGER than X's letter, which they do not in the
source art. That is the readability the playtest asked for.

## Consequences

- `KeyGlyphs` stores `KeyCap`s, not bare `Handle<Image>`s, and `KeyCap` owns the
  one sizing rule (`node_size`/`apply`/`node`). All three keycap sites - dock
  chips, anchored cues, objective-stack TAB footer - go through it, so no site
  sets `width == height` by hand any more.
- `ImageNode.rect` is the trimmed cap, so the transparent bands stop being drawn
  at all rather than being drawn empty.
- Backlog 20260728-214929 (glyphs on the web key-UI, NOVA OS help, editor chips)
  inherits `KeyCap` as the sizing path.
