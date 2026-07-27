# Review: NOVA OS fixed-width FPS + full-keybind footer

- TASK: 20260727-135213
- BRANCH: feature/nova-os-chrome-fps-footer

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

DoD proofs run by the reviewer: `topbar_status_line` / `drive_topbar_fps` /
`nova_os_footer_hints` / `nova_os_matches_nova_os_terminal_poc_structure` PASS
(4 passed); `cargo check -p nova_gameplay -p nova_os --all-targets` clean.

- [x] R1.1 (MAJOR) crates/nova_os/src/app.rs:24 - the terminal footer advertised
  `ESC/CTRL+C: CLOSE`, but Ctrl+C is inert AT THE PROMPT (the surface where this
  hint renders): `ctrl_exit` calls `terminal.exit_app()`, which returns false /
  no-op at the prompt (only Escape closes there; nova_os.rs:1533-1540). Ctrl+C is
  an app-exit chord, so the terminal set advertised an unwired key
  (`advertised-but-unwired`).
  - Response: fixed in <this round>. Changed the terminal hint to `ESC: CLOSE`
    and left a comment explaining Ctrl+C belongs on app hint sets, not the
    prompt set. Verified independently: at `TerminalMode::Prompt`, `exit_app()`
    is a no-op returning false, so only Escape sets `close.closing`. Updated the
    PoC-structure test's expected string to `ESC: CLOSE`.
- [ ] R1.2 (MINOR) crates/nova_os/src/app.rs:23 - `HINT: TYPE HELP` -> `TYPE HELP`
  is now an imperative label with no key, slightly inconsistent with the
  `KEY: ACTION` siblings.
  - Response: kept as `TYPE HELP`. It is deliberately a tip, not a keybind (there
    is no key for it); the shape difference reads fine as the trailing hint and
    matches the PoC's own trailing "type help" cue. Acknowledged as intentional.
- [ ] R1.3 (NIT) nova_os.rs `nova_os_fps_segment` - FPS >= 1000 widens to 4 chars
  (min-width format, not truncation).
  - Response: accepted as-is (reviewer agreed not realistic for this game). A
    1000+ FPS NOVA OS is not a state worth padding for.

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (trivial one-string follow-up to a MAJOR the out-of-context
  round already pinned; re-verified the claim against terminal.rs `exit_app` and
  re-ran the footer + PoC-structure tests)

The R1.1 fix is a one-token honesty correction on the footer string, verified
against the exit handler (only Escape closes at the prompt). R1.2/R1.3 are
accepted with reasoning. Fixed-width FPS, the slice refactor, wrap layout and the
strengthened tests were all confirmed correct in round 1.

Pending user checks (manual DoD, cleared at flow Finish):
- Owner watches the FPS cross 100/99 with no topbar shift, and reads the footer
  for the full, accurate prompt keybind set.
