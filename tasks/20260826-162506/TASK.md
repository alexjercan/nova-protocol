# The UI pass: any landscape resolution, and controls that match their type

- STATUS: CLOSED
- PRIORITY: 73
- TAGS: v0.12.0,ui,editor

Follows the input-mode task, which follows the review-fix task. Both halves come
from round 1 of `/nova-review`.

## Half one: correct at any landscape resolution

`ComputedNode::size()` is physical pixels; `Node::left` and `Node::top` are
logical. They are the same number at 1x, which is why this reads fine locally
and misplaces every world-anchored element on a HiDPI screen. The world-anchor
loop - project a world point, measure the element, clamp it inside the viewport
- is written out longhand four times. `ui/rail.rs:375` learned about
`inverse_scale_factor()`; `ui/plate.rs:257`, `ui/callout.rs:156` and
`keybind.rs:225` did not. Extract the loop once and there is one place left to
get the units right.

Also here: `ui/window.rs:41` places the colour picker at
`size.x - RIGHT_MARGIN - WINDOW_W` with `RIGHT_MARGIN = 264.0` against an
inspector `PANEL_W = 300.0`, so it overlaps by 36px at every window width.

Goal: correct at any landscape resolution and DPI scale. Portrait is explicitly
out - the bound is what makes this finishable.

## Half two: controls that match the type they edit

Every leaf renders as a text box, so a type's constraints have nowhere to live.
The pick list (`inspect.rs:1002`) and the leaf rules (`inspect.rs:305`) are two
independent hand-kept tables describing the same types, which is how a vector
leaf got an editor with no rule.

One declaration per field, driving the control: validated text, dropdown for an
enum, drag-number, slider for a bounded float, min/max/format carried by the
declaration rather than by the widget. The colour picker is the model - it is
the proof the pattern works. `NaN` then stops being a validation bug and becomes
a value the control cannot express.

The review-fix task lands the two small precision items first (a finiteness rule
on `x`/`y`/`z`, and dropping `aim` from the Light picks) so they do not wait on
this. A finiteness rule is the shape this system consumes, so it is not
throwaway.

## Done when

- Live run at two scale factors, and at a wide and a narrow landscape size.
- A systems range for the field rules with its `catalog_drift` roster.

## Landed

Half one: `nova_ui::screen::hang_at` is the world-anchor loop, once. It takes
the projected point (logical), the label's own `ComputedNode` (physical, and it
does the conversion), the hang it wants and the viewport, and answers where the
corner goes. `ui/plate.rs`, `ui/callout.rs` and `keybind.rs` call it; `rail.rs`
never did the projection at all (see the decision below). The colour picker now
takes its right margin from the panel it belongs to instead of a literal.

Half two: one `FieldSpec` per authored field, carrying its unit, its floor, and
the step a drag lands on. The per-kind first screens are lists of those
declarations rather than lists of names, so `DECLARED` is the whole table and a
field a kind shows cannot lack the rule it is shown with. Numbers left the
`Text` arm: `RowValue::Number` is its own variant, it wears the unit, and its
row NAME is the grip that scrubs it - which is the answer to `NaN`, because a
number reached by dragging is the old number plus a delta.

### Decisions

- **No slider, and no ceiling.** `Limit` has `Free` and `AtLeast` and no
  `Between`, because no field in the content has a real ceiling. A track
  invented for one would either refuse an edit the runtime accepts or lie about
  where the value ends. The scrub is the control every number gets; a slider
  arrives with the first genuinely bounded field.
- **The row's name is the grip.** The panel is 240px wide, and a row that spends
  pixels on a grip of its own spends them on the box holding the number. Vector
  rows use their axis letters, which is also what tints them.
- **A scrub ARRIVES at the floor; a typed number is refused by it.** They are
  different gestures: typing `-3` into a radius is a mistake, and dragging past
  zero is asking for the smallest value there is.
- **`ui/rail.rs` stayed out.** `sync_scene_tooltip` measures a ROW, not a world
  point - it has no camera and no viewport - and it already converts its units.
  It is not part of the loop that was written out four times.
- **NOVA OS was in scope after all.** Both scene panels compared a projected
  point against a physical `ComputedNode::size()` and placed a blip from it. The
  camera draws into an image at scale 1, so the projection is physical there and
  the `Node` is logical: the same defect, two lines each.

### Proof

- `cargo test -p nova_editor --lib` - 319 pass, including four new live-tree
  tests for the grip and five for the declarations.
- `cargo test -p nova_ui --lib` (48), `-p nova_os_ui --lib` (108),
  `-p nova_probe_cli --test catalog_drift`.
- New range `examples/systems/system_ui_scale.rs`: founds a ship with one
  bindable part, stamps where its chip hangs at 1024x768, and reads it again at
  2x, at 1280x600 and at 760x600. Gap 24 and lead 5 at every one.
- Mutation check: putting `keybind.rs` back on `computed.size()` in a logical
  position takes the range down at the 2x beat - `the chip hangs 47.5 over its
  part instead of 24`, exit 101.
- New range `examples/systems/system_field_controls.rs`: the rock's Radius wears
  `u`, a 40px pull at a 0.05 step moves it 3 -> 5, a pull through the floor
  arrives at 0, and the Invulnerable row has no grip.
- Re-ran `system_ship_editor`, `system_input_modes`, `bug_sandbox_soak` and
  `system_nova_os` - all green, all under the same walks they had.
- Skipped: the workspace test suite and Clippy, per the standing instruction.
