# Screen-projected-indicator widget (HUD substrate)

- TASKS: 20260708-165700 (widget + migrations), 20260708-165701 (turret lead
  pip)
- SPIKES: docs/spikes/20260708-165647-weapons-hud.md (direction),
  docs/spikes/20260709-164502-screen-indicator-architecture.md (architecture)

## What changed

`crates/nova_gameplay/src/hud/screen_indicator.rs` is a new generic
"world anchor -> UI node" widget, and the two bespoke projection overlays now
consume it:

- **The widget.** `screen_indicator(config)` returns the absolutely
  positioned node itself; `ScreenIndicatorPlugin` projects it to its anchor
  every frame through the camera tagged `ScreenIndicatorCamera`. The anchor
  is `ScreenIndicatorAnchor(Option<Entity | Point(Vec3)>)` - `None` or a dead
  entity hides the node, so consumers manage no visibility of their own.
  Sizing is `Fixed(Vec2)` or `ApparentSize { min_px }` (the collider-AABB
  union that used to live in torpedo_target.rs). Off-screen policy is `Hide`
  or `ClampToEdge { margin_px }`, the latter rotating an optional
  `ScreenIndicatorArrowMarker` descendant toward the anchor - including
  behind-camera anchors, whose direction comes from the camera-space offset
  flattened onto the view plane. `screen_indicator_layer()` is the shared
  full-screen click-through container. Public `ScreenIndicatorSystems` set;
  drivers order themselves `.before()` it.
- **Torpedo reticle** (`hud/torpedo_target.rs`): now spawns a layer + one
  indicator child (`TorpedoTargetReticleMarker`, ApparentSize 32 px minimum,
  Hide policy) and keeps a single 5-line driver that copies
  `SpaceshipPlayerTorpedoTargetEntity` into the anchor. The bespoke
  projection, sizing and visibility systems are gone.
- **Autopilot destination marker** (`hud/flight_status.rs`): same shape -
  fixed 24 px indicator, driver maps the engaged GOTO target (or None) into
  the anchor. `update_destination_marker`'s projection loop is gone.
- **Camera glue** (`hud/mod.rs`): observers mirror
  `SpaceshipCameraController` onto `ScreenIndicatorCamera`, keeping the
  widget itself camera-agnostic (promotable to bevy_common_systems, task
  20260709-164608).
- **Turret lead pips** (`hud/turret_lead.rs`, task 20260708-165701): the
  first fresh consumer and the first `Point`-anchor user. One 8 px amber pip
  per turret child of the player ship, drawn at the turret's computed
  intercept point (`TurretSectionAimPoint`). Membership is a per-frame
  reconcile system (`sync_turret_pips`) rather than add/remove observers:
  turret sections die mid-fight and ships can gain sections after the player
  marker lands, and one idempotent pass covers every ordering. The driver
  clears the anchor for `SectionInactiveMarker` turrets explicitly, because
  `update_turret_aim_point` keeps computing aim points for disabled turrets.
  The layer spawns/despawns with the player ship via the hud/mod.rs
  observers like every other overlay.

## Why this design

The weapons-HUD arc adds at least four world-anchored indicators (reticle,
destination, turret lead pip, locked-target readout, later edge arrows).
Before this change each was a fresh copy of world_to_viewport + visibility +
sizing; the architecture spike chose one widget over per-indicator systems
(duplication) and over gizmos or a second Camera2d (styling; the 0.19
second-window-camera blackout). Decisions made with the user on 20260709:
local module first (not straight into bevy_common_systems), migrate both
existing overlays now, implement both off-screen policies now, and build the
sizing modes in. The "indicator IS the node" shape (rather than the old
constructor-owns-a-layer shape) is what lets consumers attach arbitrary
content - the locked-target readout (20260708-165702) attaches text and a
health bar as plain children.

## Behavior deltas (enumerated per consumer)

Migration is not pixel-identical; the differences are deliberate:

- Reticle: a locked target behind the camera used to leave the reticle
  visible at its last projected position (the old system `continue`d on
  projection failure without hiding); it now hides. A lock whose entity
  despawned before the lock resource cleared also now hides instead of
  freezing.
- Destination marker: a destination laterally outside the viewport used to
  keep the node positioned off-view but nominally visible; it is now
  explicitly hidden. Behind-camera behavior (hidden) is unchanged.
- Both: with no `SpaceshipCameraController` camera (player dead, WASD camera
  active), indicators are now explicitly hidden; previously the update
  systems simply stopped running and left the last state on screen.

## Verification

- 29 unit/behavioral tests in the hud modules (`cargo test -p nova_gameplay
  --lib hud::`), including whole-system runs against a fabricated camera
  (90 degree vertical FOV, 800x600 target, identity transform): anchor
  lifecycle, offset, both off-screen policies with arrow rotation, apparent
  size vs its projected expectation, missing/duplicate-camera handling, the
  reticle and destination drivers, and the pip reconcile + driver (spawn per
  turret, despawn on turret death, other ships ignored, inactive turrets
  cleared).
- Scripted range `examples/12_hud_range.rs` (BCS_AUTOPILOT timeline on a
  1280x720 Xvfb display): the aim-assist locks a ship parked 150 m dead
  ahead, the reticle centers on its projection (0.0 px drift, 32 px minimum
  size), the turret lead pip sits on the projected `TurretSectionAimPoint`
  (0.0 px drift), an engaged GOTO shows the destination marker on the same
  ship (0.1 px drift), and despawning the target + disabling the turret
  hides all three indicators. Mandatory `expect` lookups and an
  asserted-at-exit backstop per the com-range retro lessons.
- Honest skips (user instruction): the full workspace test suite and clippy
  were not run locally; `cargo check --workspace` (via the example build) and
  `cargo fmt` are green, and only the newly written tests were executed.

## Difficulties

- The `apparent_size_tracks_entity_extent` test's expected value initially
  forgot the aspect-ratio division in the horizontal projection
  (`x_ndc = x / (z * aspect)`); the implementation was right, the test math
  was wrong. Deriving expected values from the projection matrix by hand
  needs the same care as the code under test.
- Running the scripted example headless: backgrounding Xvfb with
  `cd ... && Xvfb :99 ... &` backgrounds the whole chain including the `cd`,
  so cargo ran in the wrong directory. Start the display in its own command.

## Reflection

- Designing `place()` (viewport + projection + policy -> placement) as a
  pure function made the policy matrix trivially testable and kept the ECS
  system a thin shell; do the same for the pip driver and readout math.
- The one-frame projection latency (Update reads last frame's propagated
  transforms) is pre-existing and documented in the spike's open questions;
  the range example encodes it as a 25 px tolerance constant with a comment
  naming the cause.
