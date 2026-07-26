# NOVA OS CRT: render-to-texture pipeline with real text bloom + curvature

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.9.0,spike,feature,ui,hud

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

## Notes

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
