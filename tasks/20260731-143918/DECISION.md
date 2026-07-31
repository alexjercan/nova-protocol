# Decision: web app port depth - full material port plus mono typography

- STATUS: ACCEPTED
- DATE: 2026-07-31
- TASK: 20260731-143918
- TAGS: ui, web, theme

## Context

The game's UI rework (epic 20260728-175719) moved the whole in-game UI onto the
NOVA OS language and `crates/nova_ui/src/theme.rs` now mirrors the `:root` block
of `examples/ui/nova_ui_rework_poc.html`. `web/src/style.css` still carries the
navy/cyan industrial-HUD palette the game used to mirror, so the site sells a
look the game no longer has. The game theme is more than a palette: it also has
a light-3D material vocabulary (case-face gradients, rim/undercut/drop/well
bevels, CRT screen surfaces, 10 px panel radius) and a terminal typeface. How
much of that the site adopts changes the size and the risk of the work, so it
was put to the owner before planning.

## Decision

**Full material port plus mono typography.**

- `:root` swaps to the NOVA OS tokens: `--space`, `--case-0..3`, `--case-edge`,
  `--screen-0/1`, `--phosphor` / `-dim` / `-muted`, `--amber`, `--orange`,
  `--red`, `--blue`, `--text #b9ffc9`, plus a dark `--ink` for glyphs on bright
  fills.
- The light-3D vocabulary comes over too: `--face` / `--face-hot` gradients,
  `--rim`, `--undercut`, `--drop`, `--well`, `--panel-radius: 10px`.
- Component families adopt the MATERIAL, not just the colour: buttons become
  moulded faces, cards and panels become case faces, code blocks become CRT
  screens.
- Typography goes terminal-first: `--font-display`, `--font-body` and
  `--font-mono` all resolve to the JetBrains Mono stack; Rajdhani and Inter are
  dropped from the Google Fonts `@import`.

Source of truth is the PoC `:root` block, the same block `nova_ui::theme`
mirrors - so site and game share one origin rather than two hand-synced lists.

## Alternatives considered

- **Palette retint only.** Swap the token VALUES to phosphor and leave the flat
  sharp-industrial structure (1 px borders, shallow bevel, hard shadow) intact.
  Smallest, lowest-risk diff, and mono-free. Rejected: the site would be green
  but flat, so it still would not read as the same hardware as the game - the
  material is what carries the new look.
- **Full material port, proportional type kept.** Adopt the case/CRT vocabulary
  but keep Rajdhani display + Inter body so long wiki and news prose stays easy
  to read. Rejected by the owner: the terminal face is part of the identity.

## Consequences

- Nearly all of `web/src/style.css` (1635 lines, 19 tokens, 238 `var()` reads)
  is touched, plus the five mermaid colour fallbacks in `web/src/wiki.ts`.
- Long prose reads denser in mono. Accepted by the owner. The mitigation is
  metrics only - line-height, measure, heading sizes - never a fallback to a
  proportional body face.
- The port is a readability change, so it is unverified until the pages are
  rendered and seen (`render-output-eyeball`). No web page-capture rig exists,
  so building one is part of the task.
- Out of scope: re-capturing the site's game screenshots, which still show the
  retired navy UI (backlog 20260724-082856); shipping the game's Iosevka face
  to the browser (backlog 20260714-214329); `crates/nova_ui` itself, already on
  the NOVA OS tokens.
