# Scroll wheel binding for component cycle

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.4.0,input,targeting

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
