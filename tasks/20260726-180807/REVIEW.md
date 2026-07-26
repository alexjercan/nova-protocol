# Review: NOVA OS computer HTML-fidelity pass

- TASK: 20260726-180807
- BRANCH: feature/nova-os-computer-fidelity

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

The out-of-context reviewer re-ran every DoD proof and verified the flagged
risk areas independently. In-session re-verification: I had already run the full
`cargo test -p nova_gameplay drawer` suite (46 passed), `cargo fmt --check`,
`cargo check` and the web CI, and captured + inspected the AFTER render; the
out-of-context reviewer reached the same pass results, and I re-confirmed the
char-boundary safety of the `prompt_before/after_cursor` split (cursor is a byte
index kept on char boundaries by `insert_text`/`backspace`/`move_cursor_*`).

Command results (both sessions): `cargo fmt --check` PASS; `cargo check` PASS;
`cargo test -p nova_gameplay drawer` -> `46 passed; 0 failed`. All four DoD tests
present and revert-sensitive. No tests deleted/weakened; changed assertions
reflect real behaviour changes.

Pending user check (manual DoD): confirm the AFTER shots
(`shots/nova-os-welcome.png`, `shots/nova-os-active.png`) match the HTML PoC
(`shots/reference-html.png`) on readability, input box, inline completion and CRT.

- [x] R1.1 (NIT) web/src/wiki/hud.md:58,60 - player-facing wiki still said
  "ship-computer drawer" / "the drawer is there" / "permanent drawer sections".
  Align to "computer" for consistency with the swept in-game strings.
  - Response: Fixed. Heading is now "The ship computer"; the two body mentions
    are "permanent side panels" and "the computer is there".
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/drawer.rs (prompt caret) - only
  end-of-line cursor cases were tested; add a mid-string caret assertion.
  - Response: Added to `nova_os_inline_completion_is_same_line_continuation`: after
    typing `help` and moving the caret left twice, `prompt_before_cursor` is `he`
    and `prompt_after_cursor` is `lp`, pinning the before/caret/after split.
