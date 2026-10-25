# Epic: regional economy, clock and restock

- STATUS: CLOSED
- PRIORITY: 59
- TAGS: v0.16.0, epic, economy

## User facts
- Stage 5 of the game expansion. Decisions 2026-10-12: authored ask and bid are reference prices; actual price = reference x scarcity x demand from content rules (D8); a play-time clock joins the save when restock lands, with a catch-up rule per frozen body type (decision 8); a station of a robbed civilization refuses service for a play-time window (decision 10).
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Economy:` title prefix and the `economy` tag.
1. Computed prices at dock time.
2. Play-time clock in the save.
3. Restock and the robbery refusal window.

## Done when
- Prices differ between clusters for a reason the player can read, a station restocks after time away, and robbing a trader costs service at that civilization's stations for a while.

## Children
- `20261012-100027` Economy: computed prices from reference, scarcity and demand
- `20261012-100028` Economy: play-time clock in the world save
- `20261012-100029` Economy: restock and the robbery refusal window

## Owner feedback 2026-10-16
- Epic direction approved as written.

## Owner feedback 2026-10-17 (UI, ideas only, nothing implemented)
Prices, the clock, restock and the refusal window are all live now but the
player reads them as sentences or not at all. Ideas:
1. Inventory trade table: a price column per side with a delta `badge` against
   the reference price (`-18%` green, `+40%` amber) so "cheap here, dear
   there" reads at a glance; the reason (scarcity, demand, cluster type,
   civilization) in a `Tooltip` on the badge, not in the inspector prose.
2. Station pane "Market" strip under "This station": the three items it pays
   most over reference and the three it sells most under, as chips, so a
   trader plans a route from the dock.
3. Restock as a line with a relative time ("restocked 12 min ago", "stock at
   70% of target" meter), read off the frozen stamp and `WorldClock`.
4. Refusal window: a `REFUSED` Amber badge with the time left on the Station
   tab itself and on the Map station mark, instead of a dock refusal the
   player finds out about at the clamp.
5. Play-time clock in the interface header (right of the pane tabs) and on
   the Load page, so deadlines and restock times have a visible reference.
6. Chart station panel: last-seen prices beside last-seen stock, with the
   play time of the dock, so the chart doubles as a price memory.

## Review 2026-10-17 (unpushed range to 0763909fe, read at HEAD)
- bug: a neutralized station or ship keeps `TraderMarket` and
  `restock_thawed_traders` (`crates/nova_world_base/src/economy.rs:36-47`) has
  no `Without<NeutralizedMarker>` / `Without<DerelictShipMarker>` filter, so a
  looted derelict refills to target on every thaw: unlimited loot. Also breaks
  the lint rule "no market on a body that does not trade"
  (`crates/nova_world/src/generation.rs:924-927`) for thawed records. Fix:
  filter the restock query, or drop `TraderMarket` on neutralize by deferred
  command (`record_neutralized_civilization` reads it on the same event).
- likely bug (low): `read_state` (`save/mod.rs:452-476`) validates stamps
  against the clock but not the clock itself, a frozen `frozen_at`, or a frozen
  `role_preference`; a hand-edited save with NaN clock, a stamp ahead of the
  clock or a zero preference opens, then panics at the first thaw or trade
  (`clock.rs:43`, `market.rs:385`). Refuse at open as `Unreadable`.
- nit: `CHANGELOG.md:48,51,54` say save format 18 and `web/src/create/stations.md:520`
  says 17; the code is 19. Name it once.
- nit: `CHANGELOG.md:164` "Older saves are refused." hangs as a second line.
- nit: restock logs a no-op as a restock when the hold has no free mass.
- Sound: price formula (no division by zero, finite, bid <= ask, saturating
  cast), clock gating and reset, restock clamps, refusal window boundary and
  admission order, save format gate, PriceTable content kind, job expiry.

## Owner feedback 2026-10-17 (round 2, market UI)
- The Inventory pane already looks decent. Keep its layout: the quantity
  slider, the text field, All, and add Buy all and Sell all.
- Add the percentages: what the station pays most for and what is most
  expensive against the reference, a few computed figures, not prose.
- Make the mock look like the real pane, not a new layout.

## Owner feedback 2026-10-17 (round 3) and next up in the game
- The Inventory pane keeps its three columns (Tender, Trader, Storage) and
  its inspector. What the game takes from the gallery is the computed
  figures: percentages against the reference and the market summary.
- Next up (game changes this epic owns):
  - `crates/nova_interface/src/inventory/app.rs`: a "vs ref" column on the
    own and partner tables (bid or ask against the item's reference price,
    green under, amber over); Reference / Pays / Asks lines in the inspector;
    Buy all and Sell all beside Buy and Sell; a Market strip under the
    columns: pays most, sells cheapest (top three each, computed in
    `crates/nova_gameplay/src/market.rs` from the price table), the restock
    line with a stock-against-target meter, the REFUSED badge with time left
    while the civilization refuses; the play clock in the pane header.
  - The two economy review bugs (`20261017-202144`, `-202147`) land first,
    after the stance lane's `economy.rs` edits.

## Progress 2026-10-17 (Inventory figures)
- Claim: the Inventory pane shows the gallery's computed figures and keeps
  its three columns and inspector. Every figure is a pure function in
  `crates/nova_gameplay/src/market.rs`; the pane only renders.
- Evidence: `crates/nova_gameplay/src/market.rs:436` `delta_percent`, `:477`
  `market_summary` (top three bids over the catalog, top three asks among
  what the trader holds), `:538` `restock_gap` (most starved targeted item,
  play seconds to full), `:579` `refusal` (window or decay rest, `None` end
  for a seeded Enemy). `crates/nova_interface/src/inventory/app.rs:1142`
  `inventory_row` price cell and `vs ref` badge, `:1874` `row_price` (own
  rows bid vs reference bid, partner rows ask vs reference ask, an owned
  station's bid), `:3637` Pays and Asks inspector lines, `:1078` Buy all and
  Sell all, `:1607` `on_trade_all_button`, `:3726`
  `update_inventory_trade_controls`.
  `crates/nova_interface/src/inventory/market.rs:107` `update_market_strip`.
  `crates/nova_interface/src/pane.rs:300` `show_pane_clock`, gated on a
  non-empty `Chart`.
- Change: `vs ref` column on the own and partner tables; Reference, Pays and
  Asks in the inspector (the Ask / bid line is gone); Buy all and Sell all
  beside Confirm; the MARKET strip under the columns (pays most, sells
  cheapest, restock meter with play minutes to full, REFUSED with minutes
  left); the play clock in the interface title row. `market_sandbox` reads
  the Pays and Asks rows; README, wiki and CHANGELOG updated.
- Omitted for lack of data: the gallery's wall-clock `Restock in 12 min`
  (restock happens only on a thaw, so the strip shows the play time to full
  instead), its prose refusal reason line, and tonnage on the restock line
  (holds count units).
- Verification: `cargo test -p nova_gameplay --lib market` 16 passed;
  `cargo test -p nova_interface --lib -- inventory pane::tests::the_title_row_clock`
  33 passed, with `a_market_trader_prices_its_rows_against_the_reference_and_sells_the_whole_stack`
  (rows 45 cr and 24 cr for the own side, 60 cr and 32 cr for the partner,
  inspector `16 cr / 12 cr`, `24 cr (+100%)`, `32 cr (+100%)`, strip leads
  and `Hull plate x1 of 2, full in 10 min of play`, Sell all moves 10 ore
  for 240 cr) and
  `the_title_row_clock_reads_the_play_time_only_while_a_world_is_armed`.
  `market_sandbox` autopilot capture: see the lane report.

## Closure check 2026-10-25
- Claim: the stated price, restock and service-refusal gates are covered by the closed children and the market, thaw and saved-clock implementations. The creator/player UI shows relative prices, restock and refusal. The original `market_sandbox` capture is recorded above; no new frame was inspected in this closure pass.
- Verification on the current shared tree: `cargo test -p nova_gameplay --lib market` (16/16), `cargo test -p nova_world_base --lib economy` (1/1), `cargo test -p nova_world_base --lib save::tests` (47/47). The closed children record their earlier restock/refusal checks. No full player robbery/restock loop was rerun here.
- Scope: the later civilization stance graph owns refusal and redemption, superseding the old robbery-only refusal-field description in child `20261012-100029`; no obsolete field is reintroduced.
