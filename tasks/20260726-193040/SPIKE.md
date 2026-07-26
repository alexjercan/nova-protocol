# Spike: how to improve the NOVA OS CRT monitor look and feel

- DATE: 20260726-193040
- STATUS: RECOMMENDED
- TAGS: spike, ui, hud

## Question

The NOVA OS ship computer now reads clearly and matches the HTML PoC's palette
and layout. Beyond "readable", what would make it FEEL like a real green-phosphor
CRT, and which of those improvements are worth the effort? Answer: a ranked
review (well / meh / better) plus a recommended direction and seeded tasks.

## Context

The monitor is built from ordinary Bevy UI nodes (casing -> bezel -> screen ->
terminal content) with a single shader overlay on top for the CRT treatment:

- `assets/shaders/nova_os_crt.wgsl`: a straight-alpha UI material drawn ABOVE the
  terminal content. It does horizontal scanlines (`fract(uv.y*240)`), an
  edge-only elliptical vignette, a centre-peaked "volume" glow, and a
  two-frequency green-shade phosphor grain reseeded ~9x/s from a `time` uniform
  (gentle shimmer). Derivative-free so it is WebGL2-safe.
- `crates/nova_gameplay/src/hud/drawer.rs`: casing/bezel/screen nodes, topbar with
  lamp + status, footer hints, two accent slots, the terminal (scrollback +
  input box), a blinking amber block caret, and fish-style inline completion.
  Text is drawn CRISP - the per-glyph shadow was removed because Bevy's
  `TextShadow` has no blur and read as doubled text.
- Captures: `tasks/20260726-180807/shots/{before-game,reference-html,nova-os-welcome,nova-os-active}.png`.

Two hard constraints shape every option below:

1. The CRT overlay is a UI material that CANNOT sample the terminal content
   behind it (Bevy UI materials get no back-buffer/scene texture). So no effect
   that needs to read the rendered text - bloom, curvature of the content,
   content-aware anything - is possible from the current overlay node.
2. The UI renders through the render-scale blit `Camera2d`
   (`crates/nova_scenario/src/render_scale.rs`), which has no HDR/Bloom. Adding a
   Bloom post there would bloom the entire game view, not just the terminal.

## Review: what is done well / meh / could be better

### Done well
- Structure + palette fidelity to the HTML PoC; readability is solved (crisp
  bright text on near-black, no wash).
- CRT overlay is a real shader (scanlines, vignette, centre volume, animated
  green grain), derivative-free for WebGL2.
- The physical framing (casing, bezel, accent slots, topbar lamp, footer) sells
  "a device", and the input box + blinking caret + inline completion sell "a
  terminal".
- There is a repeatable capture rig (`screenshot_nova_os`) to verify visuals.

### Meh / limitations
- **No text bloom.** The single biggest gap. Real phosphor CRTs bloom bright
  glyphs into a soft halo (the HTML uses `text-shadow: 0 0 7px`). Ours is flat,
  so the screen reads as "printed green" rather than "emitting light". Blocked by
  both constraints above - it needs the content as a texture.
- **Scanlines are naive.** A hard `fract(uv.y*240) < 0.5` step: a fixed 120
  lines regardless of the screen node's pixel size, hard-edged (aliased), and
  prone to moire/shimmer when the panel resizes. Real scanlines are soft and
  output-resolution-aware.
- **No screen curvature.** The "volume" is a painted radial glow; the content is
  dead flat. No barrel distortion, no rounded screen corners (the screen is a
  hard rectangle; the HTML rounds them), no glass.
- **No glass.** No specular sheen / reflection highlight over the glass (the HTML
  has a soft white diagonal gradient). The bezel/casing are flat-shaded nodes
  with no inner-bevel depth gradient, so the "device" reads a bit papery.
- **Grain shimmer is steppy.** The 9 Hz reseed is a step function; it can read
  "digital" rather than analog. Grain is also uniform across the panel, whereas
  real tube noise concentrates in the darker regions.
- **No emission behaviour.** No phosphor persistence/ghost trails on scroll or
  type, no subtle refresh flicker/roll, no beam/retrace character. (Persistence
  and flicker are optional and risk distraction - low priority.)

## Options considered

- **A. Render-to-texture CRT pipeline (the "real" screen shader).** Route the
  terminal-content subtree to a dedicated UI camera targeting an offscreen image
  (`RenderTarget::Image` + `UiTargetCamera`), then display that image on the
  screen through ONE full CRT material that samples it: soft resolution-aware
  scanlines, barrel curvature (warp the sample UV), edge vignette, **bloom**
  (multi-tap blur of the bright green), grain, and a glass highlight. This is
  the only path to real text glow/curvature, consolidates every effect into one
  well-placed shader, and is what the user meant by "use a shader for the screen
  itself". Cost: a genuine mini render pipeline - a camera + image target sized
  to the panel and kept in sync on resize, a multi-pass or multi-tap bloom
  (WebGL2/derivative-free constraints on the blur), and re-checking input
  hit-testing/scroll now that content is drawn through an image. Highest value,
  highest effort/risk; supersedes the overlay-node approach.
- **B. Casing + glass depth pass (no RTT).** Rounded screen corners via
  `BorderRadius`, a glass specular-highlight node (soft white diagonal gradient),
  bezel inner-bevel/gradient for depth, small screw/vent detailing. Pure UI-node
  + maybe a small casing material; cheap, independent, low risk. Makes the device
  read as glass + moulded plastic. Does nothing for text glow.
- **C. Scanline + grain realism pass (shader-only, no RTT).** Improve the
  EXISTING overlay: soft resolution-aware sinusoidal scanlines (feed panel pixel
  size as a uniform), smooth/interpolated grain shimmer instead of the 9 Hz step,
  optional vertical slot-mask, and brightness-weighted noise. Cheap, independent,
  improves feel even if A is never done. Still no text glow.
- **D. Do nothing.** The current look already cleared the bar (readable, on-brand,
  shippable for v0.9.0). The CRT polish is stretch, not required. Cost of
  deferring: the screen stays "good, not great"; the text-glow gap persists.

## Recommendation

Pursue **C and B first** (cheap, independent, immediate feel wins that need no
architecture change and improve the screen even if A never lands), and scope
**A (RTT CRT pipeline)** as the headline stretch item that unlocks the one thing
nothing else can deliver - real text bloom - plus curvature and a content-aware
glass. Keep **D** honest: none of this blocks v0.9.0, so all three seeded tasks
are backlog/stretch, sequenced C -> B -> A by cost.

A is the big lever but carries the most unknowns (bloom under WebGL2 without
derivatives; image-target resize sync; input through an image). C and B de-risk
the look now and are trivially reversible. If A proves too heavy, C + B still
leave the monitor meaningfully better.

Each seeded task that makes a load-bearing rendering-architecture choice (notably
A, which replaces the overlay-node approach with an RTT pipeline) records a
`DECISION.md` citing this SPIKE.md rather than repeating it.

## Open questions

- **Bloom without derivatives:** can a fixed-tap Gaussian approximation over the
  offscreen image stay WebGL2-safe and cheap enough? (Resolve in task A with a
  prototype.)
- **Image-target sizing:** how to keep the offscreen render target sized to the
  panel across window resize / render-scale changes without a frame of stretch
  (mirror the `render_scale.rs` sync pattern). (Resolve in A.)
- **Input/scroll through an image:** does routing terminal content to its own UI
  camera keep the scrollback wheel + picking working, or does hit-testing need
  rework? (Resolve in A - prototype before committing.)
- **Curvature vs readability:** how much barrel distortion before small text at
  the corners gets hard to read? A single tunable, decided by playtest.

## Next steps

Direction-level tasks this spike seeded (for `/plan` to break into steps):

- tatr 20260726-193155: Scanline + grain realism pass (shader-only) - soft
  resolution-aware scanlines, smooth grain shimmer, optional slot-mask.
- tatr 20260726-193219: Casing + glass depth pass - rounded screen corners,
  glass specular highlight, bezel/casing depth, small detailing.
- tatr 20260726-193233: Render-to-texture CRT pipeline with real text bloom +
  curvature (the headline; supersedes the overlay-node approach).

## Fix record

(Appended by each implementing task as it lands.)
