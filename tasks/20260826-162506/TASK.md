# The UI pass: any landscape resolution, and controls that match their type

- STATUS: OPEN
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
