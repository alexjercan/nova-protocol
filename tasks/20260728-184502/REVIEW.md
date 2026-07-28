# Review: NOVA OS shell-like help + wrong-command usage messages

- TASK: 20260728-184502
- BRANCH: feat/nova-os-shell-help

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings came from a fresh subagent with no sight of the implementing
session (task id + branch + worktree + review dimensions only). It compiled and
ran the suite itself: `nix develop -c cargo test -p nova_os --lib` (20 passed),
`nix develop -c cargo check -p nova_gameplay --tests` (clean), the DoD-7 `rg`
sweep (only the canonical `rejection()` reason string + new-format assertions
remain), and the docs sweep (README/wiki/AGENTS/CHANGELOG clean; `arg_hint`
recorded in DECISION.md). In-session re-verification: independently re-ran the
nova_os suite and the DoD-7 sweep and confirmed both claims.

- [x] R1.1 (MINOR) crates/nova_os/src/terminal.rs render path - DoD-2 names
  three usage proofs but only `map goto <label>`, `help help` and a no-hint
  `spin` fallback were asserted; `ship reload <section>` (and the other ship
  verbs) set their hint at the nova_gameplay registration site with nothing
  pinning that wiring. Suggested change: assert a ship verb renders
  `Usage: ship <verb> <section>` through the registered tree.
  - Response: Fixed. Added `ship_verb_help_names_the_section_argument` in
    `crates/nova_gameplay/src/hud/nova_os_ship.rs` - it submits `ship
    section|reload|repair help` against a terminal seeded from the real
    `ship_command_tree()` and asserts `Usage: ship <verb> <section>`, pinning
    both the `.with_arg_hint("<section>")` registration and the render wiring
    end to end. Passes.
- [ ] R1.2 (MINOR) examples/ui/nova_os_terminal_poc.html:1472 - still prints
  `Available commands:`, a string retired in the live source. Suggested change:
  leave as-is (frozen design reference) or optionally update the PoC help block.
  - Response: Left as-is by design. The PoC is the frozen reference the epic
    (20260728-175719) treats as canonical; this task ships the improvement in
    the live terminal, and the PoC is not a live doc surface. Noted for a future
    PoC refresh if the epic revisits it.

No BLOCKER/MAJOR. No open `manual:` DoD items (all proofs are `test:`/`cmd:`).
