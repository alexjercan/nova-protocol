# REVIEW - ch3 overspeed picket provocation (warn-then-trip)

- Round 1
- Reviewer: out-of-context code reviewer (fresh, skeptical)
- Date: 2026-07-23
- Branch: `feat/ch3-speed-provocation` (single commit `954a56c5`)

## Scope reviewed

- `webmods/the-ledger/ledger_ch3.content.ron` - OnStart seed + three OnUpdate handlers (WARN/REARM/TRIP)
- `crates/nova_assets/tests/ledger_ch3_channel.rs` - two new tests, `pump_speed` helper, seed-pin updates
- Docs: mod CHANGELOG, bundle version, mod README, dev-guide wiki, `docs/news-0.8.0-the-ledger.md`
- Verified against the real dispatch model in `bevy_common_systems` `src/modding/events.rs::queue_system`

## State-machine soundness (traced against real pulse semantics)

I read the actual handler dispatch (`queue_system`, events.rs:376-402). For a
single `OnUpdateEvent` fire, the registered handlers for that name are walked in
index/registration order and each passing handler's actions mutate the shared
`world` IMMEDIATELY - so a `VariableSet` in one handler IS visible to a later
handler in the SAME pulse. Registration order is RON order: WARN, REARM, TRIP.
I traced every hazard the task called out:

- Continuous cold burn (speed jumps to 12, never drops): pulse 1 WARN sets
  `speed_warned=1`; REARM needs `<7` (fail), TRIP needs `==2` (fail). All later
  pulses: WARN needs `==0` (fail). Never trips. CORRECT - the hysteresis is
  load-bearing and holds.
- Same-pulse WARN->TRIP chain: impossible. WARN leaves `speed_warned=1`, TRIP
  requires `==2`. The only 1->2 transition is REARM, which requires `<7` in the
  same pulse where TRIP requires `>8` - mutually exclusive for one speed value.
  No same-pulse chain exists.
- Rearm band 7..8: correct and necessary. WARN `>8`, REARM `<7`; a value in
  [7,8] neither re-warns nor rearms, so a steady burn at ~8 cannot oscillate.
- Ordering hazard between the three: none. No two of them can pass in the same
  pulse (each pair is separated by a contradictory `speed_warned` or a
  contradictory speed comparison).

The state machine is sound.

## One-shot composition

All three new handlers gate `spotted == 0 && act == 1`, byte-identical to the
four existing zone/paint wake handlers (compared against the OnEnter handler at
ledger_ch3.content.ron:1528-1563). A wake from any provocation stamps
`spotted=1`, which disqualifies all five (including these three) on the next
pulse; the TRIP handler in turn stamps `spotted=1` and disarms the zone/paint
ones. The TRIP action shape (VariableSet spotted=1 + two SetAllegiance ->
Enemy + Vesh StoryMessage dwell 7.0) mirrors the zone-wake exactly. Faithful.

## OnStart seeding

`speed_warned` is seeded to `Number(0.0)` in OnStart next to `spotted`
(ledger_ch3.content.ron, diff line ~105-110). Seeded, and seeded to 0 - no
fail-closed-forever risk.

## Rig pin integrity

Both drift points updated:
- `armed_app()` seed list gains `("speed_warned", 0.0)` (rig line 351).
- `on_start_seeds_the_sequencer_and_spawns_the_cast` required-keys list gains
  `"speed_warned"` (rig line 381), so the pin fails if the RON drops the seed.
No helper/source drift.

## Test quality

The two new tests drive the REAL handlers (loaded from the shipped RON via
`scenario_from(CH3_RON)` + `register_non_start_handlers`) and assert on the live
`Allegiance` COMPONENT via `ship_allegiance` (not a rig variable). Coverage:
- first-breach-warns-only: `speed_warned==1`, both Neutral, `spotted==0`.
- continuous-burn-does-not-trip: holds 12 u/s x3, asserts `speed_warned` stays 1
  and `spotted==0` - this fails if TRIP were mis-gated (e.g. dropped the `==2`
  guard).
- rearm-then-trip: slow to 6 arms (->2), fresh 9 trips - both Magpies flip Enemy
  on the live component, `spotted==1`.
- prior-wake-disarms: a zone entry wakes them, then a 20 u/s burn leaves
  `speed_warned==0` and no re-flip.
Fail-first is genuine: the content handlers are the mechanism, so a wrong gate
flips a live assertion. `pump_speed` correctly fires OnUpdate (rig line 177-178
fires `OnUpdateEvent` every Update); its double-`update()` is safe (the second
pulse re-evaluates against the now-latched `speed_warned` and does nothing).

Verified: `cargo test -p nova_assets --test ledger_ch3_channel` -> 16 passed.

## RON validity and thresholds

`cargo run -p nova_assets --bin content -- lint --target webmods/the-ledger` ->
0 errors / 0 warnings / 0 findings, 5 scenarios balance-audited, 1 acked (the
pre-existing unrelated ch4 Auditor ack). `player_speed` reads clean, confirming
the sibling task's lint exception end to end. `GreaterThan`/`LessThan`/`Equal`
are the right comparisons for `>8`/`<7`/`==`. Threshold 8 u/s is sane against
the ch3 player `speed_cap: Some(25.0)` (line 146) - 32% of cap, a deliberate hot
burn rather than an accidental drift.

## Docs

Every surface that describes ch3's provocations or the mod version was updated:
- CHANGELOG 1.8.0 entry (correct warn-then-trip prose, drawn from the diff).
- bundle `version` 1.7.0 -> 1.8.0.
- README ch3 blurb (three -> including overspeed, "warns first").
- dev-guide wiki version-history line 1.7.0 -> 1.8.0.
- `docs/news-0.8.0-the-ledger.md` ch3 bullet - notably the author found and
  fixed a stale pre-stealth-rework "Magpie ambush" description, bringing it to
  the neutral-picket + overspeed design.
Grepped `web/src/wiki/scenarios.md`, `web/src/news/0.7.0.md`, `mod-portal.md`:
the remaining Magpie/picket mentions are a DIFFERENT campaign (Broadside/
Lifeline/Final Tally) and historical 0.7.0 chapter-two ship references - not
stale ch3-Quiet-Channel surfaces. The only lingering `1.7.0` is the historical
CHANGELOG heading (correct). No stale mentions.

## Findings

- NIT - ledger_ch3.content.ron:7 (header comment) and the block comment at
  ~1518: the long run-on line ("...(task 20260723-143603)), and a debris
  pinch...") pushes past the file's usual comment width and reads a bit
  breathless. Purely cosmetic; the surrounding comments are already dense.
  Suggested change (optional): rewrap the inserted clause to match the
  ~72-col comment wrapping used elsewhere in the header. No behavioral impact.

No BLOCKER, MAJOR, or MINOR findings. The state machine is correct under the
real pulse semantics, the one-shot composition is faithful to the existing four
provocations, the tests exercise the shipped handlers and would fail on a wrong
gate, lint is clean, and every doc surface is in sync.

VERDICT: APPROVE
