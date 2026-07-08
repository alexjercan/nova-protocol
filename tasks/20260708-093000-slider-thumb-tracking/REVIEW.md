# Review: Turret range slider thumb does not track its value

- TASK: 20260708-093000
- BRANCH: fix/slider-thumb-tracking

## Round 1

- VERDICT: APPROVE

Diff extracts a reusable `slider` module (`examples/08_turret_range/slider.rs`), rewires the
example onto it, and deletes the old in-example slider machinery.

Verified:

- Root cause is correct and confirmed against the source. `bevy_ui_widgets`' `slider_on_drag`
  only triggers `ValueChange` (it does not write `SliderValue`), the slider doc says thumb
  positioning is the app's job, and `SliderValue` is an immutable component (the compiler
  rejected `get_mut` on it). So the old `On<Insert, SliderValue>` thumb wiring never fired after
  spawn because nothing re-inserted the value. The fix - echo `ValueChange` into `SliderValue`
  and position the thumb on `Changed` - addresses exactly that.
- The fix is verified by a real test: `thumb_tracks_value_changes` asserts the thumb is at 25%
  after spawn and moves to 75% after a re-insert. It passes under `cargo test --example` /
  `--examples`.
- The module is genuinely reusable: no turret knowledge, a clean `slider()` + `SliderWidgetPlugin`
  API, and a doc comment that explains the widget-contract gap it closes. Ready to lift into a
  crate later.
- The example is thinner: the old `tuning_slider`, `slider_on_interaction`,
  `slider_on_change_value`, marker structs and colour consts are all gone, replaced by the module.
  The turret-specific `Knob` mapping and `observe` binding remain, correctly.
- `#[path]` on the `mod slider;` is the right call - cargo's example discovery is ambiguous when
  `08_turret_range.rs` and `08_turret_range/` coexist, and the explicit path resolves it.
- Green: `cargo clippy --workspace --all-targets` clean (only the pre-existing `hull_section.rs`
  warning), `cargo test --workspace --examples`, and a headless autopilot run (reached Playing,
  tracks + fires, cycle complete, no panic).

Honest scope note carried in TASK.md: the thumb test runs under `--example(s)`, not the default
`cargo test --workspace`; it joins the normal suite when the slider is exported to a crate. The
actual pointer-drag interaction is still not automated (no pointer headless), but the value ->
thumb path it drives is now covered by the test.

No BLOCKER/MAJOR/MINOR findings.
