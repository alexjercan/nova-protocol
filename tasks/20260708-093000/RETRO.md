# Retro: Turret range slider thumb does not track its value

- TASK: 20260708-093000
- BRANCH: fix/slider-thumb-tracking
- PR: #43 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260708-093000/TASK.md`. The exact bug the 150002 retro flagged as untested (the
interactive slider path) surfaced in the first real use.

## What went well

- Read the widget's own source instead of guessing. `slider_on_drag` only triggers
  `ValueChange` and the doc says thumb positioning is the app's job - that turned "thumb is
  stuck" into a precise contract gap in seconds. The follow-on discovery that `SliderValue` is
  an *immutable* component (the compiler rejected `get_mut`) explained why the only correct
  update path is `insert`, and why an insert-keyed observer was the wrong signal once nothing
  re-inserted.
- Fixed the class, not the instance. The request was "the thumb doesn't move"; the fix makes any
  `slider()` widget self-consistent (echo + `Changed`-gated positioning) rather than patching the
  turret example. That is also what made the "pull it into a reusable component" ask fall out for
  free.
- Turned the previously-untestable path into a test. 150002's retro explicitly flagged the slider
  interaction as uncovered; extracting the widget into a module made `position_thumbs` unit-
  testable, and `cargo test --example` does run it. The value -> thumb path now has a real
  assertion.

## What went wrong

- One compile round-trip from assuming `SliderValue` was mutable (`get_mut` in the test). Root
  cause: did not check the component's mutability before writing the test - though the same error
  is what confirmed the immutability that explains the whole bug, so it paid for itself.
- The `mod slider;` path needed `#[path]`: cargo's example discovery is ambiguous when a file and
  a same-named directory coexist. Cost one build. Worth remembering for any multi-file example.

## What to improve next time

- When a UI widget "doesn't visually update", check the widget's contract first: does it own its
  display state or delegate to the app, and is the state component immutable (insert-only)? Both
  decide what change-detection signal (`On<Insert>` vs `Changed`) is even correct.
- A retro that flags an untested path (150002 -> "slider drag not tested") is a standing TODO;
  the cheapest time to close it is when a bug in that exact path forces you back in - extract for
  testability while you are there.

## Action items

- [ ] Later export: lift `examples/08_turret_range/slider.rs` into a shared crate (nova UI/debug
      or bevy-common-systems) so it is reusable across examples/editor and its test joins the
      default suite. (The user flagged this as a "later" step.)
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro).
