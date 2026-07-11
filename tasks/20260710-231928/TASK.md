# HUD text anchored to moving objects twitches (e.g. velocity on the ship)

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.5.0, hud, bug

## Goal

Playtest bug (user, 2026-07-10): HUD text anchored to moving objects
twitches - most visibly the velocity/speed text at the ship. Root cause
(docs/spikes/20260711-103527-twitching-family-two-clocks.md): screen
indicators project in Update with the camera pose from the PREVIOUS frame
(the bcs chase camera moves later, in PostUpdate), so HUD nodes lag the
rendered world by one frame of camera motion; additionally Point anchors
are computed from raw avian `Position` while Entity anchors resolve eased
`GlobalTransform` - two pose families in one overlay.

## Steps

- [x] Check whether bevy-common-systems exposes a public system set for
      the chase-camera PostUpdate systems. ANSWER: yes -
      `ChaseCameraSystems::Sync` is public and in the bcs prelude, so NO
      cross-repo change was needed. HOWEVER the sweep found that bcs
      leaves `Sync` UNORDERED against `TransformSystems::Propagate`: if
      propagation wins the race the whole frame renders with LAST frame's
      camera pose (per-build coin flip). Nova now pins
      `ChaseCameraSystems::Sync.before(TransformSystems::Propagate)` in
      its camera controller plugin - additive set configuration, no bcs
      commit or push required. Filing the fix upstream in bcs remains
      nice-to-have (benefits other consumers) but is not blocking.
- [x] Move `update_screen_indicators` to PostUpdate. Slot discovered
      during implementation: bevy_ui runs `UiSystems::Layout` BEFORE
      `TransformSystems::Propagate`, so the originally planned
      "after Propagate, before Layout" is impossible. The landed slot is
      `after(ChaseCameraSystems::Sync).before(UiSystems::Layout)`, with
      the system composing FRESH poses via `TransformHelper` (camera pose
      the chase camera just wrote, eased anchor poses the frame renders
      with) instead of reading pre-propagation `GlobalTransform`.
      Update-schedule drivers precede the projection by schedule order;
      their now-dead `.before(ScreenIndicatorSystems)` constraints were
      removed (6 files).
- [x] Anchor pose sweep: Entity anchors resolve fresh at projection time
      (above). Ship-attached holo geometry read the ship's raw avian
      `Position` in three drivers - radius spoke chip midpoint, radius
      spoke line, trajectory ribbon ship end (+ flip gate direction for
      uniformity); all now read the ship root's eased `Transform` (the
      rendered pose). Well positions stay raw `Position` - wells are
      static so the clocks agree. Remaining Point anchors (GotoPos,
      telemetry goal, flip point) are plan geometry, not rendered bodies:
      documented as fine.
- [x] Regression `indicator_projects_with_the_frames_final_camera_pose`
      (screen_indicator.rs tests): smoothed chase camera trailing a
      cruising ship; every frame the node position must match a projection
      recomputed from the END-of-frame (rendered) camera + ship poses
      within 0.5 px, with delivery guards (camera actually moved,
      indicator actually visible). A/B-proven: registering the projection
      back in Update fails with a worst mismatch of 54 px.
- [x] cargo check (workspace, all targets) + fmt clean; full nova_gameplay
      lib suite 355/355; spike doc fix record extended.

## Notes

- Evidence (pre-fix): screen_indicator.rs Update registration +
  world_to_viewport with pre-move camera pose; plugin.rs Update set chain
  (NovaHudSystems before NovaCameraSystems); bcs chase.rs:149 (camera
  moves in PostUpdate).
- 20260710-231929 (crosshair) depends on this task's PostUpdate move; its
  pip-staleness fix chains into the same slot.
- Text VALUES (speed numbers, distances) still read whatever their
  drivers read - sub-unit differences invisible in text; only placement
  was in scope.

## Resolution

What changed: projection moved to the PostUpdate slot between the chase
camera and UI layout with TransformHelper-composed poses; the bcs
Sync-vs-Propagate ambiguity pinned from nova; three holo drivers moved to
the render-clock ship pose; one behavioral regression with A/B evidence;
test rigs updated from GlobalTransform/Position spawns to Transform
spawns (the system now composes from Transform - MORE production-shaped,
since propagation never ran inside the old single-system rigs anyway).

Alternatives considered:

- Projecting after TransformSystems::Propagate and reading GlobalTransform
  directly: cleaner reads, but bevy_ui's Layout-before-Propagate ordering
  means node positions would land one frame late - the bug reintroduced
  one schedule further down.
- Fixing the Sync-vs-Propagate ambiguity in bcs itself: correct long-term
  home, but requires a bcs push and rev bump (outward-facing); the
  additive configure_sets from nova is equivalent for this repo and keeps
  the branch self-contained.

Difficulties: the planned slot ("after Propagate") was impossible against
bevy_ui's real ordering; discovered by reading bevy_ui's plugin source
before coding rather than after a mysterious one-frame UI lag.

Self-reflection: reading the dependency's ACTUAL schedule configuration
(bcs chase, bevy_ui) before implementing avoided two wrong designs in one
task. The step description in the plan encoded assumptions about both;
next time write ordering steps as questions, not directions.
