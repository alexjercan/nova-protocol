# Faction/relation model (minimal)

Task: tasks/20260708-203708. The game needs exactly three answers about any
pair of entities - own, hostile, or neutral - to drive AI target selection
and HUD coloring. This is deliberately NOT a faction system (no alliances,
no reputation); if the game ever needs one, that is its own spike.

## The model

`crates/nova_gameplay/src/relations.rs`:

- `Allegiance` - a component enum: `Player`, `Enemy`, `Neutral`.
- `Relation` - the resolved stance: `Own`, `Hostile`, `Neutral`.
- `relation(Option<&Allegiance>, Option<&Allegiance>) -> Relation` - a pure
  function, so callers pass query results directly and unmarked entities
  read as bystanders.

The matrix, with `-` meaning `None` (no component):

| a \ b   | Player  | Enemy   | Neutral | -       |
|---------|---------|---------|---------|---------|
| Player  | Own     | Hostile | Neutral | Neutral |
| Enemy   | Hostile | Own     | Neutral | Neutral |
| Neutral | Neutral | Neutral | Neutral | Neutral |
| -       | Neutral | Neutral | Neutral | Neutral |

Only combatant sides (Player/Enemy) relate strongly. `Neutral` never
resolves `Own`, not even against itself: two neutral asteroids are not
allies in any meaningful sense, and nothing should treat them as such.

## Where allegiance comes from

- **Ship roots**: by component requirement, not spawn wiring -
  `PlayerSpaceshipMarker` requires `Allegiance::Player`,
  `AISpaceshipMarker` requires `Allegiance::Enemy`. Every marked root
  (scenario-spawned, editor-spawned, test-spawned) participates
  automatically; an explicit `Allegiance` inserted alongside the marker
  still wins, which is the escape hatch for future neutral-but-piloted
  ships.
- **Projectiles**: both spawn paths (turret bullets, torpedoes) COPY the
  shooter's allegiance onto the spawned body. Copy-at-spawn was chosen over
  resolving through `ProjectileOwner` at read time because the projectile
  stays attributable after the shooter dies, and consumers stay
  single-query.
- **Everything else** (asteroids, debris, trigger volumes): no component,
  resolves `Neutral`.

## Consumers today

- **Targeting** (`input/targeting.rs`): the signature-range auto-acquire
  only locks entities whose relation to the player is `Hostile` - so an
  enemy's torpedo is auto-acquirable while the player's own is not, and
  neutral ships are never grabbed. Deliberate cone aiming can still lock
  anything dynamic.
- **HUD reticle** (`hud/torpedo_target.rs`): the lock reticle tints by
  relation - hostile red, own (e.g. your own loitering torpedo) green,
  neutral white.

Planned consumer: AI target selection over hostiles (task 20260709-225727,
docs/spikes/20260709-225508-ai-combat-behaviors.md).

## Alternatives considered

- Scenario-spawner wiring (insert `Allegiance` in the `SpaceshipController`
  match): works, but every other spawn site (tests, editor) would need the
  same wiring; requires give it to every marker for free.
- A relations resource/lookup table keyed by faction IDs: the full-faction
  design; overkill for three sides and one question.
