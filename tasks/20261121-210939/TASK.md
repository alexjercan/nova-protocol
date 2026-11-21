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
