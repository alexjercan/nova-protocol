# ch3 overspeed: sustained ~3.5s window on the second strike (content)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.8.0, content, scenario, playtest

## Story

Umbrella 20260723-182811. Chapter 3's overspeed "second strike" (the trip that
wakes both Magpie pickets) currently fires the instant `player_speed > 8` once
armed - zero reaction time. The player asked for a real ~3-4s grace on that
second strike: after the warning, gunning it again should give you a few
seconds to realise and ease off before the pickets wake. The first strike
(warn) stays instant - it is harmless and immediate feedback is good; only the
consequence (the trip) gets the sustained window.

Today's handlers live in `webmods/the-ledger/ledger_ch3.content.ron` (lines
~1674-1792), a 3-state machine on `speed_warned` (0 unwarned -> 1 warned ->
2 armed -> instant trip). This task inserts a countdown between "armed" and
"wake", using the same `scenario_elapsed`-deadline idiom the chapter already
uses for `beat_gate` (line ~1818: `Add(Factor(Name("scenario_elapsed")),
Term(Factor(Literal(Number(5.0)))))`).

## Design (confirmed with the user: "sustained timer on 2nd strike")

Extend `speed_warned` to a 4th state and add an `overspeed_deadline` variable.
All handlers stay gated `spotted == 0 && act == 1` so they still compose
one-shot with the four zone/paint provocations.

- 0 -> 1 (WARN, unchanged): `player_speed > 8 && speed_warned == 0`. Set
  `speed_warned = 1`, fire Vesh's warning line, do NOT wake. Instant.
- 1 -> 2 (REARM, unchanged): `player_speed < 7 && speed_warned == 1`. Set
  `speed_warned = 2`. Silent hysteresis.
- 2 -> 3 (START COUNTDOWN, new): `player_speed > 8 && speed_warned == 2`. Set
  `speed_warned = 3` and stamp `overspeed_deadline = scenario_elapsed + 3.5`.
  Optional: a short Vesh "they're onto that burn - kill it NOW" beat so the
  countdown is telegraphed (author the exact line; keep it a warning, not the
  wake). Does NOT wake.
- 3 -> 2 (CANCEL, new): `player_speed < 7 && speed_warned == 3`. Player eased
  off in time: set `speed_warned = 2` (re-armed, silent), so a later fresh
  breach starts a new countdown. The run stays dark. (Cancel on `< 7`, the
  rearm band, not `< 8`, to avoid frame flip-flop right at the limit.)
- 3 -> WAKE (TRIP, new): `speed_warned == 3 && scenario_elapsed >
  overspeed_deadline && player_speed > 8`. Held above the limit for the full
  window: set `spotted = 1`, `SetAllegiance` BOTH Magpies -> Enemy, fire Vesh's
  "too hot too long - they've got you" line. This is the real 5th provocation.

Notes on correctness:
- Seed `overspeed_deadline = 0` in OnStart next to `speed_warned` (undefined-
  variable rule; the deferred gate must read a defined value before any breach).
- The cancel gate (`< 7`) and the trip gate (`> 8`) are mutually exclusive in a
  frame, so no same-frame cancel-and-trip regardless of handler order (same
  robustness argument the original 1->2 vs trip split relied on - see the prior
  content task RETRO 20260723-143603).
- Pick 3.5 (squarely in the user's "3-4s"); keep it a single literal so it is
  easy to retune. Author the exact hysteresis/countdown numbers against the
  shipped 25 u/s cap.

## Steps

- [x] Seed `overspeed_deadline = 0.0` in ch3 OnStart, next to `speed_warned`
      - and added it to the rig's `armed_app` seed list AND the
      `on_start_seeds_*` key pin in the SAME change (no drift).
- [x] Rewrite the trip block: replaced the single instant-trip handler with
      (a) the 2->3 start-countdown handler that stamps `overspeed_deadline =
      scenario_elapsed + 3.5` (same `Add(...)` idiom as beat_gate) and fires a
      last-chance Vesh shout, (b) the 3->2 cancel handler (< 7 rearm band), and
      (c) the 3->wake trip handler gated on `scenario_elapsed >
      overspeed_deadline`. WARN (0->1) and REARM (1->2) kept as-is.
- [x] Update the picket-wake header comment block to describe the
      warn -> rearm -> countdown -> wake sequence (0 -> 1 -> 2 -> 3 state).
- [x] Extend the ch3 rig `crates/nova_assets/tests/ledger_ch3_channel.rs`:
      renamed the trip test to `overspeed_warns_then_a_held_breach_wakes_both_magpies_after_the_window`
      (warn -> continuous-burn-never-arms -> rearm -> countdown-starts-not-trips
      -> before-deadline-dark -> held-past-deadline-wakes, asserting the live
      Allegiance component + overspeed_deadline == 3.5), and added
      `easing_off_during_the_countdown_cancels_the_wake` (cancel + re-arm +
      later held breach still wakes). The already-spotted-disarms test is
      unchanged and still passes.
- [x] Docs sweep from the final diff: bundle `meta.version` 1.8.0 -> 1.9.0;
      CHANGELOG 1.9.0 entry (1.8.0 entry left verbatim as dated history);
      README overspeed line; `docs/news-0.8.0-the-ledger.md` ch3 bullet; the mod
      guide's illustrative "The Ledger 1.0.0 -> 1.9.0" version walk.
- [x] Content lint + real-loader load clean: `cargo run -p nova_assets --bin
      content -- lint --target webmods/the-ledger` (0 err/warn/finding, 1
      pre-existing ack) and `cargo test -p nova_assets --test webmods_validation`.

## Definition of Done

- test: `cargo test -p nova_assets --test ledger_ch3_channel` - the new
  warn -> rearm -> countdown -> wake pin passes, including the "eased off in
  time = no wake" cancel case, the "held past the deadline = wake" case, and the
  "already-spotted disarms speed" case.
- cmd: `cargo run -p nova_assets --bin content -- lint webmods/the-ledger` is
  clean; `cargo test -p nova_assets --test webmods_validation` loads ch3.
- cmd: the full check suite the CI runs (`cargo test`, fmt, check) is green.
- manual: playtest ch3 - warn once, gun it again and confirm you get ~3.5s to
  ease off before both Magpies wake; hold the burn and confirm they wake.

## Notes

- Do NOT run the full local suite / clippy (memory: skip-local-tests-and-clippy)
  - run fmt/check + the two tests above; CI runs the rest.
- The `beat_gate` deadline idiom to mirror is at RON ~1816-1819. The rig's
  clock-pump idiom (how it advances `scenario_elapsed`) is already used by the
  existing breather assertions in `ledger_ch3_channel.rs`.
- Bump the bundle version once here; the sibling ch5 task (20260723-182855)
  will bump again on top. Sequential landing (this task first) avoids a version
  clash.

## Outcome (2026-07-23)

CLOSED. Pure-content change to ch3 (no engine work), plus the rig and docs.

**What changed and why.** The overspeed "second strike" was frame-instant: once
armed (`speed_warned == 2`), the next `player_speed > 8` tripped the same tick.
The user wanted a real ~3-4s reaction window on that strike. Added a fourth
state (`speed_warned == 3`, counting down) and an `overspeed_deadline` variable:
the armed breach now STARTS a 3.5s window (stamps `overspeed_deadline =
scenario_elapsed + 3.5`, same `Add(...)` idiom the chapter uses for `beat_gate`)
and fires a last-chance Vesh shout instead of waking; the wake fires only when
`scenario_elapsed > overspeed_deadline` with the burn still hot; a `player_speed
< 7` handler cancels the countdown back to armed if the player eases off in
time. The WARN (0->1) and REARM (1->2) handlers are untouched.

**Alternatives considered.** (1) A new engine `OnSpeedHeld`/timer event -
rejected, the `scenario_elapsed`-deadline idiom already exists and is pure
content. (2) Cancelling on `< 8` instead of `< 7` - rejected, a burn hovering
right at the limit would flip-flop cancel/re-arm every frame; reusing the 7..8
rearm band keeps the hysteresis. (3) Giving the first WARN a sustained window
too - the user said "at least the second strike" and an instant warning is good
immediate feedback with no consequence, so the warn stays instant.

**Correctness note.** The cancel gate (`< 7`) and the trip gate (`> 8`) are
mutually exclusive within any frame, so there is no order-dependent same-frame
cancel-and-trip regardless of handler evaluation order - the same robustness
argument the original rearm-vs-trip split relied on.

**Difficulties.** The DoD's lint command was copied from the prior task with the
wrong syntax (`lint webmods/the-ledger`); the CLI takes `--target
webmods/the-ledger`. Fixed the DoD text and used the correct form. No other
friction: TDD caught the behaviour change cleanly (the renamed trip test went
red for exactly the right reason against the instant-trip RON, then green).

**Verification.** `cargo test -p nova_assets --test ledger_ch3_channel` 17/17
green (incl. the rewritten held-breach test and the new cancel test); `content
lint --target webmods/the-ledger` 0 err/warn/finding (1 pre-existing Auditor
ack); `webmods_validation` loads; `cargo fmt --check -p nova_assets` clean.

**Self-reflection.** Went smoothly. Next time, sanity-check inherited DoD
command syntax before trusting it. The rig's independent speed/clock seeds (the
tracker is not run in-rig, so `player_speed` persists across `pump_clock`) made
the sustained-window test straightforward - worth remembering that pattern for
the ch5 finale rig.

Manual playtest (creep/warn/hold-vs-ease-off) stays pending for the umbrella's
Finish checkpoint.
