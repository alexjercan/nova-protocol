# Turret lead/intercept pip (HUD)

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.4.0, hud, turret, spike

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

- [x] Create `crates/nova_gameplay/src/hud/turret_lead.rs`: pip consumer
      module with `TurretLeadPipMarker` and `TurretLeadPipTurret(Entity)`
      linking each pip to its turret section. Pip visual: small fixed-size
      plain `Node` (8 px, amber `BackgroundColor`), no new sprite asset;
      `Fixed` size, `Hide` off-screen policy.
- [x] Lifecycle (adjusted from the planned observers, see Notes): the layer
      spawns/despawns with the player ship via hud/mod.rs observers like the
      sibling overlays, but pip membership is a per-frame reconcile system
      `sync_turret_pips` - one idempotent pass spawns missing pips and
      despawns orphans, covering turrets destroyed mid-fight and sections
      attached after the player marker without observer choreography.
- [x] Driver system: copy each turret's `TurretSectionAimPoint` into the
      pip's `ScreenIndicatorAnchor` as `Point`/None each frame; force None
      when the turret carries `SectionInactiveMarker` (the aim-point system
      does not clear stale values for inactive turrets - verified in
      turret_section.rs `update_turret_aim_point`).
- [x] Register the module in `NovaHudPlugin` and the hud prelude.
- [x] Behavioral tests: aim point Some -> anchor Point, inactive turret ->
      anchor None, turret despawn -> pip despawned, second turret gets its
      own pip, other ships' turrets ignored, reconcile idempotent.
- [x] Extend `examples/12_hud_range.rs`: player ship gains a
      better_turret_section; assert the pip is visible on the projected
      `TurretSectionAimPoint` while tracking (0.0 px drift) and hides when
      the section is disabled (mandatory expects, asserted-at-exit guard).
- [x] Verify: `cargo fmt`, `cargo check --workspace`, run only the newly
      written tests (skip full suite per user instruction; report skips).
- [x] Extend `docs/2026-07-09-screen-indicator-widget.md` with the pip
      consumer section.

## Notes

- Depends on: 20260708-165700 (screen-indicator widget).
- `TurretSectionAimPoint` lives on the turret section entity, a direct child
  of the ship root; per-turret pips, so a multi-turret ship shows several.
- The aim point is a bare world point recomputed each frame - this is the
  `Point` anchor case the widget was designed for.

## Resolution (20260709)

Shipped: `hud/turret_lead.rs` (one amber 8 px pip per player turret on the
turret's `TurretSectionAimPoint`, Point-anchored on the screen-indicator
widget), layer observers in hud/mod.rs, 4 behavioral tests, pip stages in
`examples/12_hud_range.rs` (PASS: pip on the projected aim point at 0.0 px
drift, hidden after the section is disabled). Doc section added to
docs/2026-07-09-screen-indicator-widget.md.

Deviation from plan: pip membership is a reconcile system rather than
add/remove observers - turrets can be destroyed mid-fight and sections can
attach after `PlayerSpaceshipMarker`, and a single idempotent pass covers
every ordering; the plan step was updated to match. The player-input turret
never drops its target input (it always aims down the camera ray), so the
example exercises the disappearing-pip path through `SectionInactiveMarker`,
the same marker the health pipeline sets on disabled sections.

Skipped honestly per user instruction: full local test suite and clippy
(check + fmt + the 29 hud tests only).
