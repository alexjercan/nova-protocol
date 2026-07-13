# Ammo HUD readout for weapon sections

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.5.0,weapons,hud

## Goal

Show remaining ammo per player weapon, diegetically ON the weapon, so the
player can see a turret or torpedo bay running dry without a corner panel.
Chunked/quantized, not a number: a draining ring for turrets, a `||||` pip row
for torpedo bays; infinite-ammo weapons show nothing; a real number only in a
debug mode. Firing a finite weapon visibly drains its readout.

## Steps

- [x] Add `crates/nova_gameplay/src/hud/ammo_readout.rs`. Define markers
      (`AmmoReadoutHudMarker` for the full-screen layer, `AmmoReadoutMarker`
      for one readout node, `AmmoReadoutSection(Entity)` for the weapon section
      it tracks) and a `prelude`. Mirror the shape of
      `hud/turret_lead.rs`.
- [x] `ammo_readout_hud()` layer bundle: `Name` + `AmmoReadoutHudMarker` +
      `screen_indicator_layer()`. Spawned/despawned with the player ship by the
      hud/mod.rs observers below.
- [x] Per-weapon readout bundle: `screen_indicator(ScreenIndicatorConfig { anchor: Some(Entity(section)), size: ApparentSize { min_px }, offset, offscreen: Hide })`
      plus `AmmoReadoutMarker`, `AmmoReadoutSection(section)`, and content
      children. Anchor by Entity so it rides on the weapon; min_px keeps it
      legible when small/distant.
- [x] Content, chunked, no art assets. Turret: a fixed `RING_SEGMENTS` (const,
      e.g. 8) child pip nodes arranged in a circle via absolute
      left/top offsets computed from the segment angle; a segment is lit or
      dim by `BackgroundColor`. Torpedo: a horizontal row of `capacity` child
      pip nodes (torpedo capacities are small), lit vs dim. Give each pip a
      marker carrying its index so the driver can address it. Tag the readout
      with which family it is (an enum component, `AmmoReadoutKind::{Turret,Torpedo}`)
      so the driver knows how to map fraction -> lit set.
- [x] Debug number: a `Text` child of the readout (marker `AmmoReadoutNumber`)
      showing `rounds/capacity`, `Visibility::Hidden` by default. Add a
      `#[derive(Resource)] AmmoReadoutDebug(pub bool)` (default false) and a
      toggle system on an unused key (follow the F11 debug-toggle pattern noted
      in hud/mod.rs `cycle_hud_visibility`; pick a free key, run_if Playing).
- [x] Reconcile system `sync_ammo_readouts` (mirror `sync_turret_pips`): keep
      exactly one readout per player weapon section that BOTH is a child of the
      player `SpaceshipRootMarker`+`PlayerSpaceshipMarker` AND has a
      `SectionAmmo`. Query turret (`TurretSectionMarker`) and torpedo
      (`TorpedoSectionMarker`) sections. Despawn readouts whose section died,
      left the player, or lost its `SectionAmmo`; spawn readouts (with the
      right `AmmoReadoutKind` and pip count) for player weapon sections with
      `SectionAmmo` that have none. Idempotent. Sections WITHOUT `SectionAmmo`
      (infinite ammo, `ammo_capacity = None`) get no readout - the "don't even
      show it" case, for free.
- [x] Driver system `drive_ammo_readouts` (mirror `drive_pip_anchors`): for
      each readout read its section's `SectionAmmo { rounds, capacity }`; turret
      -> lit segment count = `round(rounds/capacity * RING_SEGMENTS)`, torpedo
      -> lit pip count = `rounds`. Set each pip child's `BackgroundColor` (or
      Visibility) lit/dim accordingly, and update the debug `Text` to
      `"{rounds}/{capacity}"`. Read a single ammo pool so a future
      per-bullet-type magazine (20260708-162005) is a one-line source swap.
      Gate the debug Text visibility on `AmmoReadoutDebug`.
- [x] `AmmoReadoutPlugin`: register the resource + type reflections, add
      `sync_ammo_readouts` and `drive_ammo_readouts` in PostUpdate chained
      `.before(ScreenIndicatorSystems)` (same slot as TurretLeadPlugin), add the
      debug toggle system in Update run_if Playing.
- [x] Wire into `hud/mod.rs`: `pub mod ammo_readout;`, re-export its prelude,
      `app.add_plugins(ammo_readout::AmmoReadoutPlugin)`, and add
      `setup_hud_ammo_readout` / `remove_hud_ammo_readout` observers on
      `Add`/`Remove` of `PlayerSpaceshipMarker`, spawning
      `(HudTier::Instrument, ammo_readout_hud())` (copy setup_hud_turret_lead).
- [x] Tests (in-module, headless, follow turret_lead + screen_indicator test
      style): reconcile spawns one readout per player weapon section WITH
      SectionAmmo; ignores a same-ship weapon with NO SectionAmmo (infinite);
      ignores other ships' weapons; despawns on section death and on
      SectionAmmo removal; idempotent. Driver: fraction -> lit segment count
      buckets correctly at full/partial/empty for a turret; lit pip count ==
      rounds for a torpedo; debug Text reflects rounds/capacity and its
      visibility follows `AmmoReadoutDebug`.
- [x] Verify: `cargo fmt` + `cargo check -p nova_gameplay`, and run the new
      module's tests (`cargo test -p nova_gameplay ammo_readout`). Per repo
      convention CI runs the full suite; do not run the whole local suite.

## Notes

- Spike: tasks/20260712-143113/SPIKE.md (Option B,
  fully settled - do not re-litigate the substrate).
- Substrate: `crates/nova_gameplay/src/hud/screen_indicator.rs` -
  `screen_indicator`, `screen_indicator_layer`, `ScreenIndicatorConfig`,
  `ScreenIndicatorAnchorKind::Entity`, `ScreenIndicatorSize::ApparentSize`,
  `ScreenIndicatorOffscreen::Hide`, `ScreenIndicatorSystems`.
- Template consumer: `crates/nova_gameplay/src/hud/turret_lead.rs`
  (`sync_turret_pips` reconcile, `drive_pip_anchors` driver, `TurretLeadPlugin`
  ordering `.after(...).before(ScreenIndicatorSystems)`).
- Observers + tier: `hud/mod.rs` `setup_hud_turret_lead` / `remove_hud_turret_lead`
  (line ~558), spawned `(HudTier::Instrument, ...)`. `HudVisibility::shows`
  keeps `Instrument` in the Minimal level and clears it in None.
- Ammo model: `crates/nova_gameplay/src/sections/ammo.rs` -
  `SectionAmmo { rounds, capacity }` on the section entity; absence == infinite.
  `infinite_ammo` player flag forces `ammo_capacity = None`
  (`nova_scenario/src/objects/spaceship.rs`), i.e. no component.
- Markers: `TurretSectionMarker`, `TorpedoSectionMarker`,
  `SectionInactiveMarker`, `SpaceshipRootMarker`, `PlayerSpaceshipMarker` (all
  in the nova_gameplay prelude, as used by turret_lead).
- Debug toggle: no dev-overlay resource exists yet; add a tiny
  `AmmoReadoutDebug(bool)` + key toggle, patterned on `cycle_hud_visibility`.
- Depends in spirit on the reload/damage-type pass (20260708-162005): if it
  splits SectionAmmo into per-bullet-type magazines the readout should show the
  selected type; the single-pool driver read keeps that a one-line swap.
- Assumption: no ring/pip art assets - segments and pips are plain colored
  `Node`s, consistent with the other UI-pass overlays. A textured ring is a
  later polish, not this task.

## Implementation record

- Landed `hud/ammo_readout.rs` + hud/mod.rs wiring. 9 headless tests pass;
  `cargo check`/`clippy -p nova_gameplay` clean.
- Deviation from step 3: used `ScreenIndicatorSize::Fixed` (turret
  `RING_PX` square; torpedo `capacity`-wide bar) instead of `ApparentSize`.
  The gauge content is a fixed pip layout (absolute ring offsets / fixed bar
  widths), which needs a stable node size, and a constant-size status gauge
  stays legible without scaling with the weapon silhouette. Position still
  follows the weapon via the Entity anchor + a small up-right offset, so it
  still reads as riding on the weapon.
- Debug number key is F10 (F1 = editor, F11 = nova_debug; F10 free). Owns its
  own `AmmoReadoutDebug` resource because nova_gameplay cannot depend on
  nova_debug (reverse dependency).
- Turret ring lights >=1 segment while any round remains (`turret_lit_segments`
  clamps to `[1, RING_SEGMENTS]` when non-empty) so a nearly-spent turret does
  not read as empty.
