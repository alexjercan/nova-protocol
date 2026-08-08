# Fix stale path in modding-ron.md: assets/scenarios -> assets/base/scenarios, four->five built-ins

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: bug, docs, web

## The bug (verified)

`web/src/wiki/dev/modding-ron.md:98` ("Built-ins ported" section) says built-ins
are "data files under `assets/scenarios/`". That dir does not exist
(`ls assets/scenarios/` -> No such file or directory). Real path:
`assets/base/scenarios/` - which lines 41 and 43 of the SAME file already state
correctly, so :98 is an internal contradiction.

Same sentence says "All four built-ins" but `assets/base/scenarios/` holds FIVE
content files: demo, asteroid_field, asteroid_next, menu_ambience, shakedown_run.

## Fix

- :98 `assets/scenarios/` -> `assets/base/scenarios/`.
- "All four built-ins" -> "All five built-ins" (or reword to not hardcode a count).

Source: spike 20260715-235232 (creator persona review, verified).

## Done

modding-ron.md:98: "All four built-ins ... `assets/scenarios/`" ->
"All five built-ins ... `assets/base/scenarios/`". Verified 5 files in
assets/base/scenarios/ (demo, asteroid_field, asteroid_next, menu_ambience,
shakedown_run), all loaded via assets/base/base.bundle.ron.
