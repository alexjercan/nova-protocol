# Spike: Concrete architecture of the screen-projected-indicator widget

- DATE: 20260709-164502
- STATUS: RECOMMENDED
- TAGS: spike, hud, weapons, architecture

## Question

The weapons-HUD spike (tasks/20260708-165647/SPIKE.md) already chose
"one generic screen-projected-indicator widget, UI-pass" as the substrate. This
spike settles the concrete architecture so /plan can break task 20260708-165700
into steps without re-litigating design: the component/API shape, the anchor
model, off-screen handling, sizing, camera discovery, where the code lives, and
whether the two existing bespoke overlays migrate onto it.

## Context

Two in-repo copies of the projection pattern exist today, and the arc adds two
more consumers immediately:

- `hud/torpedo_target.rs`: reticle on the locked target
  (`SpaceshipPlayerTorpedoTargetEntity` resource, an `Option<Entity>`). Sizes
  itself to the target's on-screen extent by unioning the collider AABBs of the
  target subtree (`target_world_aabb`) and projecting a radius-offset point.
  Hides when there is no lock. Full-screen click-through `Node` layer with an
  absolutely-positioned child moved via `Camera::world_to_viewport` each frame.
- `hud/flight_status.rs` (`autopilot_destination_hud`): fixed-size marker on the
  engaged GOTO destination entity. Same layer + projected child pattern,
  duplicated.
- Turret lead pip (task 20260708-165701) anchors to `TurretSectionAimPoint(pub
  Option<Vec3>)` on each turret section - a bare world point recomputed every
  frame, not an entity.
- Locked-target readout (task 20260708-165702) anchors to the same lock entity
  as the reticle and renders text (distance, closing speed) plus a health bar.

So anchors come in two shapes (entity to follow, world point pushed by a driver
system), sizing comes in two shapes (fixed px, apparent size), and visibility
is always "hide when the anchor is gone" plus a policy for "anchor is behind
the camera or outside the viewport".

House style for generic widgets (from `bevy_common_systems::ui`, e.g.
`health_display`): a `widget(config) -> impl Bundle` constructor, a public
marker component, retargetable state components, a plugin whose systems run in
a public `SystemSet`, and a `prelude` module. The HUD deliberately uses the UI
pass, not a second `Camera2d` (blacks out the 3D scene on Bevy 0.19, see
comment in torpedo_target.rs) and not gizmos (debug-grade). Both nova_gameplay
and bevy_common_systems already depend on avian3d, so collider-AABB sizing
adds no dependency either way.

## Decisions (made with the user, 20260709)

1. **Location: nova hud module first.** New
   `crates/nova_gameplay/src/hud/screen_indicator.rs`. The API settles under
   real consumers in one repo; promotion to bevy_common_systems is a seeded
   follow-up (see Next steps). Alternative (implement in bevy-common-systems
   now, bump the pinned rev) rejected because every API tweak during the arc
   would need a cross-repo push + rev bump.
2. **Migrate both existing overlays** (torpedo reticle, autopilot destination
   marker) onto the widget as part of the substrate task. This kills the
   existing duplication and proves the API covers the hard cases
   (apparent-size sizing, conditional visibility). Alternatives (migrate only
   the trivial destination marker, or neither) rejected: they leave the old
   pattern and the new widget side by side.
3. **Off-screen handling: full now.** `OffscreenPolicy::Hide` and
   `OffscreenPolicy::ClampToEdge` both implemented behind an enum, clamp
   including an optional direction arrow child. Edge math is core widget
   competency and testable in isolation; the v0.5.0 off-screen-indicators task
   (20260708-165704) becomes pure consumer work. Alternative (Hide only, enum
   reserved) rejected by user.
4. **Sizing modes built in.** `Fixed(px)` and `ApparentSize { min_px }`, moving
   the `target_world_aabb` union code into the widget. Needed to migrate the
   reticle; future silhouette-tracking indicators reuse it. Alternative
   (position/visibility only) rejected: the reticle would keep bespoke code.

## Architecture

### Components (on the indicator node itself)

- `ScreenIndicatorMarker` - marker, public.
- `ScreenIndicatorAnchor(pub Option<ScreenIndicatorAnchorKind>)` with
  `enum ScreenIndicatorAnchorKind { Entity(Entity), Point(Vec3) }`.
  `None` -> indicator hidden. `Entity` -> follow that entity's
  `GlobalTransform` (hidden if it no longer resolves). `Point` -> project the
  given world point; driver systems overwrite it each frame (turret pip).
  Retargeting is just writing this component.
- `ScreenIndicatorSize` - `enum { Fixed(Vec2), ApparentSize { min_px: f32 } }`.
  `ApparentSize` unions the collider AABBs of the anchor-entity subtree
  (code moves from torpedo_target.rs) and falls back to `min_px` for `Point`
  anchors or when no AABB exists yet (spawn frame).
- `ScreenIndicatorOffset(pub Vec2)` - px offset applied after projection,
  default zero. Lets a readout sit beside its anchor point.
- `ScreenIndicatorOffscreen` - `enum { Hide, ClampToEdge { margin_px: f32 } }`.
  Hide: `Visibility::Hidden` when the anchor fails to project or is outside
  the viewport. ClampToEdge: clamp the node to the viewport rect inset by
  margin, and if a descendant carries `ScreenIndicatorArrowMarker`, show it
  and rotate it (UI `Transform` rotation) to point from screen center toward
  the anchor direction; when on-screen the arrow is hidden. Behind-camera
  anchors clamp using the camera-space direction projected onto the view
  plane (sign-flipped), so the arrow still points the right way.

### Node structure: the indicator IS the projected node

Unlike today's overlays (each spawns its own full-screen layer with one child),
the widget bundle `screen_indicator(config) -> impl Bundle` is the
absolutely-positioned node itself: `(Name, ScreenIndicatorMarker, anchor, size,
offset, offscreen policy, Node { position_type: Absolute }, Pickable::IGNORE,
Visibility::Hidden)`. Consumers parent it under a shared full-screen
click-through layer and attach arbitrary content as children (an `ImageNode`
sprite, `Text` lines, a health bar, an arrow node). A
`screen_indicator_layer() -> impl Bundle` helper provides the full-screen
`Node` + `Pickable::IGNORE` container so consumers stop copy-pasting it.
This is what makes the readout composable: its text/bar children are ordinary
UI, and the widget only moves/sizes/hides the container node.

### Camera discovery

A public `ScreenIndicatorCamera` marker component names the camera used for
projection; the widget systems take `Single<(&GlobalTransform, &Camera),
With<ScreenIndicatorCamera>>`. Nova inserts the marker where the spaceship
camera is spawned (alongside `SpaceshipCameraController`). Hardcoding
`SpaceshipCameraController` in the widget was rejected as unpromotable; the
marker matches the direction of task 20260708-224254 (dedicated listener
marker for SFX).

### Systems and scheduling

One plugin (`ScreenIndicatorPlugin`) with systems in a public
`ScreenIndicatorSystems` set: resolve anchor -> project -> apply size ->
apply offset -> clamp-or-hide -> arrow. Nova nests the set inside
`NovaHudSystems` (Update), matching today's behavior. The known one-frame
latency (reading `GlobalTransform` in Update means last frame's propagation)
is unchanged by this arc; see Open questions.

### Consumers after the migration

- Torpedo reticle: indicator with `ImageNode(target_sprite)`, driver system
  copies `SpaceshipPlayerTorpedoTargetEntity` into the anchor
  (`Entity`/`None`), `ApparentSize { min_px: 32.0 }`, `Hide`.
- Autopilot destination: anchor driven from `Autopilot.action` (GOTO target
  entity or `None`), `Fixed(24.0)`, `Hide`, tinted sprite as child.
- Turret lead pip (165701): one indicator per player-ship turret; driver
  copies that turret's `TurretSectionAimPoint` into the anchor as
  `Point`/`None`. Spawn/despawn via observers on turret add/remove under the
  player ship, following the hud/mod.rs observer pattern. `Fixed`, `Hide`.
- Locked-target readout (165702): indicator on the same lock anchor with a px
  offset; children are `Text` (distance, closing speed) and a small bar for
  `Health`. Closing speed = -(rel_velocity dot los_dir) from `LinearVelocity`
  of both ships, positive when approaching. Whether it instead nests inside
  the reticle indicator (tracking its scaled edge via UI layout) is decided at
  /plan time for 165702; the widget supports both.

## Options considered (design axes not user-facing)

- **Anchor model**: enum Entity|Point on one component (chosen) vs separate
  component types per anchor kind (two query paths, no gain) vs always-Vec3
  written by drivers (loses "hide when entity dies" for free and makes every
  consumer write a driver even for plain entity-following).
- **Node structure**: indicator-is-the-node + shared layer helper (chosen) vs
  constructor returns layer+child like today (blocks arbitrary content
  children, N full-screen nodes for N indicators).
- **Camera discovery**: public marker component (chosen) vs config field
  holding the camera `Entity` (per-indicator boilerplate, stale on camera
  respawn) vs hardcoded `SpaceshipCameraController` (unpromotable).

## Open questions

- **One-frame projection latency.** All overlays read `GlobalTransform` in
  Update, one propagation behind what is rendered. Fixing it means running
  `ScreenIndicatorSystems` in PostUpdate after transform propagation but
  before UI layout; do it inside the widget later without API change, if the
  lag ever shows (fast camera + close targets). Related to the recent camera
  interpolation work (docs/retros, camera twitch).
- **Readout attachment.** Sibling-with-offset vs child-of-reticle (tracks the
  scaled reticle edge for free). Decide when planning 165702.
- **Promotion timing.** After this arc the widget has 4 consumers and the API
  is exercised; promote to bevy_common_systems then (seeded task below).

## Next steps

Existing direction-level tasks now carry these decisions (for /plan to break
into steps):

- tatr 20260708-165700: the widget itself + migrating both existing overlays
- tatr 20260708-165701: turret lead pip as a widget consumer
- tatr 20260708-165702: locked-target readout as a widget consumer

Seeded by this spike:

- tatr 20260709-164608: promote the screen-indicator widget to
  bevy_common_systems once the API is stable (v0.5.0)
