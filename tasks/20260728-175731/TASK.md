# Units: display 1 u = 10 m everywhere (m/km, m/s)

- PRIORITY: 42
- TAGS: v0.9.0, ui, hud, gameplay
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

Player-facing units are inconsistent: the speed chip reads `u/s`, the NOVA OS
map prints `u` ranges, while combat DST, the destination readout, edge arrows
and objective/beacon chips already print `m` at an implicit 1:1 world-unit
scale. Owner direction (2026-07-28): 1 world unit displays as 10 meters,
everywhere, and the unit `u` retires from the player surface (glossary
included). Display-only: world/physics values, content RON and AI tuning are
untouched.

## Steps

- [x] One shared display-format helper (in nova_ui, next to the theme) owning
      the policy: distance in m below the km threshold, km above it; speed in
      m/s; world-units x10. Unit tests pin the boundaries, written fail-first.
      DONE: `nova_ui::units` (`distance`/`speed`/`closing_speed`), 5 unit
      tests + 3 doctests green.
- [x] Apply at every formatting site: flight_status speed chip (`u/s`),
      torpedo_target (DST + CLS), maneuver_instruments (ETA|distance readout,
      orbit `r` spoke), lock_crosshairs radar label, edge_indicators labels,
      objective_markers chips, beacon_chips, nova_os_map (contact readout,
      INFO cells, `map goto` output), and any NOVA OS command output printing
      distances (`ship`, `objectives`, `log`).
      DONE: 11 live sites converted. `ship`/`objectives`/`log` print no
      distances (HP/ammo/sections only) - sweep-confirmed, nothing to change.
- [x] Grep-sweep for stragglers in player-facing format strings; record the
      commands and final counts in this task. See Notes -> Sweep record.
- [x] Harness proof: extend an existing HUD rig (copy the nearest passing
      sibling first) to assert a known world distance/speed renders x10 in
      live chip text - the system wiring, not just the pure helper.
      DONE: `flight_status::speed_chip_tracks...` asserts `50.0 m/s` (ship
      vel len 5.0 u); `torpedo_target::readout_fills...` asserts `DST 1.50 km`
      + `CLS +200.0 m/s` (150 u / 20 u/s); new `nova_os_map::map_range_renders
      _in_metres_and_kilometres`. All are live-system tests, not pure helpers.
- [x] Docs sweep in the SAME task (keep-docs-in-sync): wiki glossary (u and
      u/s entries), hud.md, targeting-radar.md, flight-autopilot.md,
      getting-started.md, gravity-wells.md, tutorial.html if it names units,
      CHANGELOG line. Leave dated history (tasks/, old news) verbatim.
      DONE: glossary redefines m/km + m/s; getting-started, hud, flight-
      autopilot (3 sites), gravity-wells, targeting-radar converted; CHANGELOG
      [Unreleased] Interface & HUD line added. tutorial.html names no units.
      Dev authoring guide's "units per second" left verbatim (describes raw
      RON fields, which stay in world units). Past news = dated history, kept.

## Definition of Done

1. test: formatter unit tests pin the m/km threshold, m/s and the x10 scale.
2. test: an App-driven/harness assertion proves a live HUD chip renders the
   converted value (would fail if the formatting system were a no-op).
3. cmd: `grep -rn 'u/s' crates/ --include='*.rs'` and a `[0-9] u` format-string
   sweep show zero player-facing hits (internal names/comments allowed);
   commands + counts recorded here.
4. cmd: `grep -rn 'u/s' web/src/wiki/ web/src/tutorial.html` and a unit-u
   sweep show no live unit-u references; the glossary defines m/km/m-s.
5. manual: owner eyeballs the speed chip, a combat lock readout and the map
   app in game - numbers read x10 with the right unit labels.

## Notes

The map app's bearing/mark format stays; only range units change. No authored
content text mentions `u` (grepped assets/base 2026-07-28, zero hits).

### Sweep record (DoD 3 + 4, 2026-07-28)

DoD 3 - `grep -rn 'u/s' crates/ --include='*.rs'` = 89 hits, all internal:
doc comments, `//`-comments, test-assertion panic messages (turret_section,
camera_controller, flight), and the dev balance report (`content_report`/
`balance.rs`). ZERO player-facing format strings (the only two live `u/s`
sinks, flight_status + torpedo_target CLS, now use `nova_ui::units`).
`[0-9] u` format-string sweep (`grep -rEn '\{[^}]*\} u\b|[0-9] u\b|}u\b'
crates/ --include='*.rs'`) = only test-assertion messages, scenario/asset
authoring diagnostics (world-unit clearances) and the dev balance report -
no player-facing HUD/NOVA OS sinks. HudReadout widget takes no distance/speed
caller. Player-facing hits: 0.

DoD 4 - `grep -rnE '\bu/s\b|[0-9]+ ?u\b|`u`' web/src/wiki web/src/tutorial.html`
(excluding dev/) = 0 after the sweep; the only surviving "units" prose is the
new glossary m/km + m/s definitions. tutorial.html names no units. Dev
authoring guide (`guide-author-section.md`) keeps "units per second" - it
documents raw RON authoring fields, which the display change does not touch.
Past-release news posts (0.5/0.7/0.8) mention `u/s` as dated history, left
verbatim.

Owner (2026-07-28, plan gate): speed precision stays ONE decimal m/s
(`50.0 m/s`, `CLS +200.0 m/s`); distance is integer metres / 2-decimal km per
D6. Formatter lives in nova_ui beside the theme.

Spike DECISION D6 (20260728-175726) pinned the policy: 1 u = 10 m; distance
`< 1000 m` -> integer metres (`840 m`), `>= 1000 m` -> kilometres with 2
decimals (`1.24 km`); speed `m/s`; closing speed signed `m/s`; orbit radius in
metres. Follow that in the shared formatter.
