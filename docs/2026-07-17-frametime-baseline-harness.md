# Frame-time capture harness + gameplay baseline (task 20260716-123551)

Context note for future sessions. The user-facing numbers and decisions live in
`tasks/20260716-123551/frametime-baseline-report.md`; this file is the
implementation/reflection log AGENTS.md asks for.

## What changed and why

- New env-gated plugin `crates/nova_debug/src/perf.rs` (`nova_frametime`). It
  drives the real gameplay app to `Playing`, warms up, records `Time<Real>`
  per-frame deltas for a fixed window, computes nearest-rank percentile stats,
  writes `<label>.json` + a `frametime.csv` row, and exits. Env-gated by
  `NOVA_PERF` so it costs nothing in a normal run - the same contract as the
  `nova_autopilot`/`nova_screenshot` presets it sits beside.
- New example `examples/20_perf_baseline.rs`: boots any shipped scenario by id
  (env `NOVA_PERF_SCENARIO`), with a `NOVA_PERF_QUALITY` preset knob. One binary
  sweeps every scene x preset x renderer from a shell loop.
- New `scripts/perf-baseline.sh` sweep driver.

Design choices worth keeping:

- **Measure `Time<Real>` deltas, not the diagnostics store or FPS.** Wall-clock
  frame time is what a player feels; FPS is a derived reciprocal. The percentile
  math is a pure function with unit tests (nearest-rank, so every `pXX` is a real
  observed frame).
- **Force vsync off + `WinitSettings::game` + fixed resolution.** Otherwise the
  loop is pinned to a refresh, throttled when unfocused, or measured at an
  uncontrolled size.
- **JSON hand-formatted, no serde dep** in this dev-only crate; the summary line
  is always logged so the wasm/web path (no fs) still yields a number.

## Difficulties and how they were diagnosed

1. **Prebuilt binary couldn't find assets.** Running the `target/release`
   example directly, Bevy resolved `assets/` beside the executable
   (`target/release/examples/assets/...`), not the repo, so nothing loaded, the
   app never reached `Playing`, and the capture hung to timeout. Fix: set
   `BEVY_ASSET_ROOT="$PWD"`. `cargo run` masks this (it sets `CARGO_MANIFEST_DIR`
   + cwd); a bare binary does not. The script and report document it.
2. **`:0` cannot measure uncapped GPU frame time.** On the live composited
   desktop the median clamps to ~17 ms (60 Hz compositor vsync overriding
   `AutoNoVsync`) and the tail is dominated by desktop + sibling-agent CPU
   contention (identical-config mean swung 40 fps <-> 76 fps). Diagnosed by the
   resolution-insensitivity check (480p ~= 1080p) and the bimodal 17/33/50 ms
   quantisation. Conclusion: `:0` is only good for the ~5-9 ms real-present
   floor, not for tables.
3. **Xvfb + NVIDIA is the usable rig.** wgpu Vulkan renders on the real GPU into
   a headless Xvfb X11 window (Vulkan WSI needs no GLX), with no compositor and
   no visible window - so no screen hijack and it is repeatable. Caveat found by
   comparing to the `:0` floor: Xvfb's software present-copy adds a roughly fixed
   ~10 ms/frame, so its absolute means are inflated; it is right for *relative*
   comparison and stall detection.
4. **Software floor: GL path panics, Vulkan lavapipe works.**
   `WGPU_BACKEND=gl` + `LIBGL_ALWAYS_SOFTWARE=1` panicked in bevy_render adapter
   selection (bevy 0.19 wgpu GL path). Switched to forcing the software Vulkan
   ICD (`VK_ICD_FILENAMES=.../lvp_icd...json`, lavapipe/llvmpipe), which works.
5. **The interesting result was a negative one.** The at-rest frame cost is
   fixed-overhead/CPU-bound on discrete GPU (scene-flat, resolution-insensitive),
   so the graphics preset barely moves it and there is nothing to optimize at
   rest - exactly the blind-optimization trap the measure-first gate exists to
   prevent.

## Combat-burst + web capture (follow-up, same task)

Both deferred pieces were then built and captured:

- **Combat driver** (`combat_burst_driver` + a `drive` hook on the plugin). The
  trap: the weapon-range examples press `Space` to fire, but in the real ship rig
  `Space` is the main thruster ("Flight Burn") and the gun is `LMB` ("Turret") -
  a trace (`RUST_LOG=trace`, count `on_projectile_marker`) caught 0 projectiles
  until the fire button was corrected to `MouseButton::Left`. Health top-up keeps
  the burst sustained. Combat cost is genuinely volatile (bullet ramp, ammo
  reload, AI engage/evade) *and* the shared box's load swings 3->17, so several
  runs were needed to separate signal (combat ~+54%, particles ~11%) from noise.
- **Web/WebGPU** via a `perf_web` wasm bin (config from the URL query, not env),
  a `perf.html` Trunk entry, and `perf-web.sh`. Three environment fights: (1)
  bevy is a dev-dep of the root crate, so a real `[[bin]]` needed bevy promoted to
  a normal dep; (2) Trunk's wasm-opt is too old for rustc bulk-memory ->
  `data-wasm-opt="0"`; (3) headless Chromium exposes no WebGPU - the working rig
  is Chromium under Xvfb with `--enable-features=Vulkan,WebGPU --use-angle=vulkan`
  over an `http://localhost` origin, which gives a real `BrowserWebGpu` NVIDIA
  adapter. Result: web ~2x native, the real constrained target.

## What could have gone better / next time

- **Pick the rig before building.** ~19 min of release-LTO build happened before
  the compositor/present-copy problems surfaced. A 2-minute check of "can I even
  get a clean frame time on this host?" (compositor? headless GPU? present copy?)
  should precede the full build next time.
- **Combat-burst was the point and is still unmeasured.** The preset exists to
  cut particle bursts; at-rest capture can't see them. A firing autopilot
  (the `19_broadside` slice is the template) should have been scoped in from the
  start as the second capture mode. The harness has the input-hook seam; the
  script is the deferred follow-up.
- **Web is genuinely not done.** The wasm console-scrape path is designed but not
  run. If web is the constrained target the task cares about most, that run is
  the real deliverable and should be prioritised next.
