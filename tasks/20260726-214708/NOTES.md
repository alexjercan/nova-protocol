# NOTES - NOVA OS terminal UX parity

Task: 20260726-214708. All work is shell-side in
`crates/nova_gameplay/src/hud/drawer.rs` (no new commands, no CRT/shader changes).

## What changed and why

Closed the interaction/flavor gap between the in-game NOVA OS terminal and the
PoC (`examples/ui/nova_os_terminal_poc.html`). Eight behaviors:

1. **Staggered boot banner + unread events.** New `pending_rows`/`booted`/
   `seen_events` state on `NovaOsTerminal`. `begin_nova_os_boot` (OnEnter Drawer)
   clears the scrollback on the FIRST open and queues the welcome + unread-events
   rows; `drain_nova_os_boot` (real-time Update) reveals them ~130ms apart so the
   reveal runs while virtual time is frozen. `mark_nova_os_events_seen` (OnExit
   Drawer) records the seen count so "N unread events" only counts entries added
   since the last close. `clear` reprints instantly via the snapshot's unread
   count. `nova_os_welcome_rows` grew to the PoC's POST/CORE/DISPLAY/LINK block.
2. **Tab cycling.** `NovaOsTerminal::complete` now holds a cycle stem + index:
   the first Tab on an ambiguous stem lists the matches then jumps to the first,
   repeats cycle through them and wrap. Any prompt edit resets the cycle
   (`cycle_stem = None`), mirroring the PoC `resetCycle`.
3. **PageUp/PageDown paging.** A new arm in `handle_terminal_keyboard` pages the
   scrollback viewport's `ScrollPosition` by ~0.8 of a viewport, clamped with the
   same `max_drawer_scroll_y` helper `scroll_drawer_panels` uses (respecting the
   `bevy-ui-scroll-input-clamps-stored-offset` lesson).
4. **Block caret.** Width changed from a 2px bar to `DRAWER_LINE_FONT_PX * 0.6`
   (PoC `.caret` 0.6em), keeping the blink and the before/after split.
5. **Contextual footer hints.** `NovaOsAppRuntime::hints` (default = the terminal
   set) plus `rebuild_nova_os_footer_hints`, which rebuilds the footer row on
   `active_mode` change (keyed by a `Local` so ordinary prompt edits do not
   thrash it). `map`/`ship viewer` get their own footers for free when they land.
6. **Parser: arguments + multi-word launch words.** Replaced the first-word-only
   `parse_command`/`command_has_arguments` + the hardcoded `ship viewer`
   special-case with `resolve_command`: it matches the LONGEST command name that
   is a word-prefix of the input (so `ship view` beats the `ship` built-in) and
   validates the trailing words against a per-command `CommandArity`. Current
   commands stay argument-free (`CommandArity::None`); `CommandArity::UpTo(n)` is
   the capability the app tasks consume. `TerminalCommandResult` became
   `ResolvedCommand` (App/Builtin/UnexpectedArguments/Unknown).
7. **App-exit chords.** Ctrl+C / Ctrl+[ exit a running app to the terminal;
   Shift+Esc closes the computer from inside an app; plain Esc still backs out one
   level. All handled in the ONE Escape owner `close_drawer_from_menu_keys` per
   the `context-key-handled-in-one-owner` lesson; `handle_nova_os_app_keyboard`
   skips any key pressed while Control is held so the chord cannot double-fire.
8. **Objective flips announce live.** `announce_objectives_in_terminal` pushes an
   `OBJ x ...` row into the open scrollback the moment an objective completes
   while the computer is open (PoC `checkObjectives`). Completions that flip while
   closed stay in the flight log and are counted by the boot banner's unread line
   instead of dumping on open.

## Decisions / interpretations

- **Unread hook.** The flight-log entry model carries no severity, so "the most
  severe recent event" is interpreted as the most recent unread entry's message.
  Documented at `nova_os_unread_hook`. No load-bearing fork - no DECISION.md.
- **`hints` default = terminal set.** Followed the plan literally: an app that
  does not override `hints` falls back to the terminal footer (harmless), and
  apps that care override it. The DoD test registers an overriding app.
- **Live announce only while open.** Chose not to retro-dump completions that
  happened while closed (they are the "unread events" the banner counts), tracked
  with a `Local<Option<usize>>` initialized to "everything already seen".

## Difficulties

- **Multi-word vs argument ambiguity in `refresh_parse`.** Typing `ship vi`
  toward `ship view` first resolves as `ship` + bad arg (UnexpectedArguments). The
  fix: in `refresh_parse`, if the whole input is still a strict string-prefix of a
  LONGER command name, treat it as `ValidPrefix` (completion target) rather than
  the arity error. Verified by the fish-ghost and rejects-arguments tests.
- **`prompt_completion_ghost` multi-word.** Rewrote it to strip the typed prompt
  off the full `completion_hint` name rather than the first word, so a multi-word
  completion ghosts correctly.

## Self-reflection

- The parser rework touched `submit`/`complete`/`refresh_parse`/ghost at once;
  running `cargo check --tests` early (before writing new tests) caught the
  `terminal_snapshot_from_world` arity change and the snapshot-literal fanout
  cheaply. Worth doing that check-first sweep on any signature change.
- Reused the manual-`Time::<Real>` rig from `slide_drives_single_monitor_openness`
  for the staggered-boot test (per `nextstate-input-test-needs-clear-and-two-updates`
  family), avoiding a hand-rolled clock.

## Verification

- New tests (all green under `cargo test -p nova_gameplay --lib`, alongside the
  42 existing terminal/nova_os tests): `nova_os_boot_banner_staggers_and_counts_unread`,
  `nova_os_tab_cycles_ambiguous_completions`, `nova_os_page_keys_scroll_scrollback`,
  `nova_os_parser_supports_arguments_and_multiword`,
  `nova_os_footer_hints_follow_active_surface`, `nova_os_app_exit_chords`,
  `nova_os_objective_flip_announces_in_open_terminal`.
- `cargo fmt` + `cargo check` clean. Full suite runs in CI per the
  skip-local-tests convention.

## Manual acceptance (pending)

- **Caret/banner AFTER shot** (DoD manual item). Capture with a real GPU:
  `NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example
  screenshot_nova_os --features debug` and compare `nova-os-welcome.png` /
  `nova-os-active.png` against `examples/ui/nova_os_terminal_poc.html`. Not
  capturable in the headless dev sandbox (windowed GPU capture).
