# Factions

Sides in Nova Protocol are deliberately minimal - three states, no alliances or reputation. It is just enough to tell friend from foe from furniture.

## The relation model

Every ship carries an allegiance - **Player**, **Enemy** or **Neutral** - and any two things resolve to one of three relations: Own, Hostile or Neutral.

<div class="widget" data-widget="relation-matrix">
<p>The whole model in one grid: Player-Player and Enemy-Enemy resolve OWN, Player against Enemy resolves HOSTILE either way, and everything touching Neutral or an unmarked body (asteroids, debris, salvage) resolves NEUTRAL - even Neutral against Neutral. Non-combatants stay out of the fight by default.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

- **Own** - the same combatant side (Player-Player or Enemy-Enemy).
- **Hostile** - opposing combatants (Player vs Enemy).
- **Neutral** - everything else: anything marked Neutral, and any body without an allegiance at all (asteroids, debris, salvage). Neutral never relates strongly, not even to another neutral - two bystander haulers are not each other's "own" in any meaningful sense.

</details>

## What allegiance drives

Relation is the switch behind most of combat:

- **Projectile allegiance** - a round copies its shooter's side at launch and keeps it even if the shooter dies, so your torpedo stays yours and never hits your own hull.
- **Targeting** - what you can lock and what the AI will attack keys off relation and radar [signature](../targeting-radar/).
- **AI hostility** - enemy ships engage hostiles, remember who shot them, and raise their weapons only when they have a hostile target.
- **AI flight** - an enemy flies its fight on the same flight computer you do. It asks the computer to HOLD a velocity - closing while it is outside its standoff, circling once it is inside - and to keep its nose on you while the computer holds it. The standoff is the clear space it wants between the two HULLS, not between the two centres, so a carrier is fought at the same visible distance a skiff is instead of being crowded by its own size. Both speeds are read off the ship in front of you: it closes only as fast as its live drive can still stop it, and circles only as fast as that drive can hold the turn and its nose can track you, so a heavy hull flies its fight slower than a picket does instead of both moving at one number. So an enemy at its standoff is crossing your bow with its guns still bearing, and it lights only the engines that burn for that; it swings the hull off you only for a correction big enough to need the main drive, and comes back when the burn is done. A hull with its engines shot off, or with its flight computer dead, stops maneuvering at all.

## Reading it on the HUD

The [HUD](../hud/) reads allegiance two ways. An **allegiance marker** - a small filled triangle above every ship - is coloured by side (green ally, red threat, grey neutral) so you can read a whole brawl at a glance; your own ship shows none. Up close, the target viewfinder's faction caption carries the same relation (hostile red, own green, neutral steel) rather than tinting the reticle itself. A player ship spawns Player and an AI ship spawns Enemy automatically - the allegiance rides along with the ship marker at spawn, and a ship re-aligned mid-scenario (a neutral provoked into a threat) recolours its marker on the spot.
