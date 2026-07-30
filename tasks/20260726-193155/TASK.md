# NOVA OS CRT: scanline + grain realism pass (shader-only)

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.9.0,spike,feature,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

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

## Assumptions

- The overlay shader cannot sample the terminal content behind it (Bevy UI
  materials get no back-buffer). So "grain sits more in the darker regions" is
  implemented as weighting grain by the overlay's own darkness - stronger toward
  the vignetted edges, quieter in the bright centre where the text sits - not by
  the actual glyph luminance. Same for any brightness weighting.

## Steps

- [x] Add a `resolution: vec2<f32>` uniform to `NovaOsCrtMaterial` /
      `NovaOsCrtUniform` (Rust + WGSL), defaulting to zero, and feed it each frame
      from the CRT overlay node's `ComputedNode` size in the existing
      `animate_nova_os_crt` system (which already runs while the computer is open
      and sets `time`). Guard against a missing/zero size (headless/first frame).
- [x] Rewrite the scanlines: derive line spacing from `resolution.y` so the lines
      are a fixed few physical pixels apart (resolution-aware, no moire on
      resize) and use a SOFT profile (sinusoidal / smoothstep) instead of the hard
      `fract(uv.y*240) < 0.5` step. Fall back to a sane fixed density when
      `resolution` is still zero. Keep `scanline_strength` as the tunable.
- [x] Smooth the grain shimmer: interpolate the fine grain between consecutive
      time steps (lerp by `fract(time*rate)`) instead of the hard
      `floor(time*9)` reseed, so the movement is analog, not steppy. Keep the
      coarse layer static.
- [x] Add an optional subtle vertical slot-mask (aperture-grille) derived from
      `resolution.x`, low-strength so it textures without dimming the text.
- [x] Brightness-weight the grain by the overlay's darkness (stronger toward the
      vignetted edges, quieter in the bright centre) per the Assumption above.
- [x] Update the material `Default` + the `drawer_uses_crt_material_overlay...`
      test for the new field, and add a wiring test that runs
      `animate_nova_os_crt` against a spawned overlay with a `ComputedNode` and
      asserts `material.data.resolution` is populated (and `time` set).
- [x] Capture a before/after with the `screenshot_nova_os` example into
      `tasks/20260726-193155/shots/`; read the AFTER before close-out and confirm
      softer scanlines + livelier grain with text still readable.
- [x] Write NOTES.md: shader changes, the resolution-feed wiring, tradeoffs
      (WebGL2/derivative-free constraints), difficulties, self-reflection.

## Definition of Done

- `NovaOsCrtMaterial` carries a `resolution` uniform that `animate_nova_os_crt`
  populates from the CRT overlay node's `ComputedNode` each frame. (test:
  `nova_os_crt_material_receives_resolution_and_time`)
- The scanlines are soft and resolution-derived, not the hard fixed step: the
  shader no longer contains `fract(uv.y * 240.0)` and references `resolution` for
  the line spacing. (cmd:
  `! grep -q 'fract(uv.y \* 240.0)' assets/shaders/nova_os_crt.wgsl` and cmd:
  `grep -q 'resolution' assets/shaders/nova_os_crt.wgsl`)
- The grain shimmer interpolates between steps rather than hard-reseeding: the
  shader no longer contains `floor(material.time * 9.0)`. (cmd:
  `! grep -q 'floor(material.time \* 9.0)' assets/shaders/nova_os_crt.wgsl`)
- Touched drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)
- Formatting and build checks pass. (cmd:
  `nix develop --command cargo fmt --check` and cmd:
  `nix develop --command cargo check`)
- manual: the AFTER `screenshot_nova_os` capture shows softer, resolution-aware
  scanlines and a livelier-but-subtle grain, with the terminal text still crisp
  and readable.

## Notes

- Spike: `tasks/20260726-193040/SPIKE.md` (option C).
- Sibling tasks: casing/glass depth pass (`20260726-193219`), and the RTT CRT
  pipeline (`20260726-193233`).
- HTML prototype of the target feel: `examples/ui/nova_os_terminal_poc.html`
  (subtle scanlines + animated grain).
- Verify with the `screenshot_nova_os` capture example.
