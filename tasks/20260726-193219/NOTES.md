# NOTES: NOVA OS CRT casing + glass depth pass (20260726-193219)

Spike option B (no render-to-texture). Pure Bevy UI-node + material-shader work
in `crates/nova_gameplay/src/hud/drawer.rs` + `assets/shaders/nova_os_crt.wgsl`.

## What changed

- **Rounded casing stack.** `BorderRadius` moved into `Node.border_radius`
  (it is a Node FIELD in bevy_ui 0.19, not a standalone component): asymmetric
  shell corners (top 24 / bottom 15, mirroring the PoC `.case`
  `22 22 14 14`), rounded bezel (16) and screen (12).
- **Depth via gradients.** New `BackgroundGradient` helpers: the 168deg case
  body (lit top -> deep undercut) with a 1px top moulding-lip highlight
  (`nova_os_case_gradient`), a dark vertical bezel gradient
  (`nova_os_bezel_gradient`), and the recessed brand-plate gradient. Per-side
  `BorderColor` gives the bezel its dark-top / light-bottom recessed lip.
- **CRT overlay respects the rounding.** A UI `MaterialNode` is NOT clipped by
  its node's `BorderRadius`, so the overlay masks its own corners in-shader: a
  new `corner_radius` uniform (appended last, so the Rust/WGSL field order still
  matches - see the `shader-uniform-field-order-must-match-wgsl` ledger lesson)
  drives a rounded-rect SDF that clips the phosphor film to the screen edge.
  Zero disables it for headless rigs. Carries into 193233's sampling shader.
- **Detailing.** Four corner screws (full-radius disc + diagonal light->dark
  gradient + a FILLED rotated slot bar, not a border trick - dodges the
  `bevy-css-border-triangle-needs-contentbox` collapse); a top-centre vent
  grille (row of thin dark slats); the moulding seam (`.case::after`): a 1px
  inset rounded outline, light top/left, dark bottom/right.
- **Chin bar.** Bottom casing strip (~54px) below the bezel. LEFT: the recessed
  brand plate - a logo mark + "NOVACRT 9000" wordmark + "P22 GREEN PHOSPHOR . 15
  IN . TYPE CQ-4" spec line, dark glyphs with a 1px light `TextShadow` catch for
  the stamped-in look. RIGHT: an empty, named `NovaOsControlsRow` reserving
  layout space for task 20260726-214617.
- **Glass + phosphor rim.** A diagonal 118deg specular sheen over the screen
  plus a soft upper-left reflection; the phosphor rim traces the screen edge as
  a wider low-alpha glow border under a thin bright line. All `Pickable::IGNORE`.
- **Logo asset.** Bevy UI cannot draw SVG, so the PoC `.mark` compass burst was
  rendered to `assets/icons/nova_crt_mark.png` (96x96) via `inkscape`, with a
  baked light-catch underlayer for the pressed-in look.

## Bugs / difficulties

- **`BorderRadius` is a `Node` field, not a component.** First compile failed on
  every bundle carrying a standalone `BorderRadius` ("not a Bundle"). Confirmed
  against existing `nova_ui/src/widget.rs` (`Node { border_radius: ... }`) and
  the vendored `bevy_ui-0.19.0/src/ui_node.rs`; moved all radii into the Node.
- **Hard-edged glass card.** The first glass highlight was a solid
  `BackgroundColor` rectangle. Bevy UI has no blur, so it read as a grey sticky
  note, not a reflection (caught by the AFTER capture - the
  `render-output-eyeball` lesson earning its keep). Replaced with a
  `RadialGradient` fading to transparent, which gives soft edges without a blur.
- **Test rig missing the `Image` asset.** The chin loads a logo image, so
  `spawn_drawer_shell_with_crt` had to `init_asset::<Image>()` (it already
  registered `Font`/`NovaOsCrtMaterial`); production gets it via DefaultPlugins.

## Verification

- `cargo test -p nova_gameplay drawer`: 56 passed (incl. the new
  `drawer_monitor_has_physical_casing_details`).
- BEFORE baseline: `tasks/20260726-180807/shots/` (current-master monitor).
  AFTER: `tasks/20260726-193219/shots/after/` - eyeballed against
  `reference-html.png`; the device now reads as moulded plastic + glass
  (rounded corners, screws, vents, seam, chin plate, rim, soft sheen), no green
  bleed past the rounded screen corners.

## Self-reflection

- Verifying the Node/component split and the gradient API against the vendored
  bevy source BEFORE writing the tree would have saved the first failed compile;
  I checked the gradient API but assumed `BorderRadius` was still a component.
  Grep the actual struct for anything you have not personally used in-tree.
- The eyeball pass was decisive twice (the glass card, then confirming the
  final read). A widget-tree test alone would have shipped the grey card green.
