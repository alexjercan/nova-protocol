# Map: show each asteroid's material type

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.19.0, interface, map

## User facts
- Owner 2026-11-22: "in the map we should also show what type the asteroid
  is, like rock, water ice etc, just like in the 'lock' rtt".

## Delivery
- Map contacts for asteroids show the material type, with the same names
  the lock view uses.

## Findings
- The lock inset caption reads the body's `Name`
  (`crates/nova_hud/src/target_inset.rs`). A streamed rock is named by its
  kind, `generated_asteroid_name` (`crates/nova_world/src/streaming.rs`), as
  `Metal asteroid`.
- The Map named every terrain contact by its type word, `ASTEROID`
  (`terrain_name` in `crates/nova_interface/src/map/contacts.rs`), so the
  kind never reached the Map.

## Build (2026-11-22)
- The Map's terrain contact reads the body's `Name`, the inset's source; a
  body with no `Name` keeps `ASTEROID` / `PLANET`. No new UI: the contact
  panel's name row shows it on selection. The Kind row stays `TERRAIN`. An
  authored rock reads its authored name, as in the lock.
- The kind itself is already a closed enum checked at lint and load; no
  new mapping.
- Test: `the_map_names_an_asteroid_by_its_type_and_a_planet_by_its_name_as_the_lock_does`
  (`crates/nova_interface/src/map/tests.rs`), passes.
- Frame: `~/Pictures/nova-map-names/map-names-map-asteroid.png` (example
  `wiki_map_names`, seed 10, transport): panel reads `AST-146`,
  `Metal asteroid`, `TERRAIN`.
- Open: the blip label on the chart still shows the code only; the type
  shows after a click. Owner to judge.
- Not committed.

## Verification
- Map test asserts the type per asteroid; a Map frame.

## Done when
- The Map shows each asteroid's type and the owner has seen it.
