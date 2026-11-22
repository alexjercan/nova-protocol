# Planets: show names, not IDs, in the world and on the Map

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.19.0, interface, map

## User facts
- Owner 2026-11-22: "I noticed that planets show up as the ID in the world,
  we should also show them more nicely as names not ids, and also in the
  map UI".

## Delivery
- Planets show a display name, not an ID, in the world labels and on the
  Map.

## Findings
- A streamed planet's `Name` was its scenario id, as
  `sector_0_0_0_planet_0` (`crates/nova_world/src/streaming.rs`, planet
  spawn). The inset caption and the Map show `Name`.
- The generator record `SectorPlanet` had no name
  (`crates/nova_world/src/generation.rs`).
- An authored planet already needs `base.name` (serde-required), but a
  blank one passed lint.

## Build (2026-11-22)
- `SectorPlanet` gains `name`; `validate_manifest` refuses a blank one; the
  canonical text prints it; the spawn uses it.
- `generated_planet_name(seed, node, index)`
  (`crates/nova_world_base/src/clusters.rs`): the cluster's seeded syllable
  name (`seeded_name`, the civilization name generator, moved to a shared
  function in `crates/nova_gameplay/src/civilization.rs`) plus a numeral
  per world, as `Quelcorgar I`. Own stream, so no cluster draw moves.
  Unique within a cluster; across clusters not guaranteed.
- A save keeps no planets; a Load regenerates them from the seed, so the
  name is the same.
- Scenario lint: a blank planet name is an error, and the load gate runs
  the same lint.
- Tests, all pass: `a_streamed_planetoid_is_named_by_its_record_not_its_id`,
  `a_generated_planet_is_named_and_keeps_its_name_after_a_save_and_load`,
  `a_scenario_planet_without_a_name_fails_lint`, and the blank-name case in
  `a_malformed_generator_answer_is_refused_before_preparation`.
- Frames (example `wiki_map_names`, seed 10, transport):
  `~/Pictures/nova-map-names/map-names-inset.png` (caption `Quelcorgar I`),
  `map-names-map-planet.png` (panel `PLN-1`, `Quelcorgar I`, `PLANET`).
- The chart view has no planet marks, so nothing changed there.
- Not committed.

## Verification
- Asserted names in world and Map; frames of both.

## Done when
- No planet shows an ID in the world or on the Map, and the owner has seen
  it.
