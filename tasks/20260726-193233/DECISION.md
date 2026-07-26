# Decision: RTT CRT pipeline with full web parity; power collapse replaces the fade

- DATE: 20260726-220000
- STATUS: ACCEPTED
- TASK: 20260726-193233
- TAGS: decision, ui, hud, rendering

## Context

The NOVA OS screen is ordinary Bevy UI with an overlay CRT shader that cannot
sample the content behind it, so text bloom, content curvature and
content-transform effects are impossible from it. `tasks/20260726-193040/SPIKE.md`
(option A) laid out the render-to-texture alternative and its unknowns. At the
2026-07-26 plan gate the owner resolved three forks that shape this task.

## Decision

1. Build the RTT pipeline: route the terminal-content subtree to a dedicated
   UI camera targeting a `RenderTarget::Image`, display it through ONE
   sampling CRT shader (scanlines, grain, vignette, bloom, barrel curvature,
   brightness), superseding the overlay-node CRT approach entirely.
2. Full WebGL2/WASM parity is a HARD requirement: the task is not done until
   the web build renders the same pipeline. There is no native-only fallback
   scope.
3. The power collapse REPLACES the drawer's current fade in/out: on open the
   raster blooms from a horizontal scan line; on close it collapses to a line
   and then a dying dot before gameplay unpauses. The backdrop dim stays. This
   knowingly overrides the 2026-07-24 playtest note "likes the transparency
   effect + slide animation (keep them)" - the owner chose the full PoC power
   metaphor at the 2026-07-26 gate with that history surfaced. (The "slide"
   was in fact already only a fade: `drive_drawer_slide` maps openness to
   visibility + backdrop alpha, it does not translate the panel.)

## Alternatives considered

- **Keep the overlay shader and iterate on it** - rejected: it fundamentally
  cannot bloom or curve the content (no back-buffer sampling for UI
  materials); the spike's option C already extracted everything it can give.
- **Native-first RTT with the overlay as web fallback** - rejected by the
  owner at the gate; it would fork the visual identity across platforms and
  require every knob/uniform to drive two shader endpoints forever.
- **Keep the fade and layer subtle raster FX inside the screen** - rejected by
  the owner at the gate in favor of the full PoC power metaphor.

## Consequences

- A feasibility prototype on BOTH native and WASM/WebGL2 is the mandatory
  first step; a web blocker stops the task and goes back to the owner rather
  than silently shrinking scope.
- The bloom must be fixed-tap and derivative-free (no HDR/Bloom post on the UI
  blit camera - it would bloom the whole game view).
- Chin controls (20260726-214617) wire BRIGHT/SCAN to THIS shader's uniforms;
  the overlay material and its fallback nodes are deleted once parity holds.
- Scroll/picking must be re-verified through the image path; the content nodes
  keep their layout, only their render target changes.
- Every uniform added must keep Rust/WGSL field order in sync
  (`shader-uniform-field-order-must-match-wgsl` lesson).
