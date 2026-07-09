# Scroll wheel binding for component cycle

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.4.0, input, targeting

Playtest request (20260709): besides `[`/`]` and dpad, bind the component
cycle to the scroll wheel - up = next, down = prev.

bevy_enhanced_input 0.26 supports it: `Binding::mouse_wheel()` emits the
wheel as an axis (y = vertical); for the bool cycle actions, bind
`(Binding::mouse_wheel(), SwizzleAxis::YXZ, Clamp positive)` for next and
the same with `Negate` before the clamp for prev, so only the matching
scroll direction actuates. Add to the flight rig actions! block in
input/player.rs next to the existing bracket/dpad bindings; check whether
the camera or thruster contexts already consume wheel input (consume_input
is false on the flight rig, so coexistence should be fine - verify).

## Resolution (20260709)

Shipped: scroll wheel bound to the component cycle on the flight rig -
`(Binding::mouse_wheel(), SwizzleAxis::YXZ, Clamp::pos())` for next
(scroll up) and the same chain with `Negate::all()` before the clamp for
prev (scroll down), alongside the existing bracket keys and dpad. No other
system consumes the wheel (grep-verified), and consume_input stays false.
Binding data only - the observers and their gates are unchanged and already
tested; compile + input tests green. Direction convention (up = next)
matches the nose-to-tail cycle order. Verified by inspection; needs the
user's playtest for feel (wheel sensitivity produces one Start per wheel
detent).

Skipped honestly per user instruction: full local suite and clippy.
