# Cover the NOVA OS computer end to end in systems/

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.11.0,examples,testing,ui,nova-os

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

- [x] Decide the host: a new `examples/systems/nova_os.rs` on a code-built
      fixture, or a beat set on whatever `20260804-093934` lands in `systems/`.
- [x] Open the computer with a synthesized `Tab` and advance on the real
      openness state, not a dwell.
- [x] Click a chin/app control THROUGH the CRT surface with the pointer
      vocabulary from `nova_autopilot::input`, and assert the forwarded pointer
      reached the offscreen subtree (hover + activation), not just the surface.
- [x] Assert the live tree after an app switch, so a duplicate or ghosted
      screen node fails the run.

## Definition of Done

- The run opens the computer and proves a click reaches the offscreen subtree.
  (cmd: `nix develop --command cargo run --features debug -- probe run systems --correctness-only`)
- The example is included in the catalog and the systems aggregate.

## Notes

- `20260804-093934` established `examples/systems/` and is closed.
- The pointer vocabulary (`click_named`, `hover_named`, `ui_node_centre`) was
  built by closed task `20260804-094021` - do not write another copy of
  `button_by_name`.
- The forwarded pointer is `nova_os_pointer_id()`; the mapping from screen to
  image UV is `nova_os_crt_screen_to_image_uv` and it is already unit-tested
  against the shader. The live claim is that the whole chain works at once.

## Closure (2026-08-17)

`examples/systems/nova_os.rs`, on a code-built fixture (one player ship, four
sections, a three-point rig). 16 beats, 6 invariants on the roster, and TWO
production bugs the range found on its first live run.

### The stale references

Both file references had moved. The unit tests are
`crates/nova_os_ui/src/terminal/tests/crt.rs` (the NOVA OS left `nova_gameplay`
for its own crate pair, `nova_os` for the model and `nova_os_ui` for the
renderer). `20260804-093934` did land `systems/`, so this is a new file beside
its runs rather than a beat set on one of them.

### What the range clicks, and why that one

The header's `[ ESC ]` app-close control (`NovaOsAppClose`, named by this task).
The chin controls the task offered as an alternative are WINDOW-space - part of
the physical casing, not the picture - so clicking one would prove nothing about
the glass. The close control is a real `Button` behind the image camera whose
`Activate` observer returns the shell to the prompt, which means the "did it get
through?" question is answered by `NovaOsTerminal::active_mode()` rather than by
the pointer agreeing with itself.

Three separable claims, so a failure says WHERE the chain broke:

- hover: bevy's own `HoverMap` under `nova_os_pointer_id()` contains the widget,
  and the MOUSE pointer's does not (window picking cannot reach an image-camera
  node at all - a mouse hit would mean the target was never offscreen).
- press: `bevy_ui::Pressed`, which `bevy_ui_widgets` puts on a `Button` it
  dispatched a press to.
- activation: the shell is back at the prompt and the app root is gone.

The hover walk goes up the ancestors. `bevy_ui` picking reports the DEEPEST node
under the pointer, which for a labelled button is its `Text` child; the first
draft compared hits to the widget itself and read a perfectly good click as a
miss. `mirror_nova_os_hover` walks the same way for the same reason.

### BUG 1: a synthesized click never reached the screen

`forward_nova_os_pointer` read `MessageReader<MouseButtonInput>` - the CONCRETE
message. `bevy_picking::input::mouse_pick_events`, whose pointer it mirrors,
reads `WindowEvent::MouseButtonInput` - the WRAPPER. `bevy_winit` writes both for
every real click, so the two streams agree under a human hand and the forwarder
looked correct. A synthesized click writes only the half picking reads
(`nova_autopilot::input::set_mouse_button`), so under EVERY driven run the
forwarded pointer hovered correctly and no press ever arrived. Silent, because
the sampling surface is `Pickable::IGNORE`: a click that reaches nothing hits
nothing and the run stays green.

Fixed by reading the wrapper, so the forwarded pointer and the pointer it
mirrors cannot disagree about whether a button went down. `pointer_rig::click_at`
now writes the wrapper ONLY - deliberately less than winit writes, because a rig
that writes both passes a forwarder with this bug. Reverting the read reds five
map/ship click tests.

### BUG 2: the glass was measured in physical pixels

The surface rect came straight off `ComputedNode` (PHYSICAL px) and was compared
against `Window::cursor_position` (LOGICAL px). Exact at scale factor 1, which is
every box this has run on; on a HiDPI display it reads the glass at half its size
and a quarter of the window it occupies, and every click misses by the scale
factor, worse the further from the origin. Both directions now go through one
`nova_os_glass_rect` that scales by `ComputedNode::inverse_scale_factor`, pinned
by `the_glass_rect_is_measured_in_logical_pixels`.

### Aiming at something behind a render target

A node laid out by the offscreen camera reports its rect in IMAGE pixels - a
space no cursor can be placed in - so a driven run could not point at the
terminal at all. `nova_os_window_px_showing` is `forward_nova_os_pointer` run
backwards against the live surface rect, image size and CRT power.

The inverse is closed-form apart from the barrel, which is radial: `|bowed| =
r(1 + warp*r^2)` is strictly increasing for `warp >= 0`, and its Newton solve has
a derivative never below 1, so it converges from any start. Round-tripped against
the forward mapping over a 201x201 grid at four powers, inside the same half-pixel
budget the forwarded pointer is held to.

Two corrections the round trip forced, both first written wrong:

- The rim needs slack. A point the forward mapping put EXACTLY on the picture's
  edge comes back a rounding step outside it, and an exact compare made the
  outermost row of the terminal the one thing nobody could aim at.
- The raster collapse does NOT crop. It squeezes where the picture is drawn
  toward the centre scan line, so a half-powered tube still shows every image
  point. What crops is the overscan, at any power. The first draft's gate and its
  test both described the wrong mechanism while passing.

### Openness, not a dwell

`nova_os_openness` (public, `&World`, so it backs a read-only predicate) reports
the live raster power. Two beats: `state_is(PauseStates::NovaOs)` for the open,
then the raster settling for the aim - a click aimed mid-slide lands where the
picture no longer is. The entity walk it costs is a scripted run's price, not a
per-frame system's.

### API surface added

`nova_os_pointer_id`, `nova_os_window_px_showing` and `nova_os_openness` are
public on `nova_os_ui::terminal`; `nova_os_ui` re-exports `nova_os`, and
`nova_core` re-exports `nova_os_ui`, so an example reaches them without a second
dependency. No prelude globs were widened - the example imports by name.

### Verdict

Live under Xvfb, exit 0, `autopilot: cycle complete, no panic (t=2.6s)`, all 16
beats through. `nova_os_ui --lib` 105 pass (12 in `terminal::tests::crt`, 4 of
them new); `catalog_drift` green at 113 roster invariants. Both bugs are
fail-first: bug 1 stalled this run before the fix and reds five rig tests when
reverted; bug 2 is covered by a scale-factor-2 rect test.
