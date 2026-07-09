# Locked-target info readout: distance, closing speed, health (HUD)

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.4.0,hud,torpedo,spike

## Goal

Alongside the target reticle, show the locked target's range (|target -
ship|), closing speed (relative velocity along the line of sight, from
`LinearVelocity`), and a small health bar (`Health` is on target roots). All
data is already queryable; rendering only, wasm-safe. The lock comes from
`SpaceshipPlayerTorpedoTargetEntity`; the readout rides the same reticle
indicator the widget task migrated.

Spikes: docs/spikes/20260708-165647-weapons-hud.md,
docs/spikes/20260709-164502-screen-indicator-architecture.md.

## Steps

- [ ] Attachment (decided at plan time): the readout is a child of the
      reticle indicator node in `torpedo_target.rs`, absolutely positioned at
      `left: 100%` plus a small margin - it tracks the reticle's scaled edge
      via UI layout for free and inherits the reticle's visibility. Add
      `TorpedoTargetReadoutMarker` on the container plus child `Text` nodes
      (distance line, closing-speed line) and a health bar (outer fixed-width
      `Node` + inner percent-width `Node` with `BackgroundColor`, marker on
      the inner node).
- [ ] Update system in `torpedo_target.rs` (runs with the existing driver in
      `NovaHudSystems`): when the lock resolves, set distance = |target_pos -
      ship_pos|, closing speed = -(rel_velocity dot los_dir) (positive when
      approaching) from both `LinearVelocity`s, and the bar's inner width =
      `Health.current / Health.max` percent from the target root's `Health`.
      Missing `LinearVelocity` or `Health` on the target degrades gracefully:
      show distance, blank the speed line / hide the bar.
- [ ] Match the formatting conventions of `flight::flight_status_line`
      (units, precision) so the HUD reads consistently.
- [ ] Unit tests: closing-speed sign (approaching positive, receding
      negative, pure crossing near zero), formatting, health fraction clamped
      to [0, 1].
- [ ] Behavioral test: lock set -> readout text populated; lock cleared ->
      readout hidden with the reticle (inherited visibility).
- [ ] Extend `examples/12_hud_range.rs`: while locked onto the moving target,
      assert the distance text matches the actual separation within
      tolerance, the closing-speed sign matches the actual approach, and the
      bar fraction matches the target's `Health` (mandatory expects,
      asserted-at-exit guard).
- [ ] Verify: `cargo fmt`, `cargo check --workspace`, run only the newly
      written tests (skip full suite per user instruction; report skips).
- [ ] Extend `docs/2026-07-09-screen-indicator-widget.md` with the readout
      consumer section.

## Notes

- Depends on: 20260708-165700 (screen-indicator widget; reticle already a
  consumer after it).
- Ship side of the math uses the player ship root
  (`SpaceshipRootMarker` + `PlayerSpaceshipMarker`) transform and
  `LinearVelocity`.
- Closing speed sign convention: positive = closing, negative = opening;
  label it in the UI so the sign is legible (e.g. "CLS +12 m/s").
- Child-of-reticle attachment was chosen over sibling-with-offset at plan
  time (spike left it open): it tracks the ApparentSize-scaled reticle edge
  with zero extra code. If review finds it crowds large silhouettes, the
  fallback is a sibling indicator with `ScreenIndicatorOffset`.
