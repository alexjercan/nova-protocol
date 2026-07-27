# Review: NOVA OS terminal UX parity

- TASK: 20260726-214708
- BRANCH: feature/nova-os-terminal-ux

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Verification performed (out-of-context, diff-only):

- Read TASK.md (Story/Steps/DoD/Notes) and NOTES.md in full.
- Reviewed the whole `git diff master...HEAD` (single commit `8a3b93cb`, all
  substantive change in `crates/nova_gameplay/src/hud/drawer.rs`, +1198/-252).
- Ran `cargo test -p nova_gameplay --lib -- nova_os terminal`: 49 passed, 0
  failed. All 7 named DoD tests are in this set and pass:
  - `nova_os_boot_banner_staggers_and_counts_unread` - PASS. Asserts queue is
    empty-then-partial-then-full over real-time ticks and that `clear`
    reprints instantly; would fail if drain removed (partway>=1 fails) or if
    banner printed instantly (empty-after-first-update fails). Real behavior.
  - `nova_os_tab_cycles_ambiguous_completions` - PASS. Lists once, cycles,
    wraps, resets on edit. Real behavior.
  - `nova_os_page_keys_scroll_scrollback` - PASS. Asserts up<100, down>up,
    clamp<=300. Real behavior.
  - `nova_os_parser_supports_arguments_and_multiword` - PASS. multi-word wins
    longest-match, UpTo(1) accept/reject, built-ins unaffected. Real behavior.
  - `nova_os_footer_hints_follow_active_surface` - PASS. Swaps to app hints and
    back; would fail with rebuild removed (no children). Real behavior.
  - `nova_os_app_exit_chords` - PASS. Ctrl+C -> prompt, not closing;
    Shift+Esc from app -> closing. Real behavior.
  - `nova_os_objective_flip_announces_in_open_terminal` - PASS (bonus test for
    the objective step; not a named DoD item). Asserts OBJ line lands only on
    completion, not on post. Real behavior.
- Verified removed helpers (`parse_command`, `command_has_arguments`,
  `common_prefix`, `current_command_prefix`, `replace_current_command`,
  hardcoded `ship viewer` special-case) have no dangling references.
- Confirmed single-owner input: the Ctrl+C / Ctrl+[ / Shift+Esc chords live
  only in `close_drawer_from_menu_keys`; `handle_nova_os_app_keyboard` skips
  every key while Control is held, and PageUp/PageDown are gated behind
  `drawer_prompt_active` in `handle_terminal_keyboard`. No second reader over
  the same edge (respects `context-key-handled-in-one-owner`).
- Confirmed PageUp/PageDown clamp uses the same `max_drawer_scroll_y` helper as
  `scroll_drawer_panels` (respects `bevy-ui-scroll-input-clamps-stored-offset`).
- No existing tests were weakened or deleted; the parser test was rewritten to
  cover the new `resolve_command` API and asserts the multi-word/arity behavior
  rather than just executing it.

DoD manual item (caret block cursor screenshot vs the PoC via the
`screenshot_nova_os` example) is a PENDING user check - the caret width change
(`DRAWER_LINE_FONT_PX * 0.6`) is in the diff and the example exists at
`examples/screenshots/screenshot_nova_os.rs`, but the visual match is not
resolvable from the diff. Left open for the user.

In-session pass (records the ticks on the out-of-context reviewer's findings):
independently re-derived R1.1 before the review - the unconditional
`scrollback.extend` marks the terminal changed on every objective-change frame,
and `rebuild_terminal_ui` snaps the scroll to the bottom, which would fight the
new PageUp/PageDown offset. Confirmed load-bearing, fixed. Re-ran
`cargo test -p nova_gameplay --lib -- nova_os terminal` after the fixes: 49
passed, 0 failed. `cargo fmt`/`cargo check` clean.

- [x] R1.1 (NIT) drawer.rs:3444 - `announce_objectives_in_terminal` calls
  `terminal.scrollback.extend(fresh)` inside `if open { ... }` unconditionally,
  so even when `fresh` is empty it mutably derefs `ResMut<NovaOsTerminal>` and
  marks the resource changed, forcing a `rebuild_terminal_ui` pass. It only runs
  under `run_if(resource_changed::<GameObjectives>)` so the thrash is bounded to
  objective-change frames, but consider guarding with `if !fresh.is_empty()`.
  - Response: Fixed - guarded the `extend` with `if !fresh.is_empty()`. This also
    stops the spurious rebuild from snapping the scroll to the bottom while the
    player has paged up. Verified independently by the in-session pass.

- [ ] R1.2 (NIT) drawer.rs:3435 - objective flips are announced only while
  `active_mode == Prompt`. A completion that flips while an app owns the screen
  is not announced and is not re-shown on backing out to the prompt (it stays in
  the flight log). The PoC `checkObjectives` announces regardless of surface.
  NOTES documents this as intentional; flagging only as a minor spec deviation.
  - Response: Left as-is (deliberate, documented in NOTES). The scrollback is not
    visible while an app owns the screen, so announcing into it then is a no-op
    the player never sees; those completions are surfaced via the boot banner's
    unread-events count on the next open and remain in `log`. NIT, no change.

- [x] R1.3 (NIT) drawer.rs:2531 - `handle_nova_os_app_keyboard` skips ALL keys
  while Control is held, not just C / `[`. This is intentional (chord ownership)
  and no current app binds Control, but it means a future app cannot use any
  Ctrl+<key> shortcut without revisiting this owner. Worth a one-line note at the
  guard so the constraint is discoverable when the app tasks land.
  - Response: Expanded the comment at the `ctrl_held` guard to spell out that it
    blocks all Ctrl+<key> presses and that a future app Ctrl shortcut must
    revisit this owner. No behavior change.
