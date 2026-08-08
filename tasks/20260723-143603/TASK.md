# ch3 speed: warn-then-trip picket provocation on player overspeed (content)

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.8.0, content, scenario, playtest

## Story

Add the 5th picket provocation to The Ledger chapter 3
(`webmods/the-ledger/ledger_ch3.content.ron`): burning too hot while sneaking
wakes both NEUTRAL Magpie pickets. Chapter 3 is a "run dark and slow" stealth
run; the existing four provocations are two picket-watch zone entries and two
combat-lock paints, all one-shot on `spotted == 0 && act == 1`. This adds
speed as the fifth, consuming the reserved `player_speed` variable from the
sibling engine task (umbrella 20260723-143503; depends on 20260723-143530).

Behavior (confirmed with the user): threshold 8 u/s (cap is 25), WARN then
trip. Implement as a 3-state machine on a new `speed_warned` variable, all
handlers gated `spotted == 0 && act == 1` so they compose one-shot with the
existing provocations (any wake stamps `spotted = 1` and disarms the rest;
these speed handlers likewise disarm the zone/paint ones):

- state 0 -> 1 (WARN): `player_speed > 8 && speed_warned == 0`. Set
  `speed_warned = 1`, fire a Vesh warning line, do NOT wake.
- state 1 -> 2 (REARM): `player_speed < 7 && speed_warned == 1`. Set
  `speed_warned = 2` (armed). Silent - a hysteresis band (7..8) so one
  continuous burn cannot warn-and-trip in consecutive frames.
- state 2 (TRIP): `player_speed > 8 && speed_warned == 2`. Set `spotted = 1`,
  `SetAllegiance` BOTH Magpies -> Enemy, fire Vesh's "they've got you" line.
  This is the real 5th provocation.

Author the exact hysteresis numbers against the shipped speed cap; the pin
below encodes them as rig assertions.

## Steps

- [x] Seed `speed_warned = 0` in ch3's OnStart, next to `spotted`/`act` (so it
      is defined before any gate reads it - the undefined-variable rule).
- [x] Add the three OnUpdate handlers above (WARN / REARM / TRIP) after the
      existing picket-wake block (ch3 RON ~line 1505-1657), matching the
      existing Expression-filter + SetAllegiance + StoryMessage shape. Vesh
      voice: warning = "ease off the throttle, they'll hear that hull sing";
      trip = "too hot again - they've got you, both pickets going hot".
- [x] Extend the ch3 rig `crates/nova_assets/tests/ledger_ch3_channel.rs`:
      drive `player_speed` (inject the reserved variable the way the rig pumps
      the scenario clock) through the warn -> slow -> breach sequence and
      assert: (a) first breach warns but leaves BOTH Magpies Neutral and
      `spotted == 0`; (b) a continuous hold above 8 does NOT trip (rearm gate);
      (c) slow below 7 then breach again flips BOTH to Enemy on the live
      Allegiance component and stamps `spotted == 1`; (d) once any other
      provocation (zone/paint) has set `spotted`, the speed handlers are inert.
- [x] Update the picket-wake header comment in the RON and the ch3 test's
      module doc to list five provocations, and add a line to
      `webmods/the-ledger/CHANGELOG.md` / `README.md` describing the speed
      trip (write the prose from the final diff, per the ledger lesson).
- [x] Content lint clean: `cargo run -p nova_assets --bin content -- lint
      webmods/the-ledger` (expect no undefined-variable flag for `player_speed`
      - guaranteed by the sibling task's lint exception) and the
      webmods_validation load.

## Definition of Done

- test: `cargo test -p nova_assets --test ledger_ch3_channel` - the new
  warn -> rearm -> trip pin passes, incl. the "continuous burn does not trip"
  and "already-spotted disarms speed" cases.
- cmd: `cargo run -p nova_assets --bin content -- lint webmods/the-ledger` is
  clean; ch3 still loads through the real modding loader
  (`cargo test -p nova_assets --test webmods_validation`).
- manual: playtest ch3 - creep the channel under 8 u/s and slip past unseen;
  gun it, hear Vesh's warning, gun it again and confirm both Magpies go hot.

## Notes

Existing pattern to mirror: the four picket-wake handlers at
`webmods/the-ledger/ledger_ch3.content.ron:1505-1657` (Expression gate on
`spotted == 0` + `act == 1`, `VariableSet spotted = 1`, two `SetAllegiance ->
Enemy`, a Vesh `StoryMessage`). OnStart var seeding is near the top of the
file. The rig's clock-pump idiom shows how to inject a reserved engine
variable frame-to-frame. DEPENDS ON 20260723-143530 (the `player_speed`
variable must exist and be lint-exempt first).
