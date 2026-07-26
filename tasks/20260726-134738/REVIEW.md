# Review: Match NOVA OS drawer to terminal PoC

- TASK: 20260726-134738
- BRANCH: feature/nova-os-poc-fidelity

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [ ] R1.1 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:1771 - The footer
  advertises `Ctrl+C: return from app`, but this task explicitly does not
  implement app runtime and `TerminalMode` only has `Prompt`, with no Ctrl+C
  handling. Remove this hint or replace it with a currently wired action until
  the app-runtime task lands.
  - Response: fixed in working tree by changing the footer hint to
    `help: list commands`, which is currently wired.
- [ ] R1.2 (MAJOR) web/src/wiki/hud.md:56 - This live player doc still says
  "the full list lives in the ship-computer drawer", but the branch removes the
  visible objectives list and has no `objectives` command yet. Update this
  sentence to say the drawer currently only shows the terminal, with objective
  details deferred until a future command/app surface.
  - Response: fixed in working tree by updating `web/src/wiki/hud.md` and the
    matching live `CHANGELOG.md` entry to say detailed objective output is
    deferred to a future NOVA OS command/app surface.

Manual DoD still pending: compare the running drawer or captured screenshot
against `examples/ui/nova_os_terminal_poc.html` and confirm remaining
differences are accepted or recorded.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:1771 - The footer
  advertises `Ctrl+C: return from app`, but this task explicitly does not
  implement app runtime and `TerminalMode` only has `Prompt`, with no Ctrl+C
  handling. Remove this hint or replace it with a currently wired action until
  the app-runtime task lands.
  - Response: fixed in working tree by changing the footer hint to
    `help: list commands`, which is currently wired.
- [x] R1.2 (MAJOR) web/src/wiki/hud.md:56 - This live player doc still says
  "the full list lives in the ship-computer drawer", but the branch removes the
  visible objectives list and has no `objectives` command yet. Update this
  sentence to say the drawer currently only shows the terminal, with objective
  details deferred until a future command/app surface.
  - Response: fixed in working tree by updating `web/src/wiki/hud.md` and the
    matching live `CHANGELOG.md` entry to say detailed objective output is
    deferred to a future NOVA OS command/app surface.

Manual DoD still pending: compare the running drawer or captured screenshot
against `examples/ui/nova_os_terminal_poc.html` and confirm remaining
differences are accepted or recorded.
