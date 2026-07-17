# Review: Re-derive orbit-hold + lock-refire 5s windows onto the engine scenario clock

- TASK: 20260717-151537
- BRANCH: refactor/scenario-clock-timers

## Round 1

- VERDICT: APPROVE

Reviewed the diff of `crates/nova_scenario/src/loader.rs` against master. Because
implementer and reviewer share a session, the two load-bearing claims were
independently re-derived (once here, once by an out-of-context reviewer agent),
not just read off the diff.

### Load-bearing claims - both CONFIRMED

- **Behavior parity (fire timing unchanged).** First window: the old `held_secs`
  starts at 0 on the deferred-insert frame N and accumulates delta only from
  N+1; the new `started_at = now_N` fires when the sum of deltas of N+1, N+2, ...
  reaches 5.0 - identical origin, identical exclusion of the insert frame's own
  delta. Refire/reset: old `held_secs = 0.0` and new `started_at = now` both
  re-measure 5.0s from the fire frame (no cross-window drift). Modeled the
  orbit test frame-by-frame (dt=0.2, deferred flush) -> exactly 0/1/2/3 fires,
  and `tick_lock_slot` against its unit test -> every Some/None reproduced.
- **`.after(tick_scenario_clock)` ordering.** In Bevy 0.19 `.after` is a pure
  ordering edge; run conditions do NOT propagate along it. When the clock tick
  is skipped under pause (its `Unpaused` gate is false), the trackers (gated
  `scenario_is_live` only) still run and read the last frozen `scenario_elapsed`,
  so `now - started_at` is constant and nothing new fires - pause-freeze by
  construction. On live frames the tick is ordered first, so the tracker never
  reads a pre-tick stale value.

Independently verified that production pause actually freezes virtual time:
`pause_clocks` (crates/nova_menu/src/lib.rs:322) calls `virtual_time.pause()` on
`OnEnter(PauseStates::Paused)`, so the OLD delta-accumulator trackers were also
pause-correct in production - this change is true parity there, and additionally
robust to any `PauseStates::Paused` set without the menu's clock freeze (it drops
a latent nova_scenario -> nova_menu coupling).

### Findings

- [x] R1.1 (MINOR) loader.rs:413,510 - `ORBIT_HOLD_SECS`/`LOCK_REFIRE_SECS` were
  left `f32` but are now only ever compared as `... as f64`. The type + per-use
  cast is vestigial. Change both consts to `f64` and drop the two `as f64` casts.
  - Response: Fixed - both constants are now `f64`; the two casts at the compare
    sites are gone. 5.0 is exactly representable so no timing change; 22 loader
    tests still pass.
- [ ] R1.2 (NIT) loader.rs:344-357 - the `SCENARIO_ELAPSED_VAR` doc says an early
  read "fails closed via the undefined-variable rule", while the trackers' helper
  `scenario_elapsed` maps a missing var to `0.0`. Different read paths
  (expression-filter eval vs the helper), so not a contradiction; the helper doc
  already states the `None -> 0.0` pre-tick behavior. Left to discretion.

### Tests

All three modified tests still assert real behavior and would fail if the fix
were reverted/broken:
- `a_lock_slot_fires_on_acquisition_then_echoes_per_window`: the refire fires at a
  genuine 6.0s-since-acquisition crossing; breaking `>=` or the `last_fired_at =
  now` reset fails it.
- `orbit_hold_fires_once_per_window_and_recurs` /
  `player_locks_fire_their_events_and_ai_locks_never_do`: rigs register
  `(tick_scenario_clock, track_*).chain()` ungated - a faithful stand-in for
  production's `.after` clock-first ordering in the unpaused case these tests
  cover. Pause-freeze for the trackers is argued structurally and the clock's own
  pause test (`scenario_clock_freezes_while_paused`) covers the freeze path.

Suite: `cargo test -p nova_scenario --features serde --lib -- loader::` -> 22
passed, 0 failed. `cargo fmt` clean. (Bare `-p nova_scenario` without `--features
serde` fails to compile four PRE-EXISTING `ScenarioConfig` serde round-trip tests
- unrelated to this diff; recorded in NOTES.md.)

### Follow-up surfaced (not a blocker on this branch)

The 5s durations are hardcoded engine constants, not author-configurable per
scenario/well/event. Making them configurable is a separate feature; filed as its
own tatr task rather than widening this refactor.
