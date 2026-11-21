# Waypoints: point the way to a GOTO target that is not loaded

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.19.0, navigation, hud

## User facts
- Owner 2026-11-21: "some kind of waypointing system, so if I want to
  'GOTO' a station or sector that is not loaded we should get some kind of
  'directional' hint which is the right way".
- Release: v0.19.0, after the background music task.

## Agent findings (starting points, not verified)
- Map pane GOTO: `MapGotoButton`, `MapGotoMarker`
  (`crates/nova_interface/src/map/app.rs:85,101`).
- Autopilot GOTO input: `AutopilotGotoInput`
  (`crates/nova_ship/src/input/player/flight_rig.rs:56`);
  `OnGotoCompleteEvent` (`crates/nova_events/src/lib.rs:362`).
- HUD markers that may carry the hint: `crates/nova_hud/src/edge_indicators.rs`,
  `screen_indicator.rs`, `keybind_dock.rs` (`GotoCueUIMarker`).
- Open: how a target in an unloaded sector is addressed (sector coordinates,
  station ids), and whether GOTO across sectors is flown or only pointed.

## Map (2026-11-21)
Paths and lines are at HEAD 06813de8d. The design page is
`DESIGN-waypoints.html` in this folder: mockups, states, the full slice
table and the proof plan.
- Today no GOTO can name an unloaded target. The chart view hides GOTO
  and Cancel (`nova_interface/src/map/scene.rs:1233`, `:1240`), and
  `map_input` returns before GOTO in the chart (`scene.rs:422-425`). The
  local Map lists live entities only (`map/contacts.rs:352`). G needs a
  travel lock on a live entity (`nova_ship/src/input/player/flight_rig.rs:632`).
  Read from code, not run.
- `AutopilotAction::Goto` holds an `Entity`. `GotoPos` holds a `Vec3`,
  and the player never engages it (`nova_ship/src/flight/state.rs:412-424`).
  The prediction, the HUD ribbon, the destination pip and the save already
  handle `GotoPos` (`flight/prediction.rs:600-626`,
  `nova_hud/src/flight_status.rs:868`, `nova_world_base/src/save/controls.rs:108`).
  The Map route matches `Goto` only (`scene.rs:672-686`).
- An unloaded station is known by `GeneratedBodyId` in `Chart`
  (`nova_gameplay/src/chart.rs:93`), with no position. Its pose is only in
  the frozen ledger it will thaw from (`nova_world/src/frozen.rs:67`,
  `:485`). A cell is a `SectorCoord`, and its centre comes from
  `ChartSectorEdge` (`nova_events/src/generated.rs:78`,
  `nova_gameplay/src/chart.rs:401`).
- HUD carrier: the anchored nav chip (`nova_hud/src/anchored_chip.rs:109`,
  beacon shape `beacon_chips.rs:40-74`) already edge-clamps with a chevron.
  The screen indicator takes a `Point` anchor (`screen_indicator.rs:51`).
  The edge arrows are combat-only, the objective chip is gold "do this now",
  and `GotoCueUIMarker` is the keycap on the lock. None of these fit.

## Proposed decisions (awaiting owner)
1. Address: `WaypointTargetType { Station(GeneratedBodyId), Sector(SectorCoord) }`
   and a `Waypoint` component on the player ship, in `nova_gameplay`. The
   mark comes from `FrozenSectors` (station) or the cell centre (sector).
2. Flown: the chart GOTO flies `GotoPos` at the mark. When the station
   streams in, the hand-off swaps to `Goto{station}` and the travel lock.
   Only `FlightPrediction` draws a line. The hint draws no stroke.
3. Hint: a nav chip `STN-4  128.40 km`, edge-clamped with a chevron off
   screen. It moves onto the live station at hand-off and clears on
   arrival, Cancel, G, STOP, a new order or ship loss. Every chart GOTO is
   a waypoint. The radar and local GOTO do not change. A sector waypoint
   rests at the cell centre.
4. Fail loudly: unknown station, no frozen record, station missing after
   its cell is placed, mark reached without a hand-off, non-finite mark.

## Delivery
1. Map the GOTO slice (map pane, autopilot, HUD indicators, sector loading)
   and propose the hint for an unloaded target: what it points at, how it
   shows on and off screen, when it clears.
2. Owner approves the design, then build it with a playable example.

## Verification
- An asserted example: GOTO a station in an unloaded sector; the hint
  points toward it and clears on arrival.

## Done when
- A GOTO to a station or sector that is not loaded shows which way to fly,
  and the owner has played it.
