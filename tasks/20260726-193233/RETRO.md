# RETRO - NOVA OS render-to-texture CRT pipeline (20260726-193233)

- DATE: 20260727
- OUTCOME: RTT CRT pipeline landed on the branch (real text bloom + barrel
  curvature through an offscreen image + one sampling shader), superseding the
  overlay path. Native-verified, web-build-verified, 57 drawer tests green,
  one review round (M1 fixed + pinned). Degauss + micro-effects spun out to
  20260727-014148.

## What went well

- **Reading the engine source before writing the rig turned a scary "unclickable"
  lesson into a solvable mechanism.** The `verify-interaction-not-just-rendering`
  lesson and `render_scale.rs`'s own comment both said bevy_ui on an image camera
  is unclickable. Grepping `bevy_ui/src/picking_backend.rs` showed the MODERN
  picking backend matches pointers to cameras by RENDER TARGET, not window-ness -
  the lesson was true only for the legacy `ui_focus_system`. That single source
  read is the difference between STOPping the task and shipping it.
- **Prototype-first paid off exactly as the DECISION intended.** A standalone
  runnable example (`examples/ui/nova_os_rtt_poc.rs`) with an unattended
  self-verdict proved picking + click + crisp text + bloom + curvature BEFORE any
  surgery on the 5000-line drawer, and surfaced the load-bearing
  `update_is_hovered`-is-Mouse-only finding that shaped the integration.
- **The out-of-context review earned its keep.** It found M1 (the unfiltered
  hover-mirror clobbering window UI) - a latent bug that would have silently
  regressed the chin knobs task built ON this branch. Exactly the class of bug an
  author is blind to.

## What went wrong / difficulties

- **The hover-mirror shipped too broad.** `mirror_nova_os_hover` wrote `Hovered`
  on every hoverable entity, not just those rendered through the image. It passed
  all tests and rendered fine because nothing window-space needed `Hovered` DURING
  the drawer yet - a latent bug hidden by the absence of the exact consumer it
  would break. The NOTES already flagged "runtime mouse hover/click through the
  drawer wasn't eyeballed"; the review caught the code-level version.
- **Prototype debugging cost a few cycles** to a self-inflicted race: the rig's
  pointer-forwarding system fought the automated verdict for the pointer each
  frame. A `diagnose` system dumping viewport/scaling/HoverMap pinned it fast -
  worth building the introspection instead of guessing.
- **The live WebGL2 render eyeball could not be automated** (needs a real browser;
  headless WebGL2 is unreliable). `trunk build` + a wasm compile + the
  image-targets-ship-on-web precedent are strong evidence, but the actual
  in-browser render/FPS is left as owner manual acceptance - an honest gap, not a
  claimed pass.

## Lessons worth promoting (candidates for LESSONS.md)

- `bevy-ui-image-camera-pickable-via-forwarded-pointer` (domain): bevy 0.19
  `ui_picking` matches pointers to cameras by RenderTarget, so UI rendered to an
  image IS hover/clickable via a spawned `PointerId::Custom` whose
  `PointerLocation.target` is that image - the "image camera is unclickable"
  limitation is the LEGACY `ui_focus_system` only. BUT `update_is_hovered` is
  hard-coded to `PointerId::Mouse`, so the `Hovered` COMPONENT needs a manual
  mirror for the forwarded pointer, and that mirror MUST be scoped to the
  through-image subtree or it clobbers window-space `Hovered`. (Prototype + M1.)
- `bevy-ui-render-ignores-renderlayers` (domain): `bevy_ui_render` routes UI purely
  by `ComputedUiTargetCamera` and never reads `RenderLayers`, while 2D sprites DO
  respect them - so a UI camera on a dedicated `RenderLayers` layer draws its
  targeted UI AND is isolated from stray world sprites. Useful for any UI-to-image
  camera that must not pick up the render-scale upscale sprite.

## What to do differently next time

- When building a system that MIRRORS engine state for a scoped subset (hover,
  focus, visibility), scope the write query at birth - an unfiltered global write
  that "happens to work" is a latent regression waiting for the first real
  consumer. Would have avoided M1.
- For any dual-platform (native + web) feature where the web check can't be
  automated, wire the integration to reach `trunk serve` as early as possible so
  the manual eyeball is a small, early step, not a deferred end-of-task risk.
