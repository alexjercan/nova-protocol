# Notes: NOVA OS terminal input and command shell

- TASK: 20260726-115324

## What changed and why

The terminal shell lives in `crates/nova_gameplay/src/hud/drawer.rs` beside the
NOVA OS monitor it drives. That keeps the first slice close to the existing
`PauseStates::Drawer` owner instead of introducing a second pause mode or a
shared widget before another surface needs one.

The drawer now owns a `NovaOsTerminal` resource with prompt text, cursor,
scrollback, command history, completion hints, parse status and prompt mode. A
drawer-only keyboard system reads `KeyboardInput` while `PauseStates::Drawer` is
active and handles typed text, Enter, Backspace/Delete, Left/Right, Up/Down and
Tab completion. `help` and `clear` are the first command registry entries.
Gameplay-mutating commands stay deferred to the later output/runtime tasks.

## Input routing decisions

Tab is split by state. From `PauseStates::Unpaused`, Tab opens NOVA OS. From
`PauseStates::Drawer`, Tab is reserved for terminal autocomplete and does not
close the monitor. Escape remains owned by `nova_menu::toggle_pause`, so closing
NOVA OS still uses the accepted pause/freeze lifecycle and returns straight to
`Unpaused`. The gamepad right-stick click keeps its old symmetric drawer toggle
because it does not conflict with terminal completion.

## Difficulties

The first implementation of the Tab split made every drawer-mode toggle inert,
including the gamepad right-stick close path. The existing
`pad_toggles_drawer_state` test caught the regression. The fix was to split the
conditions in `toggle_drawer`: keyboard Tab is autocomplete while open, but pad
close still sets `PauseStates::Unpaused`.

The terminal UI rebuild originally used two mutable text queries filtered by
different markers. Bevy could not prove those queries were disjoint and raised
error B0001 in the render test. Switching the prompt/hint queries to a
`ParamSet` made the access explicit.

## Self-reflection

The existing drawer suite did useful integration work: it caught the gamepad
behavior that the new Tab-focused tests would have missed. Next time a keyboard
binding is split by mode, check sibling inputs that share the same function
before assuming only the named key changed.
