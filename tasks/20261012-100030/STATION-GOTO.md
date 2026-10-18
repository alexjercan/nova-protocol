# Design: cross-sector station GOTO (owner review pending)

Status: design only, 2026-10-19. Nothing here is approved. Every name, type,
default and error rule below is a proposal until the owner answers the gates
at the end. No code, type, function or test is added before that.

Source: owner idea in `TASK.md` (round 3): "GOTO a station on the chart in
another sector even with no live target: a sector destination for the
autopilot, the streaming window follows, arrival when the station
materializes."

## End state (proposed)

- In the Map's chart view, select a found station (`STN-n`) in any cell and
  press GOTO (button or `map_goto`). The player ship flies toward that
  station's cell with no live target.
- The streaming window follows the ship. No change is needed for that: the
  window is centred on the `WorldObserver`, and the open world puts it on the
  player ship.
- When the station's entity materializes in the window, the far leg hands off
  to the existing entity GOTO on that station, and the travel lock moves to
  it. The ship comes to rest one resolved margin off the station, as a local
  GOTO does today.
- Any flight input cancels the whole order, as it cancels a GOTO today.
- A station that does not come back when its cell is live fails loudly: the
  order ends, the ship brakes, and the panel says why.

## What dies

- The chart view's GOTO disable rule `!runtime.chart && ...`
  (`crates/nova_interface/src/map/scene.rs:1106-1107`). It becomes: the local
  view keeps today's rule; the chart view enables GOTO for a selected
  `MapChartMarkType::Station` only (gate G8).
- The Map module doc that says GOTO acts on contacts only
  (`crates/nova_interface/src/map/mod.rs:13-18`) and the chart-view section
  (`:20-28`) are rewritten.
- `wiki/interface.md` Map GOTO text; one changelog entry under
  `### Interface & HUD` or `### Gameplay & Flight` (per AGENTS changelog
  policy).
- No adapter or alias: the local GOTO path at `scene.rs:405-433` stays as is.

## What may break

- Speed versus streaming. `GotoPos` and `Goto` fly flip-and-burn with no
  cruise cap (`crates/nova_ship/src/flight/guidance.rs:26-50`). A long leg
  reaches a high peak speed. The window keeps 2 cells (64 km) ahead
  (`crates/nova_world_base/src/lib.rs:187-193`), and jobs run at most one per
  pool thread (`crates/nova_world/src/streaming.rs:714`). At most one
  prepared sector materializes per frame, nearest first (`streaming.rs:844`).
  A fast crossing discards jobs for cells it left (`SectorJobStats`), and
  does not queue them. Unverified: whether preparation keeps ahead at peak
  speed on a long leg.
- The G key during the far leg. `on_autopilot_goto_input` toggles a trip off
  only when the action is `Goto` (`crates/nova_ship/src/input/player/flight_rig.rs:483-493`).
  During the far leg the action is `GotoPos`, so G engages a GOTO on the
  travel lock instead, and `drive_station_goto` reads that as a cancel
  (gate G10).
- Collisions. GOTO has no collision avoidance
  (`crates/nova_ship/src/flight/state.rs:219-222`). A straight leg at high
  speed through a streamed rock field can hit a rock that materializes ahead.
- The fleet. Escorts never freeze (`crates/nova_world/src/frozen.rs:739-742`),
  so a slow escort falls behind but is not lost. Unverified how far behind.
- f32 world coordinates. There is no floating origin; positions are absolute
  engine units. A leg of tens of cells is fine; very far travel makes the
  existing precision limit easier to reach. Not new, but easier to hit.
- Station spawn hold. A station is not spawned while the observer overlaps
  its clearance (`streaming.rs:323-334`, `:940-975`). A far leg that parks
  inside that clearance keeps the station from ever spawning. The design must
  never park there (see arrival criteria).

## What must fail loudly

- The station's cell is live, no entity wears the station's
  `GeneratedBodyId`, and no `PendingSectorStation` holds it: the station is
  gone. End the order, engage `Stop`, write a note (gate G5).
- The far leg completes (ship at rest at the aim point) before the station
  materializes: same check; if the cell is live and the station is absent,
  the same failure. Never report a scenario GOTO completion for the far leg.

## Feature slice (code facts)

| Concern | Owner today | Evidence |
| --- | --- | --- |
| Map GOTO entry | `map_input` in `MapPanePlugin` | `crates/nova_interface/src/map/scene.rs:405-433`, chain at `map/mod.rs:170-191` |
| GOTO button | `MapGotoButton`, `goto_requested` | `map/app.rs:122-127`, `:208-212`; disable rule `scene.rs:1106-1114` |
| Chart selection | `MapRuntime::chart_selected: Option<MapChartMarkType>` | `map/app.rs:213-223`, `map/chart.rs:83-92` |
| Station code | `chart_mark_code` (`STN-n` = found order) | `map/chart.rs:147-169` |
| Mark cell | `chart_mark_cell` (`id.origin`) | `map/chart.rs:174-180` |
| Route hint | `route_hint(from, to)` | `map/chart.rs:291`, shown at `:1087` |
| Chart data | `Chart`, `ChartStation { id: GeneratedBodyId, .. }` | `crates/nova_gameplay/src/chart.rs:90-103`, `:161-166` |
| Body id | `GeneratedBodyId { origin: SectorCoord, stem, index }` | `crates/nova_events/src/generated.rs:189-197` |
| Cell math | `SectorCoord::containing`, `centre` | `generated.rs:68`, `:78` |
| Autopilot state | `Autopilot { action, phase, .. }` (present = engaged) | `crates/nova_ship/src/flight/state.rs:177-200` |
| Actions | `AutopilotAction` is `Copy`; `Goto { target: Entity }`, `GotoPos { position: Vec3 }` | `state.rs:215-237` |
| Completion | `PlayerAutopilotCompleted { action }` | `state.rs:203-211`; events `crates/nova_scenario/src/loader/trackers.rs:281-303` (GotoPos reports nothing) |
| Target loss | `DisengageType::TargetGone` removes `Autopilot` | `crates/nova_ship/src/flight/autopilot.rs:408`, `:693-715` |
| Flight tick | `NovaFlightSystems` chain in `FixedUpdate` | `crates/nova_ship/src/flight/mod.rs:189-210` |
| Cancel | flight input removes `Autopilot` | `crates/nova_ship/src/input/player/flight_rig.rs:395-399` (and `:433`, `:492`, `:562`, `:608`, `:706`) |
| Travel lock | `TravelLock(Option<Entity>)` | `crates/nova_ship/src/input/targeting/state.rs:97` |
| Streaming centre | `WorldObserver` on the player ship | `crates/nova_world/src/streaming.rs:46-55`; inserted `crates/nova_world_base/src/lib.rs:415` |
| Current cell | `track_current_sector` writes `CurrentSector` | `streaming.rs:137`, `:552-584` |
| Stage order | `Cleanup -> Observe -> Request -> Collect -> Materialize -> Adopt -> Retire`, `Update` | `crates/nova_world/src/lib.rs:758-829` |
| Thaw id | a thawed body gets its `GeneratedBodyId` back | `crates/nova_world/src/frozen.rs:673` |
| Appear precedent | `Added<GeneratedBodyId>` re-applies marks | `crates/nova_scenario/src/marks.rs:44` |
| Open-world chain | chart, goal, boards after `Retire` | `crates/nova_world_base/src/lib.rs:257-283` |
| HUD | GOTO chip and anchor read `GotoPos` already | `crates/nova_hud/src/flight_status.rs:490`, `:644`; `crates/nova_hud/src/situation.rs:152` |
| Saves | `Autopilot` is not saved | no match in `crates/nova_world_base/src/save/` |
| Bounds | the open world authors no bounds | `crates/nova_authoring/src/base_content/scenarios/open_world.rs:81` |

Crate graph (from `Cargo.toml`): `nova_ship` and `nova_world` do not know the
open-world game; `nova_world_base` depends on both; `nova_interface` depends
on `nova_world_base` and `nova_ship`. The sector edge lives in
`WorldConfig<NovaLayeredWorld>` (`crates/nova_world/src/lib.rs:425-437`).

## Proposed design (recommendation A, every name gated)

Owner: `nova_world_base`, next to `Chart` and the job boards. It has the
config, the station query, and is already the layer that joins streaming to
gameplay. `nova_ship` and `nova_world` do not change.

Proposed component, on the player ship root:

```rust
/// A GOTO to a found station that may stand outside the streamed window.
/// Present = the far leg is live. Removed on handoff, cancel or failure.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct StationGoto {
    /// The charted station.
    pub station: GeneratedBodyId,
}
```

Proposed system, in the `nova_world_base` chain after
`NovaWorldSystems::Retire` (`lib.rs:257-283`), so a station spawned this frame
is on hand:

```rust
pub(crate) fn drive_station_goto(
    mut commands: Commands,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    ships: Query<(Entity, Ref<StationGoto>, Option<&Autopilot>), With<PlayerSpaceshipMarker>>,
    stations: Query<(Entity, &GeneratedBodyId), With<StationMarker>>,
    pending: Query<&PendingSectorStation>,
    roots: Query<&SectorRoot>,
)
```

Proposed pure step, for the unit proof:

```rust
pub(crate) enum StationGotoStepType {
    /// The station stands: engage `Goto { target }` and `TravelLock(Some(target))`, drop `StationGoto`.
    Handoff(Entity),
    /// Not live yet: hold or engage `GotoPos` at the aim point.
    FarLeg(Vec3),
    /// The cell is live and the station is neither spawned nor held: engage `Stop`, drop `StationGoto`, note.
    Lost,
    /// The helm was taken (`Autopilot` absent or another action, not this frame's insert): drop `StationGoto`.
    Cancelled,
}
```

Map change (`scene.rs:405-433`): in the chart view, with a
`MapChartMarkType::Station(id)` selected, run the same helm refusals, then
insert `StationGoto { station: id }` only. One path for a live and a far
station: the system hands off on the same frame when the station stands.
The note reads `GOTO SET: STN-n`; the Destination field reads
`GOTO STN-n  <route_hint>` while the far leg is live (`scene.rs:1046-1062`).

### Before

```text
map_input (Update, MapPaneSystems)
  -> selected live contact (Entity)
  -> insert Autopilot::engage(Goto{target}) + TravelLock(Some(target))
autopilot_system (FixedUpdate)
  -> arrival_target(Goto) -> fly -> Completed -> PlayerAutopilotCompleted
  -> target gone -> TargetGone -> remove Autopilot
track_current_sector (Observe) -> CurrentSector follows the ship
chart view: GOTO disabled
```

### After (proposal)

```text
map_input (chart view, Station(id) selected)
  -> insert StationGoto{station: id}
drive_station_goto (Update, after Retire)
  station entity with id stands  -> Autopilot Goto{entity} + TravelLock, remove StationGoto
  else cell not live             -> Autopilot GotoPos{aim} (engage once, keep)
  else held in PendingSectorStation -> keep far leg (wait)
  else                           -> Autopilot Stop, remove StationGoto, note "GOTO LOST: STN-n"
  Autopilot gone or replaced     -> remove StationGoto (cancel)
autopilot_system (unchanged) flies GotoPos, then Goto
track_current_sector (unchanged) -> window follows, cells materialize
existing Goto completion -> PlayerAutopilotCompleted{Goto{station}}
```

## Travel handoff and thaw guards

1. Aim point (gate G3). Proposal: `id.origin.centre(edge)` in engine units.
   The station's cell enters the window when the ship is 2 cells out, so the
   handoff normally happens about 48-80 km before the aim point and the far
   leg never parks. No ledger read is needed.
2. Handoff frame. The system runs after `Retire`, so a station spawned by
   `materialize_ready_sector` or `materialize_pending_bodies` this frame is
   queried whole. The `FixedUpdate` autopilot reads the new action on its
   next tick. The `Goto` leg replans from the current velocity; no stop
   between legs.
3. Held station. While `PendingSectorStation` holds the id, the observer is
   inside its clearance; the order waits on the far leg. This only happens if
   the ship already stands near the station.
4. Moved station. Stations are dynamic thrusterless hulls
   (`crates/nova_scenario/src/objects/station.rs:15`) and can be adopted into
   another cell. The lookup is by `GeneratedBodyId` over all stations, not by
   cell, so a moved station that is live is found. A station moved into a
   cell that is not live reads as "not live yet" until the far leg reaches
   the origin cell, then as lost. Unverified how often a station leaves its
   origin cell; proposal: accept the loud failure.
5. Cleanup, Retry, Load, scenario change. The player ship despawns with the
   world, and the component with it. Proposal: `StationGoto` is not saved,
   as `Autopilot` is not saved (gate G6).
6. Docked start. The existing refusals apply (`scene.rs:411-423`): no order
   is set without the helm.
7. Ship swap. The component is on the ship root; a new player ship starts
   with no order.

## Arrival criteria (proposal, gate G4)

- Handoff: an entity with `StationMarker` and the target `GeneratedBodyId`
  exists. Then the far leg ends without a completion event.
- Arrival: the existing `Goto` completion. The ship rests one resolved margin
  (`arrival_standoff`, 500 m, `crates/nova_ship/src/flight/state.rs:519`)
  off the station's face (`state.rs:219-222`, `autopilot.rs:408-482`), and
  `PlayerAutopilotCompleted { action: Goto { target } }` fires, so
  `OnGotoCompleteEvent` fires for the station's id (`trackers.rs:288-295`).
- Not arrival: the far leg's `GotoPos` completion. It reports no scenario
  event today (`trackers.rs:299-302`).

## Proof plan (each item gated)

- Unit (pure, `nova_world_base`): `station_goto_step` returns `Handoff` when
  the station stands, `FarLeg` when its cell is not live, waits when it is
  held, `Lost` when its cell is live and it is neither spawned nor held, and
  `Cancelled` when the helm is taken. Proposed name:
  `a_station_goto_hands_off_when_the_station_stands_and_fails_when_its_live_cell_lacks_it`.
- Map unit (`crates/nova_interface/src/map/tests.rs` near the GOTO button test at `:655`): in
  the chart view, GOTO is enabled for a selected station mark and disabled
  for a cell, wreck or job mark; pressing it inserts `StationGoto` and no
  `Autopilot`.
- Full-app range: a new `examples/systems/system_world_station_goto.rs`
  (shape of `system_world_chart`, `editor_app` + New Game on a pinned seed).
  Find a station in another cell, return out of its window, GOTO it from the
  chart. Assert state, not timing:
  1. the far leg is `GotoPos` at the origin cell centre and `StationGoto`
     stands;
  2. `CurrentSector` changes along the leg;
  3. on the handoff frame the ship's distance to the aim point is above one
     cell edge (the far leg never parked), `Autopilot` is `Goto { station }`
     and `TravelLock` names the station;
  4. `PlayerAutopilotCompleted` fires with that target and the ship is at
     rest within the standoff band;
  5. a burn input mid-leg removes both `Autopilot` and `StationGoto`.
  Log `SectorJobStats` and the peak speed for the speed-versus-streaming risk.
  Lavapipe cost: the leg is real flight time; set
  `NOVA_AUTOPILOT_DEADLINE`.
- Playable example (`examples/playable/`, README row): per the owner's
  playable-example rule; gated as a new example.

## Owner gates

- G1 Owner. (A) `nova_world_base` component and system: the game layer that
  already joins streaming and gameplay; no change in `nova_ship`. (B) a new
  `AutopilotAction` variant in `nova_ship`: one engaged state, but the action
  is `Copy` and a `GeneratedBodyId` is not, and `nova_ship` learns sectors.
  (C) generic in `nova_world`: reusable, but station semantics enter the
  engine crate. Recommend A.
- G2 Names. `StationGoto`, `drive_station_goto`, `StationGotoStepType`.
  Alternative `BodyGoto` if G8 widens to wrecks and jobs. Recommend
  `StationGoto`.
- G3 Aim point. (a) origin cell centre: no lookup, coarse route line. (b) the
  frozen record's pose: exact, needs a ledger read and a live-cell path.
  (c) regenerate the cell: exact for unchanged cells, costs a job. Recommend
  (a).
- G4 Arrival. (a) handoff at materialize, arrival at the existing `Goto`
  rest: the ship ends at the station. (b) end at materialize: the ship brakes
  48-80 km out and the player flies the rest. Recommend (a).
- G5 Failure. (a) `Stop` and a `GOTO LOST: STN-n` note. (b) remove
  `Autopilot`: the ship coasts at speed. (c) hold the far leg: the ship parks
  at the cell centre with no reason given. Recommend (a).
- G6 Save. (a) not saved, as `Autopilot`. (b) saved: a Load resumes travel,
  and a save format bump. Recommend (a).
- G7 Speed and collision. (a) no cruise cap, straight leg, as local GOTO:
  the risk is a high-speed hit. (b) an authored cruise cap on the far leg: a
  new explicit setting whose value the owner sets. No number is proposed.
  Recommend (a) for the first cut, with the range logging peak speed.
- G8 Targets. (a) station marks only. (b) stations, wrecks and held jobs'
  bodies: wider, more failure cases (wrecks are taken by jobs, job bodies
  spawn late). Recommend (a).
- G10 G key during the far leg. (a) leave it: G retargets to the travel
  lock and cancels the station GOTO. (b) the toggle also stops a player
  `GotoPos` (`flight_rig.rs:483-493`): G stops the trip as it does today,
  but `nova_ship` changes. Recommend (b).
- G9 Proof. Approve the unit test, the map test, the systems range and the
  playable example as listed, or name the subset.

Recommendation: A, `StationGoto`, cell-centre aim, handoff then existing
GOTO arrival, `Stop` on loss, not saved, no cruise cap, stations only, G stops the far leg.
