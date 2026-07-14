# Review: centralize gameplay HUD palette into nova_ui

- TASK: 20260714-214118
- BRANCH: ui/hud-palette

## Round 1

- VERDICT: APPROVE

A zero-hue-change centralization (option A, per the user). Load-bearing claim
re-verified: nothing renders differently.

- Value-parity: each of the 9 relocated HUD consts now references a
  `nova_ui::theme::semantic` accent whose value is byte-identical to the original
  literal (verified against the grep of the exact HUD const values). A new
  `nova_ui` test pins those semantic values to the original literals, so a future
  drift is caught and must be deliberate.
- Correct semantic grouping: only the exactly-repeated accents were merged - THREAT
  (the 1.0,0.35,0.3 combat red used by reticle + lock + faction-hostile, 3 sites),
  BACKDROP (the 0.15,0.15,0.15,0.8 readout backdrop, 2 sites), plus the brand/
  faction singletons (NAV, OBJECTIVE, ALLY, NEUTRAL). The per-widget tuned red/amber
  variants and the diegetic 3D colours are correctly LEFT LOCAL - merging them would
  have changed a hue.
- `cargo check --workspace --all-targets --features debug` clean; `nova_ui` 2 tests
  pass; no unused-import warnings (each touched file still uses `Color` elsewhere).

No findings. The full `nova_gameplay` suite was not run locally (repo policy - CI
covers it); justified here because no gameplay logic changed, only const
definitions relocated at identical values. Web-palette alignment (a real visual
change) is correctly deferred to a QA-gated follow-up.
