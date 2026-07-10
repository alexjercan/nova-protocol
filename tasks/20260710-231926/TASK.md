# Diegetic flight status v1: rehome the bottom-left status text and delete it

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0,hud,ux


## Goal

Replace the bottom-left flight status line (hud/flight_status.rs,
flight::flight_status_line) with diegetic presentation and delete it in
the same change. Spiked 2026-07-10; the user's questionnaire answers
fixed the direction:

- **Speed**: a numeric chip anchored to the player ship with an offset
  parking it just outside the velocity sphere (screen_indicator
  substrate). Always visible.
- **Mode + phase**: a ship-anchored chip showing verb and phase
  (`AP GOTO - BURN`) only while the autopilot is engaged; manual mode
  shows no chip (quiet HUD = manual). A family-wide shader tint
  reinforces it - that part is split out as task 20260710-234115.
- **Orbit radius**: a radius spoke holo while ORBIT is engaged - a thin
  world-space line (ribbon/ring visual language, unlit NAV_CYAN) from the
  well center to the ship, with the current radius as a chip riding it.
  The planned ring and its `r | v_circ` chip stay as-is.
- **Dropped without replacement**: the `GRAV <name>` coasting cue (the
  yellow gravity sphere carries it) and the standalone GOTO distance (the
  destination chip already shows distance, ETA, closing speed).

Deletion criterion: flight_status_line, its tests, and the bottom-left
text node go away here - the goal is REPLACEMENT, not addition.

## Steps

- [ ] Rework `flight_status_hud` in crates/nova_gameplay/src/hud/flight_status.rs:
      replace the absolute-positioned Text node with a screen_indicator
      layer holding two ship-anchored chips (pattern: the chip children
      in maneuver_instruments.rs): a speed chip and a mode chip, both
      font 12, TextColor(NAV_CYAN), NoWrap, Fixed size, Offscreen::Hide,
      anchored to ScreenIndicatorAnchorKind::Entity(ship) with fixed
      pixel offsets that park them beside the velocity sphere (tunable
      consts, e.g. right of the ship point and stacked one row apart;
      final values by eye in /work).
- [ ] Replace `update_flight_status_text` with two drive systems in
      flight_status.rs: `drive_speed_chip` (always: `<speed> u/s` from
      LinearVelocity) and `drive_mode_chip` (engaged only: `AP <VERB> -
      <PHASE>` from Autopilot; clears anchor + text when no Autopilot).
      Register them before ScreenIndicatorSystems in NovaHudSystems.
- [ ] Delete `flight::flight_status_line`, `flight::GravStatus`, and
      their unit tests in crates/nova_gameplay/src/flight.rs; move the
      verb/phase label formatting into flight_status.rs as a small pure
      fn with its own tests.
- [ ] Add the radius spoke to
      crates/nova_gameplay/src/hud/maneuver_instruments.rs: a
      `RadiusSpokeMarker { ship }` world-space entity synced by a new
      `sync_radius_spoke` system while the player's ORBIT action is
      engaged and the well exists - a thin cylinder (reuse
      HoloAssets::segment_mesh + material, `segment_transform` pattern
      from holo_instruments.rs; consider making segment_transform
      pub(crate) instead of duplicating) stretched from the well center
      to the ship position, updated every frame, despawned when the
      maneuver or well ends. Add it to the despawn sweep in
      hud/mod.rs remove_hud_flight_status.
- [ ] Add a `RadiusSpokeChipUIMarker` chip to the maneuver_instruments
      layer showing the current radius (`r <dist>` from ship-to-well
      distance) anchored at the spoke midpoint
      (ScreenIndicatorAnchorKind::Point), driven by a
      `drive_radius_spoke_chip` system; hidden outside engaged ORBIT.
- [ ] Re-dock the keybind hint cluster in
      crates/nova_gameplay/src/hud/keybind_hints.rs from bottom 28px to
      bottom 8px (the status line under it is gone); update the comment
      that references the line.
- [ ] Unit tests: speed/mode chip drive systems (manual vs engaged,
      formatting, clear-on-disengage) in flight_status.rs; spoke + chip
      lifecycle (spawn on engaged plan, tracks ship, despawn on
      disengage) in maneuver_instruments.rs, mirroring the existing
      orbit-ring test.
- [ ] Sweep for stragglers: `grep -rn "flight_status_line\|GravStatus"`
      must come back empty; `cargo check` and `cargo fmt` clean; run the
      newly written tests only (per repo test policy).
- [ ] CHANGELOG.md under [Unreleased]: one Changed line - the bottom-left
      flight status line is replaced by ship-anchored speed and mode
      chips plus an ORBIT radius spoke holo.

## Notes

- Spike: docs/spikes/20260710-234019-diegetic-flight-status.md
- The destination marker (autopilot_destination_hud) in flight_status.rs
  stays untouched; examples/12_hud_range.rs asserts on it.
- No consumers of flight_status_line/GravStatus outside flight.rs and
  hud/flight_status.rs (verified 2026-07-10).
- hud/mod.rs setup_hud_flight_status keeps working as-is if
  flight_status_hud keeps its name/config; only the bundle internals
  change. remove_hud_flight_status needs the spoke marker added.
- Chip offset is fixed-px in v1 (spike open question resolved: simplest
  first); the velocity sphere has world radius 5.0/5.6 u, so pick
  offsets that clear it at typical chase-camera distance.
- Speed format `{:5.1} u/s` matched the old line; keep one decimal.
