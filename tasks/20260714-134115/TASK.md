# Ship-prototype content kind: GameShips + *.ship.ron + ShipSource + ship-modifications (folds 113414)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, modding, scenario, spike

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

## Re-based v2 (20260714, spike tasks/20260714-150410)

GATED ON the content-model foundation (20260714-150508). In v2 the kind is a DATA flag,
not an extension: add `Content::Ship(SpaceshipConfig)` to the `Content` enum + one
`register_content` router arm -> `GameShips`; NO bespoke `*.ship.ron` loader. So this
task shrinks to: the `Content::Ship` variant, the `GameShips` registry, the `ShipSource`
resolution at spawn, and the `ShipModification` component set. Ships get authored as
`Ship((..))` content items in a bundle. The old `*.ship.ron` framing above is superseded.

## DEFERRED (20260714, /flow 134115 -> re-order)

Deferred until a REAL CONSUMER exists (a mod or fleet scenario that references a ship
prototype). Decision (user, during /flow): NO built-in ship is reused, so the reference/
resolution machinery (ShipSource on ScenarioObjectKind::Spaceship + spawn-time
resolution against GameShips + churning every built-in to Inline) would be speculative -
and it is trickier than sections were (a ship prototype is the whole SpaceshipConfig and
must resolve BEFORE the spawn bundle is built, but the spawn runs in a Commands closure
with no resource access, unlike the section observer's Res<GameSections> hook). So build
this WITH its first consumer, during/after the mods+demo work (20260714-134127), not now.
On the content-model foundation (150508) the DEFINE side is just a `Content::Ship` variant
+ a `GameShips` registry + one router arm; the reference side is the real work and needs a
consumer to justify its shape. Ship-modifications stay deferred too (mirror
SectionModification when needed). Priority dropped to 22.
