# Screen-projected-indicator widget (HUD substrate)

- STATUS: CLOSED
- PRIORITY: 78
- TAGS: v0.4.0, hud, spike

## Goal

Generalize the `torpedo_target` world-to-viewport trick into one reusable
screen-projected-indicator widget so every weapons-HUD indicator (lead pip,
target info, edge arrows) is a thin consumer instead of a fresh copy of
projection + visibility + off-screen handling. Migrate both existing bespoke
overlays (torpedo reticle, autopilot destination marker) onto it in this task.

Architecture is settled - do not re-litigate:
docs/spikes/20260709-164502-screen-indicator-architecture.md (and the earlier
direction spike docs/spikes/20260708-165647-weapons-hud.md).

## Steps

- [x] Create `crates/nova_gameplay/src/hud/screen_indicator.rs` with the
      widget surface: `ScreenIndicatorMarker`,
      `ScreenIndicatorAnchor(Option<ScreenIndicatorAnchorKind>)` with
      `enum { Entity(Entity), Point(Vec3) }`,
      `ScreenIndicatorSize { Fixed(Vec2), ApparentSize { min_px: f32 } }`,
      `ScreenIndicatorOffset(Vec2)`,
      `ScreenIndicatorOffscreen { Hide, ClampToEdge { margin_px: f32 } }`,
      `ScreenIndicatorArrowMarker`, `ScreenIndicatorCamera` marker,
      `screen_indicator(config) -> impl Bundle` (the indicator IS the
      absolutely-positioned node; Pickable::IGNORE, starts Hidden),
      `screen_indicator_layer() -> impl Bundle` (full-screen click-through
      container), `ScreenIndicatorPlugin`, public `ScreenIndicatorSystems`
      set, `prelude`. Follow the bevy_common_systems widget house style
      (health_display.rs).
- [x] Move `target_world_aabb` (and its two unit tests) from
      `hud/torpedo_target.rs` into the widget for the ApparentSize mode.
- [x] Implement the pure helpers with unit tests first: clamp-to-viewport-rect
      with margin, arrow angle from screen center toward the (possibly
      behind-camera) anchor direction, behind-camera handling via camera-space
      direction projected onto the view plane (sign-flipped), apparent-size
      fallback to `min_px` for Point anchors / missing AABBs.
- [x] Implement the per-frame systems in `ScreenIndicatorSystems`: resolve
      anchor (None or dead entity -> Hidden), project via
      `Single<(&GlobalTransform, &Camera), With<ScreenIndicatorCamera>>`,
      apply size mode, apply px offset, apply off-screen policy (Hide, or
      clamp + show/rotate the `ScreenIndicatorArrowMarker` descendant; arrow
      hidden while on-screen).
- [x] Nova glue in `hud/mod.rs`: register `ScreenIndicatorPlugin`, nest
      `ScreenIndicatorSystems` in `NovaHudSystems`, add observers
      `On<Add, SpaceshipCameraController>` / `On<Remove, ...>` inserting and
      removing `ScreenIndicatorCamera` (mirrors the loader.rs camera handoff),
      export the widget prelude from the hud prelude.
- [x] Migrate the torpedo reticle: `torpedo_target.rs` becomes a consumer -
      indicator node with `ImageNode(target_sprite)`,
      `ApparentSize { min_px: 32.0 }`, `Hide`; a driver system copies
      `SpaceshipPlayerTorpedoTargetEntity` into the anchor as `Entity`/None.
      Delete the bespoke projection/sizing/visibility systems.
- [x] Migrate the autopilot destination marker: `flight_status.rs` consumer -
      `Fixed(24.0)` tinted sprite, `Hide`; driver maps `Autopilot.action`
      GOTO target to anchor `Entity`/None. Delete `update_destination_marker`.
- [x] Behavioral tests (SystemState pattern, torpedo_target.rs precedent):
      anchor None -> Hidden, despawned anchor entity -> Hidden, Point anchor
      uses min_px fallback, offset shifts the node, clamp policy keeps the
      node inside the margin rect and rotates the arrow toward the anchor.
- [x] Add scripted range example `examples/12_hud_range.rs` following the
      11_com_range.rs pattern (BCS_AUTOPILOT timeline, mandatory `expect`
      lookups, asserted-at-exit guard, non-zero exit on failure): player ship
      + lockable target; assert reticle visible and centered on the target's
      projection while locked, hidden after the lock drops; assert the
      destination marker during a GOTO leg.
- [x] Verify: `cargo fmt`, `cargo check --workspace`, run only the newly
      written tests (user instruction: skip the full local test/clippy suite;
      report the skips honestly).
- [x] Document in `docs/2026-07-09-screen-indicator-widget.md`: what/why,
      alternatives from the spike, migration notes. Any "behavior unchanged"
      claim must be written from an enumeration of the actual consumers
      (retro lesson), and numbers must name their config.

## Notes

- Relevant files: crates/nova_gameplay/src/hud/{mod,torpedo_target,flight_status}.rs,
  crates/nova_scenario/src/loader.rs (camera handoff observers),
  crates/nova_gameplay/src/camera_controller.rs, examples/11_com_range.rs.
- avian3d is already a dependency of both nova_gameplay and
  bevy_common_systems, so `ColliderAabb` in the widget adds nothing.
- Keep the Update-schedule semantics of today (one-frame projection latency is
  pre-existing and out of scope; see spike Open questions).
- UI-pass only: no second Camera2d (blacks out the 3D scene on Bevy 0.19, see
  torpedo_target.rs comment), no gizmos.
- Promotion to bevy_common_systems is follow-up task 20260709-164608, not this
  task.
- Spike appendix (decisions made with the user, 20260709): location = nova hud
  module first; migrate BOTH overlays; off-screen Hide + ClampToEdge both now;
  sizing modes built in.

## Resolution (20260709)

Shipped: `hud/screen_indicator.rs` (widget: anchor enum, Fixed/ApparentSize
sizing, Hide/ClampToEdge policies with rotated arrow, layer helper, camera
marker, plugin + public system set), both overlays migrated to thin
driver-only consumers, camera glue observers in hud/mod.rs, 24 new
unit/behavioral tests (fabricated-camera projection tests included), scripted
range `examples/12_hud_range.rs` (PASS: reticle drift 0.0 px, GOTO marker
drift 0.1 px, both hide on target death). Full write-up with behavior deltas:
docs/2026-07-09-screen-indicator-widget.md.

Deliberate behavior deltas: behind-camera / dead-entity anchors now hide the
reticle instead of freezing it; laterally off-screen destinations hide
explicitly; a missing chase camera hides all indicators.

Difficulties: test expected-value math forgot the aspect division (the
implementation was right); Xvfb backgrounding swallowed a `cd` and ran cargo
in the wrong directory once. Reflection: pure `place()` function made the
policy matrix testable; keep drivers 5-line and pure-math helpers separate in
the consumer tasks. Skipped honestly per user instruction: full local test
suite and clippy (check + fmt + new tests only).
