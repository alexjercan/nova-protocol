# Editor preview controller spams PD 'root not found' errors

- STATUS: OPEN
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
