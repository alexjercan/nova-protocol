# Review: ch3 overspeed - sustained ~3.5s window on the second strike

- TASK: 20260723-182850
- BRANCH: feature/ch3-overspeed-window

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Findings: none.

What the out-of-context reviewer verified (re-confirmed in-session by re-running
the same three checks and re-tracing the state machine before adopting):

- All three checks pass in the worktree: `ledger_ch3_channel` 17/17 green (incl.
  the rewritten `overspeed_warns_then_a_held_breach_wakes_both_magpies_after_the_window`
  and the new `easing_off_during_the_countdown_cancels_the_wake`); `content lint
  --target webmods/the-ledger` 0 error/warning/finding (5 scenarios audited, 1
  pre-existing Auditor ACK); `webmods_validation` loads (1 passed).
- State machine traced: the five `speed_warned` handlers are mutually exclusive
  per frame. CANCEL (3->2) and TRIP (wake) share `warned==3` but split on
  `speed<7` vs `speed>8`, so no same-frame cancel-and-trip regardless of handler
  order; START (2->3) and TRIP key on different states, and the deadline is
  stamped `now+3.5` so `elapsed>deadline` is false on the stamping frame. No
  double-fire, no stuck state, no warn-and-trip in one frame.
- `overspeed_deadline` is seeded 0.0 in OnStart and mirrored in BOTH the rig's
  `armed_app` list and the `on_start_seeds_*` key pin in the same diff - no
  seed-helper drift.
- The "before the deadline the run is still dark" assertion is load-bearing:
  `pump_clock` re-seeds only `scenario_elapsed`, leaving `player_speed` at 9.0,
  so the burn is provably held (9>8) while the clock (2.0) sits under the
  deadline (3.5). Tests would fail against the reverted instant-trip RON; both
  assert on the live `Allegiance` component, not just variables. No existing
  assertion was weakened.
- Design idioms followed (the `Add(Factor(Name("scenario_elapsed")), ...)`
  beat_gate pattern, the Expression-filter shape, `spotted==0 && act==1` gating,
  ~72-col comments). Docs/honesty: 1.9.0 CHANGELOG entry added above the
  untouched dated 1.8.0 block; bundle version, README, news, mod-guide walk all
  match the RON; TASK.md Outcome claims match the code.

Pending manual check (human-acceptance gate, batched at the flow Finish - not
resolved by this APPROVE): playtest ch3 - warn once, gun it again and confirm
~3.5s to ease off before both Magpies wake; hold the burn and confirm they wake.
