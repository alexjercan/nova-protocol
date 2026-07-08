# Drive section placement in the editor autopilot

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0, example, editor, testing

Follow-up to the harnessed editor example (20260708-100000). That example only clicked the
"create ship with a controller" button; the retro flagged extending it to drive section
*placement*, which needs simulating the grid pointer picking.

## Steps

- [x] Extend `examples/09_editor.rs`'s autopilot into a frame-paced state machine: create a ship,
      select a hull section, then click on the ship to place the section.
- [x] Drive placement through the real picking pipeline: select the section by inserting `Pressed`
      on its button (the editor's `button_on_setting` sets `SectionChoice` on `Add<Pressed>`), then
      inject a synthetic mouse `PointerInput` over an existing section so avian's physics-picking
      backend raycasts a hit and the editor's own `on_click_spaceship_section` places the section.
- [x] Verify placement actually happened (section count grows) and confirm it is deterministic.
- [x] Green: headless run (create -> select -> place 1->2 sections -> cycle complete, no panic,
      no `q_root` spam), repeated 3x with the same result; `cargo clippy --workspace
      --all-targets`; `cargo test --workspace` (examples_smoke runs 09_editor).

## Resolution

`09_editor`'s autopilot is now a small state machine (`Phase`: CreateShip -> SelectSection -> Aim
-> Press -> Release -> Verify -> Done), each phase waiting a few frames for the editor/physics to
catch up. Placement is driven entirely through public input - no editor code changed:

- Select: insert `bevy::ui::Pressed` on the hull section's button (found by `Name` from the
  `GameSections` catalog), which the editor's `button_on_setting` turns into `SectionChoice`.
- Aim: project a preview section's world position to the screen with `Camera::world_to_viewport`
  (the editor camera is static during the autopilot, so this is deterministic) and build a pointer
  `Location` from the camera's `RenderTarget`.
- Press/Release: write synthetic `PointerInput` messages (Move, then Press/Release Primary) for
  the mouse pointer; avian's `PhysicsPickingPlugin` raycasts the section's collider and generates
  the `Pointer<Press>` the editor's `on_click_spaceship_section` observer consumes, placing the
  new section one unit along the hit normal.
- Verify: assert the `SectionMarker` count grew (1 -> 2).

Headless: `created a ship with a controller` -> `selected the 'Reinforced Hull Section' section`
-> `placed a section (1 -> 2 sections)` -> `cycle complete, no panic`, deterministic across
repeats.

## Notes

Driving the real physics-picking pipeline (rather than faking the placement) means the example
now regression-covers the whole editor placement path - camera projection, picking, and the
placement observer - headless. `examples_smoke` asserts no-panic/clean-exit, not the specific
"placed a section" line, so a picking miss would not fail CI; the run is deterministic in
practice (static camera + known section position). Placing further section kinds (thruster/turret
with their input-binding branches) is a possible extension.
