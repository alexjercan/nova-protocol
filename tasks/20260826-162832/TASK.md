# Loop capture takes a LoopProfile: resolution, fps, quality, frame cap

- STATUS: CLOSED
- PRIORITY: 35
- TAGS: v0.12.0, autopilot, tooling, capture

## Outcome

Add a public, typed `LoopProfile` to Nova's loop capture system so production consumers can choose the real capture-window resolution, encoded resolution, frame rate, quality, and frame cap without copying the recorder.

The motivating downstream is `nova-showcase`. It produces 9:16 shorts and needs a native portrait source:

```rust
LoopCapturePlugin::new(LoopProfile {
    window_resolution: (1080, 1920),
    output_resolution: (1080, 1920),
    fps: 60,
    crf: 18,
    frame_cap: 1200,
})
```

Exact field types and constructor details may follow Nova conventions. Keep the API small and explicit.

## Current v0.11 evidence

The current implementation is in:

- `crates/nova_autopilot/src/loops.rs`
- `crates/nova_autopilot/src/capture.rs`
- `crates/nova_debug/src/harness.rs`

It hardcodes:

```text
CAPTURE_RESOLUTION = 1920x1080
LOOP_RESOLUTION = 1280x720
LOOP_FPS = 30
LOOP_CRF = 34
LOOP_FRAME_CAP = 600
```

`LoopCapturePlugin` pins `TimeUpdateStrategy::ManualDuration` from `LOOP_FPS`. The encoder scales PNG readbacks to `LOOP_RESOLUTION` and invokes libvpx-vp9 with `LOOP_CRF`. `force_capture_resolution` separately pins the primary window to `CAPTURE_RESOLUTION`.

This makes the current 9:16 high export enlarge the central 405x720 portion of a 720p WebM to 1080x1920. A 3.7-second source measured only 362 KB at about 783 kbps. Configuring only the encoder is insufficient: the profile must also control the actual primary-window resolution and deterministic frame clock.

## Required behavior

1. Add a public `LoopProfile` owned by the loop-capture subsystem.
2. Make `LoopCapturePlugin` accept a profile, preferably through `LoopCapturePlugin::new(profile)`.
3. Preserve a default construction path whose behavior exactly matches the existing documentation-loop defaults.
4. Use the profile's FPS for both:
   - `TimeUpdateStrategy::ManualDuration` on armed runs.
   - FFmpeg input/output cadence.
5. Use the profile's window resolution for the primary capture window.
6. Use the profile's output resolution in the FFmpeg scale step. Equal window and output resolutions must avoid accidental downscaling.
7. Use the profile's CRF and frame cap in encoding and failure checks.
8. Keep `loop_start`, `loop_end`, and `loop_written` compatible with the existing action-based Autopilot timeline.
9. Keep the unarmed smoke path free of recording writes and manual-time changes.
10. Preserve explicit failure behavior for missing plugins, invalid state transitions, frame-cap overflow, screenshot failure, and FFmpeg failure.
11. Export the profile through the same public preludes/harness surface used by capture examples.

## Window-resolution integration

Resolve the current split between `LoopCapturePlugin` and `force_capture_resolution` deliberately. Existing examples register `force_capture_resolution` as a Startup system.

A reasonable design is:

- Insert the selected `LoopProfile` resource even when capture is unarmed.
- Let `force_capture_resolution` read an optional profile and otherwise use the existing `CAPTURE_RESOLUTION` default.
- Ensure a configured profile wins deterministically and does not race another Startup system.

A different simple design is acceptable if existing default examples remain clear and production callers need only one obvious source of capture settings.

## Validation

Validate profile invariants early with actionable errors. At minimum, dimensions, FPS, and frame cap must be nonzero. Keep validation proportional; do not impose arbitrary production limits.

## Tests and proof

Add focused tests for behavior rather than implementation details:

- Default profile reproduces 1920x1080 capture, 1280x720 output, 30 fps, CRF 34, and 600 frames.
- A portrait profile produces FFmpeg arguments for 1080x1920, 60 fps, and CRF 18.
- Manual frame duration comes from the selected FPS.
- Frame-cap enforcement uses the selected cap.
- Invalid zero-valued settings fail clearly.
- Unarmed capture remains a no-op.
- Existing loop examples compile against the default API.

Run only affected checks per `AGENTS.md`. If practical, arm one short portrait capture and use `ffprobe` to prove 1080x1920 at 60 fps. Do not commit generated media.

## Scope boundary

Do not add showcase-specific policy, caption behavior, or project manifests to Nova. Nova owns the configurable deterministic recorder. `nova-showcase` will select the production portrait profile from its v0.12 capture capsule.

For the frozen Nova v0.11 capsule, `nova-showcase` will carry a local backport. Do not retrofit or migrate v0.11 as part of this task.

## Decision

`LoopProfile` is a plain `Resource` struct with five public fields
(`window_resolution`, `output_resolution`, `fps`, `crf`, `frame_cap`), all in
integer pixels/frames. A caller writes the whole struct, so a profile reads as
one complete answer rather than a default plus edits;
`LoopCapturePlugin::default()` carries exactly the documented docs-loop values
and the `LOOP_FPS` / `LOOP_CRF` / `LOOP_FRAME_CAP` / `LOOP_RESOLUTION`
constants remain, now documented as that default's field list.

Window integration: the plugin inserts the profile **before** its arming gate,
during plugin build. `force_capture_resolution` takes it as
`Option<Res<LoopProfile>>` and falls back to `CAPTURE_RESOLUTION`. Because a
plugin build runs before any `Startup` system, no `Startup` ordering decides
the window size, and the smoke path renders the same framing the capture will.

`CAPTURE_RESOLUTION` changed from `(f32, f32)` to `(u32, u32)` so the fleet
default and the profile field share one type; the single f32 cast now lives in
`force_capture_resolution`.

Validation is `LoopProfile::validate()` (zero dimension, zero fps, zero cap),
called from `Plugin::build` where an invalid profile panics with the field
name. Nothing else is bounded - 4K60 is a cost decision, and the frame cap
already fails a runaway loop.

`encode_args` omits the `-vf scale` step entirely when output equals window, so
a native-resolution master is never resampled through itself.

## Proof

- `cargo test -p nova_autopilot --lib loops::` - 16 passed, including the
  default-profile, portrait-args, frame-duration, profile-frame-cap,
  zero-value-validation, invalid-profile-panic and unarmed-no-op tests.
- `cargo test -p nova_autopilot --doc` - 7 passed (the module's
  `LoopCapturePlugin::new` example compiles).
- `cargo check -p nova_autopilot --all-targets`, `cargo check -p nova_debug
  --all-targets`, `cargo check --examples --features debug` - all clean.
- `cargo fmt --all -- --check` clean.
- Live portrait capture: `loop_cockpit` temporarily pointed at
  `LoopProfile { (1080,1920), (1080,1920), 60, 18, 1200 }`, run armed under
  `xvfb-run -s "-screen 0 1080x1920x24"`. 443 frames, "7.4s at 60 fps".
  `ffprobe`: `vp9 1080x1920 r_frame_rate=60/1 duration=7.384 size=3998662`.
  The temporary patch was reverted; no generated media committed.
- Live default capture (regression): same example on
  `LoopCapturePlugin::default()`, `xvfb-run -s "-screen 0 1920x1080x24"`. 231
  frames, "7.7s at 30 fps". `ffprobe`: `vp9 1280x720 r_frame_rate=30/1
  duration=7.700 size=221451` - the shipped docs-loop behaviour unchanged.

Not run: the full workspace test suite and Clippy, by standing instruction.
