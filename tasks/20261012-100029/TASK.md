# Economy: restock and the robbery refusal window

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.16.0,economy

## User facts
- Epic `20261012-100025`. Restock refills a station's stock toward its targets by elapsed play time at thaw. Decision 10 (2026-10-12): a station of a civilization whose hull the player neutralized refuses service for a play-time window; when the stance graph lands this becomes an edge flip.

## Delivery gate
- Restock rate and the refusal window are content constants with a reason; the refusal is a visible HUD line at admission. Record the neutralized-hull civilization on the player save, not on the station.

## Verification
- Asserted examples: leave with empty stock, return after the window, stock is back; neutralize a hull, its civilization's station refuses, the window passes, it admits.

## Done when
- Both proofs are permanent and the design page's D8 pitfall line matches the code.

## Agent decisions 2026-10-16
- The restock rate and the refusal window are two required `PriceTable` fields, `restock: RestockRule { recovery_seconds }` and `robbery: RobberyRule { refusal_seconds }` (`crates/nova_gameplay/src/market.rs`), authored in `crates/nova_authoring/src/base_content/market.rs` as 1800 s and 900 s of play with their reasons. Lint and load refuse a non-finite or non-positive value.
- `restocked_count` moves a thawed trader's count toward its target by `target * elapsed / recovery` whole units, never past the target, in both directions; items the target does not name are untouched; a refill adds only what the hold's free mass takes.
- The consumer is `restock_thawed_traders` (`crates/nova_world_base/src/economy.rs`), keyed on the thawed hold landing (`Added<ShipInventory>` beside `ThawedAfter`), so a held or fresh body never restocks and a Load restocks only by the saved clock.
- `CivilizationId` moved from `nova_world` to `nova_gameplay::civilization` and sits on `TraderMarket`, because docking admission in `nova_ship` cannot see `nova_world`. No reputation and no stance graph: the record is `NeutralizedCivilizations` (civilization -> play-time stamp of the latest neutralize) on the player's save (`WorldSaveState.neutralized`, format 15 -> 16), cleared with the ledger, validated on open and on save (no stamp ahead of the clock).
- Attribution is `LastAttacker` (`crates/nova_ship/src/input/ai/threat.rs`): the ship root that dealt the last attributed hit, resolved the way threat tracking resolves it. A hull records its civilization when `NeutralizedMarker` lands while its last attacker is the player's ship or a fleet escort; a thawed already-neutralized hull records nothing. A living station counts as a hull.
- `DockRefusalType::NeutralizedHull` is checked after `EnemyStation` and before `HostileStation`, only for the player's ship, never at an owned or dead station; the HUD line is `<NAME> REFUSES DOCKING: HULL NEUTRALIZED`. `forget_served_refusals` drops an entry once the window passed, so the record never grows past the live refusals.
- The `TODO(20261012-100028)` on owned-station income was removed: no income rule is approved, the docs now say an owned station earns nothing.

## Proof 2026-10-16
- Unit: `a_thawed_stock_drifts_toward_its_target_by_the_elapsed_play_time`, `the_restock_and_refusal_clocks_must_be_positive_seconds` (`crates/nova_gameplay/src/market.rs`); `a_record_refuses_until_its_stamp_falls_behind_the_cutoff` (`crates/nova_gameplay/src/civilization.rs`); `a_station_of_a_neutralized_civilization_refuses_the_player_within_the_window` (`crates/nova_ship/src/sections/docking_section/tests.rs`); `a_saved_neutralized_record_resumes_and_a_stamp_ahead_of_the_clock_is_refused` (`crates/nova_world_base/src/save/tests.rs`). The 13 market and civilization tests, 45 docking tests and 36 save tests pass.
- Live `system_station_restock` (Xvfb + lavapipe, seed 6): the station in cell (2,0,0) had PdcRound (its largest target, 162) set to 0 and CapacitorBank (untargeted) to 2; the cell retired at 0, the clock was advanced half a recovery (900 s), the thaw read `ThawedAfter` 912.8 s and restocked PdcRound 0 -> 82 of 162 (`restocked_count` of 912.8 s) with CapacitorBank still 2; emptied again, a zero-elapsed freeze/thaw left it at 0; after a reset to 0, a retire, a whole recovery (1800 s) on the clock, a leave save (generation 6, format 16, stock 0 on disk) and a Load through the menu, the thaw restocked it to 162 over the saved 1813.5 s. `autopilot: cycle complete, no panic (t=126.4s)`.
- Live `system_station_refusal` (Xvfb + lavapipe, seed 10): the player docked at 'Fenhalka station', undocked, neutralized 'Fenhalka civilian' (a separate hull) with damage sourced from the player's root; the record stamped Fenhalka at 37.8 s; the HUD read `Fenhalka station REFUSES DOCKING: HULL NEUTRALIZED` (screenshot inspected: orange line beside the velocity chip, DOCK chip reads DOCK REFUSED); a berth attempt was refused with `NeutralizedHull`; the leave save (generation 3, format 16) still refused; after a Load through the menu the record and the HUD line held; with the clock set to 938.8 s the record emptied at 939.8 s and the player docked again. `autopilot: cycle complete, no panic (t=71.2s)`.
- `cargo check --workspace --tests` and `cargo check -p nova-protocol --examples --features debug` clean; `catalog_drift` roster test passes with the two ranges (591 invariants); `content gen` regenerated `assets/base/market/base.content.ron` and `content lint` reports 0 errors.

