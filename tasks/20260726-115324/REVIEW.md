# Review: NOVA OS terminal input and command shell

- TASK: 20260726-115324
- BRANCH: feature/nova-os-terminal-shell

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:684 - `handle_terminal_keyboard` only runs under `in_state(PauseStates::Drawer)`, so its `MessageReader<KeyboardInput>` does not advance while gameplay is unpaused. Retained gameplay text events can be read the first time the drawer opens and inserted into the terminal prompt. Run the system every Playing frame and drain events unconditionally, processing them only when `PauseStates::Drawer`, or add an always-on drain/entry reset. Add a production-path test that sends text while Unpaused, opens the drawer, and asserts the prompt stays empty.
  - Response: fixed in follow-up patch; `handle_terminal_keyboard` now runs during `GameStates::Playing`, drains every `KeyboardInput`, and processes only while `PauseStates::Drawer`. Added `terminal_ignores_text_typed_before_drawer_opens`.
- [x] R1.2 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:559 - `parse_command` only looks at `current_command_prefix`, so extra tokens are silently accepted: `help garbage` runs help and `clear garbage` clears scrollback. The task called for command-word and verb+argument parsing with helpful errors, so parse and validate the full line. Reject unexpected args for `help` and `clear`, and add tests that fail if the tail is ignored.
  - Response: fixed in follow-up patch; `help` and `clear` now reject unexpected arguments with error rows and parse hints. Added `terminal_rejects_unexpected_command_arguments`.

Verification commands run by reviewer:
`nix develop --command bash -lc 'cargo test -p nova_gameplay drawer && cargo test -p nova_gameplay terminal'`
`nix develop --command cargo test -p nova_menu escape_closes_the_drawer_to_unpaused`

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

R1.1 is resolved: `handle_terminal_keyboard` now runs during
`GameStates::Playing`, drains every `KeyboardInput`, and only processes input
while `PauseStates::Drawer`. The new
`terminal_ignores_text_typed_before_drawer_opens` test covers the stale-message
case.

R1.2 is resolved: `help` and `clear` now reject unexpected arguments, report
errors, and the new `terminal_rejects_unexpected_command_arguments` test would
fail if argument tails were ignored.

No new findings.

Commands run by reviewer:
`nix develop --command bash -lc 'cargo test -p nova_gameplay drawer && cargo test -p nova_gameplay terminal'`
`nix develop --command cargo test -p nova_menu escape_closes_the_drawer_to_unpaused`
