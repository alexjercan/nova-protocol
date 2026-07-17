# Design / fix record - re-derive the 5s scenario-event windows onto the clock

Task: 20260717-151537. Branch: refactor/scenario-clock-timers.

## What changed

`crates/nova_scenario/src/loader.rs` only:

- Added `scenario_elapsed(&NovaEventWorld) -> f64`, the single clock reader
  (same `None -> 0.0` fallback as `tick_scenario_clock`, which now reuses it).
- `OrbitHold` went from `{ well, held_secs: f32 }` (a delta accumulator) to
  `{ well, started_at: f64 }` (the clock reading when the window opened).
  `track_orbit_holds` dropped `Res<Time>` for `Res<NovaEventWorld>`, reads
  `now = scenario_elapsed(...)`, and fires when `now - started_at >=
  ORBIT_HOLD_SECS`, resetting `started_at = now` (mirrors the old
  `held_secs = 0.0` reset-from-now). The R1.2 rule is intact: a well without a
  scenario id still resets the window and retries.
- `LockEcho` slots went from `Option<(Entity, f32)>` (accumulated seconds) to
  `Option<(Entity, f64)>` (clock timestamp of last fire). `tick_lock_slot`'s
  third arg is now the absolute clock `now` instead of a per-call delta; it
  fires on acquisition (target changed) and again once `now - last_fired_at >=
  LOCK_REFIRE_SECS`. `track_player_locks` dropped `Res<Time>` for
  `Res<NovaEventWorld>` and reads `now` once for both slots.
- Both trackers are registered `.after(tick_scenario_clock)` so they read this
  frame's clock. They stay gated on `scenario_is_live` only (NOT Unpaused).
- Rewrote the plugin comment block that used to justify the trackers by "they
  fire only when a Time<Virtual> delta threshold is crossed, and that clock is
  frozen under pause": it now states they derive from `scenario_elapsed`, so a
  paused frame reads a frozen clock and no window can advance - correct by
  construction, not by the virtual-time assumption.

## Why

The scenario clock (task 20260717-112647) already tracks live-unpaused seconds
under an EXPLICIT `Unpaused` gate and clears with the event world at teardown.
The orbit-hold and lock-refire windows were the last two places measuring
scenario time by accumulating their own `Time` delta; their pause-correctness
depended on the implicit "Time<Virtual> delta ~= 0 while paused" fact spelled
out in the old comment. Deriving both from the clock single-sources the
pause/teardown/retry semantics and removes that fragile dependency: if the
pause mechanism ever stopped freezing virtual time, the clock's own `Unpaused`
gate would still hold and both windows would stay frozen.

## Decisions / tradeoffs

- **Ordering (`.after(tick_scenario_clock)`).** The tick is Unpaused-gated; the
  trackers are not. In Bevy `.after` is an ordering constraint only, so when the
  tick is skipped under pause the trackers still run and read the last (frozen)
  clock value - exactly the desired behavior. Without the ordering the trackers
  could read last frame's clock (a one-frame, ~16ms lag on a 5s window -
  harmless), but ordering makes the tests deterministic against fixed manual
  time steps.
- **Reset-from-now, not fixed cadence.** `started_at = now` / `last_fired_at =
  now` on fire preserves the old `= 0.0` semantics (each window is measured from
  the previous fire), so timing is byte-for-byte equivalent, not just close.
- **Behavior parity is the bar.** No fire-timing change was intended; the three
  affected tests keep their original assertion counts.
- **Component reset stays free.** `OrbitHold`/`LockEcho` are entity components
  that die with the (despawned) ship on teardown, so a retry re-arms both
  windows naturally; the clock reset just backs that up.

## Tests

- Updated `orbit_hold_fires_once_per_window_and_recurs` and
  `player_locks_fire_their_events_and_ai_locks_never_do` to register
  `(tick_scenario_clock, track_orbit_holds/track_player_locks).chain()` so
  `scenario_elapsed` advances under the same ordering production uses -
  otherwise `now` would never move and "held is quiet" would pass for the wrong
  reason.
- Rewrote `a_lock_slot_fires_on_acquisition_then_echoes_per_window` to thread an
  increasing absolute `now` (a small `at(dt)` closure) instead of per-call
  deltas; same acquisition/echo/retarget/clear sequence.
- `cargo test -p nova_scenario --features serde --lib -- loader::` -> 22 passed,
  0 failed (includes the clock tests, unchanged). `cargo fmt` clean.

## Gotcha worth recording

`cargo test -p nova_scenario` (crate in isolation) does NOT compile: four
pre-existing tests (loader.rs ~2324-2360) call `ron::to_string`/`from_str` on
`ScenarioConfig`, whose serde derives are behind the crate's optional `serde`
feature. The feature is only pulled in by workspace feature unification, so CI's
`cargo test --workspace --features debug` builds them but a bare `-p` run does
not. Run the crate's tests with `--features serde` (or at the workspace level).
This is unrelated to this change - noting it so the next session does not chase
it as a regression.
