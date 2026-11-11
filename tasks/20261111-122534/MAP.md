# Feedback polish: slice map (read-only, 2026-11-11)

Evidence from two read-only passes over master at 652ae0003. Paths relative
to the repo root.

## Reference: the v0.17.0 torpedo chain
- Optional sound fields on the section config: `crates/nova_ship/src/sections/torpedo_section/mod.rs:248-281`
  (`Option<AssetRef<AudioSource>>`, `#[reflect(ignore)]`, serde default/skip),
  defaults `None` at :345-348. The torpedo carries a copy
  (`TorpedoLaunchConfig`, `bay.rs:260`, inserted :387).
- Presentation sounds: `crates/nova_scenario/src/objects/ship_design.rs:196,202`,
  copied to `ShipFeedbackSounds` (`spaceship.rs:1733-1745`,
  `crates/nova_ship/src/ship_audio/feedback.rs:32-75`). The world generator
  destructures every presentation field (`crates/nova_world/src/generation.rs:988-1027`).
- Lint: empty path `unnamed_files` (`crates/nova_scenario/src/lint/ship.rs:744-752`,
  used by intake :669-675 and mining :771-777; torpedo/railgun/docking do not
  call it: :485, :510, :553). Undeclared file: `resource_ref_violations`
  (`crates/nova_assets/src/mod_refs.rs:300`); new files go in
  `assets/base/base.bundle.ron` resources (sounds from :274).
- Base refs `crates/nova_authoring/src/base_content/assets.rs`; set in
  `base_content/sections/torpedo_bay.rs:211-216`, `ships/mod.rs:255-256`;
  editor voice `crates/nova_editor/src/scenario.rs:646-647,1686-1690`.
  Regenerate with `cargo run -- content gen`; parity test
  `crates/nova_authoring/tests/content_ron_parity.rs`.
- Playback: observers registered `crates/nova_ship/src/ship_audio/mod.rs:285-301`,
  volumes :84-198. Ignition `combat.rs:363-398` (`SfxThrottle`,
  `play_sfx_at`). Dud/intercept :451-487. Kill stinger :495-510 on
  `PlayerKillConfirmed` (`nova_gameplay/src/juice.rs:103`). Routing
  `routing.rs:35-56`: `Hull` for the player's ship, `Exterior` positional.
- Graphics: `GraphicsBudget::for_quality` `crates/nova_gameplay/src/settings.rs:265-301`
  (High particles + 6 lights, Medium 3, Low none). Lights are `LightFlash`
  requests (`nova_gameplay/src/transient_light.rs:81`, answered :153-187,
  cap :63-75); call sites never check the tier. Particle-style meshes check
  `budget.as_deref().is_none_or(|b| b.particles)`. Sound plays on every tier.
  Low tests: `trail.rs:228`, `transient_light.rs:280`, `settings.rs:606`.
- Launch light: `torpedo_section/render.rs:655-690` (srgb 0.55/0.72/1.0,
  20000 lm, range 10, 0.12 s). Blast shell mesh: `death.rs:126-223`.
- HUD notice: write `HudNotice(String)` (`crates/nova_hud/src/hud_notice.rs:74`),
  one slot, 4.0 s + 0.4 s fade, plays `hud_notice.wav` on a new text.
  Posters: dock refusal :209-221, DOCK press refusal :245-268, MATCH :224-243,
  job done :273-296. Tests `notice_app` :395.

## Docking
- Request `DockingConnectionRequest` (`docking_section/connection.rs:274`) ->
  `on_docking_connection_request` :326-412; `connect()` :420-458; ports set
  Extending :407-411. Release `DockingReleaseRequest` :305 ->
  `on_docking_release_request` :757-781 -> `release_connection` :972-1024
  (also runs for destroyed port :668-686 and lost hull :690-712).
  `restore_docking_connection` :511 fires nothing (load, thaw).
- No nova_ship dock event. Scenario `OnDocked/OnUndocked` from
  `crates/nova_scenario/src/loader/trackers.rs:397-438`, named hulls only.
- `DockingSectionConfig` `mod.rs:84-120` has no sound field. Sleeve animation
  `drive_docking_section_state` `mod.rs:267-296`. `machinery.rs:1-11` covers
  PDC housing, bay iris, intake, mining lids.
- Base: `docking_port_section` (`assets/base/sections/base.content.ron:2765-2876`),
  builder `base_content/sections/docking_port.rs:62-111`, used by
  `ships/block.rs:793` and `stations/mod.rs:50`.
- HUD on dock: mode chip `crates/nova_hud/src/flight_status.rs:647-699`.
- Tests: `docking_section/tests.rs:368,666,713,1601`; `trackers.rs:1755,1845`.
  Examples `examples/systems/system_docking_ports.rs`, `system_station_docking.rs`,
  `examples/screenshots/loop_docked_helm.rs`.

## Pickup
- `run_cargo_intakes` `crates/nova_ship/src/sections/cargo_intake_section.rs:516-705`;
  accept :624-655: `inventory.add`, `ShipInventoryReport::PickedUp` :641-647,
  `CargoCanisterTaken { entity: intake }` :649. Events :150-174. Sounds
  required :109-113, played `ship_audio/machinery.rs:115-212`
  (`SALVAGE_PICKUP_VOLUME` 0.22). Report queue
  `nova_gameplay/src/inventory.rs:635-793`, drained by `trackers.rs:565-606`.
- HUD: `pickup_sight.rs` (`sync_pickup_sight` :231), `cargo_canister_chips.rs`
  (:37 100 m, :156). No `+N` anywhere.
- Tests: `cargo_intake_section/tests.rs:179,215,342,518`; `pickup_sight.rs:799`;
  `machinery.rs:400-450`. Example `examples/screenshots/loop_intake_compare.rs`.

## Railgun impact
- Damage: `apply_damage` `crates/nova_gameplay/src/damage.rs:322-345` via
  `spend_piercing_damage` :479-497 from `rounds.rs:962,1120` (source = round).
  `SurfaceImpact { entity, kind, at }` :291-301, no source. Round markers:
  `RailgunSlugProjectileMarker` (`railgun_section/firing.rs:190-206`, Pierce
  :203), `TurretBulletProjectileMarker` (`turret_section/render.rs:238`).
  Slug removal observer `wake.rs:397-425`.
- Impact sound bank fixed: `impact_sound.rs:26-29,73-80`;
  `on_surface_impact_play_sfx` `ship_audio/combat.rs:213-246`.
- Sparks `ImpactSparks { at, count, force }` (`impact_spark.rs:39-48,150-192`,
  cuboid meshes, `TempEntity(0.28)`). Juice `juice.rs:415-519`, sparks at the
  section origin; `ShakeSettings` :111-160; `CameraShakeInput.add_trauma`.
- Railgun visuals: muzzle `LightFlash` `render.rs:294-303`, kick :316-339,
  slug `CappedLight` `wake.rs:507-536`, wake :157, plugin `mod.rs:401-422`.
  Charge glow (long-lived light pattern) `render.rs:213-233,258-285`.
- No ricochet/miss event, no decal. Tests `combat.rs:588,663`, `juice.rs:849`,
  `rounds.rs:2509`; example `examples/playable/railgun_wake_bench.rs:893`.

## Kill credit
- `torpedo_section/kill_credit.rs`: stamp :60-91 (NovaBlast +
  TorpedoSectionPartOf only), window :18, spend :95-134, confirm :138-156,
  registered `torpedo_section/mod.rs:1053-1055`. Stinger `combat.rs:495-510`.
  Jolt `juice.rs:574-586` (High only). General destroy kick :529-570.
- Attribution: `LastAttacker` (`input/ai/threat.rs:281`, stamp :287-314,
  `resolve_damage_attacker` :232-260 via `ProjectileOwner`; slug owner
  `firing.rs:192`). Consumer `nova_world_base/src/stance.rs:187-212`.
- Defeat: `DefeatedMarker` (`integrity/neutralize.rs:52,240`), destroy
  `explode.rs:301-302`. Test `kill_credit.rs:233` (rig :166-224).

## Sector crossing
- `CurrentSector` `crates/nova_world/src/streaming.rs:137`, written by
  `track_current_sector` :573-603 (assign on change :595-598, insert on arm
  :600), set `NovaWorldSystems::Observe` (`lib.rs:806`). Observer placed by
  `nova_world_base/src/lib.rs:359-363`.
- Chart: `chart_visited_cell` `nova_world_base/src/chart.rs:80-99`,
  `Chart::has_visited/visit` `nova_gameplay/src/chart.rs:188-200`, scheduled
  `lib.rs:265-272`. Map view `crates/nova_interface/src/map/chart.rs:428`.
- Save: `request_world_save` `nova_world_base/src/save/session.rs:386-414`
  (desktop only). Tests `nova_world/src/tests.rs:1781`,
  `save/tests.rs:2239,2280`; example `examples/systems/system_world_chart.rs`.

## Station lights
- `station_layout.rs:148-155,203-218,405,465,984-998,1036-1058`; spawned via
  `nova_scenario/src/objects/station.rs:415-448` (`StationMarker` :448)
  through the ship pipeline. No light on any section except the railgun
  charge glow. Docking render `docking_section/render.rs:1-60`. Authored
  lights `nova_scenario/src/objects/light.rs:248-300`. Frozen pieces keep
  `PointLight` children (`frozen_piece.rs:434-441,733-746`). No light census
  in `nova_probe`; soft gate `fps_within_baseline`.

## Sounds
- Generated by `scripts/gen-world-sfx.py` (cue table :1032-1061) and
  `gen-ui-sfx.py` over `scripts/nova_sfx.py`; mono 44.1 kHz 16-bit, -3 dBFS.
  README tables `assets/base/sounds/README.md:66-148`. No unused file.
