# Review: Investigate Entity-despawned command error on menu to game transition

- TASK: 20260713-175352
- BRANCH: fix/menu-despawn-command-warn

## Round 1

- VERDICT: APPROVE

- [x] R1.1 (MINOR) docs/development.md:59 - the Examples section maintains
  the current example list ("Current set: ...12_hud_range") and does not
  include the new `13_menu_newgame`; add it to the list.
  - Response: fixed in the review-response commit (list now ends
    `13_menu_newgame`). Reviewer verified the line in the committed diff.

Basis for the verdict (shared-session rule: load-bearing claims re-verified
independently):

- The falsification is properly evidenced, not asserted: 10 harnessed runs
  across the three transitions (1 Sandbox via 09_editor, 6 New Game, 3
  editor Play), each with a delivery guard (the click marker + "reached
  Playing" line), zero command errors. The rig is recorded in TASK.md with
  the exact build flags and env.
- The pin can actually fail (would-it-fail-without-it): re-checked the
  sabotage evidence - stale-entity command injected after committing the
  pin, run aborts exit 134 with the exact error shape from the web log
  ("Entity despawned ... its index now has generation 1"), then reverted
  via git checkout (worktree clean, verified).
- Pin wiring verified against the smoke test's real assertions
  (tests/examples_smoke.rs: exit success + "reached Playing" + "cycle
  complete" in stderr): the recorded runs satisfy all three on both paths,
  and the example is in HARNESSED_EXAMPLES so CI executes it.
- `FallbackErrorHandler(panic)` is swapped only under BCS_AUTOPILOT, so an
  interactive `cargo run` keeps Bevy's default warn behavior - the pin
  cannot crash a normal play session.
- Checks run by the reviewer: `cargo check --all-targets` green without
  features (the example compiles with the harness cfg'd out) and with
  `--features debug` (from the work phase); `cargo fmt` clean. Full test
  suite + clippy skipped per repo policy (CI owns them; the smoke test
  itself runs there under xvfb).
- Honesty: TASK.md's Steps were rewritten to record N/A outcomes instead of
  silently ticking; the no-CHANGELOG decision is stated with its reason.
- Rider commits 244081e (previous task's retro) is documentation only,
  forced onto this branch by the background session's checkout write guard.
