# NOVA OS CRT: casing + glass depth pass

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.9.0,spike,feature,ui,hud

## Story

Make the physical monitor read as glass + moulded plastic rather than flat
panels, using Bevy UI nodes (and a small casing material if needed) - no
render-to-texture. Rounded screen corners via `BorderRadius`; a soft glass
specular-highlight overlay (a diagonal white gradient like the HTML screen
sheen); bezel/casing inner-bevel or gradient for depth instead of flat borders;
and small detailing (screws/vents) beyond the two accent slots. Independent of
the shader work and low risk.

## Notes

- Spike: `tasks/20260726-193040/SPIKE.md` (option B).
- Touches the casing/bezel/screen node tree in
  `crates/nova_gameplay/src/hud/drawer.rs`; round the CRT overlay corners to match.
- Verify with the `screenshot_nova_os` capture example.
