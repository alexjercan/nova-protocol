# Review: Web render-scale / resolution lever

- TASK: 20260718-004723
- BRANCH: render-scale-lever

## Round 1

- VERDICT: APPROVE

Reviewed the diff (`git diff master...HEAD`, 408-line `render_scale.rs` + the
`GraphicsBudget.render_scale` field, the perf isolation override, the example,
docs, and the measurement report). Ran an independent out-of-context pass
(three finder angles: line-by-line, removed-behavior + cross-file, cleanup +
altitude + conventions) plus a manual re-derivation of the load-bearing
render-graph claims, since implementer and reviewer share a session.

Load-bearing claims independently re-verified (all hold):

- **Only the blit Camera2d targets the window** when downscaling (scenario +
  target-inset cameras target Images), so no "second window camera blacks out the
  scene" (`screen_indicator.rs:17`) conflict.
- **RenderLayers isolation is correct**: blit camera + sprite on layer 1, the
  world on the default layer 0 - neither renders the other's content.
- **Screen-space projection stays aligned**: the HUD renders into the SAME
  reduced image (via `IsDefaultUiCamera` on the scenario camera) that
  `world_to_viewport` projects through, so target markers / lock reticles land
  correctly - confirmed by the captured Low screenshot, not just reasoning.
- **Teardown restores the pre-lever single-window-camera state** (target back to
  Window, `IsDefaultUiCamera` removed, blit despawned) - the exact config the HUD
  rendered under before this change.

Refuted finder candidates (recorded so they are not re-raised): high-DPI sprite
sizing (the sprite `custom_size` is deliberately in logical units to match the
default Camera2d projection, which spans the full physical window; the texture is
physical x scale - independent by design); VRAM leak on resize (Bevy asset
refcounting frees the old `Handle<Image>` once state, camera target, and sprite
are updated to the new one in the same run); missing `render_scale` on a
`GraphicsBudget` construction site (compile-enforced - the crate builds); module
placement in `nova_scenario` (forced by the dependency graph: the reconcile needs
both `GraphicsBudget` from `nova_gameplay` and `ScenarioCameraMarker` from
`nova_scenario`, and `nova_gameplay` cannot depend on `nova_scenario`).

No BLOCKER or MAJOR findings. The lever is correct (screenshots + tests + the web
sweep showing frame time responds to `render_scale`), delivers the Goal
(lever + measure-first web isolation), and is cheap when off. Open findings are
MINOR/NIT; the implementer addressed the two worth fixing (see Round 2 note).

- [x] R1.1 (MINOR) crates/nova_scenario/src/render_scale.rs:200 - the blit
  sprite's `custom_size` is reassigned every frame unconditionally, marking the
  `Sprite` changed every frame (needless change-detection churn). Guard it like
  the `image` field: only write when it differs from `Some(window.size())`.
  - Response: Fixed - guarded the `custom_size` write (Round 2).
- [x] R1.2 (MINOR) crates/nova_scenario/src/render_scale.rs:222 - teardown resets
  the `RenderTarget` immediately (via the `&mut` query) but despawns the blit
  camera and removes `IsDefaultUiCamera` via deferred `Commands`, so for one
  frame the scenario camera renders to the window while the not-yet-despawned
  blit still draws its stale image on top (a 1-frame stale frame on a live
  Low->High switch). Make the target reset deferred too (`commands.insert`) so all
  teardown changes apply together.
  - Response: Fixed - teardown now defers the target reset (Round 2).
- [ ] R1.3 (MINOR) crates/nova_scenario/src/render_scale.rs:250 -
  `create_scaled_target` duplicates the WebGL2-safe target creation in
  `nova_gameplay::hud::target_inset::create_render_target` (same format + None
  view choice, same rationale). Left as-is: the two differ (fixed inset size vs a
  dynamic window fraction) and live in different crates; a shared helper would
  couple `nova_scenario`'s upscale to the HUD's inset for ~5 saved lines. Noted so
  the WebGL2-safe format choice is kept in sync by the shared code comment.
  - Response: Acknowledged, left to discretion (see above).
- [ ] R1.4 (NIT) crates/nova_scenario/src/render_scale.rs:102 -
  `reconcile_render_scale` runs every `Update` frame with no `run_if`. Kept: the
  idempotent-reconcile-every-frame matches the codebase's existing pattern
  (`hud/target_inset`), the per-frame cost is a `single()` + two tiny-query
  checks, and gating it correctly would need to fire on budget change OR scenario
  camera spawn OR window resize - more complexity than the saved work. The main
  per-frame write (R1.1) is the part worth removing, and it was.
  - Response: Acknowledged, intentional.
- [ ] R1.5 (NIT) crates/nova_scenario/src/render_scale.rs - a rapid window-resize
  drag on Low reallocates the offscreen texture each frame the size changes.
  Transient (Low only, only while actively dragging; the perf/web targets use a
  fixed size), so left as a known limitation rather than debounced.
  - Response: Acknowledged, known limitation.

## Measurement note (not a code finding)

The measure-first gate returned that `render_scale = 0.7` is ~neutral on the only
web rig available (a strong discrete GPU that is overhead-bound over WebGPU, not
fill-bound). Shipping it at 0.7 is a user-approved product decision for the
low-end hardware the Low preset targets, and the report + CHANGELOG + code
comment state this honestly rather than claiming a measured general win. That is
the right call for a review to accept: the code delivers the lever and the
honest measurement; the value is a product judgment, not a correctness defect.
