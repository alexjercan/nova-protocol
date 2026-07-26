# NOVA OS CRT: casing + glass depth pass

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.9.0, spike, feature, ui, hud

## Story

Make the physical monitor read as glass + moulded plastic rather than flat
panels, using Bevy UI nodes (and a small casing material if needed) - no
render-to-texture. Rounded screen corners via `BorderRadius`; a soft glass
specular-highlight overlay (a diagonal white gradient like the HTML screen
sheen); bezel/casing inner-bevel or gradient for depth instead of flat borders;
and small detailing (screws/vents) beyond the two accent slots. Independent of
the shader work and low risk.

Scope extension (2026-07-26 PoC fidelity review): this task also owns the
monitor CHIN - the PoC's bottom casing strip under the bezel (`.chin` in
`examples/ui/nova_os_terminal_poc.html`) with the recessed brand plate on the
bottom LEFT: the NovaCRT 9000 logo mark + wordmark + spec line ("P22 green
phosphor . 15 in . type CQ-4"), dark glyphs stamped INTO the plastic (the
pressed-in look via a light catch on the lower edge, as close as Bevy UI
allows; ship the logo mark as a small image/svg-derived asset if UI nodes
cannot draw it). The chin's right side reserves layout space for the controls
row; making those controls FUNCTIONAL is task 20260726-214617, which depends
on this one for the geometry. Also add the phosphor rim glow tracing the
screen edge (the PoC `.rim` layer) and the moulding-seam inner outline
(`.case::after`) if they read well in Bevy UI.

## Notes

- Spike: `tasks/20260726-193040/SPIKE.md` (option B).
- Touches the casing/bezel/screen node tree in
  `crates/nova_gameplay/src/hud/drawer.rs`; round the CRT overlay corners to match.
- Verify with the `screenshot_nova_os` capture example.
- Blocks: 20260726-214617 (chin controls need the chin bar + plate).
- Priority re-slotted 0 -> 45 in the 2026-07-26 PoC fidelity review (see the
  v0.9.0 tracker's grooming history).
