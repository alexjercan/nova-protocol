# SFX distance rolloff: perceptual (geometric) curve, not linear

- STATUS: OPEN
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

- [ ] In `crates/nova_gameplay/src/audio.rs`, change `distance_attenuation` from
      a linear ramp to a geometric decay: `t = (d-NEAR)/(FAR-NEAR)`, factor =
      `FLOOR^t` remapped from `[FLOOR,1]` to `[0,1]` so it is 1 at NEAR and 0 at
      FAR. Add a tunable `SFX_ROLLOFF_FLOOR` constant (smaller = steeper / more
      perceived range).
- [ ] Rewrite the `distance_attenuation` unit test for the new shape: near->1,
      far->0, beyond->0, monotonic decreasing, and convex (midpoint < 0.5, i.e.
      it drops below the old linear line - the whole point).
- [ ] Verify: fmt, clippy --all-targets, cargo test --workspace, headless
      10_gameplay autopilot (reaches Playing, no panic). Shared CARGO_TARGET_DIR.
- [ ] Update the attenuation paragraph in `docs/2026-07-08-audio-sfx-system.md`
      (linear -> geometric/perceptual, why, and the tunable floor).

## Notes

- Depends on: 20260708-213155 (CLOSED). Pure-function tuning change; NEAR/FAR
  stay as-is (user said the far/zero distance is good).
