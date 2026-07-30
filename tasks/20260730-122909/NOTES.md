# NOTES: the measured collapse and the measured fix

## Fail-first evidence (2026-07-30)

The rig is `crates/nova_gameplay/src/hud/chip_layout_rig.rs` - an App with
bevy_ui's real taffy layout and bevy_text's real measurement (`default_font`,
no font handle, exactly what the chips use in game). The first run of
`the_objective_chip_backs_its_whole_label`, against the UNCHANGED chip bundle:

```
objective marker chip: the chip fill Vec2(20.0, 10.0) is smaller than an
independent layout of "BEACON 1" (Vec2(58.0, 15.0)) plus the chip's own
padding/border (needs at least Vec2(78.0, 25.0)) - the background covers only
part of the text
```

20x10 px is exactly `chip_node()`'s frame: 9+9 padding + 1+1 border wide,
4+4 + 1+1 tall. The glyphs rendered 58 px wide over it - the reported "label
square in the top left of the text".

## Mechanism, confirmed against the engine (not against theory)

`taffy_drops_the_text_measure_when_a_text_node_has_children` (in the rig's own
tests) lays out the SAME `chip_node() + Text("BEACON 1")` bundle twice, once as
a leaf and once with a single absolutely-positioned child:

- leaf: measures its text, box > frame;
- with a child: box == the frame EXACTLY (20x10), the text measure dropped.

taffy runs a node's measure function only on the leaf path, so a `Text` node
that also has children becomes a container and measures its (empty) in-flow
content instead. That test stays as a regression pin: if a future bevy measures
container text too, it fails and the chips can be simplified again.

## After the fix

Same rig, same string, objective chip:

```
chip  = size Vec2(94.0, 25.0)  rect (0,0)..(94,25)  frame 10/5 per side
label = size Vec2(58.0, 15.0)  rect (26,5)..(84,20)
```

94 = 8 (diamond) + 8 (chip_node column_gap) + 58 (label) + 18 (padding) + 2
(border); the label sits fully inside the content box (10..84, 5..20). The
beacon chip passes the same shared assertion with no diamond.

## Eyeball (ledger `render-output-eyeball`)

`screenshot_combat` grew a `hud-nav-chips.png` beat: a plain nav beacon and a
marked objective spawned side by side for one frame (the dedupe means one entity
cannot show both chips), captured, then torn down. Run under Xvfb on the real
GPU; both pills are full-width, the diamond sits inside the gold pill, and each
chevron parks centred over its pill.

## Sweep (DoD 3)

The multiline `Text::new ... children!` grep over `crates/nova_gameplay/src/hud`
and `crates/nova_ui/src` matched 7 more files
(comms_panel, edge_indicators, keybind_dock x2, nova_os, objective_stack,
torpedo_target) plus the rig's own deliberate case. All reviewed by hand: every
one is a sibling-then-children shape - the `Text` is a leaf and the `children!`
belongs to a different entity. `objective_stack` and `torpedo_target` already
use the container + text-child shape the chips just moved to. No further real
hits.

## Bugs hit while building this

- The rig's first App panicked every frame on `Resource does not exist`.
  `UiPlugin` pulls in the accessibility and picking backends and runs
  `ui_focus_system`, and `widget::viewport_picking` + the text/image content
  passes need `HoverMap`, `PointerState`, `Assets<Image>` and
  `Assets<TextureAtlasLayout>` - all supplied by render plugins the rig does not
  want. Fixed by adding `AccessibilityPlugin`, `InputPlugin`, `PickingPlugin`,
  `InteractionPlugin` and registering the two asset collections directly. The
  system name is hidden without bevy's `debug` feature; `BEVY_BACKTRACE=full`
  and grepping the backtrace for the system's parameter signature named it.
- The first capture beat respawned its two chip subjects every frame: the
  teardown used `chip_subjects.take()`, which reset the spawn guard back to
  `None`. It flooded the log with `insert_beacon_render` errors and starved the
  rest of the script (`feature-autopilot.png` never fired). Fixed with separate
  `chips_spawned` / `chips_gone` flags.
- Hand-assembling the shot's beacon from `BeaconMarker + BeaconLabel` tripped
  the render observer (no `BeaconRenderConfig`) and B0004 (no `Visibility` for
  the orb child). Spawning the real `beacon_scenario_object(BeaconConfig)`
  bundle instead is both quiet and production-faithful.
