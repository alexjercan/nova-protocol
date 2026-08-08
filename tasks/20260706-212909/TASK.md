# Editor preview controller spams PD 'root not found' errors

- STATUS: CLOSED
- PRIORITY: 84
- TAGS: v0.4.0, bug, editor

Surfaced while testing the editor-preview change (20260525-132950). In the editor,
`bevy_common_systems::physics::pd_controller::update_controller_root_torque` logs
`root entity <id> not found in q_root` every frame (flooding the console).

Cause: the editor preview ship uses `preview_section` (no `RigidBody` on the root -
`SpaceshipPreviewMarker`), but the controller section it places still inserts a functional
`PDController` (via `controller_section(...)`). The PD controller looks up a root rigid body
that the lightweight preview doesn't have, so it errors continuously. It is log-noise only:
no crash, and it stops once you enter the scenario (Play).

Fix direction: the editor preview should not carry live controller behavior. Either
- give the preview a controller variant that renders/pickable-only, without `PDController`, or
- gate the PD controller system so it doesn't run in the editor state, or
- teach the PD system to no-op quietly when its root is absent (least targeted).

Prefer making the preview inert (consistent with 20260525-132950's intent that the editor
ship is a visual config preview, not a live combat ship).

## Fix

Took the preferred "render-only preview controller" direction. The bcs
`update_controller_root_torque` iterates `(PDController, PDControllerInput, PDControllerTarget,
PDControllerOutput)` and errors when the target root has no rigid body - and that system is not
gated to a game state, so it runs in the editor too.

- Added `preview_controller_section(config)` (nova_gameplay controller_section.rs): the
  `ControllerSectionMarker` + render mesh, but no `PDController`. It renders and is pickable but
  carries no live controller.
- The editor's two preview-controller spawn sites (`create_new_spaceship_with_controller` and the
  section-placement handler) now use `preview_controller_section` instead of `controller_section`.
- Gated `insert_controller_section_target` on `With<PDController>`, so a preview controller does
  not even get a spurious `PDControllerTarget`.

With no `PDController` and no `PDControllerTarget`, a preview controller can never match the bcs
PD query, so the "root not found" error is structurally impossible for it. Real (scenario)
controllers are unchanged: they carry `PDController` in the same `controller_section` bundle, so
the `With<PDController>`-gated observer still targets their root.

## Steps

- [x] Diagnose: bcs PD torque system runs in all states and errors on a target root with no
      rigid body; the editor preview controller carries a live `PDController`.
- [x] Add render-only `preview_controller_section`; rewire the editor's two preview sites to it.
- [x] Gate `insert_controller_section_target` on `With<PDController>` so previews get no target.
- [x] Co-located tests: a preview controller carries no `PDController`; only a live controller
      gets a `PDControllerTarget`.
- [x] Green: `cargo clippy --workspace --all-targets`, `cargo test --workspace` (58 nova_gameplay
      incl. 2 new; examples_smoke under Xvfb), headless editor boot (no panic, no q_root spam).

## Notes

The exact spam needs a UI "add controller" click, so it is not reproduced headless; instead the
fix is proven structurally by `only_a_live_controller_gets_a_pd_target` (the preview controller
has neither component the bcs query requires) plus a clean editor boot. The other preview kind
sections (turret/thruster/torpedo) also carry live behavior on a non-physics root; if any of
them prove noisy too, generalising the render-only-preview idea to them is a follow-up - out of
scope for this controller-specific report.
