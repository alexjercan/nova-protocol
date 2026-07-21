# Retro: Gauntlet time-trial - HudReadout action + timer + clean-run bonus

- TASK: 20260716-174729
- BRANCH: feature/hud-readout-timer (landed c6e2138c)
- REVIEW ROUNDS: 1 (out-of-context APPROVE; 2 MINOR + 1 NIT, R1.2 fixed)

Process only; see TASK.md close-out for what shipped.

## What went well

- The design decision (Step 1) kept the "no new features" theme honest: exactly
  ONE new action (`HudReadout`, generic per the spike + DoD "usable by any mod")
  is the whole engine addition; the clean-run counter, gating and final-time
  display all reuse EXISTING vocabulary (CreateScenarioArea OnEnter, VariableSet,
  expression-filtered Outcome). Mirroring the proven `StoryMessage -> StoryFeed
  -> comms_panel` sync pattern meant no new architecture.
- The "final time on Victory: frozen readout vs message interpolation" fork was
  resolved against the ACTUAL code, not a guess: the outcome overlay is a scrim
  that dims but does not hide the HUD, and `tick_scenario_clock` is Unpaused-
  gated so `scenario_elapsed` freezes - so the frozen Instrument-tier readout
  holds the final time and NO interpolation was needed (minimal, per the Notes).
- Compounding win: the missing_docs lint enabled earlier THIS session
  (133032/121316) auto-enforced doc comments on every new public item of this
  feature - the build would have warned on any undocumented `HudReadout*` type,
  so the docs could not rot at birth.
- Review traced the correctness crux (clean-run gating) to the bcs
  `queue_system` handler ordering and confirmed exactly ONE Victory fires (clean
  XOR plain) - the kind of "fires wrong/both banners" bug a compile + a green
  test would not catch.

## What went wrong

- Minor: `sprout land` does not accept `-F` (only `-m`); my first land invocation
  failed on it. Re-ran with `-m subject -m body` (plain text, no backticks - the
  backtick-substitution lesson from the prior task applied).

## What to improve next time

- For a scenario-vocabulary feature, default to "add the minimal generic action;
  express the rest with existing actions/filters" - it keeps the engine surface
  small and the feature mod-reusable, and it is what let this land as one action.

## Action items

- No ledger lesson (reinforces existing verify-against-code + minimal-surface
  principles).
- PENDING USER ACCEPTANCE (batched to the user at report, not self-ticked):
  FEEL/BALANCE playtest - fun to re-fly, graze radii (16/18/30u) tuning, frozen
  readout legibility behind the ~40% Victory scrim. If the readout is too faint,
  the recorded fallback is minimal `{variable}` interpolation in the Outcome
  message.
