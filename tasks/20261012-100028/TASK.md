# Economy: play-time clock in the world save

- STATUS: OPEN
- PRIORITY: 57
- TAGS: v0.16.0,economy

## User facts
- Epic `20261012-100025`. Decision 8 (2026-10-12): add a play-time clock to the save when the first consumer (restock) lands, with a catch-up rule per frozen body type; nothing earlier needs it.

## Agent findings
- No clock exists; the only time is scenario `Elapsed` (grep `WorldClock|world_time|WorldTime` in `crates/nova_world*` and `nova_scenario`: no hits). Frozen sectors do not simulate (`crates/nova_world/src/lib.rs:116-117`).

## Delivery gate
- `WorldClock { play_seconds }` in the save state, advanced only while unpaused and armed; `FrozenBody` records `frozen_at`; a thaw hook receives the elapsed play time. Format bump, **(breaking)**.

## Verification
- Unit test: pause does not advance the clock; save and load keep it; a thaw hook sees the right elapsed time.

## Done when
- The clock is in the save and one consumer (restock) reads it.

## Agent decisions 2026-10-16
- `WorldClock { play_seconds: f64 }` lives in `nova_world` (`crates/nova_world/src/clock.rs`), beside the ledger it stamps. `NovaWorldPlugin` owns it: advanced by `advance_world_clock` in `NovaWorldSystems::Observe` from `Time<Virtual>` delta (zero under every `ClockFreeze` hold), reset by `clear_sector_work` with the ledger, restored by `restore_resumed_world` on the arming frame.
- `freeze_body` takes `&WorldClock` and stamps `FrozenBody.frozen_at` (play seconds). `thaw_record` and `materialize_sector` take `&WorldClock`.
- The thaw hook is a component: every body thawed from a record spawns with `ThawedAfter { play_seconds }` (clock less `frozen_at`). A held ship or station regenerates from its entry and carries none; a generated body carries none. The restock consumer reads `Added<ThawedAfter>` per owner type.
- `elapsed_since` panics on a stamp ahead of the clock: no save or session produces one.
- Save format 14 -> 15; `WorldSaveState.clock`; the header does not show play time.
- Job expiry is not in this task. The design has only the D1 pitch line "some offers expire" (`DESIGN.html:347`) and `jobs.rs:57` "no job expires until the clock lands"; no decision names a window, what expires or where it is recorded. Not implemented, not invented.
- No restock consumer here. Task `20261012-100029` is the first consumer; this task stays OPEN until it reads `ThawedAfter`.

## Proof 2026-10-16
- `nova_world` clock tests: `the_clock_counts_unpaused_time_only` (0.2 s after two 100 ms frames, unchanged over five paused frames, 0.3 s after the unpause), `elapsed_is_the_clock_less_the_freeze_stamp`.
- `nova_world` frozen tests: `a_thawed_body_carries_the_play_time_it_stood_frozen` (frozen at 10 s, thawed at 25 s, each rock `ThawedAfter { 15.0 }`, fresh rocks none); `ending_the_session_forgets_every_frozen_cell` also asserts the clock resets.
- `nova_world_base` save tests: `a_saved_clock_resumes_at_the_play_time_it_was_written_with` (leave save writes 123.5 s, `open_world` reads it, `load` restores the resource); the 35 save tests pass.
- Live (proof lane, Xvfb + lavapipe): `system_world_resume --features debug` create phase crossed (0,0,0) -> (1,0,0); `world.ron` format 15; `state.2.ron` holds `clock: (play_seconds: 0.942221343)` and 27 `frozen_at: 0.942221343` stamps on the 25 retired cells' bodies. Observed once, unrelated to the clock: the example's `inject_fixture` can run before any sector streamed (`system_world_resume.rs:845` panic); a retry passed.
- `cargo check --workspace --tests` and `cargo check -p nova-protocol --examples --features debug` clean; `cargo fmt` clean.

## Handoff to 20261012-100029 (served 2026-10-16)
- The consumer landed: `restock_thawed_traders` (`crates/nova_world_base/src/economy.rs`) reads `ThawedAfter` beside the thawed hold (`Added<ShipInventory>`) after `NovaWorldSystems::Retire` and moves each targeted count toward the trader's target by `restocked_count`. Unit proof `a_thawed_stock_drifts_toward_its_target_by_the_elapsed_play_time` (`crates/nova_gameplay/src/market.rs`); live proof `system_station_restock` passed under Xvfb + lavapipe on 2026-10-16: a thaw after 912.8 s of play restocked PdcRound 0 -> 82 of 162, a zero-elapsed thaw changed nothing, and a Load restocked it to 162 from the saved clock over 1813.5 s (details in the 100029 task file).
- The `TODO(20261012-100028)` in `thaw_station` was removed; no income rule is approved, so an owned station earns nothing and `docs/sections.md` and `station_buy.rs` say so.
- Save format 15 -> 16 with the 100029 record (`WorldSaveState.neutralized`); the clock entry in `CHANGELOG.md` now names restock as its reader.

## Job expiry review 2026-10-16 (owner decision pending)
- The session todo paired this task with "job expiry". Not implemented: `JobBoard` is not frozen or saved (`post_station_boards` re-posts a board deterministically from state each time a station is live), `JobId { station, slot }` mints body ids per slot so a re-rolled offer would collide with a live posting's bodies, and no approved text names what expires (offer or accepted job), the duration, or where the deadline is recorded. `crates/nova_gameplay/src/jobs.rs:55-60` states that no offer expires and why.
- Options for the owner: (a) accept "no expiry" and drop the clause; (b) approve an authored `offer_seconds` on the board, a saved posting stamp per slot, a HUD deadline and a re-roll rule that retires the slot's bodies first. This task stays OPEN until the owner decides.
