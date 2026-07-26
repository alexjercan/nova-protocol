# NOVA OS CRT: scanline + grain realism pass (shader-only)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.9.0,spike,feature,ui,hud

## Story

Improve the EXISTING `assets/shaders/nova_os_crt.wgsl` overlay (no architecture
change) so the CRT reads more analog: soft, resolution-aware scanlines instead of
the current hard `fract(uv.y*240) < 0.5` step (feed the panel's pixel size as a
uniform so the line count tracks the real screen size and does not moire on
resize); a smooth/interpolated grain shimmer instead of the 9 Hz step reseed; an
optional subtle vertical slot-mask; and brightness-weighted noise so grain sits
more in the darker regions. Cheap, independent, reversible - improves feel even
if the render-to-texture pipeline is never built.

Keep it derivative-free (WebGL2-safe) and keep text readable.

## Notes

- Spike: `tasks/20260726-193040/SPIKE.md` (option C).
- Sibling tasks: casing/glass depth pass, and the RTT CRT pipeline.
- Verify with the `screenshot_nova_os` capture example.
