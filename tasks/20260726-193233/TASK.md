# NOVA OS CRT: render-to-texture pipeline with real text bloom + curvature

- PRIORITY: 44
- TAGS: v0.9.0, spike, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

Give the NOVA OS screen a real CRT by rendering the terminal content to an
offscreen texture and displaying it through ONE full CRT shader, unlocking the
one thing the current overlay-node approach fundamentally cannot do: real text
bloom (bright green glyphs blooming into a soft halo, like the HTML
`text-shadow: 0 0 7px`). The same pipeline also enables barrel curvature (warp
the sample UV), content-aware glass/vignette, and consolidates scanlines + grain
into the same shader.

Approach: route the terminal-content subtree to a dedicated UI camera targeting a
`RenderTarget::Image` (`UiTargetCamera`), sized to the panel and kept in sync on
resize (mirror the `crates/nova_scenario/src/render_scale.rs` target-sync
pattern); display that image on the screen via a CRT material that samples it
(soft scanlines, curvature, vignette, fixed-tap bloom of the bright green,
grain, glass highlight). This SUPERSEDES the current overlay-node CRT approach.

Beyond the CRT look, this offscreen-render capability is a reusable primitive:
once the terminal content is a texture, a sampling shader can do things the
overlay-node approach can never do - true bloom, a CRISP barrel UV-warp of the
content (the "3D curved screen" the HTML prototype could only fake at the edges,
because CSS can only curve content via a displacement map that BLURS the text),
per-content chromatic aberration at the edges, phosphor persistence/ghost trails
on scroll, boot power-on/CRT-degauss transitions, and future NOVA OS "apps"
(map/ship viewer) rendered through the same glass. It is the foundation for the
"more cool things" the screen could do, not just a one-off effect.

## Steps

- [x] PROTOTYPE FIRST, native AND web (the decision makes web parity a hard
      requirement): a minimal in-repo rig (feature-gated module or a scratch
      example) that routes a small text subtree through
      `UiTargetCamera` + `RenderTarget::Image` and displays it via a material
      that samples the image. Verify on native and on the trunk/WASM build:
      (a) text renders into the image and back out crisply, (b) a fixed-tap
      bloom is affordable on WebGL2, (c) wheel scroll + `Hovered` picking
      still work on nodes whose render target is the image. Record findings
      in `tasks/20260726-193233/NOTES.md`. A web blocker STOPS the task and
      goes back to the owner.
- [x] Image target + camera: spawn a dedicated UI camera targeting an `Image`
      sized to the screen panel's physical pixels; mirror the
      `crates/nova_scenario/src/render_scale.rs` target-sync pattern
      (`ImageRenderTarget` swap + resize convergence) so window resizes and
      panel relayouts never show a stretched frame.
- [x] Route the terminal-content subtree (terminal + any active app UI) to
      that camera via `UiTargetCamera`; replace the screen area's visible
      surface with a `MaterialNode` whose material binds the image
      (`AsBindGroup` texture + sampler) - keep the headless/no-render fallback
      harmless for tests.
- [x] Port the CRT treatment into the sampling shader
      (`assets/shaders/nova_os_crt.wgsl` successor): the 193155 soft
      resolution-aware scanlines, slot mask, grain and vignette, PLUS
      fixed-tap derivative-free bloom of the bright green, barrel-curvature UV
      warp (with a corner mask matching 193219's screen rounding), and a
      brightness-multiply uniform reserved for 20260726-214617. Keep
      Rust/WGSL uniform field order in lockstep
      (`shader-uniform-field-order-must-match-wgsl`).
- [x] Power collapse open/close per the DECISION: feed `DrawerOpenness` into
      the shader as the power uniform; on open the raster blooms from a
      single horizontal line with a brightness overshoot (PoC `crt-on`), on
      close it collapses to a line then a dying dot (PoC `crt-off`). The
      casing/backdrop keep a fast fade; the existing contract that gameplay
      unpauses only at openness zero stays intact
      (`drive_drawer_slide` keeps its state machine, its visual mapping
      changes).
- [x] Degauss on app launch/exit: a brief wobble+flash uniform pulse where
      `openApp`/`closeApp` transitions happen; cherry-pick the cheap
      micro-effects from the Notes inventory (hum bar, retrace beam, flicker,
      jitter) only where they read well - each is a uniform-driven term, none
      may harm text readability. (DEFERRED to 20260727-014148; degauss +
      micro-effects are polish, not a DoD item - the sampling pipeline that
      unlocks them landed here.)
- [x] Delete the superseded overlay CRT path (the overlay `MaterialNode`
      spawn and the UI-node scanline/vignette fallback) once the sampling
      shader carries everything; update the widget-tree tests that assert the
      overlay.
- [x] Tests: uniform-sync test (resolution/time/power fed from the panel +
      openness), widget-tree asserts the sampling surface + absence of the old
      overlay, existing scroll/input tests green through the image path.
- [x] Verify: `screenshot_nova_os` AFTER captures on native; run the
      trunk/WASM build and eyeball the same scene on WebGL2; store shots
      under `tasks/20260726-193233/shots/`; update CHANGELOG + wiki hud page;
      NOTES.md with what changed, difficulties and self-reflection. (Native
      AFTER captures stored, `trunk build` green, CHANGELOG + wiki + NOTES
      done; the live WebGL2 render eyeball remains owner MANUAL ACCEPTANCE -
      needs a real browser, see NOTES.md.)

## Definition of Done

- The terminal renders through the offscreen image + sampling CRT shader with
  visible text bloom and crisp barrel curvature, on native AND the web build.
  (manual: native AFTER capture + a WebGL2 eyeball, both stored/described in
  NOTES.md; cmd: `nix develop --command trunk build`)
- Opening the computer plays the raster power-on bloom; closing collapses to a
  line/dot before gameplay unpauses; the backdrop dim is preserved. (test:
  existing openness-gates-unpause test stays green; manual: capture/feel
  check)
- Scrollback wheel scrolling and app-chrome clicks work unchanged through the
  image path. (test: existing drawer scroll tests green)
- The overlay CRT path is gone - one shader owns the whole treatment. (cmd:
  `grep -rn "NovaOsScanlineMarker\|NovaOsVignetteMarker" crates/nova_gameplay/src`
  returns nothing)
- Touched tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)

## Notes

- DECISION: `tasks/20260726-193233/DECISION.md` (RTT supersedes the overlay;
  full web parity is a hard requirement; power collapse replaces the fade -
  all three confirmed by the owner at the 2026-07-26 plan gate).
- Spike: `tasks/20260726-193040/SPIKE.md` (option A, the headline). This is a
  load-bearing architecture change - record a `DECISION.md` citing the SPIKE.md
  when it is planned/kicked off.
- Reference look: the HTML PoC prototype in `examples/ui/nova_os_terminal_poc.html`
  (commits 7a7068d8, 50b33371) shows the target - bloom, tube curvature, animated
  grain, subtle scanlines. Note the CSS could only fake screen curvature at the
  edges; the crisp content bow is exactly what this Bevy RTT pipeline adds via a
  UV warp in the sampling shader.
- Constraints to design against (from the spike): UI materials cannot sample the
  content behind them; the UI blit `Camera2d` has no HDR/Bloom and must not be
  given one (it would bloom the whole game view); keep the CRT shader
  derivative-free for WebGL2.
- Open unknowns to prototype BEFORE committing: fixed-tap Gaussian bloom under
  WebGL2; image-target resize sync without a stretched frame; scrollback
  wheel/picking still working when content renders through an image.
- Verify with the `screenshot_nova_os` capture example.
- PoC micro-effect inventory (2026-07-26 fidelity review) - candidate effects
  this pipeline unlocks, cherry-pick what reads well: power-on raster bloom
  from a single scan line + power-off collapse to a dying dot (`crt-on`/
  `crt-off`), degauss wobble + flash on app switch, the slow mains-hum bar
  drifting down the tube, the occasional fast retrace beam, the 4.5 s
  brightness flicker, the rare vertical-hold jitter slip, and the
  phosphor-strike flash on freshly printed lines. Pair the audible ones
  (degauss coil, power sweeps) with task 20260726-214639.
- Priority: slotted p41 (last) in the 2026-07-26 PoC fidelity review, then
  RE-SLOTTED to p44 the same day (owner decision): this pipeline SUPERSEDES
  the overlay shader, so building further work on that shader (the BRIGHT/SCAN
  uniform endpoints in 20260726-214617, corner masking in 20260726-193219)
  before it lands is throwaway by design. It now runs right after the casing
  pass (p45, pure chrome, no interaction) and BEFORE the chin controls (p43),
  which wire their knobs to THIS task's sampling shader. Sound (p42) and shell
  UX (p41) are independent and float freely.
- Blocks: 20260726-214617 (the BRIGHT knob needs the sampling shader for a
  true >1.0 brightness multiply; SCAN wires to this shader's scanline
  uniform).
