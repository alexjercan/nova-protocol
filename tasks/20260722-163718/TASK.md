# Per-beat objective pacing gaps: instruction vs reveal (mainline)

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.8.0, content, scenario, pacing, playtest

Split the single mainline pacing gap into per-beat categories so an
instructional objective lands mid-read while a reveal keeps the full absorb
beat. From an out-of-context pacing review of all four mainline scenarios.
Umbrella: 20260722-163542.

The gap values are a FEEL call - keep them as named, comms-derived constants so
the owner can tune them after playtest.

## Model (pacing.rs, all derived from comms_panel so they cannot drift)

- REVEAL_GAP = COMMS_DWELL_SECS + COMMS_FADE_OUT_SECS (8.4s; the current
  BEAT_GAP, unchanged) - line fully lands + fades, THEN the task. Threat /
  situation reveals.
- INSTRUCTION_GAP = COMMS_MIN_SECS (4s) - objective posts mid-read while the
  coaching line is still up (the line still holds its full 8s dwell, only the
  objective posts early). Coaching lines the objective echoes.
- MID_GAP = (COMMS_DWELL_SECS + COMMS_MIN_SECS) / 2 (~6s) - reveal-then-
  instruction beats.

## Steps

- [x] comms_panel.rs: make COMMS_MIN_SECS `pub` (exported via the module
  prelude, like the other COMMS_* constants) so pacing can derive
  INSTRUCTION_GAP from it.
- [x] pacing.rs: add REVEAL_GAP (= the current BEAT_GAP expression),
  INSTRUCTION_GAP (= COMMS_MIN_SECS), MID_GAP (= (dwell+min)/2). Keep a BEAT_GAP
  alias = REVEAL_GAP if convenient, or migrate call sites. Document each as
  playtest-tunable and cross-reference the comms constants.
- [x] shakedown.rs: give `stamp_gate` a `delay: f64` parameter (or a sibling)
  so each transition stamps its category's gap into VAR_GATE. Apply:
  INSTRUCTION_GAP on beats 1->2, 2->3, 3->4, 4->5, 5->6, 7->8, 8->9; MID_GAP on
  6->7 and 9->10; REVEAL_GAP on the beat-12 scavenger telegraph
  (mark_clock VAR_SCAV_GATE). The opening->obj1 seed stamp and the tight
  10->11 combat beat stay as-is.
- [x] lifeline.rs: open_gate(VAR_SCREEN_GATE, REVEAL_GAP).
- [x] broadside.rs: contact beat -> MID_GAP AND move `mark(ID_HAULER, ...)` out
  of the gated_once and onto the transition (OnStart-adjacent), so the nav
  marker exists during the distress line; ambush/defend -> REVEAL_GAP; gunship
  taunt -> REVEAL_GAP.
- [x] final_tally.rs: survey -> MID_GAP; picket -> MID_GAP; cast-off/break ->
  REVEAL_GAP.
- [x] Update shakedown pacing tests: `settle_beat` must advance past the
  LONGEST gap (REVEAL_GAP + margin) so every beat posts regardless of category;
  the not-posted-before / posted-after guards should still hold. Re-check the
  `_and_beats_breathe` gate-stamp count and any beat that changed gap.
- [x] Regenerate content: `cargo run -p nova_assets gen`, commit the changed
  assets/base/scenarios/*.content.ron in the SAME change.

## Definition of Done

- test: `cargo test -p nova_assets --lib scenario::` green.
- test: `cargo test -p nova_assets --test content_ron_parity` green (the guard
  --lib skips - run it explicitly before landing).
- cmd: `cargo fmt --check -p nova_assets -p nova_gameplay` clean;
  `cargo check -p nova_assets -p nova_gameplay` clean.
- cmd: `cargo run -p nova_assets lint` -> 0 errors.
- cmd: probe menu_newgame + lifeline + broadside all OK (loads, reached
  Playing, 0 invariant violations, 0 errors).
- manual: owner playtests shakedown - instructional objectives land as you read
  to the keypress; threat reveals still get their beat.

## Notes

- The mechanism already supports it: mark_clock/open_gate take an arbitrary
  delay; only stamp_gate bakes a constant. No new gating mechanism needed.
- Shortening beat 2->3 to INSTRUCTION_GAP also halves the crate-pickup dead
  window (pickups are guarded on setup_last==3, which latches when the objective
  posts).
- Do NOT repeat task 20260722-142341's stale-RON miss: the parity guard lives in
  tests/, not --lib.
