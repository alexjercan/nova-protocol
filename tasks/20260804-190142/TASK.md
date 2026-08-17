# Synthesize wheel scroll in nova_autopilot so driven runs can reach a row past the fold

- STATUS: CLOSED
- PRIORITY: 94
- TAGS: v0.11.0,autopilot,testing,ui

## Story

`nova_autopilot::input` synthesizes pointer motion and buttons but no WHEEL
event, so a driven run cannot scroll a list. The scenarios picker DOES
support wheel scroll (`scroll_menu_lists`, `crates/nova_ui/src/widgets.rs:72`),
so `examples/ui/menu_scenarios.rs` permanently skips the row past the fold and
reports 5 of 6 rows measured. That coverage gap is a property of the HARNESS,
not of the UI.

Surfaced as a review process signal on 20260804-094021 (rounds 3 and 4).

## Steps

- [x] Add wheel synthesis to `crates/nova_autopilot/src/input.rs` beside the
      existing pointer vocabulary, writing the same `WindowEvent` the UI
      scroll systems read, with unit tests in that module`s `mod tests`.
- [x] Use it in `examples/ui/menu_scenarios.rs` so a row reported as
      `RowPlacement::Unreached` because its centre is outside the list box is
      scrolled into view and then clicked, instead of skipped.
- [x] The run reports every row measured and none skipped.

## Closure (2026-08-17)

Both files had moved since the task was filed: the reader is
`scroll_viewports` in `crates/nova_ui/src/screen/scroll.rs`, and the example is
`examples/systems/menu_picker.rs`.

### The driver

`scroll_lines(f32)` and `scroll_pixels(f32)` beside the other pointer gestures,
over one `turn_wheel` body. Two constructors rather than one, because the units
are not interchangeable to a reader: `scroll_viewports` multiplies a LINE by its
own 20 px line height and takes a PIXEL as it stands, so a beat that has
MEASURED its gap must be able to spend it without knowing the reader's line
height. Vertical only, with gamepad and touch - nothing in the fleet scrolls
sideways.

`turn_wheel` writes BOTH the concrete `MouseWheel` message and the
`WindowEvent::MouseWheel` wrapper. The task said "the same `WindowEvent` the UI
scroll systems read", but the two halves have DIFFERENT readers and neither is
optional: `scroll_viewports` reads the concrete message, and `bevy_picking`
builds `PointerAction::Scroll` from the wrapper alone
(`bevy_picking-0.19.0/src/input.rs:175`). `bevy_winit` writes both for every
real notch (`state.rs:330` and `state.rs:846`), so this does too. Unlike a
button press there is no `ButtonInput`-style accumulator in between, so nothing
lands a frame late and no message has to be withheld.

### The example

`RowPlacement` gained `PastTheFold(f32)`: laid out, but outside the list box on
Y, carrying the logical-pixel wheel delta that brings it in with one row-height
of margin. The walk aims the pointer at the list first (the wheel goes to the
pane under the pointer, and after a selection the pointer is still on a row that
may itself have scrolled away), turns the wheel, then waits ONE driven frame -
the scroll lands in `Update` and layout moves the rows in `PostUpdate`, so
looking again immediately would read pre-scroll rects and scroll a second time,
overshooting.

A fold on X stays `Unreached`: the list scrolls on Y alone, so that is a layout
the wheel cannot fix and must be reported rather than chased.

The settle budget now covers both cases through one `settle_or_skip`, which is
what bounds the scroll. A row the wheel genuinely cannot reach - a list already
at its end - is dropped after `ROW_SETTLE_FRAMES` attempts instead of scrolled
at for the rest of the run.

### Measured

Live under Xvfb, `NOVA_AUTOPILOT=1 cargo run --example menu_picker --features
debug`, exit 0:

    scenarios pane widths HELD across 13 selections (list=331.0 details=481.0)
      - coverage: 13 rows, none skipped

7 of the 13 rows needed the wheel, each converging in a single scroll (every
`probe: scrolled ...` line is followed by that row's own measurement, so nothing
oscillated). The task's "5 of 6" was written against a smaller scenario set; the
picker lists 13 now and the run reaches all of them.

Three tests in `input::tests`, all confirmed fail-first (dropping the wrapper
write reds `a_wheel_scroll_writes_both_halves_a_real_notch_writes`). The shared
test app now registers `Messages<WindowEvent>`, which `WindowPlugin` provides in
a real app - without it the wrapper half of every gesture went nowhere and could
not be asserted on at all.
