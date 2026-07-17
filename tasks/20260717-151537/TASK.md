# Re-derive orbit-hold + lock-refire 5s windows onto the engine scenario clock

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.7.0,scenario,refactor

## Goal

The two 5s scenario-event timers in `crates/nova_scenario/src/loader.rs` each
accumulate their OWN `time.delta_secs()` and are pause-correct only by the
implicit "Time<Virtual> is frozen under pause (delta ~= 0)" assumption that the
loader comment (lines ~257-263) spells out. The engine scenario clock
`scenario_elapsed` (task 20260717-112647, `SCENARIO_ELAPSED_VAR`) already
tracks live-unpaused seconds under an EXPLICIT `Unpaused` gate and resets with
the event world at teardown/retry. Re-derive both windows from that clock so
their pause/teardown/retry semantics are single-sourced on the engine clock and
no longer ride the fragile Time<Virtual> assumption.

Behavior is intended to stay identical (fire once per 5s window, recurring); the
change is WHERE the 5 seconds is measured from.

## Steps

- [x] Change `OrbitHold` (loader.rs:392) from `{ well: Entity, held_secs: f32 }`
      to `{ well: Entity, started_at: f64 }`, where `started_at` is the
      `scenario_elapsed` reading when the current window opened (engagement or
      last fire).
- [x] Rewrite `track_orbit_holds` (loader.rs:403): drop `time: Res<Time>`, add
      `world: Res<NovaEventWorld>`, read `now = scenario_elapsed` via
      `world.get_variable(SCENARIO_ELAPSED_VAR)` (None -> 0.0, same fallback as
      `tick_scenario_clock`). New engagement / well switch -> `started_at = now`.
      Held same well -> fire when `now - started_at >= ORBIT_HOLD_SECS` and reset
      `started_at = now` (mirrors the current `held_secs = 0.0` reset-from-now).
      Keep the R1.2 rule: reset the window even when the well has no scenario id.
- [x] Change `tick_lock_slot` (loader.rs:491) signature from
      `(state: &mut Option<(Entity, f32)>, current, delta_secs: f32)` to
      `(state: &mut Option<(Entity, f64)>, current, now: f64)`. State stores
      `(target, last_fired_at)`. Acquisition (target changed) -> set
      `Some((target, now))`, return `Some(target)`. Held same target -> fire when
      `now - last_fired_at >= LOCK_REFIRE_SECS` and set `last_fired_at = now`.
      Clearing (`None`) -> `*state = None`.
- [x] Change `LockEcho` (loader.rs) `travel`/`combat` fields to
      `Option<(Entity, f64)>` and rewrite `track_player_locks` (loader.rs:525):
      drop `time: Res<Time>`, add `world: Res<NovaEventWorld>`, read `now` once
      and pass it to both `tick_lock_slot` calls.
- [x] Order both trackers AFTER the clock tick so they read THIS frame's clock:
      add `.after(tick_scenario_clock)` to the `track_orbit_holds` and
      `track_player_locks` registrations (loader.rs:275, ~281). The trackers stay
      gated on `scenario_is_live` ONLY (not Unpaused): reading the frozen clock
      under pause means no window can advance, so no new fire happens - which is
      now correct BY CONSTRUCTION rather than by the Time<Virtual> assumption.
- [x] Rewrite the loader comment block (loader.rs ~257-268): the sibling
      trackers no longer "fire only when a Time<Virtual> delta threshold is
      crossed"; state that they now DERIVE their windows from `scenario_elapsed`,
      so they freeze under pause together with the clock/pulse and reset with the
      event world at teardown - single-sourced, not reliant on Time<Virtual>.
- [x] Update the doc comments on `OrbitHold`, `track_orbit_holds`,
      `ORBIT_HOLD_SECS`, `tick_lock_slot`, `LOCK_REFIRE_SECS`, `LockEcho` to
      describe the clock-timestamp mechanism instead of the delta accumulator.
- [x] Update `orbit_hold_fires_once_per_window_and_recurs` (loader.rs:1694): the
      test drives `track_orbit_holds` alone with a manual 0.2s Time step; register
      the clock tick alongside it, e.g.
      `(tick_scenario_clock, track_orbit_holds).chain().run_if(scenario_is_live)`,
      so `scenario_elapsed` advances each frame. The per-window assert counts
      should hold unchanged (same delta feeds the clock).
- [x] Rewrite `a_lock_slot_fires_on_acquisition_then_echoes_per_window`
      (loader.rs:1812) to pass an increasing absolute `now` instead of per-call
      deltas (thread a running `t` and add the same steps), asserting the same
      acquisition/echo/retarget/clear sequence.
- [x] Update `player_locks_fire_their_events_and_ai_locks_never_do`
      (loader.rs:1841): register the clock tick so `now` advances, and adjust any
      timing so the end-to-end acquisition + fire assertions still hold.
- [x] Run `cargo check -p nova_scenario` and the newly written/updated tests for
      this module (`cargo test -p nova_scenario`), plus `cargo fmt`. Per repo
      policy skip the full local clippy/test sweep; CI runs the suite.
- [x] Add a CHANGELOG.md entry under the current unreleased section and write a
      `tasks/20260717-151537/NOTES.md` design/fix record (what moved to the
      clock, why, the one-frame ordering decision, and that behavior is
      intentionally unchanged).

## Notes

- Relevant files: `crates/nova_scenario/src/loader.rs` (trackers, clock,
  comment block, tests), `CHANGELOG.md`.
- The clock: `SCENARIO_ELAPSED_VAR = "scenario_elapsed"` (loader.rs:341);
  `tick_scenario_clock` (loader.rs:347) reads/writes it in `NovaEventWorld`
  via `get_variable`/`insert_variable` with a `None -> 0.0` fallback;
  `register_clock_and_pulse` (loader.rs:362) chains tick before the OnUpdate
  pulse under `scenario_is_live.and_then(in_state(PauseStates::Unpaused))`.
- Constants stay 5.0: `ORBIT_HOLD_SECS` (loader.rs:385), `LOCK_REFIRE_SECS`
  (loader.rs:475). Only the measurement source changes.
- Ordering rationale to verify at implement time: `.after(tick_scenario_clock)`
  is an ORDERING constraint only; when the clock is skipped under pause the
  trackers still run (their own `scenario_is_live` condition holds) and read the
  last-written frozen value. Confirm Bevy applies `.after` across differing run
  conditions as ordering-only (it does) before relying on it.
- The event world is CLEARED at teardown (`teardown_scenario_entities` ->
  `world.clear()`), so `scenario_elapsed` restarts at 0 on retry; `OrbitHold`
  and `LockEcho` are entity components that die with their (despawned) entities,
  so a retry naturally re-arms both windows against a fresh clock.
- Behavior parity is the acceptance bar: the change is measurement source, not
  timing. If a test count must change, that is a signal the mechanism drifted -
  investigate before "fixing" the assertion.
