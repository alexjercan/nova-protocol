# Turret range slider thumb does not track its value

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.4.0, example, turret, bug

Follow-up to the turret range sliders (20260707-150002). Reported after merge: dragging a
tuning slider changes the value (the readout updates and the turret retunes) but the thumb does
not move. Also: factor the slider out into a reusable unit that can be exported later.

## Root cause

bevy's `ui_widgets` core slider deliberately does not maintain its own display state: on a drag
it fires a `ValueChange` event but neither writes its `SliderValue` back nor moves the thumb -
that is the app's job (per the `bevy_ui_widgets` slider docs). `SliderValue` is also an
*immutable* component, so it can only be replaced via `insert`. The 150002 example positioned
the thumb from `On<Insert, SliderValue>` observers, but nothing ever re-inserted `SliderValue`
after spawn (the `ValueChange` handler wrote the turret config instead), so the insert observer
never fired again and the thumb sat still.

## Fix

Pull the slider into a reusable, turret-agnostic module (`examples/08_turret_range/slider.rs`)
and close the widget's state loop there:

- `echo_value_into_slider` (global observer) writes each `ValueChange` back into the source
  slider's `SliderValue` (via `insert`, since it is immutable), so the widget's own state tracks
  the drag.
- `position_thumbs` moves the thumb whenever `SliderValue`/`SliderRange` changes - gated on
  `Changed` rather than `On<Insert>`, so it fires for the re-insert (and the initial spawn).
- `highlight_hovered_thumb` keeps the hover styling.

`SliderWidgetPlugin` registers the three; the example builds sliders with `slider(min, max,
value)` and observes `ValueChange` for its own turret binding. The module knows nothing about
turrets, so it can be lifted into a shared crate later (the "export" step).

## Steps

- [x] Diagnose why the thumb did not move (core widget does not write `SliderValue`; the old
      `On<Insert>` wiring only fired on spawn; `SliderValue` is immutable).
- [x] Extract a reusable `slider` module (bundle + `SliderWidgetPlugin` + value-echo, thumb
      positioning, hover styling), turret-agnostic.
- [x] Rewire `08_turret_range.rs` onto it; delete the old in-example slider machinery.
- [x] Test the thumb tracking (`thumb_tracks_value_changes`, runs under `cargo test --example`).
- [x] Green: `cargo clippy --workspace --all-targets`, `cargo test --workspace --examples`,
      headless autopilot run (reached Playing, cycle complete, no panic).

## Notes

The thumb test lives in the example module, so it runs under `cargo test --example
08_turret_range` / `--examples`, not the default `cargo test --workspace`. When the slider is
later exported to a crate, that test travels with it and joins the normal suite.
