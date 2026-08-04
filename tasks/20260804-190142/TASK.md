# Synthesize wheel scroll in nova_autopilot so driven runs can reach a row past the fold

- PRIORITY: 0
- TAGS: backlog
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -

## Story

`nova_autopilot::input` synthesizes pointer motion and buttons but no WHEEL
event, so a driven run cannot scroll a list. The scenarios picker DOES
support wheel scroll (`scroll_menu_lists`, `crates/nova_ui/src/widgets.rs:72`),
so `examples/ui/menu_scenarios.rs` permanently skips the row past the fold and
reports 5 of 6 rows measured. That coverage gap is a property of the HARNESS,
not of the UI.

Surfaced as a review process signal on 20260804-094021 (rounds 3 and 4).

## Steps

- [ ] Add wheel synthesis to `crates/nova_autopilot/src/input.rs` beside the
      existing pointer vocabulary, writing the same `WindowEvent` the UI
      scroll systems read, with unit tests in that module`s `mod tests`.
- [ ] Use it in `examples/ui/menu_scenarios.rs` so a row reported as
      `RowPlacement::Unreached` because its centre is outside the list box is
      scrolled into view and then clicked, instead of skipped.
- [ ] The run reports every row measured and none skipped.
