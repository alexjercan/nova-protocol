# ch3 overspeed: sustained ~3.5s window on the second strike (content)

- STATUS: OPEN
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

- [ ] Seed `overspeed_deadline = 0.0` in ch3 OnStart, next to `speed_warned`
      (RON ~line 107-115) - and add it to the rig's `armed_app` seed list AND
      the `on_start_seeds_*` key pin in the SAME change (the
      `seed-helper-drifts-from-source` lesson: the reviewer checks for drift).
- [ ] Rewrite the trip block (RON ~1750-1792): replace the single instant-trip
      handler with (a) the 2->3 start-countdown handler that stamps
      `overspeed_deadline`, (b) the 3->2 cancel handler, and (c) the 3->wake
      trip handler gated on the deadline. Keep the WARN (0->1) and REARM (1->2)
      handlers as-is. Match the existing Expression-filter + SetAllegiance +
      StoryMessage shape and the ~72-col comment width (the prior task took a
      NIT for an over-wide comment).
- [ ] Update the picket-wake header comment block (RON ~1677-1685) to describe
      the warn -> rearm -> countdown -> wake sequence instead of the old
      warn -> rearm -> instant-trip.
- [ ] Extend the ch3 rig `crates/nova_assets/tests/ledger_ch3_channel.rs`: drive
      `player_speed` AND the scenario clock (the rig pumps both reserved vars)
      through: (a) first breach warns, both Magpies stay Neutral, `spotted == 0`;
      (b) slow under 7, breach again -> countdown starts but pickets stay Neutral
      until the clock passes the deadline; (c) hold above 8 past `+3.5s` ->
      BOTH flip Enemy on the live Allegiance component and `spotted == 1`;
      (d) breach, then drop under 7 BEFORE the deadline -> NO wake, and a later
      held breach past a fresh deadline DOES wake (cancel + re-arm works);
      (e) once another provocation (zone/paint) has set `spotted`, the speed
      handlers are inert. Assert on the live component, not just the variable.
- [ ] Docs sweep from the final diff (the `keep-docs-in-sync` +
      `ephemeral-news-draft-drifts-behind-content` lessons): update
      `webmods/the-ledger/CHANGELOG.md` (bump `meta.version` in the bundle and
      add the entry), `README.md` if it describes the overspeed mechanic, the
      wiki version-history, and RE-READ `docs/news-*.md` the-ledger bullet
      against the new behaviour and rewrite it if it now reads stale.
- [ ] Content lint + real-loader load clean: `cargo run -p nova_assets --bin
      content -- lint webmods/the-ledger` and `cargo test -p nova_assets --test
      webmods_validation`.

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
