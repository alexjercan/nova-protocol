# Ship-prototype content kind: GameShips + *.ship.ron + ShipSource + ship-modifications (folds 113414)

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-113418/SPIKE.md

Goal: ships as a content KIND, the ship analogue of the section catalog (113408) +
prototype refs (113411), one level up. Add `GameShips` (`HashMap<ShipId,
SpaceshipConfig>`), a `*.ship.ron` loader (a `SpaceshipConfig` prototype - its
sections already use section-prototype refs), and a `ShipSource = Inline(SpaceshipConfig)
| Prototype(ShipId)` on `ScenarioObjectKind::Spaceship`, resolved at spawn against
`GameShips`. Ship-level modifications reuse 113411's component-observer model (a
`ShipModification` analogue on the ship root, inert where N/A - controller/speed-cap/
infinite-ammo-style deltas; decide the starter set). Folds the closed 113414.
Independently shippable; it is the ship kind the bundle loader (20260714-134119) then
merges. `spike` until planned.
