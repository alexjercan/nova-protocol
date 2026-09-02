# Add independent mouse sensitivity settings

- STATUS: CLOSED
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

## Proof

- `cargo fmt` on the eight changed crates' files: clean.
- `cargo check -p nova_input -p nova_ship -p nova_menu --all-targets`: clean, no
  warnings.
- `cargo check -p nova_menu --target wasm32-unknown-unknown`: clean. The shared
  settings store is the only WASM-visible seam and it compiles.
- `cargo test -p nova_input --lib sensitivity`: 3 passed. The ranges project
  percentages onto their own raw span, a corrupt or out-of-range value clamps,
  and moving a gain reaches a binding that already exists.
- `cargo test -p nova_ship --lib "camera::"`: 22 passed, including
  `the_look_sensitivity_scales_mouse_look_and_never_the_stick`,
  `the_free_camera_sensitivity_scales_only_its_own_mouse_look` and
  `the_free_camera_sensitivity_never_touches_keyboard_movement`.
- `cargo test -p nova_ship --lib flight_rig`: 13 passed, including
  `the_rcs_sensitivity_scales_the_mouse_and_never_the_stick`.
- `cargo test -p nova_menu --lib settings`: 36 passed, including
  `the_mouse_sensitivities_persist_default_and_clamp`,
  `the_mouse_group_shows_three_sensitivity_sliders_and_no_reset`,
  `dragging_a_sensitivity_slider_sets_only_its_own_path` and
  `dragging_a_sensitivity_slider_ticks_once_per_detent_and_not_while_it_rests`.
- Skipped on instruction: the full workspace test suite and workspace Clippy.

## Decisions the spec did not settle

- The gain lives on the BINDING, not in the observer: a `MousePath` component
  tags a mouse-motion binding and one `PreUpdate` system writes its `Scale`
  before `EnhancedInputSystems::Update`. `camera_rotate` is one action read by
  both the mouse and the right stick, so scaling in the observer would have
  scaled the pad too. Tagging also makes "reaches a rig that already exists"
  fall out for free - no rig rebuild, nothing to re-spawn on resume.
- RCS lost its `RCS_STICK_SCALE` cancel. The mouse gain used to be applied in
  the observer, which forced the stick to carry `1.0 / RCS_AIM_SENSITIVITY` to
  undo it. With the gain on the mouse binding the stick needs no modifier and
  full deflection is full intent, which is what it already was.
- The sliders speak percent and the resource stores raw. `SliderRange` is in
  percentage points and `SliderStep` is `(max - 100) / 20`, so whole-percent
  readouts and 20 equal detents come from the widget rather than from rounding.
- The Audio row builder is now shared (`SliderRow` / `build_slider_row`) instead
  of copied. Entity names are unchanged, so the audio tests still key on them.
