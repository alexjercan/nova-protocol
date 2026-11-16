# Map: timed repair and field rebuild

## Owners

| Concern | Owner | File |
| --- | --- | --- |
| Pure rules: plate start, plate step, rebuild site, durations | `nova_gameplay` | `src/inventory.rs` (beside `HULL_PLATE_HEALTH`) |
| Job state on the ship root, the tick, cancel, end message | `nova_ship` | `src/sections/repair_jobs.rs` (new) |
| Respawn of a lost section | `nova_scenario` | `src/objects/spaceship.rs` `rebuild_design_section` (unchanged) |
| Ship pane: start, cancel, job rows, lost list | `nova_interface` | `src/ship/{app,sections,scene,rebuild}.rs`, `src/ship/repair_jobs.rs` (new) |
| Flight chip and end notice | `nova_hud` | `src/flight_status.rs`, `src/hud_notice.rs` |
| Hull alarm comment | `nova_ship` | `src/ship_audio/cues.rs` |

Why `nova_ship` owns the jobs: it is the lowest crate that has every input
both jobs read (`Health`, `ShipInventory`, `ShipDesignSections`,
`rebuild_anchored`) and the `SectionRebuildRequest` the existing respawn
observes. `HullStripJob` sets the precedent (component in `nova_ship`). Unlike
the strip, the tick also lives in `nova_ship`, so a job runs in flight with no
pane plugin.

## Clock

`PauseStates` frozen variants pause `Time<Virtual>` (`nova_gameplay/src/lib.rs`
`PauseStates` doc). The tick runs in `Update` and reads `Res<Time>` (virtual),
so TAB and pause hold every job, as `advance_hull_strip` does.

## Before

```
P key / Repair button -> SectionRepairCommand
  -> apply_ship_section_commands (app.rs) -> repair_section (sections.rs)
     -> plan_plate_repair -> inventory.remove(N plates); health.current = ...  (instant)
Rebuild button (only when docked + Rebuild service) -> SectionRebuildCommand
  -> apply_ship_rebuild_commands (rebuild.rs) -> plan_section_rebuild(partner: Option)
     -> take parts, pay credits -> trigger SectionRebuildRequest
        -> rebuild_design_section (nova_scenario) -> spawn_design_section
```

## After

```
P key / Repair button -> SectionRepairCommand
  -> apply_ship_section_commands -> plan_plate_repair (free plates = hold - queued)
     -> ShipRepairJobs.push(Plates)                          (nothing spent)
Rebuild button
  docked  -> plan_section_rebuild(RebuildSiteType::Station) -> instant, as before
  undocked-> plan_section_rebuild(RebuildSiteType::Field)   -> ShipRepairJobs.push(FieldRebuild)
Cancel button -> RepairJobCancelRequest (nova_ship observer) -> RepairJobEnded(Cancelled)

advance_ship_repair_jobs (nova_ship, Update, Time<Virtual>)
  Plates: each PLATE_REPAIR_SECS -> plan_plate_step -> remove 1 plate, +HULL_PLATE_HEALTH
  FieldRebuild: after field_rebuild_duration_secs -> plan_section_rebuild(Field)
     -> remove parts -> trigger SectionRebuildRequest -> rebuild_design_section
  end -> Repaired / Rebuilt report, RepairJobEnded message
RepairJobEnded -> nova_hud HudNotice; Ship pane note
ShipRepairJobs -> flight chip "REPAIR HULL-3 40%"; pane job rows with bars
```

## Proposed types

- `nova_gameplay`: `PLATE_REPAIR_SECS`, `FIELD_REBUILD_MIN_SECS`,
  `FIELD_REBUILD_SECS_PER_PART`, `field_rebuild_duration_secs(&BuildRecipe)`,
  `plan_plate_step(Option<&Health>, bool, u32) -> Result<f32, PlateRepairRefusalType>`,
  `enum RebuildSiteType<'a> { Field, Station { partner: StationPartner<'a>, own_cr: u32 } }`,
  `plan_section_rebuild(items, prototype, recipe, site, lost, anchored, own)`.
  `SectionRebuildRefusalType::NotDocked` dies: undocked is the field site.
- `nova_ship`: `ShipRepairJobs(Vec<RepairJob>)`, `RepairJob { code, work, elapsed_secs }`,
  `RepairWorkType::{Plates { section, queued, spent }, FieldRebuild { section, parts, duration_secs }}`,
  `RepairJobKey`, `RepairJobCancelRequest` (EntityEvent), `RepairJobEnded` (Message),
  `RepairJobEndType`, `advance_ship_repair_jobs`, `RepairJobSystems`.
- `nova_interface`: `RepairJobCancelCommand`, job rows block in the panel.

## What dies

- Instant plate repair and `repair_section`.
- `SectionRebuildRefusalType::NotDocked`; the lost list no longer hides undocked.
- Stale "Nothing repairs a hull today" comments.

## Must fail loudly

- A field rebuild of a section the design lacks: the existing
  `rebuild_design_section` panic.
- A cancel for a ship root with no `ShipRepairJobs`: no-op, the pane only
  sends keys it read.
