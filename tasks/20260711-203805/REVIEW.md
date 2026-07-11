# Review: F1 to editor must be Sandbox-only

- TASK: 20260711-203805
- BRANCH: fix/f1-sandbox-only

## Round 1

- VERDICT: APPROVE

Small diff (one run_if + one regression test + CHANGELOG), reviewed
in-session with independent re-derivation instead of the out-of-context
pass (proportionality: 97 insertions, one behavior):

- Re-derived the condition semantics: and_then short-circuits, so GameMode
  is only read inside the Scenario state; the resource always exists
  (NovaGameplayPlugin init). Composition matches the mode-gate pattern
  used by setup_scenario and the editor routing.
- Swept for other F1 consumers/documentation: none exist (docs, examples,
  keybind hints) - no contradiction shipped.
- The test satisfies the would-it-fail-without-it ledger rule in both
  directions: NewGame half fails on the pre-fix code (state would flip),
  and the Sandbox half fails if the F1 path itself breaks (delivery
  guard); the null branch carries the editor-scenario-load counter guard.
- nova_editor 4/4, cargo check --workspace clean, fmt clean.

No findings. APPROVE.
