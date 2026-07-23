# NOTES - ch3 overspeed picket provocation (warn-then-trip)

## What shipped

A fifth way to wake The Ledger chapter 3's two NEUTRAL Magpie pickets: burning
too hot through the channel. Chapter 3 is a "run dark and slow" stealth run;
the four existing provocations are two picket-watch zone entries and two
combat-lock paints. This adds SPEED as the fifth, consuming the reserved
`player_speed` readout from the sibling engine task (20260723-143530).

Behaviour (user-chosen): threshold 8 u/s (the ch3 player speed cap is 25),
warn-then-trip via a 3-state machine on a new `speed_warned` variable:

- WARN (`player_speed > 8 && speed_warned == 0`): set `speed_warned = 1`, Vesh
  calls it, pickets stay asleep.
- REARM (`player_speed < 7 && speed_warned == 1`): set `speed_warned = 2`,
  silent. The 7..8 band is hysteresis so one continuous burn cannot
  warn-and-trip in consecutive frames.
- TRIP (`player_speed > 8 && speed_warned == 2`): stamp `spotted = 1`,
  `SetAllegiance` BOTH Magpies -> Enemy, Vesh's "they've got you" line.

All three OnUpdate handlers also gate `spotted == 0 && act == 1`, so they
compose one-shot with the zone/paint provocations: any wake stamps `spotted`
and disarms these, and a trip here disarms those.

### Diff surface

- `webmods/the-ledger/ledger_ch3.content.ron`: seed `speed_warned = 0` in
  OnStart (next to `spotted`); three OnUpdate handlers after the picket-wake
  block; header + picket-wake comments updated (four -> five provocations).
- `crates/nova_assets/tests/ledger_ch3_channel.rs`: `pump_speed` helper (injects
  `player_speed` exactly as `pump_clock` injects the clock); `speed_warned`
  added to `armed_app`'s seed list AND the `on_start_seeds_...` key pin
  (seed-helper-drifts-from-source); two new tests
  (`overspeed_warns_then_a_fresh_breach_wakes_both_magpies`,
  `a_prior_wake_disarms_the_overspeed_provocation`); module-doc deliverable 5b.
- Docs (keep-docs-in-sync): mod `CHANGELOG.md` 1.8.0 entry; `the-ledger.bundle.ron`
  version 1.7.0 -> 1.8.0; mod `README.md` ch3 blurb; `web/src/wiki/dev/guide-make-a-mod.md`
  version-history line; `docs/news-0.8.0-the-ledger.md` ch3 bullet (which was
  itself stale - still described the pre-stealth-rework "Magpie ambush" - so it
  was brought current to the neutral-picket design plus overspeed).

## Why this design

The reserved `player_speed` variable (engine task) let the whole feature be
PURE CONTENT: three OnUpdate handlers + Expression filters, no Rust. Warn-then-trip
with a hysteresis band (rather than an instant one-shot) was the user's call and
is the fair choice - an accidental nudge over 8 earns one warning, never an
instant blown run; only a deliberate second burn after slowing wakes them. The
7..8 gap is the load-bearing detail: without it a steady burn at ~8 u/s would
oscillate warn/rearm every frame.

## Verification

- `cargo test -p nova_assets --test ledger_ch3_channel`: 16 tests green,
  including the two new ones and the `on_start_seeds_...` pin (now requires
  `speed_warned`). The new tests drive the REAL handlers (loaded from the
  shipped RON via `include_str!`) and assert on the live `Allegiance` COMPONENT,
  covering: first breach warns only (both Neutral, spotted 0); a continuous burn
  above 8 never trips (rearm gate); slow-under-7 then re-breach flips both Enemy
  and stamps spotted; a prior zone wake leaves the speed handlers inert.
- `content lint --target webmods/the-ledger`: 0 errors / 0 warnings / 0 findings
  (the 1 acked is the pre-existing unrelated ch4 Auditor ack) - `player_speed`
  reads clean, proving the sibling task's lint exception end to end.
- `webmods_validation`, `content_lint_gate`, `content_report_gate`,
  `balance_audit_gate`: all green - ch3 loads through the real modding loader
  and the shipped-content gates still pass.
- fmt clean.

Fail-first is inherent: the content handlers ARE the mechanism under test; with
them absent the first assertion (`speed_warned == 1` after `pump_speed(9)`)
would read 0 and trip.

Probe: SKIPPED (not measured). No autopilot example plays this mod scenario -
ch3 is a stealth run driven through the modding loader by deliberate slow
piloting; an autopilot would fly a normal profile and the run report would not
exercise the creep-vs-overspeed decision. The behaviour proof is the
production-faithful rig above plus the manual playtest batched to Finish.

## Reflection

Smooth - the sibling engine task had already de-risked the hard part (exposing
speed), so this was authoring against a proven readout. The one thing worth
flagging for future ledger work: the `docs/news-0.8.0-the-ledger.md` ch3 bullet
was stale (pre-stealth-rework) before I touched it - a reminder that ephemeral
release-note drafts drift behind the content and need a re-read against the
current RON, not just an append, whenever a chapter changes.
