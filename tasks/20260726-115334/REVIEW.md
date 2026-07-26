# Review: NOVA OS app runtime

- TASK: 20260726-115334
- BRANCH: feat/nova-os-app-runtime

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

The out-of-context reviewer independently reran every DoD proof and scrutinized
the load-bearing claims. Results it observed:
`cargo test -p nova_gameplay -- drawer terminal` -> 54 passed, 0 failed (all 5 new
app tests present and green); `cargo fmt --check` exit 0 (run bare); `cargo check
-p nova_gameplay` exit 0 (only the unrelated pre-existing `proc-macro-error2`
future-incompat note).

It verified the four highest-risk claims and found them solid:

- Escape single-owner: `close_drawer_from_menu_keys` is the sole actor on
  Escape/Start in `Drawer`; `nova_menu::toggle_pause` maps `Drawer -> Drawer` and
  early-returns without consuming the edge, and the app keyboard system skips
  Escape. No frame in which one press both exits the app and closes the drawer.
- Input ownership: the two `MessageReader` cursors are independent; no
  double-processing or missed events; typing cannot reach the prompt in app mode.
- `sync_nova_os_app_ui` diff-guard: correct for launch/exit/switch/reopen; never
  duplicates or leaks the app root; no panic path (`single()` is `Ok(..) else
  return`).
- `sync_nova_os_app_commands` change-detection: reads through `Deref`, only
  assigns on a real diff, so it does not thrash `rebuild_terminal_ui`.

No BLOCKER or MAJOR. Findings, all MINOR/NIT, and their resolution:

- MINOR (launch keystroke bleed): the Enter that submits an app word was read by
  `handle_nova_os_app_keyboard` the same frame in app mode, so an Enter-sensitive
  app would self-trigger on launch. FIXED: the app keyboard system now only
  processes keys on frames where the same app was already live last frame
  (`Local<Option<&'static str>>`); every transition (launch, switch, reopen Tab)
  drops the buffer. Regression-pinned by
  `nova_os_launch_keystroke_does_not_bleed_into_the_app` (an Enter-exit test app).
  This also resolves the reviewer's NIT about buffered events on gamepad-close +
  reopen.
- MINOR (inline completion ghost ignored app words): FIXED - `prompt_completion_ghost`
  now chains `app_commands` in the same builtin-then-app order as the hint.
- MINOR (`nearest_command` did-you-mean excluded app words): FIXED - `nearest_command`
  (and `parse_command`) now take `app_commands`; pinned by
  `nova_os_typo_of_an_app_word_is_suggested`.
- NIT (coverage): added `nova_os_app_launch_word_rejects_arguments` and the two
  tests above. App-switch through the diff-guard remains proven by inspection only.

Post-fix verification: `cargo test -p nova_gameplay -- drawer terminal` -> 57
passed, 0 failed; `cargo fmt --check` exit 0.

Pending user check (manual): none - this task has no `manual:` DoD item; the
lifecycle is proven headlessly.
