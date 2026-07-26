# NOVA OS terminal UX parity: boot stagger, unread events, Tab cycling, paging, block caret, contextual hints, arg parsing

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.9.0,feature,ui,hud,input

## Story

The in-game NOVA OS terminal has command parity with the PoC
(`examples/ui/nova_os_terminal_poc.html`) but still lacks the shell FEEL the
PoC nails. This task closes the interaction/flavor gap in
`crates/nova_gameplay/src/hud/drawer.rs` - purely shell-side work, no new
commands and no CRT/shader changes.

## Steps

- [ ] Staggered boot banner: on the first open of a session the welcome rows
      print one-by-one (~130 ms apart, `Time<Real>` - virtual time is frozen),
      like the PoC's `printBanner`; `clear` reprints the banner instantly.
- [ ] Welcome parity + unread events: match the PoC's POST/CORE/DISPLAY/LINK
      block shape, and add the unread-events hint line ("N unread events.
      <hook for the most severe recent event> - try `log`.") derived from
      `DrawerFlightLog` entries accumulated since the drawer last closed.
- [ ] Tab completion cycling: when several commands match, the first Tab
      prints the match list to the scrollback and repeat presses cycle through
      the matches (PoC `completeInput`); today the shell stops at the common
      prefix and goes no further.
- [ ] PageUp/PageDown page the scrollback by ~0.8 viewport heights; today only
      the mouse wheel scrolls.
- [ ] Block caret: a filled block roughly one character wide (PoC `.caret`,
      0.6em x 1.15em) instead of the current 2 px bar; keep the blink and the
      before/after split.
- [ ] Contextual footer hints: the footer swaps hint sets per surface
      (terminal vs each running app - the PoC `HINTS` map); apps supply their
      hint set through `NovaOsAppRuntime` so `map`/`ship viewer` get theirs
      for free when they land.
- [ ] Parser support for commands WITH arguments and multi-word launch words:
      today every command hard-rejects arguments ("takes no arguments"), which
      blocks the PoC's `repair <part>` and the `ship view` two-word launch.
      Ship the parser/completion capability now (current commands stay
      argument-free); the app tasks consume it.
- [ ] App-exit chords from the PoC: Ctrl+C (and Ctrl+[) exit the running app
      back to the terminal; Shift+Esc closes the whole computer from inside an
      app (plain Esc keeps backing out one level).
- [ ] Objective flips announce while the computer is open (PoC
      `checkObjectives` pushes an "OBJ x ..." log line): verify the
      `DrawerFlightLog` ObjectiveCompleted path already renders this in `log`
      output and the open scrollback picks it up; close any gap found.

## Definition of Done

- First open types the banner line-by-line and reports unread events; `clear`
  is instant. (test: `nova_os_boot_banner_staggers_and_counts_unread`)
- Tab lists and cycles ambiguous matches. (test:
  `nova_os_tab_cycles_ambiguous_completions`)
- PageUp/PageDown page the scrollback. (test:
  `nova_os_page_keys_scroll_scrollback`)
- The parser accepts an argument-taking command registration and a multi-word
  launch word without breaking the argument-free built-ins. (test:
  `nova_os_parser_supports_arguments_and_multiword`)
- Footer hints change when an app is active. (test:
  `nova_os_footer_hints_follow_active_surface`)
- Ctrl+C exits an app; Shift+Esc closes the computer from inside an app.
  (test: `nova_os_app_exit_chords`)
- Caret reads as a block cursor. (manual: screenshot vs the PoC via the
  `screenshot_nova_os` example)

## Notes

- PoC references: `printBanner`, `completeInput`/`resetCycle`, the `HINTS`
  map, the keydown handler (paging, chords), `.caret` CSS.
- The epic (`tasks/20260725-104330/TASK.md`) explicitly called for the
  Ctrl+C-style app-exit chord; the runtime shipped Escape-only.
- Sound hooks for these interactions belong to 20260726-214639, not here.
