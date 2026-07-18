# Render-scale / resolution lever for the graphics preset (task 20260718-004723)

Context note for future sessions. The user-facing before/after numbers and the
tier decision live in `tasks/20260718-004723/render-scale-report.md`; this file
is the implementation / reflection log AGENTS.md asks for.

## What changed and why

The v0.7.0 frame-time baseline (`tasks/20260716-123551`) found web to be the one
over-budget target and, unlike the discrete-GPU native path, fill / overhead
bound with almost no headroom. Its strongest concrete direction was a
render-scale lever on the Low preset: on a fill-bound path, dropping the number
of pixels shaded buys more than the existing particle/scatter toggles. This task
adds that lever.

- **`GraphicsBudget` gains a `render_scale: f32`** (`crates/nova_gameplay/src/settings.rs`),
  defaulted per tier by `GraphicsBudget::for_quality`: `High`/`Medium` = `1.0`
  (native window resolution, unchanged path), `Low` = `0.7` (draws ~49% of the
  pixels). Per the user's direction the lever is not web-only and only Low drops
  resolution; Medium/High keep the crisp look. Helpers: `is_native_resolution()`
  and `render_target_size(window_physical)` (clamps the fraction to
  `[MIN_RENDER_SCALE, 1.0]` and keeps every axis >= 1px so the target is never a
  zero-area, fatal wgpu allocation).

- **New `nova_scenario::render_scale` module** (`crates/nova_scenario/src/render_scale.rs`)
  with `RenderScalePlugin`, registered by `NovaScenarioPlugin` only when
  rendering. One idempotent reconcile system (`reconcile_render_scale`) drives
  the whole lever off `GraphicsBudget` + window size:
  - `render_scale >= 1.0`: nothing - the scenario camera renders straight to the
    window, exactly as before, so the crisp tiers pay zero cost.
  - `render_scale < 1.0`: create an offscreen `Image` sized
    `render_scale * window_physical`, point every `ScenarioCameraMarker` camera
    at it (setting the image target's `scale_factor` so the camera still reports
    the window's logical viewport), and spawn one blit `Camera2d` targeting the
    window that both draws a full-window sprite of the image AND is the
    `IsDefaultUiCamera`.

## Design choices worth keeping

- **World in the image, UI on the window** (revised after task 20260718-132638 -
  see below). The 3D world renders into the reduced image; the HUD/menus render
  full-resolution on the blit `Camera2d`, which targets the window. This is
  forced by bevy_ui: `ui_focus_system` only delivers a cursor to a WINDOW-targeted
  camera, so UI on the image-targeted scenario camera renders but is *unclickable*
  (the bug the first cut shipped). Keeping UI on the window also keeps it crisp.
  The world->screen projection stays aligned NOT by sharing a coordinate space
  but by setting the scenario camera's image-target `scale_factor` to
  `image_physical / window_logical`, so `world_to_viewport` /
  `logical_viewport_size` (read by `hud/screen_indicator`) return window-space
  coordinates even though the render is at fewer pixels. No RenderLayers isolation
  is needed: the blit is the only `Camera2d` and a `Camera3d` never draws 2D
  sprites.

  The first cut instead made the scenario camera the `IsDefaultUiCamera` and baked
  the HUD into the reduced image ("whole frame, one coordinate space"). It
  rendered correctly - the screenshots looked right - but every click missed,
  because the reviewer verified *rendering* (screenshots) and never *interaction*.
  Lesson: an image-targeted UI camera is a rendering-vs-picking trap.

- **Reconcile, don't spawn-time-configure.** The scenario camera is spawned by
  the loader (`nova_scenario::loader::on_load_scenario`) and is scenario-scoped
  (despawned/respawned on scenario change); quality can also change live from the
  settings menu, and the window can resize. A single idempotent reconcile that
  mutates only on a real diff (missing/wrong-sized target, quality flip, resize)
  converges for every ordering of those, mirroring the existing idempotent
  reconcile style in `hud/target_inset`. The lever gates on a scenario camera
  existing, so the menu/editor (whose cameras are not `ScenarioCameraMarker`)
  keep full resolution and the blit camera never covers them.

- **`RenderTarget` is a separate component in Bevy 0.19.** It is a required
  component of `Camera` (auto-inserted, default `Window(Primary)`), not a
  `Camera.target` field - so the reconcile queries `&mut RenderTarget` and
  removing the redirect is `*target = RenderTarget::default()`. `RenderTarget`
  has no `PartialEq`, hence the `targets_image` handle-matching helper to avoid
  churning change-detection every frame.

- **WebGL2-safe target.** `Image::new_target_texture(w, h, Rgba8UnormSrgb, None)`
  - the same format + default-view choice `hud::target_inset::create_render_target`
  landed for exactly this reason: a `Some` view format needs
  `DownlevelFlags::VIEW_FORMATS`, absent on WebGL2-class GPUs, where it is a fatal
  render-validation error. Since this lever exists for weak web GPUs, the target
  must not reintroduce that.

- **New `examples/21_render_scale_shot.rs`.** Boots a shipped scenario at a chosen
  preset and captures the primary window to a PNG (`BCS_SHOT`). Because it reads
  the primary window, a Low shot is the real upscaled frame - so a camera-stack
  misconfig (black/empty window) shows up as a bad PNG, which frame-time capture
  alone cannot distinguish from a genuine "fewer pixels" win.

## Difficulties / bugs hit along the way

- **`Camera.target` does not exist on 0.19.** First cut compiled against a
  `Camera.target` field; the error (`no field target on Mut<Camera>`) pointed to
  `RenderTarget` having become a standalone required component. Switched the query
  to `&mut RenderTarget`.

- **`nova_scenario` lib tests need `--features serde`.** Running
  `cargo test -p nova_scenario` bare fails to compile pre-existing `loader.rs`
  tests that `ron::to_string`/`from_str` a `ScenarioConfig` (its `Serialize`/
  `Deserialize` derives are serde-feature-gated). Not caused by this change - the
  new `render_scale::tests` pass under `cargo test -p nova_scenario --features serde`.

- **Contended measurement host.** Parallel background agent jobs (other sprout
  worktrees compiling/testing) drove load to ~12-19 during the sweeps. Software
  raster is pure CPU, so its absolute numbers are noisy; see the report for how
  the High-vs-Low comparison was read around that.

## Measurement outcome (the inconvenient part)

The web measure-first gate did not confirm the baseline's premise. On the only
web/WebGPU rig available (a discrete RTX 3060 Ti), `render_scale = 0.7` moved the
frame time ~0% (isolated cleanly with the new `render_scale` override: same Low
tier at 1.0 vs 0.7 - asteroid p50 30.7 -> 29.8, broadside 17.9 -> 18.2). The
`min` (GPU floor) barely tracks resolution, so that GPU's browser frame is
overhead-bound, not fill-bound; the upscale pass roughly cancels the fill saved.
A real win only appears at an aggressive, visibly soft `0.5`. The lever is aimed
at the weaker fill-bound hardware the Low preset exists for (iGPUs, phones),
which the strong rig cannot represent - the baseline said as much. Kept at 0.7
(user decision), documented honestly as a low-end knob, not a measured
general win. Full numbers + the not-pursued alternatives (aggressive 0.5, a
web-only canvas scale-factor with no extra pass) are in the task report.

## Self-reflection / what to do differently

- Mapping the camera + UI render architecture up front (one Explore pass:
  single window camera, no `UiTargetCamera`, no `RenderLayers` anywhere, the
  `IsDefaultUiCamera` rule) is what made the whole-frame-into-one-image design
  obviously correct rather than a guess. Worth doing before any render-graph
  change.
- The measurement gate is only as good as a quiet host. Next time, either serialize
  against other agent jobs or lean on the GPU/web rigs (less CPU-noise-sensitive
  than software raster) from the start rather than trusting a contended sw sweep.
