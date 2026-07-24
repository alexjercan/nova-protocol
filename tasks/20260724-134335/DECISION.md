# DECISION: drawer-open background treatment + left-panel scope

- DATE: 20260724
- TASK: 20260724-134335
- STATUS: ACCEPTED (owner call at the /flow gate, 2026-07-24)

## Context

The playtest rework changes how the Tab drawer OPENS: hide the flight HUD and
make the background not fight the drawer for readability, keeping the top status
strip and the lower-left keybind hints visible. The written scope asked to keep
the gray transparent backdrop and ADD a blur of the scene behind.

bevy 0.19 offers NO UI backdrop-filter (`bevy_ui` only has box-shadow
`blur_radius`). A real scene blur therefore requires a camera post-process:

- A custom fullscreen gaussian-blur render node (uniform frosted look, reuses
  `bevy_post_process::gaussian_blur`, but a render-graph ViewNode + pipeline to
  build and verify WebGL2/wasm-safe since the game ships on web); or
- The built-in `DepthOfField` (Gaussian mode) on the camera (reuses maintained,
  web-supported infra, far less code, but depth-dependent so the blur reads
  non-uniform and needs a depth prepass + tuning).

## Decision

1. BLUR: DROPPED for this task. Owner chose HEAVY GRAY ONLY - deepen the
   existing gray dim so the frozen scene reads as an inert field, with NO camera
   post-process. Neither the custom node nor DoF was judged worth the cost/risk
   this sprint. The acceptance item "background blurry" becomes "background gray
   enough that you do not notice the old UI is gone".
2. LEFT PANEL: this task builds the left-panel SHELL + a titled placeholder
   section that slides from the left; the comms/flight-log CONTENT remains task
   20260724-102309, which fills the existing shell.

## Consequences

- No render-graph / camera / shader work; no WebGL2 blur risk. Lower scope.
- Diverges from the written scope's "ADD a blur" - recorded here so the
  divergence is auditable, not silent. If a future playtest still wants a true
  blur, re-open with the custom-node vs DoF fork above as the starting point.
- The HUD hide reuses the existing `HudTier` / `apply_hud_visibility` machinery
  (a `HudDrawerExempt` marker for the status strip + keys), not a new axis.
