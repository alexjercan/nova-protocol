# Epic: stations as the place loss is redeemed

- STATUS: CLOSED
- PRIORITY: 89
- TAGS: v0.16.0, epic, stations

## User facts
- Stage 1 of the game expansion (design page Part 5). Owner decisions 2026-10-12: stations are a new `Content::Station` kind with their own object kind and frozen body type (decision 1); the generator places at least one per living civilization and a dead one per extinct civilization (2); services only while docked, held jobs stay on the Map (2b); stations are dynamic thrusterless bodies with self-levelling controllers, in orbit of the cluster's planet when it has one (3, 3a); a living station owns the helm while docked (3b); rebuild and refit only docked, sections are never items, each section config owns a parts recipe (4); one connection per hull (6b); a destroyed ship drops hold canisters and a share of section parts (7); the chart layer lands with the station contacts (9b).
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Stations:` title prefix and the `stations` tag.
1. Content kind, object kind and frozen body type.
2. Dynamic thrusterless body that self-levels; orbit placement proof.
3. Generator placement per civilization; dead stations.
4. Docking admission and helm ownership.
5. Parts recipes on sections; rebuild and refit while docked.
6. Buy stock, store and withdraw parts, sell goods while docked.
7. Hold canisters and section parts on destruction.
8. Map: station contacts, held jobs, chart layer, visited record.

## Owner decisions
- 2026-10-13 (reverses the invulnerability from 20261012-100000): stations take damage. Approved proposal:
  1. A living station that loses all turrets or all controllers becomes a dead station (services stop, hold lootable, docked player gets the helm); a high collapse floor keeps it from being peeled to nothing; stored items stay with it. No despawn.
  2. The station damage gate in `on_damage` (`crates/nova_gameplay/src/integrity/health.rs`) is removed; targeting by side as for ships.
  3. Living stations spawn with an AI controller (turret aim, point defense, retaliation); thrusterless AI gets no maneuver.
  4. Enemy and Player stations fire on hostile sides; Neutral stations only retaliate. Primary fire holds while docked; point defense must not fire through the docked ship.
  5. New station design about 150-250 m: hub, four spokes and a ring on the grid, PDC turrets around hub and ring, several controllers, several docking ports; new Hull-kind truss and solar panel prototypes with simple generated meshes plus greeble kits; the design stocks PdcRound for its turrets.
  6. Proofs: the station torpedo proof becomes damaged-but-level-and-in-orbit; new proofs for derelict on last turret lost and docking port destroyed while docked releases; a screenshot of the new look.

- Landed 2026-10-13: ad4e89637 (damage floor at the design's collapse threshold, 0.5 for stations; dead on last weapon or last flight computer), e87d2729e (station AI aims by side, holds guns on a docked ship, point defense masked behind it), 86e971480 (truss_hull_section, solar_panel_section, `station` style with six greebles; ring_station 177 sections 230 m, spire_station 217 sections 205 m, yard_station 190 sections 270 m, each civilization draws one; waystation gained a second bridge), 073d48450 (orbit torpedo proof now damaged-but-level, screenshot_station_showcase example, docs). Owner additions: parts in the section gallery and greeble catalog, several station styles, showcase example. `content lint` re-run by orchestrator: 0 errors; screenshots reviewed. Agent-chosen values for owner review: damage floor 0.5, armed station stock 1000 PdcRound and 20000 credits, no AI leash, sever keeps station velocity (no momentum conservation), wreck canisters now inherit ship velocity. Gaps: turrets small in close shots, no turret-firing screenshot, wiki hull page image placeholders for the two parts, gallery labels say "NEW".

- Epic close 2026-10-13: Done when proven in one run, `examples/systems/system_station_round_trip.rs` (3626b51f6): New Game modal on seed 10, a PDC section zeroed as fixture, dock at a living generated station, Ship pane Rebuild (recipe parts and credits asserted, OnRebuilt once), undock, retire (frozen 0.00 m from where left), thaw (same state), Leave, Load through the Load screen, station as left and the section whole. Passed twice under lavapipe. Not covered: `seed: None`, combat loss, flown approach (system_station_docking), an orbiting station's pose, station credits inside the frozen record.

- Owner feedback 2026-10-14: Neutral stations and ships are too sensitive. A light scrape from a collision makes them open fire. Add a damage threshold before a Neutral retaliates, and make a collision deal no damage below a minimum impact.

## Done when
- A New Game player loses a section, docks at a generated station, rebuilds it from parts and credits, undocks, leaves the sector, returns, and the station is where and as they left it after a reload.

## Children
- `20261012-095959` Stations: Content::Station kind, object kind and frozen body type
- `20261012-100000` Stations: dynamic thrusterless body that self-levels; orbit proof
- `20261012-100001` Stations: generator placement per civilization; dead stations
- `20261012-100002` Stations: docking admission and helm ownership
- `20261012-100004` Stations: parts recipes on sections; rebuild and refit while docked
- `20261012-100005` Stations: buy stock, store and withdraw parts, sell goods while docked
- `20261012-100006` Stations: hold canisters and section parts on destruction
- `20261012-100007` Stations: Map shows station contacts, held jobs and the chart layer
