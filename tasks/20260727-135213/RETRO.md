# Retro: NOVA OS fixed-width FPS + full-keybind footer

- TASK: 20260727-135213
- BRANCH: feature/nova-os-chrome-fps-footer
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES -> R2 APPROVE)

## What went well

- The fixed-width FPS fix was small and directly test-pinned: a shared
  `nova_os_fps_segment` (used by both formatters, de-duplicating the two code
  paths that had drifted) plus assertions that the segment length is constant
  across digit counts - the no-reflow property proven, not eyeballed.
- Converting the hint `[&str; 3]` to a slice up front (rather than bumping to
  `[&str; 6]`) was the right shape for a "list ALL current keys" feature that
  will grow, and let terminal vs per-app footers keep different lengths.
- Respected the repo writing-style rule (no arrow glyphs) from the start:
  `UP/DN`, `PGUP/PGDN`, not the unicode arrows.

## What went wrong

- The footer advertised `CTRL+C: CLOSE`, but Ctrl+C is inert AT THE PROMPT -
  it is an app-exit chord, and `exit_app()` is a no-op at the prompt (only
  Escape closes the computer there). I listed the keys I knew the NOVA OS
  handles without checking each against the SURFACE the terminal footer renders
  on. This is `advertised-but-unwired` (now x5) - a keybind hint is per-surface,
  and I put an app-surface chord on the terminal set. Caught by out-of-context
  review, which traced the exit handler to the no-op.

## What to improve next time

- When listing keybinds in a per-surface hint, verify EACH key does something on
  THAT surface (trace it to its handler branch), not just that the app handles
  it somewhere. A key that acts on a different surface is `advertised-but-unwired`
  on this one.

## Action items

- [x] Bumped ledger `advertised-but-unwired` to x5 with the per-surface keybind
  nuance.
- [x] Fixed the terminal hint to `ESC: CLOSE`; documented in the constant that
  Ctrl+C belongs on app hint sets.
