# Economy: play-time clock in the world save

- STATUS: CLOSED
- PRIORITY: 57
- TAGS: v0.16.0, economy

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

## Agent decisions 2026-10-17 (owner approved: unaccepted offers expire on the play-time clock, accepted jobs never expire)
- The window is authored content: `PriceTable.jobs: JobOfferRule { offer_seconds }` (`crates/nova_gameplay/src/market.rs`), required, finite and above 0, faulted by lint and load. The base table sets 1200 s (`crates/nova_authoring/src/base_content/market.rs`): longer than one round trip to the next cluster, shorter than the 1800 s restock recovery, so a board turns over before a station's stock does.
- A slot carries a `SlotPosting { posting, posted_at }` in the `JobPostings` session resource (`crates/nova_gameplay/src/jobs.rs`); `JobId` gains `posting`, so body ids read `<stem>_job_<slot>_<posting>_<part>` and a renewed slot's bodies never collide with the old posting's. `JobPostings` is saved (`WorldSaveState.postings`, format 16 -> 17) and validated at open and at save: every board is `JOBS_PER_BOARD` wide, no stamp is ahead of the clock, every held job is its slot's current posting. Completed ids are not checked: the renew can land after a same-frame snapshot.
- `post_station_boards` renews a slot the player does not hold when its job is completed (a hand-in renews at once) or its offer stood past the window. It despawns every live body the old posting could have spawned (`JobId::possible_bodies`), bumps the posting, stamps the clock and posts the board again. Frozen records of old postings stay in the ledger; a thaw spawns them and the next frame's renew despawns them. Bodies retired this frame are excluded from the wreck field, so a Distress caller despawned this frame cannot be drawn again.
- A held job pins its slot (`station_job_board` returns the held job for that slot), and no slot of any station draws a body a held job is about. A dropped job stays posted under its id until its own window passes.
- The Station pane shows `Reward N cr, expires in M min` per posted row (`offer_deadline_text`) and refuses with `accept: this offer expired` (`JobAcceptRefusalType::Expired`, checked after AlreadyDone, before TargetGone). An accept of an offer the board no longer posts, or whose slot moved on, is the same refusal instead of the former panic.
- `WorldClock` and `advance_world_clock` moved to `nova_gameplay::clock` (`crates/nova_gameplay/src/clock.rs`) because the pane reads the clock below `nova_world`; `nova_world` re-exports them (precedent `GeneratedBodyId`). `ThawedAfter` moved into `crates/nova_world/src/frozen.rs`; `crates/nova_world/src/clock.rs` is gone.
- Renewal runs only while the station's 27-cube is live and settled, in every Update frame (also under the interface pause), so an expired offer is replaced the frame after its deadline and the pane only ever shows the refusal in that one-frame race or on a stale click.

## Proof 2026-10-17
- `nova_gameplay` jobs tests (10): `a_renewed_slot_posts_a_new_offer_under_a_new_id_and_a_held_job_pins_its_slot`, `an_offer_expires_at_its_stamp_plus_the_window_and_the_accept_refuses_it` (also: a read on the posting frame has the whole window with no float residue, and residue on a whole minute does not add a minute), `postings_validate_refuses_a_stale_held_posting_and_a_stamp_ahead_of_the_clock`; market tests (12) with the `jobs.offer_seconds` fault; clock tests (2) in their new crate.
- `nova_world_base` save tests (37): `a_saved_posting_record_resumes_and_a_stale_held_posting_is_refused` (postings opened at 50 s with the clock at 200 s survive a leave save and a Load; a held job renewed behind the save's back refuses as Unreadable "not its slot's current posting").
- `nova_authoring` `base_price_table_has_no_faults`; `content gen` added `jobs: (offer_seconds: 1200.0)` to `assets/base/market/base.content.ron`; `content lint` 0 errors.
- `catalog_drift` roster: 594 invariants, the three new markers of `system_station_jobs`.
- Live (Xvfb + lavapipe, `system_station_jobs --features debug`, run 3 of 2026-10-17): all nine claims passed, `cycle complete, no panic`. Slots 0 and 1 renewed at 1241.9 s (expired) while the held procurement on slot 2 stayed pinned; after the retire, a second bump and the thaw, slots 0 and 1 renewed again at 2465.8 s and slot 2 stayed pinned; the hand-in at 2478.5 s renewed slot 2 to posting 1 with `expires in 20 min`. `station-jobs-renewed.png` shows both posted rows with `Reward 2,450 cr, expires in 20 min` and `Reward 1,092 cr, expires in 20 min`; the Your jobs row sits below the frame's fold.
- Two walk failures on the way, both resolved: (1) `posted_at + window - now` left float residue on the posting frame, which the minute ceil showed as `21 min`; fixed in `SlotPosting::remaining` and `offer_deadline_text`, covered by the assertions named above. (2) A walk that pinned an Escort and left the station lost its hauler to the raiders, which dropped the job and let the slot renew after its window: designed behaviour, so the walk now pins the procurement.
- Not run: workspace tests, Clippy. `cargo fmt --check` is clean for the change set; `nova_editor` and `nova_ship` carry pre-existing drift outside it.
