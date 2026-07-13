# Review: Screen-projected-indicator widget (HUD substrate)

- TASK: 20260708-165700
- BRANCH: weapons-hud (implementation commit c9499c6)

## Round 1

- VERDICT: APPROVE

Verified independently: `cargo fmt --check` clean; `cargo check --workspace`
green; the 24 new hud tests pass (`cargo test -p nova_gameplay --lib hud::`);
`examples/12_hud_range.rs` compiles both with and without the `debug`
feature and its scripted run passed (reticle drift 0.0 px, GOTO marker drift
0.1 px, both indicators hide on target death). The arrow-rotation convention
(`arrow_angle` = `atan2(dir.x, -dir.y)` for up-pointing art) was checked
against bevy_ui 0.19 source: `UiTransform::compute_affine` applies
`Mat2::from(rotation)` directly in y-down UI coordinates, which matches the
derivation and the unit test. Behavior deltas from the migration are real,
deliberate, and honestly enumerated per consumer in
docs/retros/20260709-screen-indicator-widget.md. Spec check: every ticked step in
TASK.md is actually delivered. Honest skips (full local suite, clippy) match
the user's standing instruction.

Findings are MINOR/NIT only; none block the substrate.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/screen_indicator.rs:438,
  472-476 - every visible indicator writes `*visibility =
  Visibility::Visible` (and clamped arrows rewrite rotation + `Inherited`)
  unconditionally each frame, dirtying `Changed<Visibility>` so visibility
  propagation re-walks the subtree every frame even when nothing changed.
  For a substrate that will back every HUD indicator, keep change detection
  meaningful: use `visibility.set_if_neq(...)` in both
  `update_screen_indicators` (both the Hidden and Visible writes) and
  `update_arrows`.
  - Response: fixed in 3a18507 - set_if_neq on every widget visibility write (hidden and visible paths) and on the arrow (rotation now written only when it changes).
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/screen_indicator.rs:366 -
  with two or more `ScreenIndicatorCamera` cameras, `Option<Single>` makes
  the system skip entirely, silently freezing all indicators at stale
  positions (the doc comment only covers the zero-camera case). Either
  document the freeze on `ScreenIndicatorCamera` or degrade loudly: a plain
  `Query` + `iter().next()` with a `warn_once!` on ambiguity keeps
  indicators live and diagnosable.
  - Response: fixed in 3a18507 - plain Query with a warn_once first-camera fallback; new behavioral test multiple_cameras_use_the_first_and_stay_live.
- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/screen_indicator.rs:247 -
  `clamp_to_rect` accepts a negative `margin_px` and would clamp indicators
  to a rect larger than the viewport. Clamp the margin to `>= 0` or note the
  expectation on `ClampToEdge::margin_px`.
  - Response: fixed in 3a18507 - margin clamped to >= 0, doc comment updated.
- [x] R1.4 (NIT) examples/12_hud_range.rs:42 - `CENTER_TOLERANCE_PX = 25`
  is generous against the measured 0.0/0.1 px drift; 10 px would still
  absorb one frame of motion at these speeds and catch subtler projection
  regressions earlier. Take or leave.
  - Response: fixed in 3a18507 - tolerance tightened to 10 px; scripted run re-passed (drift 0.0 / 0.1 px).

## Round 2

- VERDICT: APPROVE

All four round-1 findings verified against commit 3a18507: set_if_neq on all
seven visibility writes plus the arrow rotation guard (R1.1); the camera
lookup is a plain Query with a warn-once first-camera fallback and the new
multiple_cameras_use_the_first_and_stay_live test passes (R1.2); negative
margins clamp to zero with the doc updated (R1.3); the range tolerance is
10 px and the scripted run re-passed at 0.0/0.1 px drift (R1.4). 25 hud
tests green, fmt clean. No new findings.
