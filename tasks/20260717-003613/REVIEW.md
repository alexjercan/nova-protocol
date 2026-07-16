# Review: Diegetic HP v1 - per-section mesh damage tint/glow + retire the generic bar

- TASK: 20260717-003613
- BRANCH: spike/diegetic-hp

## Round 1

- VERDICT: REQUEST_CHANGES

Scope reviewed: implementation commit eaf2a316 (the spike commit 92d32332 is
research, not judged here). Ran `cargo test -p nova_gameplay damage_tint`
(3 passed) and `cargo check --workspace` (green). Independently re-verified the
load-bearing interaction: fired projectile/muzzle meshes are children of the
world-level projectile entity and carry no `SectionRenderOf`, so `owning_section`
does not resolve them to a section and they are never tinted. Turret sub-part
meshes (base/yaw/pitch/barrel) do carry `SectionRenderOf(turret)` and correctly
tint with the turret's health. The retire-the-bar deletion is clean (no lingering
`HealthDisplay*` references anywhere).

- [x] R1.1 (MINOR) crates/nova_gameplay/src/sections/damage_tint.rs:139 - the
  capture is single-shot on `Added<MeshMaterial3d>`, and if
  `materials.get(&material.0)` returns `None` that one frame (material asset not
  yet in `Assets`), the mesh is `continue`d past and never re-armed - so it is
  silently never tinted. For the standard gltf path this should not fire (a
  `Scene` only instantiates once its material dependencies are loaded), which is
  why it is MINOR rather than blocking, but the failure mode is a silent,
  whole-mesh loss of the feature. Harden it: either drop the `Added` filter and
  match player-section descendant meshes `Without<SectionDamageTint>` each frame
  (self-re-arming), or keep `Added` but `warn!` when the `None` branch is taken so
  the silent case becomes visible. Prefer the self-re-arming query.
  - Response: Fixed. Capture split into `mark_section_meshes` (on `Added`: the
    ChildOf walk + player gate, tags `PendingSectionTint`) and
    `resolve_pending_tints` (retries the clone each frame over the near-empty
    pending set until the asset loads, then clears the marker). New test
    `capture_rearms_until_material_asset_loads` reserves a handle with no asset,
    asserts the mesh stays pending across frames, inserts the asset, and asserts
    capture then completes - it fails if the old single-shot behaviour returns.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/sections/damage_tint.rs:187 -
  `grade_section_tints` writes `base_color` and `emissive` via `get_mut` every
  frame for every tinted section, unconditionally. `get_mut` flags the
  `StandardMaterial` as changed each frame, forcing a per-frame material
  re-extraction to the render world even when nothing moved. Negligible for a
  dozen sections but a needless steady-state cost and a change-detection smell.
  Compute the target `(base_color, emissive)` first and only `get_mut`/write when
  it differs from the material's current values (or gate the system on
  `Changed<Health>` plus `Added/Removed<SectionInactiveMarker>`).
  - Response: Fixed. `grade_section_tints` now computes the target
    `(base_color, emissive)`, reads the material immutably, and only takes a
    `get_mut` (change-flagging) borrow when the value actually differs - so an
    idle ship stops re-flagging its materials each frame.
- [x] R1.3 (NIT) crates/nova_gameplay/src/sections/damage_tint.rs:57 -
  `GLOW_PEAK` uses an HDR-range red (2.2) that only visibly "glows" when a bloom
  pass is enabled; without bloom it reads as a bright-red emissive (still a valid
  cue). Not a code defect - just fold a note into the pending playtest (confirm
  bloom is on, or the glow is legible without it) so the ramp is tuned against
  what actually renders.
  - Response: Acknowledged, no code change. Added to the pending-playtest scope
    in NOTES.md (confirm bloom is on / the glow reads without it when tuning the
    ramp).

Non-blocking observations (no action required):

- The pending on-ship legibility playtest is correctly left unchecked in TASK.md
  and documented in NOTES.md; that is the honest state, not a review gap.
- Player-ship scope gate, dead/inactive handling, and per-section material
  cloning (proven by the end-to-end test asserting the shared source material is
  never mutated) all look correct.

## Round 2

- VERDICT: APPROVE

Verified the round-1 fixes against commit 42c0dd5b:

- R1.1: capture is now two-phase (`mark_section_meshes` +
  `resolve_pending_tints`) and self-re-arming. The new test
  `capture_rearms_until_material_asset_loads` independently confirms it -
  material absent -> mesh stays `PendingSectionTint` across frames -> asset
  inserted -> capture completes and the marker clears. Would fail under the old
  single-shot behaviour.
- R1.2: `grade_section_tints` reads the material and only takes a mutable borrow
  on a real change, so an idle ship no longer re-flags materials each frame.
- R1.3: acknowledged, note folded into NOTES.md's pending playtest.

`cargo test -p nova_gameplay damage_tint`: 4 passed. No new issues introduced by
the fixes. The one remaining open item (on-ship legibility playtest) is a manual
step correctly recorded in TASK.md/NOTES.md, not a code finding.
