# Units: display 1 u = 10 m everywhere (m/km, m/s)

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.9.0,ui,hud,gameplay

## Story

Player-facing units are inconsistent: the speed chip reads `u/s`, the NOVA OS
map prints `u` ranges, while combat DST, the destination readout, edge arrows
and objective/beacon chips already print `m` at an implicit 1:1 world-unit
scale. Owner direction (2026-07-28): 1 world unit displays as 10 meters,
everywhere, and the unit `u` retires from the player surface (glossary
included). Display-only: world/physics values, content RON and AI tuning are
untouched.

## Steps

- [ ] One shared display-format helper (in nova_ui, next to the theme) owning
      the policy: distance in m below the km threshold, km above it; speed in
      m/s; world-units x10. Unit tests pin the boundaries, written fail-first.
- [ ] Apply at every formatting site: flight_status speed chip (`u/s`),
      torpedo_target (DST + CLS), maneuver_instruments (ETA|distance readout,
      orbit `r` spoke), lock_crosshairs radar label, edge_indicators labels,
      objective_markers chips, beacon_chips, nova_os_map (contact readout,
      INFO cells, `map goto` output), and any NOVA OS command output printing
      distances (`ship`, `objectives`, `log`).
- [ ] Grep-sweep for stragglers in player-facing format strings; record the
      commands and final counts in this task.
- [ ] Harness proof: extend an existing HUD rig (copy the nearest passing
      sibling first) to assert a known world distance/speed renders x10 in
      live chip text - the system wiring, not just the pure helper.
- [ ] Docs sweep in the SAME task (keep-docs-in-sync): wiki glossary (u and
      u/s entries), hud.md, targeting-radar.md, flight-autopilot.md,
      getting-started.md, gravity-wells.md, tutorial.html if it names units,
      CHANGELOG line. Leave dated history (tasks/, old news) verbatim.

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
content text mentions `u` (grepped assets/base 2026-07-28, zero hits). If the
spike (20260728-175726) picks a different m/km threshold than the default, follow the
spike.
