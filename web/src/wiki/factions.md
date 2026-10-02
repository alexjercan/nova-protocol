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

## Retaliation

One exception rides on top of the side-only model. An **armed** Neutral ship that is hit answers whoever hit it, whatever that ship's side: the two read Hostile to each other, and only to each other - everyone else still reads the shooter's and the Neutral's own sides as usual. A later shot from a different ship replaces who it answers.

<details class="explain">
<summary>Show explanation</summary>

It lets go the moment any of these is true: the ship it was answering is gone or neutralized; the Neutral stops being an armed combatant; or either ship strays beyond the Neutral's leash. A fresh hit can always name a new target. An **unarmed** Neutral never answers this way, and a Player-aligned ship the player hits stays allied - only a Neutral hitting back works like this.

</details>

It shows up everywhere relation is read: acquisition and sensing, the [allegiance marker](../hud/#allegiance-markers) over the Neutral's hull, the [target viewfinder](../hud/#target-viewfinder) caption, the [Map](../interface/#the-map) contact kind, and the threat-lock audio cue. A [dock](../sections/docking/) request reads it too - see [Docking port](../sections/docking/#who-lets-you-dock).

## What allegiance drives

Relation is the switch behind most of combat:

- **Projectile allegiance** - a round copies its shooter's side at launch and keeps it even if the shooter dies, so your torpedo stays yours and never hits your own hull.
- **Targeting** - what you can lock and what the AI will attack keys off relation and radar [signature](../targeting-radar/).
- **AI hostility** - enemy ships engage hostiles, remember who shot them, and raise their weapons only when they have a hostile target.
- **AI flight** - an enemy flies its fight on the same flight computer you do. It asks the computer to HOLD a velocity - closing while it is outside its standoff, circling once it is inside - and to keep its nose on you while the computer holds it. The standoff is the clear space it wants between the two HULLS, not between the two centres, so a large hull is fought at the same visible distance as a small hull instead of being crowded by its own size. Both speeds are read off the ship in front of you: it closes only as fast as its live drive can still stop it, and circles only as fast as that drive can hold the turn and its nose can track you, so a heavy hull flies its fight slower than a picket does instead of both moving at one number. Both are speeds relative to YOU: it matches whatever you are doing and flies its fight on top of that, so running in a straight line does not shake it off the ring. So an enemy at its standoff is crossing your bow with its guns still bearing, and it lights only the engines that burn for that; it swings the hull off you only for a correction big enough to need the main drive, and comes back when the burn is done. Put your guns on it and it weaves: three legs, each flown until the ship has actually carried its whole hull clear of the line you were holding, so a large ship's weave is slow and wide and a small ship's is a flick. Its nose stays on you through the weave. A hull with its engines shot off, or with its flight computer dead, stops maneuvering at all - it cannot weave either, so it fights on straight.

## Reading it on the HUD

The [HUD](../hud/) reads allegiance two ways. An **allegiance marker** - a small filled triangle above every ship - is coloured by side (green ally, red threat, grey neutral) so you can read a whole brawl at a glance; your own ship shows none. Up close, the target viewfinder's faction caption carries the same relation (hostile red, own green, neutral steel) rather than tinting the reticle itself. A player ship spawns Player and an AI ship spawns Enemy automatically - the allegiance rides along with the ship marker at spawn. A scripted `SetAllegiance` re-alignment changes a ship's own side for good and recolours its marker on the spot; [retaliation](#retaliation) is different - the Neutral's side never changes, but its marker still reddens for you while it is answering your fire, and returns to grey once it lets go.
