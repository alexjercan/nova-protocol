# NOVA OS terminal UX parity: boot stagger, unread events, Tab cycling, paging, block caret, contextual hints, arg parsing

- STATUS: CLOSED
- PRIORITY: 41
- TAGS: v0.9.0, feature, ui, hud, input

## Story

The in-game NOVA OS terminal has command parity with the PoC
(`examples/ui/nova_os_terminal_poc.html`) but still lacks the shell FEEL the
PoC nails. This task closes the interaction/flavor gap in
`crates/nova_gameplay/src/hud/drawer.rs` - purely shell-side work, no new
commands and no CRT/shader changes.

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Steps

- [x] Staggered boot banner: on the first open of a session the welcome rows
      print one-by-one (~130 ms apart, `Time<Real>` - virtual time is frozen),
      like the PoC's `printBanner`; `clear` reprints the banner instantly. A
      small pending-rows state on `NovaOsTerminal` plus a real-time drain
      system fits the existing `resource_changed`-driven
      `rebuild_terminal_ui` without new UI plumbing.
- [x] Welcome parity + unread events: match the PoC's POST/CORE/DISPLAY/LINK
      block shape in `nova_os_welcome_rows`, and add the unread-events hint
      line ("N unread events. <hook for the most severe recent event> - try
      `log`.") derived from `DrawerFlightLog` entries appended since the
      drawer last closed (track a seen-index on close, next to the existing
      `seen_story` bookkeeping).
- [x] Tab completion cycling: extend `NovaOsTerminal::complete` with cycle
      state (stem + index, reset on any input edit like the PoC `resetCycle`):
      the first Tab on an ambiguous stem prints the match list to the
      scrollback, repeat presses cycle through the matches; today the shell
      stops at the common prefix and goes no further.
- [x] PageUp/PageDown page the scrollback by ~0.8 viewport heights from
      `handle_terminal_keyboard`, adjusting the scrollback viewport's
      `ScrollPosition`; respect the
      `bevy-ui-scroll-input-clamps-stored-offset` lesson (mirror whatever
      clamping `scroll_drawer_panels` already does).
- [x] Block caret: a filled block roughly one character wide (PoC `.caret`,
      0.6em x 1.15em) instead of the current 2 px bar; keep the blink and the
      before/after split (monospace font, so a `font_size * 0.6` width
      constant is exact).
- [x] Contextual footer hints: a `hints()` method on `NovaOsAppRuntime`
      (default = the terminal set) and a footer rebuild on `active_mode`
      change, so the footer swaps hint sets per surface (the PoC `HINTS` map)
      and `map`/`ship viewer` get theirs for free when they land.
- [x] Parser support for commands WITH arguments and multi-word launch words:
      rework `parse_command`/`command_has_arguments`/completion so command
      names are word sequences with per-command arity, instead of
      first-word-only with a blanket "takes no arguments". Remove the
      hardcoded `ship viewer` special-case in `parse_command` as part of this.
      Ship the capability now (current commands stay argument-free); the app
      tasks consume it.
- [x] App-exit chords from the PoC: Ctrl+C (and Ctrl+[) exit the running app
      back to the terminal; Shift+Esc closes the whole computer from inside an
      app (plain Esc keeps backing out one level). Per the
      `context-key-handled-in-one-owner` lesson these belong in the ONE
      state-branched owner (`close_drawer_from_menu_keys` /
      `handle_nova_os_app_keyboard`) - never a second reader over the same
      input edge.
- [x] Objective flips announce while the computer is open (PoC
      `checkObjectives` pushes an "OBJ x ..." log line): verify the
      `DrawerFlightLog` ObjectiveCompleted path already renders this in `log`
      output and the open scrollback picks it up; close any gap found.
- [x] Tests for each behavior (names in the DoD); input-driving tests follow
      the `nextstate-input-test-needs-clear-and-two-updates` lesson - copy the
      existing press-helper, do not hand-roll the cadence. Capture an AFTER
      shot for the caret/banner look and record the work + self-reflection in
      `tasks/20260726-214708/NOTES.md`.

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
