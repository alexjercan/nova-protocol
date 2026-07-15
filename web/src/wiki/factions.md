# Factions

Sides in Nova Protocol are deliberately minimal - three states, no alliances or reputation. It is just enough to tell friend from foe from furniture.

## The relation model

Every ship carries an allegiance - **Player**, **Enemy** or **Neutral** - and any two things resolve to one of three relations:

- **Own** - the same combatant side (Player-Player or Enemy-Enemy).
- **Hostile** - opposing combatants (Player vs Enemy).
- **Neutral** - everything else: anything marked Neutral, and any body without an allegiance at all (asteroids, debris, salvage). Non-combatants stay out of the fight by default.

## What allegiance drives

Relation is the switch behind most of combat:

- **Projectile allegiance** - a round copies its shooter's side at launch and keeps it even if the shooter dies, so your torpedo stays yours and never hits your own hull.
- **Targeting** - what you can lock and what the AI will attack keys off relation and radar [signature](../targeting-radar/).
- **AI hostility** - enemy ships engage hostiles, remember who shot them, and raise their weapons only when they have a hostile target.

## Reading it on the HUD

The [HUD](../hud/) carries the relation in the target viewfinder's faction caption (hostile red, own green, neutral steel) rather than tinting the reticle itself. A player ship spawns Player and an AI ship spawns Enemy automatically - the allegiance rides along with the ship marker at spawn.
