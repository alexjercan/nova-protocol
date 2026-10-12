# Epic: stations as the place loss is redeemed

- STATUS: OPEN
- PRIORITY: 89
- TAGS: v0.16.0,epic,stations

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
