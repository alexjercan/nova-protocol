# Review: NOVA OS rename + nova_os crate extraction

- TASK: 20260727-015156
- BRANCH: refactor/nova-os-crate

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Verification notes (round 1 reviewer, re-confirmed in-session):

- DoD1 `grep -rInE '\b[Dd]rawer\b' crates/ src/ --include=*.rs` -> 0 matches
  outside `crates/nova_editor/`. PASS.
- DoD2 `cargo check -p nova_os` (and via `cargo check --workspace`) -> exit 0.
  PASS.
- DoD3 `cargo tree -p nova_ui` shows no `nova_os`; `cargo tree -p nova_gameplay`
  shows both `nova_os` and `nova_ui`; graph acyclic. PASS.
- DoD4 `cargo test -p nova_os` = 11 ok; `cargo test -p nova_gameplay nova_os`
  = 64 ok; `cargo check --workspace` exit 0; `cargo doc --workspace --no-deps`
  clean (only the `proc-macro-error2` future-incompat dep note, not a rustdoc
  warning); `RUSTDOCFLAGS="-D warnings" cargo doc -p nova_os` clean, so
  `#![warn(missing_docs)]` is genuinely clean. PASS.
- Change-detection preserved: `sync_nova_os_app_commands` compares via the
  immutable `app_commands()` accessor and only takes the `&mut`
  `set_app_commands` path on a real change; `drain_nova_os_boot` reads
  `has_pending_boot_rows()` immutably and early-returns before any `ResMut`
  deref. Behaviorally identical to master.
- No tests dropped: master's `drawer.rs` had 69 `#[test]`; branch has 58 in
  `hud/nova_os.rs` + 11 in the `nova_os` crate = 69. The moved logic tests
  assert real behavior and would fail if the model broke.
- `submit`/`resolve_command`/content-builder bodies are a faithful extraction
  (only `pub(crate)`->`pub` visibility changes, no logic drift).
- Every new public `NovaOsTerminal` accessor has >=1 gameplay call site; fields
  stay private. Doc/web sweep clean (remaining `drawer` hits are the unrelated
  "junk drawer" idiom / the web wiki's own scroll drawer). AGENTS.md crate
  table updated.

Pending user check (not resolvable in review):

- DoD5 (manual): owner eyeballs a live NOVA OS screenshot (Tab-open render +
  a command + an app launch). Structure is covered by the green integration
  tests + a clean `probe playable` run, but the visual eyeball is the human
  acceptance gate.

Spec-text note (not a finding): TASK.md's Target-boundary list mentions a
`replace_current_command` method that never existed on master or the branch -
an imprecise spec line, not a regression. Left as-is (task records are
append-only history).

- [x] R1.1 (NIT) crates/nova_os/src/terminal.rs:706-713 - `prompt_before_cursor`
  / `prompt_after_cursor` slice `prompt[..cursor]` directly; sound because the
  edit methods keep `cursor` on a char boundary, but a panic-on-bad-boundary
  reliance now that `cursor` is reachable only through the crate's getters. Add
  a `debug_assert!(prompt.is_char_boundary(cursor))` to lock the invariant.
  - Response: Adopted. Added `debug_assert!(terminal.prompt.is_char_boundary(
    terminal.cursor))` to both helpers.
