# Review: Target inset view (RTT scope of the locked ship)

- TASK: 20260710-104421
- BRANCH: feature/target-inset-view

## Round 1

- VERDICT: APPROVE

Scope reviewed: `crates/nova_gameplay/src/hud/target_inset.rs` (new),
`hud/mod.rs` wiring, `nova_debug/src/lib.rs` egui fix, `examples/12_hud_range.rs`
verification, docs + CHANGELOG + spike update.

Independent verification (shared-session blind-spot guard): re-derived the one
claim the unit tests do NOT cover - that the in-scene section highlight
actually RENDERS, not just that the entity reconciles. Cropped/enlarged the
inset region of a live capture: the emissive red shell is clearly visible
around the selected section, with the thruster blooming. Load-bearing visual
claim holds. Also re-checked the coexistence argument against the actual code:
every camera query is marker-filtered (camera_controller.rs, loader.rs,
menu, editor), so the unmarked inset camera trips no `Single<Camera>` -
consistent with the clean autopilot runs.

Correctness / design notes that are NOT blocking:

- [x] R1.1 (MINOR) hud/target_inset.rs `drive_inset_camera` - the inset camera
  keeps rendering the scene into its texture while HUD visibility is
  Minimal/None. In those levels `apply_hud_visibility` hides the panel (Chrome
  tier), so the RTT pass is done for a texture nobody sees while the player is
  focused on a lock. Cheap fix: gate the spawn/keep on
  `HudVisibility.shows(HudTier::Chrome)` (or despawn the camera when the panel
  is tier-hidden), so hiding the HUD also stops the second render. Low impact
  (needs hidden-HUD + active focus lock simultaneously); left to discretion.
  - Response: Fixed. `drive_inset_camera` now takes `Res<HudVisibility>` and
    the `framed` gate requires `hud_visibility.shows(HudTier::Chrome)`, so
    hiding the HUD tears the camera down (not just the panel). New test
    `camera_absent_while_hud_chrome_is_hidden` (with a delivery guard that
    shows chrome again and asserts the camera returns). 7 tests pass, fmt +
    example autopilot still green.

- [ ] R1.2 (NIT) hud/target_inset.rs `drive_inset_camera` - if two inset
  cameras ever coexisted, `q_camera.single_mut()` would `Err` and the spawn
  branch would add a third. The reconcile's own invariant (spawn only when
  zero exist, commands flush between frames) prevents this, and a test covers
  the no-duplication case, so it is not a real defect - a `for`-despawn-extras
  guard would just make the invariant explicit. Take it or leave it.
  - Response: Left as-is. The invariant (spawn only when zero exist; commands
    flush between frames) plus `camera_does_not_duplicate_across_frames` cover
    it; a despawn-extras guard would add code for a state the reconcile cannot
    reach. NIT, not addressed by design.

## Round 2

- VERDICT: APPROVE

R1.1 verified fixed on the new diff: the `HudVisibility::shows(HudTier::Chrome)`
gate is in `drive_inset_camera` and the new test exercises both the hidden
(no camera) and restored (camera returns) paths with a delivery guard. R1.2
accepted as a deliberate no-op (NIT). No new findings. Branch ready to land.

Positives worth recording: the probe-first discipline paid off beyond the
blackout answer (surfaced the egui bleed and the BCS_SHOT black-capture timing
gotcha); tests carry delivery guards (assert the positive state before flipping
the condition); the highlight is a uniform overlay rather than a per-material
mutation, which is the right call given heterogeneous section materials; docs
are thorough and honest about what is unmeasured (WASM perf). The egui fix is
correctly scoped to nova_debug with a filed follow-up (20260712-201603) for the
bcs root fix.

Check suite: per repo policy the full test suite + clippy run in CI, not
locally (memory: skip-local-tests). Ran here: `cargo test -p nova_gameplay
target_inset` (6 pass), `cargo fmt --check` (clean), `cargo check --workspace`
non-debug (clean), `12_hud_range` + `10_gameplay` autopilots (PASS, no panic).
