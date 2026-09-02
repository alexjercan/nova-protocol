# Add independent mouse sensitivity settings

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.13.0, input, settings, ui

Add three independent mouse sensitivity controls to Settings > Controls. Mouse
look, RCS translation, and the free camera have different feel requirements and
must not share one gain.

## Decisions

- Add a dedicated `MOUSE` Controls group containing:
  - `Look Sensitivity`
  - `RCS Sensitivity`
  - `Free Camera Sensitivity`
- Use the Audio slider presentation: a ticked track, a live whole-percent
  readout, and the existing UI detent tick sound. Use 20 equal slider intervals.
- Percentages are relative to each setting's own raw `100%` baseline:

  | Setting | Range | Default | Raw range | Pre-change behavior |
  | --- | ---: | ---: | ---: | ---: |
  | Look | `100%..=300%` | `200%` | `0.000333333..=0.001` | `300%` |
  | RCS | `100%..=500%` | `100%` | `0.03..=0.15` | `100%` |
  | Free Camera | `100%..=300%` | `200%` | `0.005..=0.015` | `200%` |

  Look and Free Camera therefore step by 10 percentage points. RCS steps by 20
  percentage points. Store and apply the raw engine values; percentages are the
  player-facing projection.
- Look affects the mouse side of normal ship steering, free look, and turret
  aim. RCS affects only mouse-driven RCS translation. Free Camera affects only
  mouse look on `WASDCameraController`; it does not change keyboard movement.
- Never scale gamepad sticks with these settings.
- Changes apply immediately from both the main-menu and pause-menu Settings
  panels, including to input rigs that already exist.
- Persist all three values through the native and WASM settings store. Missing
  fields in an older store use the defaults above. In particular, the new Look
  default is intentionally about two-thirds of the pre-change gain.
- Do not add a sensitivity reset button. The existing binding reset remains a
  keybinding operation and does not appear as a misleading reset on `MOUSE`.

## Acceptance

- The `MOUSE` group shows the three named sliders with the agreed ranges,
  defaults, detents, live percentage labels, and one UI tick per crossed detent.
- Each slider changes only its named mouse path. Tests prove that the other two
  mouse paths and the corresponding gamepad path keep their values.
- A sensitivity changed while paused takes effect after resuming without a
  respawn or scenario reload.
- The persisted settings round-trip non-default raw values. A pre-sensitivity
  settings blob still loads with all three defaults.
- Slider values are clamped when loading corrupt or out-of-range persisted
  numbers.
- A focused menu/settings test covers the new group and labels. Focused ship
  input tests cover the three gains and gamepad isolation.
- Add one concise `[Unreleased]` changelog entry for the player-facing controls
  change.

## Verification

Run only focused formatting, `nova_menu` settings tests, and affected
`nova_ship` input/camera tests. Check both native persistence behavior and the
WASM-safe compile path affected by the shared settings store.
