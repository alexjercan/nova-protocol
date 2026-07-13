# Fix bcs InspectorDebugPlugin to assign PrimaryEguiContext only to window cameras

- STATUS: OPEN
- PRIORITY: 15
- TAGS: v0.5.2, debug, egui, bcs, chore

## Goal

Move the debug-inspector egui-context placement fix upstream into
bevy-common-systems (`~/personal/bevy-common-systems`), so nova does not need
its local workaround. Root-fix the "first camera wins" hack that lets a
render-to-texture camera steal the inspector UI.

## Background

`src/debug/inspector.rs` `on_add_camera` assigns `PrimaryEguiContext` to the
FIRST `Camera` added and then skips (guard: `if !q_context.is_empty() return`).
With a second camera that renders to an `Image` instead of the window - nova's
target-inset RTT camera (task 20260710-104421) - the inspector egui can land
inside the offscreen texture if that camera's `Add` fires first.

nova currently works around this locally in `nova_debug`
(`keep_inspector_on_window_camera`): a per-frame reconcile that keeps
`PrimaryEguiContext` on a window-targeting camera and off any `Image`-target
camera. That fully resolves the symptom, but the root cause is in bcs.

## Steps

- [ ] In bcs `src/debug/inspector.rs`, change `on_add_camera` so it never
      assigns `PrimaryEguiContext` to a camera that renders to an Image: skip
      cameras whose `RenderTarget` is `RenderTarget::Image(_)` (a window
      camera has no `RenderTarget` component or a `Window` variant). Consider
      also re-homing the context if the current holder is/was an RTT camera,
      so order does not matter (mirror nova's reconcile logic, or make the
      observer robust enough that a reconcile is unnecessary).
- [ ] Verify against a scene with a second RTT camera (nova's target inset is
      the real case; a bcs example with a render-to-texture camera would be a
      good regression rig).
- [ ] Bump the bcs rev in nova's `crates/*/Cargo.toml` (currently
      a35b74c460fb42879acd963bab45b7d88a9ba2cc) to the fixed commit, then
      REMOVE nova's local workaround `keep_inspector_on_window_camera` from
      `crates/nova_debug/src/lib.rs` (and its `RenderTarget` /
      `PrimaryEguiContext` imports) so there is one implementation, not two.
- [ ] Re-run a debug example with the target inset focused
      (`BCS_AUTOPILOT=1 NOVA_INSET_SHOT=1 cargo run --example 12_hud_range
      --features debug`) and confirm the inspector stays on the window and out
      of the inset.

## Notes

- bcs is the user's own project at `~/personal/bevy-common-systems`; changes
  there need explicit approval and a coordinated rev bump in nova, which is
  why the fix landed locally first.
- Relevant files: bcs `src/debug/inspector.rs` (`on_add_camera`), nova
  `crates/nova_debug/src/lib.rs` (`keep_inspector_on_window_camera` - the
  workaround to remove once upstream is fixed).
- Discovered during task 20260710-104421 (target inset view); see
  tasks/20260710-104421/NOTES.md ("Adjacent fix").
- v0.5.2 plan pass (2026-07-13): the user approved implementing the bcs
  change in this flow. Logistics: develop and commit the fix in
  ~/personal/bevy-common-systems (its own repo, its own commit); test nova
  against it locally via a `[patch]` path override BEFORE any rev bump; the
  final rev bump in nova's Cargo.tomls only builds once the bcs commit is
  on GitHub, and pushing bcs is the user's call - stop and ask when the fix
  is ready. bcs master is currently a35b74c = exactly the rev nova pins, so
  there is no drift to absorb.
