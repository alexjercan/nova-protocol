# Promote the thruster shells: check the candidates, ship the picks

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.12.0,art,ship,content

Rewritten 2026-08-24 for v0.12.0. Audit:
`tasks/20260815-231945/CONTENT-AND-ART.md` section 1. The multi-cell
question this task used to defer is now OPEN as `20260824-120531`; this task
stays 1x1-only and lands early - it is mechanical and independent.

## Goal

Promote the 1x1 thruster shell candidates from `art/part-candidates/shells/`
to real thruster looks, each candidate CHECKED before promotion. Close the
determinism-gate CI gap while touching the generator.

## The candidates

Five 1x1x1: `shell_bell`, `shell_gimbal`, `shell_twin`, `shell_paddle`,
`shell_vector`. Owner review already happened once: `20260817-013639`
closure records bell + vector KEPT (the gallery labels agree,
screenshot_thruster_gallery.rs:232, :272). Re-confirm the picks from the
gallery rather than assuming them; gimbal/twin/paddle are candidates too.
The large `shell_bank` (3x3x1) and `shell_capital` (5x5x3) belong to
`20260824-120531`, not here.

## The checks (per candidate, before any promotion)

- Exhaust geometry agrees with the thrust convention: thrust -Z, bell opens
  +Z (clearance.rs:64-66 `exit_normal`); the exit_pocket / exhaust lane
  clearance fits the mesh silhouette (shell_skin.rs:321).
- Triangle budget and material sanity at ship render distance; the gallery
  render is the judging view.
- Recipe determinism stays under `gen-thruster-shells.py --check`
  (lines 145-167 rebuild and byte-compare).

## The promotion path (audited, mechanical)

1. Move the picked .glb(s) to `assets/base/gltf/` (the greeble pattern -
   assets/base/gltf/greebles/README.md), recipe stays the source, --check
   points at the new path.
2. Register an `AssetRef<WorldAsset>` in
   crates/nova_authoring/src/base_content/assets.rs (pattern at lines 22-26).
3. Set `render_mesh` + `render_mesh_transform` on `basic_thruster_section`
   (base_content/sections/standard.rs:320-360; `render_mesh: None` today at
   :355). The exhaust cone spawns for every thruster either way
   (thruster_section.rs:734-745), so the plume comes free.
4. `cargo run content gen`; commit the regenerated RON (never hand-edit).
5. Verify: gallery row plus a wfc_ships / wfc_arena render - both inherit
   the look through the prototype automatically.

If promotion replaces the primitive look, no id changes; new prototypes get
new builder ids (content lint guards duplicates). The one-socket test
(standard.rs:695) only breaks if link points change - promotion alone does
not touch them.

## The CI gap (fold in here)

`gen-thruster-shells.py --check` and the greeble twin run in NO CI step
(.github/workflows/ci.yaml has no python step; recorded unfixed in
`20260817-013639`). Add the check line so the byte-reproducibility gate
actually gates.

## Done when

- Owner has picked from the gallery; picked shells fly on real ships in a
  render; checks recorded per candidate here.
- The determinism gates run in CI.
- Large formats live in `20260824-120531`, explicitly not here.
