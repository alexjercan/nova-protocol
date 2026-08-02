# NOVA OS monitor chrome: fixed-width FPS + full current-keybind footer

- PRIORITY: 41
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

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

## Story

The top-right FPS number should not reflow the topbar as it changes, and the
footer should be an honest, complete cheat-sheet of the keys that actually
work in the current surface.

## Steps

- [x] Fixed-width FPS: added `nova_os_fps_segment(fps)` = right-align to 3 chars
      in the monospace topbar font (` 99`/`100`/` --` all 3-wide), shared by both
      `nova_os_status_text` (spawn) and `topbar_line_with_fps` (live). 100 -> 99
      no longer reflows.
- [x] Audit the actual current keybinds and list them. `NOVA_OS_TERMINAL_HINTS`
      is now the full prompt set: `TAB: COMPLETE`, `ENTER: RUN`, `UP/DN: HISTORY`,
      `PGUP/PGDN: SCROLL`, `ESC/CTRL+C: CLOSE`, `TYPE HELP` (ASCII, no arrow
      glyphs per repo style). Changed the hints type from `[&str; 3]` to a slice
      so terminal + apps can list different counts (context-sensitive footer via
      `nova_os_footer_hints` kept). The initial footer spawn now iterates the same
      constant (was a duplicated hardcoded 3).
- [x] Footer layout fits the full set: added `FlexWrap::Wrap` + a small row gap
      so the row never overflows on a narrow screen.

## Definition of Done

- FPS width is stable across digit-count changes; the footer lists every
      currently-active keybind for the surface. (manual: owner watches FPS
      cross 100/99 with no shift and reads the footer for the full key set)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay -- topbar_status_line drive_topbar_fps nova_os_footer_hints nova_os_matches_nova_os_terminal_poc_structure)
      [The template's `drawer` filter matches 0 tests; these live under
      `hud::nova_os::tests::*`.]

## Close-out

What changed and why:
- Fixed-width FPS: a shared `nova_os_fps_segment(fps)` right-aligns the value to
  3 chars in the monospace topbar font, used by BOTH the spawn-time
  `nova_os_status_text` and the live `topbar_line_with_fps` (they had duplicated
  the formatting). ` 99`, `100`, ` --` are all 3 wide, so the topbar no longer
  reflows as the reading crosses a digit boundary.
- Full keybind footer: `NOVA_OS_TERMINAL_HINTS` became a slice (was `[&str; 3]`)
  listing the real prompt bindings - TAB/ENTER/UP-DN/PGUP-PGDN/ESC-CTRL+C/help -
  kept ASCII (no arrow glyphs, per repo writing style). The trait
  `NovaOsAppRuntime::hints` and `nova_os_footer_hints` return slices too, so the
  context-sensitive footer (terminal set vs a running app's set) still works with
  a variable count. The initial footer spawn now iterates the same constant
  instead of a duplicated hardcoded 3, and the row got `FlexWrap::Wrap` so the
  longer set never overflows a narrow screen.

Difficulties:
- The `[&str; 3]` -> slice change rippled to the trait, the free function, the
  test app's `hints()` override, and the initial spawn's `for hint in hints`
  (now `for &hint`). All caught by the compiler.
- Several existing tests pinned the old FPS format (`FPS: 60`) and the old 3
  footer strings; updated each to the fixed-width / new-hints values and ADDED a
  fixed-width assertion (`nova_os_fps_segment(Some(100)).len() == 3`, etc.) that
  directly proves the 100 -> 99 no-reflow property.

Self-reflection: converting the fixed array to a slice up front (rather than
bumping it to `[&str; 6]`) keeps the terminal and per-app footers free to differ
in length - the right call for a "list ALL current keys" feature that will grow.
