# Implementation notes

Branch: `fix/per-beat-pacing-gaps` (in-place, per session config). Umbrella
20260722-163542.

## What changed and why

Follow-up to 20260722-142341, which made EVERY mainline objective wait a full
comms-dwell gap (8.4s) - too rigid for instructional beats where the objective
echoes a coaching line ("Now hand her to the computer" -> "Press [G]"). An
out-of-context pacing review classified every line->objective beat; this splits
the single gap into three comms-derived, playtest-tunable categories and applies
the right one per beat.

- `pacing.rs`: `REVEAL_GAP` (8.4s, = old BEAT_GAP), `INSTRUCTION_GAP`
  (= COMMS_MIN_SECS, 4s), `MID_GAP` (~6s). `comms_panel.rs`: `COMMS_MIN_SECS`
  now `pub` (INSTRUCTION_GAP derives from it).
- `shakedown.rs`: `stamp_gate` gained a `delay` param; 8 instruction beats use
  4s, the two reveal-then-instruct beats (6->7 coast, 9->10 paint) use 6s, the
  scavenger telegraph stays 8.4s.
- `lifeline` reveal 8.4s; `broadside` contact 6s (+ marker fix, below), ambush
  8.4s, gunship 8.4s; `final_tally` survey/picket 6s, cast-off 8.4s.
- Broadside contact feel bug: the hauler nav marker was withheld inside the
  gated objective for the full gap (no HUD target for 8s). Moved
  `mark(ID_HAULER)` to OnStart (with the hauler spawn); only the objective TEXT
  waits now, matching shakedown/final_tally.

## Test meaningfulness

`settle_beat` advances past the LONGEST gap (REVEAL_GAP), so the existing walk
would pass even under a uniform gap - it does not pin the split. Added
`instruction_objectives_land_mid_read_not_after_the_full_reveal_gap`: after the
beat 1->2 transition, advancing only `INSTRUCTION_GAP` posts the objective,
while it is still absent just before it. Reverting beat 1->2 to REVEAL_GAP makes
the final assert fail (the deadline would be 8.4s out).

## Verify

- `cargo test -p nova_assets --lib scenario::` : 22 pass (incl. the new pin).
- `cargo test -p nova_assets --test content_ron_parity` : 2 pass (regenerated
  RON committed in the same change - the guard --lib skips).
- `cargo test -p nova_gameplay --lib hud::comms_panel` : 5 pass.
- fmt/check clean; `cargo run -p nova_assets lint` : 0 errors.
- probe menu_newgame: OK (loads shakedown - the biggest change - reached
  Playing, 0 invariant violations, 0 errors, 295 frames).
- probe lifeline / broadside: BOTH `reached_playing PASS` (frame 47, content
  loads and runs) then FAIL on an identical 12-line wgpu render OOM ("Caught
  rendering error: Out of Memory ... Quitting due to OutOfMemory RenderError")
  at the SAME frame across both scenarios. This is a HOST render-device limit on
  heavy combat scenes under software rendering (system RAM was 22Gi free; the
  lighter menu_newgame ran clean past frame 295; lifeline's content is
  byte-identical to before this task - a no-op 8.4->8.4). Ledger:
  `gpu-example-local-skip` (heavy render examples unreliable under lavapipe -
  one smoke attempt, then headless tests + CI). Not a regression; CI runs the
  real render path.

## Tuning

The three gap values are named constants derived from comms_panel and documented
as playtest-tunable. Owner should playtest shakedown and nudge if a value feels
off (the manual acceptance item on the umbrella).
