# Review: HUD visibility levels: tilde cycles ALL/MINIMAL/NONE

- TASK: 20260711-180501
- BRANCH: feature/hud-visibility-levels

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/mod.rs:109-112 -
  apply_hud_visibility has `.after(ScreenIndicatorSystems)` but no upper
  bound, so its order against Bevy's VisibilityPropagate/CheckVisibility
  (same PostUpdate) is an arbitrary topo tie-break: if it lands after
  propagation, every write lands one frame late and freshly spawned holos
  (Visibility::Visible at spawn) flash for a frame at level None - the
  exact artifact the design exists to prevent. Works today by luck; any
  unrelated system can flip it. Add
  `.before(VisibilitySystems::VisibilityPropagate)` (or `.before
  (UiSystems::Layout)` mirroring the indicator set).
  - Response: fixed in 53639ab - .before(bevy::ui::UiSystems::Layout) added (mirrors the indicator set; layout runs upstream of transform + visibility propagation), in both the plugin and the test fixture.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/mod.rs:210-217 - the
  one-shot Inherited restore stomps the gravity sphere's self-driven
  Hidden-in-flat-space state on every level change that shows Instruments
  (runs in PostUpdate after the sphere's Update driver), flashing the
  yellow sphere for one frame. Add a self-driven opt-out marker on the
  gravity sphere spawn and filter it out of the restore branch.
  - Response: fixed in 53639ab - HudSelfDrivenVisibility opt-out component, tagged on the gravity sphere spawn; restore skips it, Hidden enforcement still applies; pinned by self_driven_roots_skip_the_restore_but_not_the_hide.
- [x] R1.3 (MINOR) hud tests - nothing pins the enforcement running AFTER
  ScreenIndicatorSystems; moving it before (the regression the verify-first
  step was about) still passes all three tests, because the widget is
  simulated between updates and the test app has no system in the set.
  Register a stand-in system in ScreenIndicatorSystems that writes
  Visibility::Visible each frame and assert Hidden wins.
  - Response: fixed in 53639ab - fake_widget_drive registered in_set(ScreenIndicatorSystems) in the test app; the indicator test asserts the same-frame win across consecutive updates, so moving the enforcement before the set fails the suite.
- [x] R1.4 (MINOR) nova_menu tests - the absorbed menu behavior
  (HudVisibility None on enter, All on exit) has no test; it is now a plain
  resource assertion. Add it to the menu-entry and sandbox-exit tests.
  - Response: fixed in 53639ab - menu-entry test asserts HudVisibility::None, sandbox-exit test asserts the restore to All.
- [x] R1.5 (NIT) nova_menu test helper doc still mentions the removed
  status-bar Single system. Update.
  - Response: fixed in 53639ab.
- [x] R1.6 (NIT) the juice.rs gizmo exclusion ("FX, not HUD") lives only in
  TASK.md; add a line next to HudTier in mod.rs so the boundary is
  discoverable in code.
  - Response: fixed in 53639ab - doc line on HudTier notes the juice.rs FX exclusion.

Round 1 notes (verified clean): tier coverage is complete (all 12 module
roots + status bar; reconciled children are descendants of tagged roots);
level.is_changed() cannot miss a restore (all writers run upstream of the
PostUpdate apply, including double changes in one frame); no stuck-at-None
trap exists; Backquote is a physical key (non-US layouts mislabel the hint,
accepted, consistent with the game's physical bindings); close record and
docs match the diff. cargo check clean; cargo test -p nova_gameplay hud::
78 passed including the 3 new tests; e2e throwaway harness verified the
full cycle in the real app (recorded in TASK.md).

## Round 2

- VERDICT: APPROVE

Verified against the fix commit (53639ab):
- R1.1: double-bounded registration confirmed in plugin and test fixture;
  UiSystems::Layout is upstream of transform + visibility propagation, so
  writes land in the same frame deterministically.
- R1.2: opt-out marker on the gravity sphere only; restore skip + hide
  still enforced, pinned by the new test.
- R1.3: the ordering contract is executable - the stand-in driver runs in
  the real ScreenIndicatorSystems set and the test asserts the same-frame
  win across consecutive frames; with the enforcement moved before the set
  the driver would write Visible last and the assertion fails.
- R1.4/R1.5/R1.6 confirmed. 4 hud tests + 7 nova_menu tests green,
  cargo check --workspace clean, fmt clean, 09_editor smoke green.

No new findings. APPROVE.
