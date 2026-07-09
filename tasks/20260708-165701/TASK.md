# Turret lead/intercept pip (HUD)

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.4.0,hud,turret,spike

## Goal

Draw the turret's already-computed intercept point. The turret plugin computes
`TurretSectionAimPoint(Option<Vec3>)` via `lead_intercept_point`
(crates/nova_gameplay/src/sections/turret_section.rs) but nothing renders it.
A HUD pip at that projected point shows the player the lead the turret is
taking. Pure rendering off existing data, wasm-safe, first fresh consumer of
the screen-indicator widget.

Spikes: docs/spikes/20260708-165647-weapons-hud.md,
docs/spikes/20260709-164502-screen-indicator-architecture.md.

## Steps

- [ ] Create `crates/nova_gameplay/src/hud/turret_lead.rs`: pip consumer
      module with `TurretLeadPipMarker` and `TurretLeadPipTurret(Entity)`
      linking each pip to its turret section. Pip visual: small fixed-size
      plain `Node` (about 8 px, tinted `BackgroundColor`), no new sprite
      asset; `Fixed` size, `Hide` off-screen policy.
- [ ] Lifecycle via observers (hud/mod.rs pattern): on
      `On<Add, PlayerSpaceshipMarker>` spawn one pip per turret child of the
      ship root (`ChildOf` == root + `With<TurretSectionMarker>`, the
      player.rs:158 enumeration pattern) under a `screen_indicator_layer()`;
      despawn pips on `On<Remove, PlayerSpaceshipMarker>` and when their
      turret entity is despawned (`On<Remove, TurretSectionMarker>`).
- [ ] Driver system: copy each turret's `TurretSectionAimPoint` into the
      pip's `ScreenIndicatorAnchor` as `Point`/None each frame; force None
      when the turret carries `SectionInactiveMarker` (the aim-point system
      does not clear stale values for inactive turrets - verified in
      turret_section.rs `update_turret_aim_point`).
- [ ] Register the module in `NovaHudPlugin` and the hud prelude.
- [ ] Behavioral tests: aim point Some -> anchor Point, inactive turret ->
      anchor None, turret despawn -> pip despawned, second turret gets its
      own pip.
- [ ] Extend `examples/12_hud_range.rs`: give the ship a turret and a moving
      target; assert a pip is visible near the projected
      `TurretSectionAimPoint` while tracking, and disappears when the turret
      loses its target (mandatory expects, asserted-at-exit guard).
- [ ] Verify: `cargo fmt`, `cargo check --workspace`, run only the newly
      written tests (skip full suite per user instruction; report skips).
- [ ] Extend `docs/2026-07-09-screen-indicator-widget.md` with the pip
      consumer section.

## Notes

- Depends on: 20260708-165700 (screen-indicator widget).
- `TurretSectionAimPoint` lives on the turret section entity, a direct child
  of the ship root; per-turret pips, so a multi-turret ship shows several.
- The aim point is a bare world point recomputed each frame - this is the
  `Point` anchor case the widget was designed for.
