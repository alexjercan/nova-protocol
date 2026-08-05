# Pointer-trace / jitter rig (exploratory, task 20260805-091151)

The instrumentation that pinned the `click_named` flake, kept as a patch
because it is a diagnostic, not shippable code. It proves the failure mode is
achievable; it is NOT the implementation.

Apply, build, run:

```sh
git apply tasks/20260805-091151/prototype/pointer-trace.patch
nix develop --command cargo build --example menu_newgame --features debug
```

Three env vars, all inert unless `NOVA_POINTER_TRACE` is set (the trace plugin
owns the other two):

| Env | Effect |
| --- | --- |
| `NOVA_POINTER_TRACE=1` | logs window events, `PointerInput`, the hover map, `Pressed`, and every `Pointer<Press\|Click\|Release>` |
| `NOVA_POINTER_JITTER=<n>` | every `n` frames, writes a `CursorMoved` at `(10, 10)` the way `bevy_winit` writes a real one (message only, no `Window` write) |
| `NOVA_POINTER_PIN=1` | re-asserts the driver's last synthesized position in `First`, after the real events and before `PickingSystems::Input` - the prototype of the fix |

The reproduction (fails 3/3 without the pin, passes 4/4 with it):

```sh
DISPLAY=:99 NOVA_AUTOPILOT=1 NOVA_POINTER_TRACE=1 NOVA_POINTER_JITTER=7 \
  nix develop --command cargo run --example menu_newgame --features debug
```

SUPERSEDED by the landed fix. This rig's stray is message-only, and the pin
that shipped detects a stray from `Window::cursor_position` - which is what
`bevy_winit` writes for a real one. The faithful rig, and the guard that
outlives both, is `crates/nova_autopilot/tests/pointer_pin.rs`.

Limitations: `NOVA_POINTER_JITTER` INJECTS the stray event. It reproduces the
failure signature exactly, but it does not prove which ambient X event fires in
CI - 218 runs across three ambient shapes never produced one. The pin numbers
come from `ui/` only, on one box, at scale factor 1.
