# Review: Audio/SFX system

- TASK: 20260708-162011
- BRANCH: feature/audio-sfx-system

## Round 1

- VERDICT: APPROVE

The branch delivers the Goal: five cues play off real gameplay seams through the
reusable bcs `SfxPlugin`/`SoundBank`, placeholders are generated + committed, the
wav decoder is enabled correctly (on a normal dep, not the dev-only root bevy),
wasm is covered by the existing copy-dir, and the crate-tier boundary is
respected (only the event->sound map is game-specific). Independently ran
`cargo fmt --check` (clean), `cargo clippy --all-targets` (clean), and the audio
unit tests (4 pass). The three unit tests are meaningful (throttle semantics,
engine-volume mapping, key/file drift guard) and no existing test was weakened.
Deviations from the plan (observer seams over editing weapon systems;
`SoundBank::load` over the gated collection) are sound and documented.

Findings are all non-blocking and left to the implementer's discretion; given
this is a "feel & juice" release, R1.1 and R1.2 are worth a cheap follow-up.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/audio.rs:139 - the explosion cue is
  the one one-shot with no throttle, yet it is the most likely to stack: when a
  multi-section ship dies, every section gets `IntegrityDestroyMarker` in the
  same frame, so N explosions play at once (volume 0.6 each) and can clip. This
  is exactly the storm the turret/impact throttles guard against. Suggest either
  a short explosion min-interval in `SfxThrottle` (consistent with the other
  cues) or an explicit decision to accept a louder "ship died" stack, documented.
  - Response: fixed in follow-up commit. Added `last_explosion` to `SfxThrottle`
    and an `EXPLOSION_MIN_INTERVAL` of 0.06s, applied in
    `on_destroyed_play_explosion` the same way as turret/impact. A multi-section
    death now sounds one explosion; distinct kills >60ms apart still each sound.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/audio.rs:105 - `engine_volume` clamps
  the *sum* of per-thruster inputs to 1.0, but `ThrusterSectionInput` is a 0..1
  throttle per thruster (`thruster_impulse_system` does `input.clamp(0.0, 1.0)`),
  and a ship maneuvers with several thrusters at once. So the sum is >= 1
  whenever more than one fires, pinning the hum to max and making it effectively
  binary - the exponential smoothing then mostly smooths on/off transitions, not
  throttle level. Suggest averaging over the active thrusters (sum / count) so
  the hum tracks throttle proportionally.
  - Response: fixed in follow-up commit. `update_thruster_loop_volume` now
    averages the throttle over the active thrusters (sum / count) and
    `engine_volume` maps that 0..1 average, so the hum tracks burn intensity
    instead of pinning to max. Doc comments updated.
- [x] R1.3 (NIT) crates/nova_assets/src/lib.rs:register_sounds - inserts a fresh
  `SoundBank` on every `OnEnter(GameAssetsStates::Processing)`; on a scene reload
  that re-enters Processing this drops and reloads the handles. Harmless today
  (Processing is entered once), but a `resource_exists` guard or load-once would
  avoid the churn if scene reloads become common.
  - Response: left as-is (NIT). Processing is entered once in practice and the
    reload churn is harmless; a guard can be added if scene reloads become common.

## Round 2

- VERDICT: APPROVE

Verified the two fixes against the new diff:

- R1.1: `EXPLOSION_MIN_INTERVAL` (0.06s) + `last_explosion` in `SfxThrottle`,
  applied in `on_destroyed_play_explosion` - matches the turret/impact pattern.
  Confirmed. [x]
- R1.2: `update_thruster_loop_volume` folds to `(sum, count)` and feeds
  `sum / count` to `engine_volume`, whose semantics/doc are now "average
  throttle". Confirmed. [x]
- R1.3: accepted as a NIT, left for later. [x]

Re-ran `cargo fmt --check` (clean), `cargo clippy --all-targets` (clean), the
audio unit tests (4 pass, including the extended default-throttle test), and a
headless `BCS_AUTOPILOT=1 10_gameplay` run (reached Playing, cycle complete, no
panic, no sound asset errors). No new findings. Ready to merge.
