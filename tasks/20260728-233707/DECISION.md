# DECISION: Input-prompt glyph home = assets/input-prompts (Alt only), license in credits/

- DATE: 20260728-000000
- STATUS: ACCEPTED
- TASK: 20260728-233707
- TAGS: decision, assets, ui

## Context

The UI-rework spike (20260728-175726) imported the FREE Input Prompts pack
(JulioCacko, CC0) under `examples/ui/assets/input-prompts/` in three styles
(Alt/Dark/White) for the HTML PoCs, with provenance in a local NOTICE.md.
Backlog 20260728-214929 left "canonical asset home" as an open question. The
real game cannot load from `examples/`, only Alt is used, and the project's
single source of truth for third-party attribution is `credits/CREDITS.md` +
`credits/licenses/`. Owner directive 2026-07-28 settled the fork.

## Decision

The canonical home is the game asset tree: `assets/input-prompts/keyboard/Alt/`
with pack filenames verbatim. Only the Alt style ships; Dark and White are
deleted from git. Attribution moves into `credits/CREDITS.md` (Third-party
assets entry) with the CC0 text at `credits/licenses/FREE-Input-Prompts_CC0-1.0.md`;
the ad-hoc NOTICE.md is absorbed and deleted. Every consumer references the one
copy: the game via `asset_server.load("input-prompts/...")`, the web easter egg
via a webpack copy of `../assets/input-prompts`, the PoC review copy via a
relative-path rewrite in its existing onRoute script.

## Alternatives considered

- **Stay under `examples/ui/assets/`** - the game cannot load it there, so real
  HUD adoption would force a second copy; PoC-local assets also dodge the
  credits pipeline that ships with every build. Rejected.
- **Web home (`web/src/assets/`)** - works for the site, but the native game
  bundles only `./assets/`; same duplication problem mirrored. Rejected.
- **Keep all three styles** - Dark/White have no consumer; 1.6M of dead PNGs in
  git and in every shipped build. Rejected; a future style can be re-imported
  from the pack if a surface needs it.

## Consequences

Easier: the HUD dock (20260728-175742) and all later adopters (20260728-214929)
load glyphs like any other game asset; licensing is uniform with Kenney/Iosevka/
space-3d; shipped builds carry ~800K instead of 2.4M. Harder: the PoC review
copy needs the small src-rewrite shim, and any future non-Alt style use means a
fresh import rather than an already-present folder.
