# Survive section loss: timed repair and field rebuild of lost sections

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.19.0, survival, repair

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45). This is the owner's top worry
for v0.19.0.

## User facts
- Damage is too punishing. When the ship starts to lose sections, the run is
  over in practice. "if you lose the docking section you are basically done,
  you cannot do anything after that".
- Wants on-ship repair that rebuilds lost sections while adrift.
- TAB pause plus plate repair is fine. Repair must take time, shown as a
  progress bar.
- Owner call 2026-11-16: rebuilding in the field is now wanted. This reverses
  decision 4 in `tasks/20261012-095526/DESIGN.html:887` ("Rejected: rebuild
  anywhere with parts") for lost sections away from a station.

## Agent findings
- A section at 0 HP is destroyed at once, leaf or not
  (`crates/nova_gameplay/src/integrity/core.rs:371-381`,
  `crates/nova_ship/src/sections/integrity.rs:407-427`). A cut graph leaves an
  inert wreck fragment (`integrity.rs:520-650`). Collapse at 0.05 of pinned
  max peels every section (`integrity.rs:33,1080-1201`).
- Lost docking port: the connection releases
  (`docking_section/connection.rs:700-718`). Market, refine, storage, job
  accept and hand-in, Station pane, Rebuild, Swap and Add all need a dock
  (`crates/nova_interface/src/inventory/mod.rs:1-27`,
  `crates/nova_interface/src/jobs.rs:379,398`,
  `crates/nova_gameplay/src/inventory.rs:2189,2227`). Rebuild needs a dock,
  so a lost dock is a permanent soft-lock. Five of six start hulls have one
  port; patrol_picket has two.
- Plate repair (`inventory.rs:1784-1816`): 20 HP per plate
  (`inventory.rs:1752`), instant (`crates/nova_interface/src/ship/sections.rs:550-560`),
  no dock check (`crates/nova_interface/src/ship/app.rs:428-466`), refuses a
  destroyed section (`inventory.rs:1791-1793`). Plates: 40 cr, market only,
  no refine recipe. Transport starts with 0 plates.
- Station Rebuild: `plan_section_rebuild` checks docked, Rebuild service,
  prototype, lost, anchored, payment (`inventory.rs:2187-2253`). A lost
  section is a design section with no live child
  (`crates/nova_ship/src/sections/design_sections.rs:1-114`). Saves keep the
  design plus surviving sections; a missing one "is not spawned again"
  (`crates/nova_scenario/src/objects/spaceship.rs:591-653`). Respawn site:
  `spaceship.rs:1338`.
- TAB freezes the game clocks (`crates/nova_gameplay/src/lib.rs:182-184`).
  A timed repair on the sim clock does not advance while the pane is open.
- Stale comments: "Nothing repairs a hull today"
  (`crates/nova_ship/src/ship_audio/cues.rs:333,859`).

## Open decisions (owner)
- Field rebuild cost: plates, parts, credits, time, or a mix.
- Which sections a field rebuild can restore: all, or a safe set (dock,
  controller, thruster).
- Whether field rebuild needs a dedicated section (repair bay or fabricator)
  or works from any hull. Links to task 20261116-132140 (crafting).
- Whether the repair timer runs while TAB is open, or the pane must close.

## Decisions (owner said implement 2026-11-16; agent calls, stated)
- Clock: repair and field rebuild progress on the gameplay clock only. TAB
  and pause freeze them, so healing takes real fight time. The Ship pane says
  so, and a compact HUD line shows progress in flight. Owner may flip this to
  run under the pane.
- Plate repair is a per-section job: N plates queued, one plate spent and
  20 HP added at the end of each plate interval. Nothing is spent up front,
  so a cancel, a save or a lost section never loses an unspent plate. One
  job per section; jobs on different sections run side by side. A job stops
  with a reason when the hold has no plate or the section is destroyed.
- Field rebuild: any lost, anchored design section, undocked, no Rebuild
  service needed. Cost = the section's build recipe parts from the ship hold
  only, no credits (no seller). Parts are checked at start and taken at
  completion. Takes longer than plate repair. Docked station Rebuild stays
  instant and unchanged.
- No new section is required for field work; any hull can do it.
- Jobs are saved (owner correction, 2026-11-16, superseding "not saved"). A
  world save keeps each job by section design id; a Load resumes it. A save
  whose job names a section the design lacks is refused. A v0.18.0 save
  loads with no jobs.
- Durations are named constants next to `HULL_PLATE_HEALTH`, not content
  fields, in this pass.

## Delivery
1. Timed plate repair: the Ship pane queues a repair, a progress bar fills on
   the sim clock, health rises when it completes. Cancel refunds unspent
   plates.
2. Field rebuild of a lost, anchored section without a dock, using the
   existing `plan_section_rebuild` checks minus the dock and service gates.
3. A lost docking port is never a permanent soft-lock.
4. Delete the stale comments in `cues.rs`. Update wiki and changelog.

## Verification
- Unit tests for the repair and field-rebuild plan functions (refusals and
  costs).
- ECS asserted example: a timed repair completes after its duration; a field
  rebuild restores a lost docking port and docking succeeds after it.
- A playable example under `examples/playable/` with a damaged start hull.

## Done when
- A player who loses the docking port restores it in the field and docks
  again.
- Plate repair takes visible time with a progress bar.
- Wiki and changelog describe the new repair rules.

## Delivery record (2026-11-16)
- Landed: timed plate jobs and field rebuild (`crates/nova_ship/src/sections/repair_jobs.rs`),
  `RebuildSiteType::{Field, Station}` in `crates/nova_gameplay/src/inventory.rs`,
  Ship pane Repairs block, HUD repair chip and end notices, docs, lesson text
  and media, `examples/playable/field_repair_sandbox.rs`. Map: `MAP.md`.
- Found and fixed on the way: the ship graph builder inserted `ConnectedTo`
  on a section despawned the same frame, which ended the app
  (`crates/nova_ship/src/sections/integrity.rs`, now `try_insert`). Proof:
  `a_section_despawned_in_the_graph_build_frame_leaves_the_rest_connected`
  failed with the plain insert and passes with the fix.
- Verified: targeted unit and ECS tests (nova_ship repair_jobs and integrity,
  nova_gameplay field_rebuild and plate_step, nova_interface ship::, nova_hud
  flight_status, nova_scenario field rebuild, nova_authoring lesson_media);
  content gen and lint clean; fmt clean. Live under Xvfb and lavapipe:
  field_repair_sandbox smoke walk PASS (DOCK-1 rebuilt in 20 s after the pane
  closed); a plate job held 20 s with TAB open, then finished 8 s after close;
  three manual launches without a crash; system_hold_events and
  lesson_interface_ship walks PASS; frames inspected for the Repairs block,
  the one-line hint and the HUD chip with the RCS row hidden.
- Dock on a field-rebuilt port, live: the field_repair_sandbox walk now
  berths at the waystation after the rebuild and asserts the dock holds on
  the rebuilt DOCK-1 (`DockingConnection` names that section) with the pair
  at rest. PASS under Xvfb and lavapipe; rebuild 22 s, dock 2.5 s later.
- Not run: workspace tests and clippy (CI runs them).
