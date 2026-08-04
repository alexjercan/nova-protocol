# Cover the NOVA OS computer end to end in systems/

- PRIORITY: 0
- TAGS: backlog,examples,testing,ui
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- DEPENDS ON: 20260804-093934

## Story

Cover the NOVA OS ship computer as a whole system in `examples/systems/`:
open it the way a player does (Tab), drive its shell, and take a real click
through the render-to-texture CRT screen.

Seeded by `20260804-094021` (DECISION D1). The RTT pipeline's structure,
uniforms, pointer mapping and hover mirroring are unit-tested
(`crates/nova_gameplay/src/hud/nova_os/tests/crt.rs`), and
`rtt_element_renders_its_subtree` adds the image-camera claim. What no test and
no run ASSERTS is the live one: press Tab in a real app, the computer opens,
and a click lands on a widget that lives behind an image camera.

It is `systems/`, not `ui/`: per the category contract the subject is the
nova_os system (terminal model, shell, app runtime, the CRT pipeline over it),
not a staged interface flow, and the `ui/` roster is fixed at five runs by the
spike `20260804-003244`.

Not urgent: `screenshot_nova_os` still opens the computer under CI smoke and
exits clean, so a panic on the RTT path still fails the build. What is missing
is assertions.

## Steps

- [ ] Decide the host: a new `examples/systems/nova_os.rs` on a code-built
      fixture, or a beat set on whatever `20260804-093934` lands in `systems/`.
- [ ] Open the computer with a synthesized `Tab` and advance on the real
      openness state, not a dwell.
- [ ] Click a chin/app control THROUGH the CRT surface with the pointer
      vocabulary from `nova_autopilot::input`, and assert the forwarded pointer
      reached the offscreen subtree (hover + activation), not just the surface.
- [ ] Assert the live tree after an app switch, so a duplicate or ghosted
      screen node fails the run.

## Definition of Done

- The run opens the computer and proves a click reaches the offscreen subtree.
  (cmd: `nix develop --command cargo run -p nova_probe -- run systems`)
- The example is smoked with its category.
  (test: `systems_reach_playing_without_panic`)

## Notes

- Needs `20260804-093934` first: it establishes `examples/systems/` and its
  smoke list.
- The pointer vocabulary (`click_named`, `hover_named`, `ui_node_centre`) is
  built by `20260804-094021` - do not write a fourth copy of `button_by_name`.
- The forwarded pointer is `nova_os_pointer_id()`; the mapping from screen to
  image UV is `nova_os_crt_screen_to_image_uv` and it is already unit-tested
  against the shader. The live claim is that the whole chain works at once.
