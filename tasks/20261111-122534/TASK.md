# Feedback polish for docking, pickup, railgun hits, kills and sector crossing

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.18.0, polish, audio

Planned 2026-11-11 from the v0.17.0 post-release review. Owner direction:
apply the torpedo feedback treatment (light, trail, sound, notice, stinger)
to the other bare moments.

## User facts
- v0.17.0 torpedo chain is the reference quality bar.

## Agent findings (polish inventory, bare spots)
- Docking and undock: port sleeve animation only
  (`crates/nova_ship/src/sections/docking_section/render.rs:1-8`); no sound
  anywhere, `ship_audio/machinery.rs:3-5` does not cover ports; no DOCKED
  notice or flash.
- Canister pickup / cargo intake: sounds exist (`salvage_pickup`,
  `cargo_eject`, `bay_door`) but no visual and no HUD flash.
- Sector crossing: no visual, no sound, no notice.
- Railgun: slug, tracer, wake and muzzle light exist
  (`railgun_section/wake.rs`, `render.rs:296`) but impact is the shared PDC
  spark (`crates/nova_gameplay/src/impact_spark.rs`). No ricochet visual or
  sound for PDC rounds.
- Kill stinger plays only for torpedo kills (`assets/base/sounds/README.md:136`).
- Station lights: none; lights exist only as authored scenario objects
  (`crates/nova_scenario/src/objects/light.rs`).
- Fine already: section destruction, asteroid carve, mining beam, thruster
  burn, RCS puffs.

## Decisions (owner said start 2026-11-11; the calls below are the agent's, stated)
Mapping evidence (read-only, 2026-11-11): the two map reports are summarised
in `MAP.md` beside this file.
- Docking: `DockingSectionConfig` gains optional `clamp_sound` and
  `release_sound` (torpedo-bay style, lint refuses an empty path). New
  `DockClamped` / `DockReleased` entity events fire from the request paths in
  `docking_section/connection.rs` (the joint holds at :407, the player or AI
  release at `on_docking_release_request`), never from a load or a thaw
  (`restore_docking_connection` fires nothing) and never from the damage
  release paths. Audio observers in `ship_audio/machinery.rs`. A `LightFlash`
  (cold, short) at the port on the clamp. HUD notice `DOCKED <NAME>` and
  `UNDOCKED` for the player's hull only. New generated wavs `dock_clamp.wav`
  and `dock_release.wav` from `scripts/gen-world-sfx.py`.
- Pickup: `CargoCanisterTaken` carries the stacks it took (item id + count)
  so a HUD line can name them; the inventory report queue stays the scenario
  path. New `nova_hud` pickup tick: `+N <ITEM>` lines above the keybind
  dock, up to 3, 1.5 s each, no sound (the take sound is the cue), on every
  tier. A warm `LightFlash` at the intake on the take.
- Railgun: `SurfaceImpact` gains `source: Option<Entity>` like
  `HealthApplyDamage`. A slug hit (source carries
  `RailgunSlugProjectileMarker`) throws a steel-blue penetration `LightFlash`
  and a dense `ImpactSparks` burst at the contact point. `RailgunSectionConfig`
  gains optional `impact_sound`, carried on the slug like `TorpedoLaunchConfig`
  and played instead of the fixed pierce cue. New `railgun_impact.wav`.
- PDC ricochet: DROPPED. No miss, glancing or deflect event exists and the
  rounds carry no incidence angle; inventing one is a physics change, not
  polish. Hull scorch decal: DROPPED, no decal system exists and the carve
  marks already record hits.
- Any-kill stinger: the torpedo-only `TorpedoBlastCredit` becomes a
  `PlayerKillCredit` stamped on any attributed hit whose attacker resolves
  to the player (`resolve_damage_attacker` in `input/ai/threat.rs`), same
  0.5 s window, same `PlayerKillConfirmed` event. The stinger and the camera
  jolt follow unchanged. The module moves out of `torpedo_section`.
- Sector crossing: `track_current_sector` writes a `SectorCrossed { from, to }`
  message only when a previous cell existed (arm and load are not crossings).
  The HUD posts `SECTOR <code>` or `SECTOR <code> - FIRST VISIT` through
  `HudNotice`, whose own sound is the ping. No chart animation.
- Station lights: a `StationBeacon` point light on each docking port of a
  `StationMarker` hull, spawned by `nova_scenario/objects/station.rs`, no
  shadows, short range, slow pulse, amber while that port holds a
  connection. Gate: none on Low (`GraphicsBudget::transient_lights == 0`),
  on otherwise. Frozen pieces already keep light children across a thaw.
- Every new sound is generated, in the base bundle resource list and the
  sounds README; Low keeps every sound and notice.

## Agent findings (during delivery)
- `examples/systems/system_civilization_stance` panics at its last beat
  (`berth and dock once the Enemy decayed`, `system_civilization_stance.rs:511`,
  "has no free docking port") on master BEFORE this task (reproduced at
  652ae0003 on 2026-11-11 under Xvfb + lavapipe) and after it. The player's
  hull destroys a station docking port right after the refused berth (log:
  `integrity: destroyed 1 node (docking_port_section x1)`), and the parked
  industrial holds the other. The walk is not in CI. Not fixed here.
- The reworded `start_verbs` lesson note from the key row task did not fit
  the lesson lint (2 lines of 64); fixed in 95e207556.

## Delivery
- Owners: `nova_ship` docking_section and railgun_section render,
  `nova_gameplay` impact_spark and juice, `nova_hud` notices, `nova_world_base`
  station layout for lights, `nova_authoring` sound fields. Dies: nothing.
  May break: sound README inventory, lint for presentation sounds, low-end
  probe frame budgets if lights land on stations.
- Sequence: docking -> pickup -> railgun and ricochet -> any-kill stinger ->
  sector crossing -> station lights. One commit per moment.

## Verification
- Asserted examples per moment: event fires once per occurrence (one dock =
  one clamp, one pickup = one tick), notice text and timing, Low keeps sound
  and drops the light.
- Capture sheet for docking and railgun impact under NOVA_CAPTURE for the
  visual claims; compare against the v0.17.0 torpedo sheet.
- No timing assertions.

## Delivery record (2026-11-11, master, nothing pushed)
| Moment | Commit | Proof |
| --- | --- | --- |
| Docking clamp sound, cold collar flash, `DOCKED <NAME>` / `UNDOCKED` | afe3ac0cf | nova_ship docking_section + machinery 59 pass; nova_hud hud_notice 4; lint 26; `system_station_docking` smoke posts both notices twice |
| Pickup tick `+N ITEM` above the key row, warm intake flash | 0830f330f | nova_ship cargo_intake 26; nova_hud pickup_tick 2; `system_hold_events` smoke prints `+3 STONE ORE` |
| Railgun hit: steel-blue entry flash, dense spall, `impact_sound` | a17620eae, fixture a6b47f24c | railgun_section 28, ship_audio::combat 15, damage 39, rounds 39; `screenshot_railgun` frame `railgun-hit.png` |
| Any player kill confirms the stinger | 5e86729fe | kill_credit 1 (PDC, slug, blast, ram once; hostile, self, derelict, late none); stance walk logs 3 confirms |
| Sector crossing notice with FIRST VISIT | c12589655 | nova_world observer 7, nova_interface sector_notice 4; `system_world_chart` smoke posts two crossings, none on New Game or Load |
| Station port beacons | dbffbf156, standoff 0f3956e1e | beacon 8; dark-berth pair `beacon-dark-high.png` / `beacon-dark-low.png` (rig off, temporary uncommitted patch) |

- Dropped: PDC ricochet (no glancing event, no incidence angle) and hull
  scorch decals (no decal system). Both stay out until a system exists.
- Lesson lint fix 95e207556 (key row note) rides along.
- The first beacon sat at the port origin and lit nothing: the High and Low
  dark frames were identical. Moved 10 m past the face; the plating beside
  the port now reads in `beacon-dark-high.png`.
- Not verified in a frame: the clamp flash and the intake blink (0.2 s
  under lavapipe's clamped clock); both are the torpedo launch light's
  shape and are pinned by observer tests. Sounds are proved by playback
  tests only; nothing was auditioned.
- Skipped: workspace tests and clippy (house rule). `cargo check --examples
  --keep-going` clean after a6b47f24c.

## Done when
- [x] Docking, pickup, a railgun hit, any player kill and a sector crossing each
  have a sound and a visual or notice at High, and a sound at Low.
- [x] Sound README and wiki list the new optional fields.
