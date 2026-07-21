# Review: Gauntlet time-trial - HudReadout action + timer + clean-run bonus

- TASK: 20260716-174729
- BRANCH: feature/hud-readout-timer

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. Deep verification against a real build/test run, all PASS:

- BUILD + missing_docs: `cargo build -p nova_scenario -p nova_gameplay
  -p nova_assets` 0 warnings; `cargo doc` warning-free - every new public item
  (HudReadoutFormat/ActionConfig, HudReadoutEntry/HudReadouts/HudReadoutPlugin,
  set_hud_readout) carries a `///`. No missing_docs regression.
- New-variant safety: the actions dispatch match is EXHAUSTIVE (no `_`), so the
  compiler forced handling of `HudReadout`; lint.rs handles it explicitly.
  `content lint` 0 errors on gauntlet.
- Tests NON-vacuous: gauntlet_course 12/12, readout 3/3, scenario 2/2 - removing
  the timer wiring fails `on_start_shows_the_run_timer_on_the_scenario_clock`;
  removing a Victory branch/gate fails the two crash-gated tests.
- Clean-run gating TRACED against bcs queue_system: on FINISH, `gate==7` sets
  `gate=8`, then `gate==8 AND crash==0` XOR `gate==8 AND crash>0` fires exactly
  one Victory (never both/neither). `crash` seeded 0 in OnStart; increments only
  on the 3 player-ship graze OnEnters gated `gate<8`.
- Frozen-readout VERIFIED: `tick_scenario_clock` is Unpaused-gated, the outcome
  holds Paused, so scenario_elapsed freezes; the overlay is a 60%-alpha scrim
  (GlobalZIndex 9) that does NOT hide the Instrument-tier HUD. Final time latches
  and the row persists - DoD "final time on Victory" met.
- Version 1.3.0 + assertion updated; gen-portal.py lists gauntlet 1.3.0; docs
  (action reference in guide-author-scenario.md + scenario-system.md, CHANGELOG,
  gauntlet README) updated; npm run ci green.

- [ ] R1.1 (MINOR) crates/nova_scenario/src/lint.rs - the HudReadout lint is
  structural (errors on empty slot/variable, tracks the bound var into used_vars
  so a readout of a never-set variable warns, scenario_elapsed exempted) but not
  SEMANTIC (a Time format on a non-time counter is not caught).
  - Response: accepted, no change. Semantic format/variable validation is
    unknowable statically for a generic modding surface; structural validation +
    the used-vars warning is the right bound. The variant is explicitly handled,
    not a silent fallthrough.
- [ ] R1.2 (MINOR) crates/nova_scenario/src/world.rs:96 - the sync comment said
  the readout set is "rebuilt every frame", which overstates it under the outcome
  pause (the sync stops re-running; the value latches).
  - Response: fixed - the comment now states it is rebuilt each frame the sync
    runs WHILE the scenario is live, and under the outcome/pause freeze the last
    value latches and the row persists (which is how final-time-on-Victory holds).
- [ ] R1.3 (NIT) crates/nova_gameplay/src/hud/readout.rs - the strip centers via
  a hard-coded `margin.left: -80px` (half the fixed 160px width); a wider label
  or second longer readout would sit off-center.
  - Response: accepted, no change. Fine for the current single time readout;
    revisit if a multi-row/variable-width readout is authored. Cosmetic +
    playtest-adjacent (folds into the feel pass).

## Pending manual acceptance (does NOT block APPROVE)

- (pending) FEEL/BALANCE playtest: is the time-trial fun to re-fly; are the graze
  radii (16/18/30u) tuned so a clean run is neither trivial nor impossible; does
  the dimmed frozen time readout read clearly behind the ~40% Victory scrim (if
  too faint, the deferred fix is minimal `{variable}` interpolation in the
  Outcome message). Left as pending user confirmation, NOT self-ticked.
