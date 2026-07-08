# Harnessed editor example for smoke testing

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.4.0, example, editor, testing

The gameplay examples (03/06/07/08) are wired to the autopilot + screenshot harness and run in
`examples_smoke`, but the editor - which is the default "game" the `nova_protocol` binary runs -
had no headless coverage. This was called out in the 20260706-212909 retro: editor-preview bugs
could only be argued, not demonstrated. Add an example that runs the same editor with the harness.

Also: the editor-app construction should be shared, not open-coded per launcher, so the example
tests the exact app the game ships (the user asked to "factor it into a module if it's not
already a separate module").

## Steps

- [x] Factor the editor-app construction into `editor_app(render)` in nova_core (the editor is
      `AppBuilder`'s default game); use it from both the binary (`src/main.rs`) and the example.
- [x] Add `examples/09_editor.rs`: `editor_app(true)` + the autopilot/screenshot harness. The
      autopilot, once in Playing, activates the "create a ship with a controller" button once, so
      the run exercises a real editor action (the controller-preview path) rather than just booting.
- [x] Register `09_editor` in `tests/examples_smoke.rs`.
- [x] Green: headless run reaches Playing, creates the ship, cycle complete, no panic, and (as a
      bonus regression on 20260706-212909) no `root not found` PD spam; `cargo clippy
      --workspace --all-targets`; `cargo test --workspace` (examples_smoke runs 09_editor);
      binary builds in both feature modes.

## Resolution

`editor_app(render)` in nova_core is now the single editor-app constructor, shared by the binary
and the new example, so the example is the same editor the game runs (plus the harness).

`examples/09_editor.rs` attaches `nova_autopilot().input(editor_autopilot)` + `nova_screenshot()`.
The autopilot waits for `GameStates::Playing` (the editor switches its inner state to Editor
there), finds the "Create New Spaceship Button V2" entity by `Name`, and triggers a
`ui_widgets::Activate` on it exactly once (guarded by a `EditorAutopilotDone` resource). That runs
`create_new_spaceship_with_controller`, i.e. the editor's controller-preview path.

Headless run: `reached Playing` -> `created a ship with a controller` -> `cycle complete, no
panic`, with no `root not found in q_root` in the log - so it doubles as a regression test for the
20260706-212909 preview-controller fix.

## Notes

Driving the button via `Activate` on a `Name`-matched entity is the seam that makes editor UI
actions reachable from the autopilot (the editor has no keyboard shortcuts for these). The
example only exercises the create-with-controller action; placing sections needs pointer picking
on the grid, which the autopilot does not simulate - a possible future extension. The
`bevy::prelude` import is gated to `debug` because only the harness code names bevy types.
