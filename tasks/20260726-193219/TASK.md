# NOVA OS CRT: casing + glass depth pass

- PRIORITY: 45
- TAGS: v0.9.0, spike, feature, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

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

## Steps

- [x] Capture the BEFORE state first (`render-output-eyeball` lesson): run the
      `screenshot_nova_os` example and store the shots under
      `tasks/20260726-193219/shots/`.
- [x] Round the casing stack in `setup_drawer`
      (`crates/nova_gameplay/src/hud/drawer.rs`): `BorderRadius` on the monitor
      root (asymmetric like the PoC `.case`: larger top, smaller bottom), the
      bezel, and the screen. Give the CRT overlay `MaterialNode` matching
      rounding - verify whether `BorderRadius` clips a MaterialNode in bevy_ui
      0.19; if not, add a cheap corner mask to `nova_os_crt.wgsl` (it will
      carry over into 193233's sampling shader).
- [x] Add depth via bevy_ui gradients (`BackgroundGradient`/`LinearGradient`,
      verified present in bevy_ui 0.19 `gradients.rs`): the PoC's 168deg
      case-body gradient with a 1px top highlight, bezel inset shading, and a
      recessed-plate gradient. Where a gradient fights a border, layer nodes
      instead.
- [x] Moulding seam: an inset 1px rounded-border node inside the casing (the
      PoC `.case::after`), light top / dark bottom.
- [x] Detailing: four corner screws (small full-radius nodes with a radial-look
      gradient + a rotated slot line via `UiTransform`) and the top-center vent
      strip (a row of thin dark nodes; a tiny repeating-pattern image asset is
      the fallback). Watch the `bevy-css-border-triangle-needs-contentbox`
      lesson if any border trick is used on zero-content nodes.
- [x] Chin bar: a bottom casing strip below the bezel (PoC `.chin`, ~54px).
      LEFT: the recessed brand plate - logo mark + "NovaCRT 9000" wordmark +
      "P22 green phosphor . 15 in . type CQ-4" spec line, dark stamped-in
      glyphs (inset gradient + light catch on the lower edge). Ship the logo
      mark as a small PNG under `assets/icons/` rendered from the PoC's SVG
      path (bevy UI does not draw SVG). RIGHT: an empty, named controls-row
      container node reserved for 20260726-214617.
- [x] Glass: a diagonal white specular sheen over the screen
      (`LinearGradient`, ~5% alpha fading to transparent, PoC `.glass`) plus
      one soft low-alpha highlight rectangle rotated via `UiTransform`; both
      `Pickable::IGNORE`, above content, below nothing interactive.
- [x] Phosphor rim: trace the screen edge with a thin bright phosphor border +
      a wider low-alpha glow border (two nested rounded nodes approximating
      the PoC `.rim` line/glow pair).
- [x] Update the drawer widget-tree tests: chin bar + plate + reserved
      controls slot exist, screw/vent nodes present, casing/bezel/screen carry
      the expected `BorderRadius`.
- [x] Capture AFTER shots, LOOK at them against
      `tasks/20260726-180807/shots/reference-html.png`, iterate until the
      device reads as moulded plastic + glass; record what changed, tradeoffs
      and self-reflection in `tasks/20260726-193219/NOTES.md`.

## Definition of Done

- The monitor tree carries the physical details: rounded casing/bezel/screen,
  seam, screws, vents, chin bar with brand plate and a reserved controls slot.
  (test: `drawer_monitor_has_physical_casing_details`)
- The CRT overlay respects the screen rounding - no green bleed outside the
  corner radius. (manual: AFTER capture inspection)
- The brand plate sits bottom-left with logo, wordmark and spec line reading
  as stamped-in. (manual: AFTER capture vs the PoC)
- Before/after captures exist under `tasks/20260726-193219/shots/` and the
  AFTER was eyeballed against the HTML reference. (manual: owner confirms the
  device reads as glass + moulded plastic)
- Touched tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)

## Notes

- Spike: `tasks/20260726-193040/SPIKE.md` (option B).
- Touches the casing/bezel/screen node tree in
  `crates/nova_gameplay/src/hud/drawer.rs`; round the CRT overlay corners to match.
- Verify with the `screenshot_nova_os` capture example.
- Blocks: 20260726-214617 (chin controls need the chin bar + plate).
- Priority re-slotted 0 -> 45 in the 2026-07-26 PoC fidelity review (see the
  v0.9.0 tracker's grooming history).
- Verified at planning (2026-07-26): bevy_ui 0.19 ships `BorderRadius`
  (already used in `nova_ui/src/widget.rs` and several HUD files) and UI
  gradients (`gradients.rs`: `LinearGradient`, `Gradient`,
  `BackgroundGradient`); `UiTransform` rotation is already used by
  `screen_indicator.rs`. Unverified and flagged in the step: whether
  `BorderRadius` clips a `MaterialNode`.
- The casing appears/disappears with the drawer's openness; 193233 later
  replaces the fade mapping with the power collapse - nothing here depends on
  which mapping is live.
