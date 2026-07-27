# NOVA OS monitor chrome: fixed-width FPS + full current-keybind footer

- STATUS: OPEN
- PRIORITY: 41
- TAGS: v0.9.0,feature,ui,hud

Playtest feedback on the NOVA OS monitor chrome (the thin text rows at the top
and bottom edges of the screen).

1. The FPS readout in the top-right of the monitor changes width when the
   value changes digit count (100 -> 99), shifting the layout. It must be
   fixed-width.
2. The footer should list ALL currently available keybinds, not just three
   hints. We have added PageUp/PageDown (paging), Ctrl+C, Tab, Esc, arrow-key
   history, etc.; the footer should surface the real, current binding set.

Code: `crates/nova_gameplay/src/hud/nova_os.rs` - FPS: status text
`nova_os_status_text()` ~944-953, topbar FPS driver
`drive_nova_os_topbar_fps()` ~2295-2330, marker prefix
`NOVA_OS_TOPBAR_FPS_MARKER` ~2261, diagnostic ~2268-2272. Footer hints spawn
~4001-4033, rebuild `rebuild_nova_os_footer_hints()` ~2154-2188; hint source
`crates/nova_os/src/app.rs` `NOVA_OS_TERMINAL_HINTS` ~11-15 and
`nova_os_footer_hints()` ~137-145. Keybind handling: PageUp/Down ~1640-1650,
Ctrl+C ~1728-1732, Tab ~1295-1310.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Story

The top-right FPS number should not reflow the topbar as it changes, and the
footer should be an honest, complete cheat-sheet of the keys that actually
work in the current surface.

## Steps

- [ ] Fixed-width FPS: pad/format the FPS value to a stable width (e.g. right-
      align to 3 digits, or fixed-width cell) so 100 -> 99 does not change the
      rendered width. Prefer a monospace fixed-cell so no glyph-width jitter.
- [ ] Audit the actual current keybinds per surface (terminal prompt vs app):
      Tab, Esc, Ctrl+C, PageUp/PageDown, arrow history, and any others we
      added. Update `NOVA_OS_TERMINAL_HINTS` / `nova_os_footer_hints()` so the
      footer lists the full set (context-sensitive per active surface).
- [ ] Make sure the footer layout fits the full set without overflow (wrap or
      compact the labels as needed) and stays legible.

## Definition of Done

- FPS width is stable across digit-count changes; the footer lists every
      currently-active keybind for the surface. (manual: owner watches FPS
      cross 100/99 with no shift and reads the footer for the full key set)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay drawer)
