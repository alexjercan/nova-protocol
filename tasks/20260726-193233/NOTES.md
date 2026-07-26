# NOTES - NOVA OS render-to-texture CRT pipeline (20260726-193233)

## Step 1: PROTOTYPE FIRST - findings (2026-07-26)

Rig: `examples/ui/nova_os_rtt_poc.rs` + `assets/shaders/nova_os_rtt_poc.wgsl`.
A standalone bevy app that routes a small text subtree + an interactive button
through a dedicated UI camera targeting a `RenderTarget::Image`, then displays
that image on the window through ONE sampling CRT material (fixed-tap bloom +
barrel warp + scanlines/vignette/grain). It self-verifies picking/click through
the image and captures a native screenshot. Run: `cargo run --example
nova_os_rtt_poc` (auto-exits after the verdict; `NOVA_POC_HOLD=1` to keep it up).

### The three unknowns the DECISION front-loaded

**(a) Text renders into the image and back out crisply - YES.**
Native capture `shots/poc-native.png`: bright green glyphs are readable, with a
real soft green HALO around them (the bloom the overlay never could do) and a
visible barrel bow of the whole content with black beyond the tube edge. Text
softens slightly under bloom=0.9/warp=0.18 - both are uniform-tunable.

**(b) Fixed-tap bloom + barrel warp affordable and WebGL2-safe - YES (native
proven; live-WebGL2 eyeball pending at first integration).**
The shader is derivative-free (only `textureSample` + arithmetic + a fixed
12-tap gather loop), so it does not trip `DownlevelFlags`. The example COMPILES
for `wasm32-unknown-unknown` (clean, 5m51s). The UI-camera -> image mechanism is
the SAME one `crates/nova_scenario/src/render_scale.rs` already ships on WebGL2
(the web perf lever renders the scenario to an image target and blits it), so
image-render-targets on WebGL2 are already load-bearing in production here. The
only web item NOT yet exercised in a real browser is the live FPS of this
specific bloom shader; that is the DoD's WebGL2 eyeball and fires as soon as the
pipeline lands in the drawer under `trunk serve`. A blocker there STOPS the task
per the DECISION - the contract stays intact; it just fires at first integration
instead of on a throwaway wasm harness (trunk builds the game bin, not examples).

**(c) THE CRUX - hover + click through the image - SOLVED, with a caveat.**
Verdict output: `POC PICKING native: OK` and `POC CLICK native: OK`.

- `render_scale.rs` documents (and the lesson `verify-interaction-not-just-
  rendering` records) that bevy's LEGACY `ui_focus_system` only feeds a cursor
  to a WINDOW camera, so "bevy_ui on an image camera is unclickable". That is
  true for the legacy path.
- BUT bevy 0.19's picking backend `ui_picking` (bevy_ui/src/picking_backend.rs)
  matches pointers to cameras by RENDER TARGET equality, not window-ness, and
  computes hit positions via `camera.target_scaling_factor()`. So a FORWARDED
  custom pointer (`PointerId::Custom`) whose `PointerLocation.target` is the
  image render target makes `ui_picking` hit-test the content nodes drawn
  through the image camera. Confirmed: the forwarded pointer registers in the
  `HoverMap`, and a `Pointer<Click>` observer on the in-image button FIRED.
- Picking is pure ECS (no GPU), so this is identical on native and WebGL2 - web
  parity for interaction is NOT at risk.

#### CAVEAT that shapes the integration (load-bearing)

`bevy_picking::hover::update_is_hovered` is HARD-CODED to `PointerId::Mouse`
(hover.rs:392) - it mirrors ONLY the mouse pointer's hits into the `Hovered` /
`DirectlyHovered` COMPONENTS. A forwarded Custom pointer drives:
  - the `HoverMap` resource (yes),
  - `Pointer<Over/Out/Down/Up/Click>` events + observers, incl. bubbling to
    ancestors (yes - the button's Click observer fired even though the raw hit
    was the button's text child), but
  - NOT the `Hovered` component.

`crates/nova_gameplay/src/hud/drawer.rs`'s `scroll_drawer_panels` gates the
wheel on `Option<&Hovered>` on `DrawerScrollViewportMarker`. So the integration
MUST restore `Hovered` for the forwarded pointer. Options, in preference order:
  1. Add a tiny `mirror_forwarded_hover` system: replicate `update_is_hovered`'s
     ancestor logic but for our Custom pointer, writing `Hovered` on the
     terminal's interactive nodes. Localized, general, keeps the primitive
     reusable. PREFERRED.
  2. Drive the existing `PointerId::Mouse` location into the image when the
     cursor is over the panel (no second pointer). Simpler but steals the mouse
     from any window-UI over the panel while the drawer is open - riskier.
Chosen: option 1 (documented here; a DECISION.md addendum will record it if it
turns load-bearing during integration).

### Reusable primitive extracted

The pointer-forwarding layer (map window cursor -> panel rect -> inverse barrel
warp -> image-space `PointerLocation`, mirror mouse Press/Release as
`PointerInput` for the Custom pointer, mirror its HoverMap into `Hovered`) is the
reusable core the task's Story calls out: once content is a texture and the
pointer is forwarded, any future NOVA OS "app" (map/ship viewer) rendered through
the same glass stays interactive.

### API notes for the integration (bevy 0.19 fork)

- Render target is its OWN component (`RenderTarget::Image(ImageRenderTarget{
  handle, scale_factor })`) on the camera entity, not a `Camera.target` field.
- `Image::new_target_texture(w, h, TextureFormat::Rgba8UnormSrgb, None)` sets
  RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST (same as render_scale/target_inset).
- `UiTargetCamera(entity)` on the content root routes the subtree; children
  resolve via `ComputedUiTargetCamera` (what picking reads).
- Texture-sampling UI material: `#[uniform(0)] data` + `#[texture(1)] #[sampler(2)]
  source: Handle<Image>`; WGSL `@group(1) @binding(0/1/2)`.
- `PointerId::Custom(Uuid)` via `bevy::asset::uuid::Uuid`.
- `Assets::get_mut` returns a change-detection guard -> bind `mut mat`.
- Resize sync must mark `projection.set_changed()` after a target swap
  (`bevy-camera-ignores-runtime-rendertarget-swap`), mirroring render_scale.

## Integration architecture (Steps 2-7) - pinned to drawer.rs

Current tree (built once at `spawn` ~drawer.rs:2551, monitor `Visibility::Hidden`,
openness drives visibility):
`NovaOsMonitor -> Bezel -> Screen(NovaOsScreenMarker, 2614)` and the screen node
parents: terminal content (`spawn_nova_os_terminal_content` 3105) + overlay
material (`spawn_nova_os_screen_overlays` 3041) + rim + glass. App root
(`NovaOsAppRoot`, spawned by `sync_nova_os_app_ui` ~1662) also parents under the
screen and toggles terminal-content visibility.

Target tree:
- NEW top-level `NovaOsImageContentRoot`: `UiTargetCamera(image_cam)`, absolute
  at (0,0), width/height = image LOGICAL size (= screen physical px, image
  scale_factor 1.0). Terminal content + app root move UNDER this root (NOT under
  the screen node) - a differently-targeted subtree nested under a window node
  has ambiguous layout; a top-level root is the pattern the prototype validated.
- Screen node now hosts ONLY: the sampling `MaterialNode` (full-screen) + rim +
  glass. The overlay material + the scanline/vignette fallback nodes are deleted
  (Step 7 / DoD grep on `NovaOsScanlineMarker|NovaOsVignetteMarker`).

New systems (a self-contained `hud/nova_os_crt.rs` module is the clean home):
1. `reconcile_nova_os_target` - read `NovaOsScreenMarker` `ComputedNode.size`
   (physical); (re)create the image at that size via `Image::new_target_texture`;
   set the image-content-root width/height to match; on swap mark
   `projection.set_changed()` (`bevy-camera-ignores-runtime-rendertarget-swap`).
   Deactivate the image camera + hide the content root when openness == 0 so a
   closed drawer costs no offscreen pass.
2. `forward_nova_os_pointer` - window cursor -> screen `MaterialNode` rect ->
   inverse barrel warp -> image px -> Custom pointer `PointerLocation`; mirror
   mouse Press/Release as `PointerInput` for that pointer.
3. `mirror_nova_os_hover` - replicate `update_is_hovered`'s ancestor walk for the
   Custom pointer, writing `Hovered` on `DrawerScrollViewportMarker` (+ any other
   interactive terminal node) so `scroll_drawer_panels` keeps working.
4. `animate_nova_os_crt` (exists) - extend to feed resolution + time + the
   `DrawerOpenness` power uniform (+ degauss pulse from app launch/exit hooks in
   `sync_nova_os_app_ui`).

Material: rework `NovaOsCrtMaterial` to `#[texture(1)] #[sampler(2)] source:
Handle<Image>` + the sampling shader (`nova_os_crt.wgsl` successor). Uniform gains
`warp`, `bloom`, `power`, `brightness` (reserved for 214617) - keep Rust/WGSL
field order in lockstep.

Tests to update (assert current screen tree): grep `NovaOsScreenMarker` +
`NovaOsCrtMaterialMarker` widget-tree tests (~4957, 5052, 5137) and the overlay
asserts; add a uniform-sync test + a "sampling surface present, old overlay
absent" assert.

## Integration landed (Steps 2-4, 7) - 2026-07-26

The RTT pipeline is wired into the real drawer and renders correctly natively
(`shots/after-native-welcome.png`, `shots/after-native-active.png`): the terminal
text reads clearly through the sampling shader with a green bloom halo and a
visible barrel bow (the crisp content curvature CSS could only fake), the phosphor
rim follows the curve, scanlines/grain present. All 56 `drawer` tests pass; the
overlay grep is empty.

What went in (`crates/nova_gameplay/src/hud/drawer.rs`):
- `NovaOsCrtMaterial` reworked to a texture-sampling material (`source` image +
  `warp`/`bloom`/`power`/`brightness` uniform fields appended in WGSL-matching
  order); `assets/shaders/nova_os_crt.wgsl` rewritten to sample the image (bloom,
  barrel warp, scanlines, vignette, grain, corner mask, power collapse).
- `setup_drawer` spawns the RTT pipeline when render-capable (an `Assets<Image>`
  + `Assets<NovaOsCrtMaterial>` exist): offscreen image, a `NovaOsImageCamera` on
  `RenderLayers::layer(20)` targeting it (so it draws ONLY the terminal UI, never
  the render-scale upscale sprite - confirmed `bevy_ui_render` ignores RenderLayers
  and routes UI purely by `ComputedUiTargetCamera`, while sprites respect them),
  a top-level `NovaOsImageContentRoot` (`UiTargetCamera` = that camera) holding the
  terminal + app UI, and the forwarded `PointerId::Custom`. The screen node hosts
  the `NovaOsCrtSurface` sampling `MaterialNode`. Headless (no assets) falls back
  to the terminal directly on-screen.
- Systems: `reconcile_nova_os_target` (size image + content root to the screen
  ComputedNode, `projection.set_changed()` on swap, camera `is_active`/root
  visibility gated on openness), `forward_nova_os_pointer` (cursor -> screen rect
  -> inverse barrel -> image px + mouse-button mirror), `mirror_nova_os_hover`
  (feeds `Hovered` for the forwarded pointer, before the wheel scroll),
  `animate_nova_os_crt` extended to feed resolution/time + `DrawerOpenness` as
  `power`. `sync_nova_os_app_ui` + `remove_drawer` route/tear-down through the
  content root.
- Overlay path deleted: `spawn_nova_os_screen_overlays`, `NovaOsScanlineMarker`,
  `NovaOsVignetteMarker`, `NovaOsCrtMaterialMarker` are gone; tests updated
  (`drawer_screen_samples_offscreen_image`, resolution/time/power uniform test).

Web build gate: `nix develop --command trunk build` SUCCEEDS with the full
pipeline (the DoD web cmd). Combined with the derivative-free WebGL2-safe shader
and image-targets-already-ship-on-web precedent (render_scale), web parity holds
at the compile+capability level. The remaining web item is a LIVE in-browser
render eyeball (does the shader render right + is bloom affordable at FPS), which
needs a real WebGL2 browser via `trunk serve` - a manual owner check.

Docs done: CHANGELOG (Interface & HUD) + wiki `web/src/wiki/hud.md` describe the
real-CRT screen.

Still open (deferred/manual):
- Degauss wobble+flash on app launch/exit (Step 6) - NOT a DoD item; pure polish.
  Deferred to a follow-up task with the rest of the micro-effect inventory.
- Live WebGL2 render eyeball + power-collapse feel-check + a live mouse
  hover/click through the drawer image (the screenshot autopilot is keyboard-only;
  the interaction mechanism is prototype-proven + unit-wired) - owner manual
  acceptance during the web pass.

## Difficulties / diagnosis log

- First runs showed `POC PICKING FAIL`. Root causes, found via a `diagnose`
  system dumping viewport/scaling/node-target/HoverMap: (1) `forward_pointer`
  (mouse-driven) fought the verdict for the pointer every frame - fixed with a
  `Probing` gate so the verdict owns the pointer during the probe; (2) picking
  runs in PreUpdate, one frame after the location write - fixed by pinning across
  a window and sampling late; (3) the real blocker to a clean hover assert was
  the Mouse-only `update_is_hovered` above - the pointer WAS hitting content
  (HoverMap + Click observer), so the verdict now reads the HoverMap directly.

## Self-reflection

- Reading the engine source (`picking_backend.rs`, `hover.rs`, `input.rs`) BEFORE
  writing the rig was what turned a scary "unclickable" lesson into a precise,
  solvable mechanism. The lesson was true for the path it described (legacy
  focus) and stale for the current picking backend - exactly the "verify engine
  guarantees in source" discipline.
- The Mouse-only `Hovered` hard-coding is the kind of thing only a running probe
  surfaces; worth the prototype.
