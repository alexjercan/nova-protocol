# SFX distance rolloff: perceptual (geometric) curve, not linear

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0,audio,polish

## Goal

User feedback on the distance attenuation (20260708-213155): the linear amplitude
rolloff feels like the sound "stays the same volume then instantly goes to zero."
Root cause: loudness perception is logarithmic, so a linear *amplitude* ramp is
perceptually flat for most of NEAR..FAR and then cliffs near the end. Replace it
with a geometric (constant-dB-per-distance) rolloff so the *perceived* fade is
even across the whole range, still reaching exactly zero at FAR.

## Steps

- [x] In `crates/nova_gameplay/src/audio.rs`, change `distance_attenuation` from
      a linear ramp to a geometric decay: `t = (d-NEAR)/(FAR-NEAR)`, factor =
      `FLOOR^t` remapped from `[FLOOR,1]` to `[0,1]` so it is 1 at NEAR and 0 at
      FAR. Add a tunable `SFX_ROLLOFF_FLOOR` constant (smaller = steeper / more
      perceived range).
- [x] Rewrite the `distance_attenuation` unit test for the new shape: near->1,
      far->0, beyond->0, monotonic decreasing, and convex (midpoint < 0.5, i.e.
      it drops below the old linear line - the whole point).
- [x] Verify: fmt, clippy --all-targets, cargo test --workspace, headless
      10_gameplay autopilot (reaches Playing, no panic). Shared CARGO_TARGET_DIR.
- [x] Update the attenuation paragraph in `tasks/20260708-162011/NOTES.md`
      (linear -> geometric/perceptual, why, and the tunable floor).

## Notes

- Depends on: 20260708-213155 (CLOSED). Pure-function tuning change; NEAR/FAR
  stay as-is (user said the far/zero distance is good).

## Outcome

Replaced the linear amplitude rolloff with a geometric (constant-dB-per-distance)
decay so the perceived volume fades evenly instead of "flat then cliff". Added a
tunable `SFX_ROLLOFF_FLOOR` (0.05). Measured factor/dB across the band: -2.8 dB at
50u, -7 at 95, -14.8 at 170, -24.6 at 245, -38.7 at 300, zero at 320 - a smooth
even fade vs the old linear line (which was only -6 dB at the midpoint). Rewrote
the unit test to assert the curve is convex (midpoint < 0.5) and in-range;
NEAR/FAR unchanged. Verified fmt, clippy --all-targets (clean), cargo test
--workspace (5 audio tests), headless 10_gameplay autopilot (Playing, no panic).
Still volume-only; stereo panning remains the documented future step. FLOOR is a
tune-by-ear knob (smaller = starts dropping sooner).
