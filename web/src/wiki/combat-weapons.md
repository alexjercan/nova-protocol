# Combat & weapons

Two weapon families - precise turrets and area-effect torpedoes - feed one typed-damage model, so what you shoot matters as much as where you shoot it.

<figure class="figure">
    <!-- Capture: assets/wiki-combat.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-combat.png</span
        >
        <span class="figure__placeholder-note"
            >A firefight: turret tracers with a lead pip on
            the target and a torpedo curving in, ideally
            with a section being blown off.</span
        >
    </div>
</figure>

## Turrets

A [turret](../sections/turret/) is an articulated mount that aims at your combat lock with **true intercept lead** - the solution is computed in the shooter's own frame, so a moving ship's rounds actually land - bounded by its yaw and pitch limits and fire rate. Its rounds are sensor projectiles: they deal damage on contact with no physical shove, they [pierce](#piercing) what they destroy, and they curve through [gravity wells](../gravity-wells/) like everything else. The point-defense cannon is tuned to chip a target down over a visible burst rather than delete it, and prioritizes shooting down inbound torpedoes. A mount can carry **more than one barrel** - a twin-barrel PDC aims and fires every muzzle it has, throwing two streams at once (each at its own fire rate) that share the turret's one magazine, so it also drains that magazine twice as fast.

## Torpedoes

A torpedo homes on the combat lock with **proportional-navigation** guidance - turning toward where the target will be - after an arming gate clears (a short time or distance from launch, so it cannot go off in your lap). It detonates on a proximity fuze and deals **blast (area) damage** that falls off from the center, so torpedoes are about zoning and catching clustered or fragile targets where turret fire is precise and pointed.

<figure class="figure">
    <!-- Capture: assets/wiki-combat-torpedo.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-combat-torpedo.png</span
        >
        <span class="figure__placeholder-note"
            >A salvo in flight: launch burst at the bay,
            drive plumes curving onto the lock.</span
        >
    </div>
</figure>

## Cover & line of fire

Rounds are physical: a bullet expends itself on the **first tangible thing**
it meets, so an asteroid between you and a hostile eats the burst that was
meant for you. Hostiles respect the same geometry - a gunner with a rock on
the firing line **holds fire** instead of hosing the rock, and won't waste a
torpedo on cover either. Its attack orbit keeps it circling all the while,
so expect the pressure back the moment its motion clears the angle. That
makes hard cover a real tool: breaking the line of sight buys you a pause in
the pressure, not just a bullet sponge. Two caveats: destructible debris is
only cover until it gets shot away, and intangible volumes (beacon rings,
trigger zones) stop nothing. Point-defense is the exception - a turret
swatting an inbound torpedo keeps firing no matter what is in the way.

## Ammo & reloading

Weapons carry a finite magazine, shown by a small diegetic gauge riding on the weapon: a **ring** on each turret that drains as it fires, and a **row of pips** on the torpedo bay, one per loaded torpedo. Running dry is not the end of the fight - a spent weapon **auto-reloads**. Turrets dump their magazine then reload it to full after a few seconds; the torpedo bay slowly rearms one torpedo at a time. While a weapon is reloading the gauge fills back up as a **reload sweep** in a dimmer shade of the same color, so you can read at a glance how close it is to firing again. Because ammo always comes back, magazine size is a firing-rhythm limit, not a way to be permanently disarmed. (Some tutorial or sandbox ships fly with unlimited ammo, and then carry no gauge at all.)

## Damage types

Every round carries a damage type, and each turret has a loaded-ammo slot that sets its rounds' type (the ammo readout is color-coded to match). A damage type is **not** a damage multiplier - the same round deals the same number to a hull, a thruster or a turret. What a type changes is how the round **travels** through what it hits, which is something you can watch happen:

- **Kinetic** - the punch. The hardest single hit, and closing fast makes it harder still. It carries on only through what it **destroys**, spending its damage as it goes, and stops dead at anything it fails to kill.
- **Pierce** - the rake. Lower damage per hit, but dealt **in full to every section it crosses**, alive or dead, and never worn down by depth. Closing fast buys it more **depth**, never more damage.
- **Explosive** - the torpedo's blast. Area damage falling off from the centre; no line of flight, so no travel rule and no speed term. Its identity is its radius and its magnitude.

So the two guns answer different problems. Against one thin target the slug wins outright. Against something deep - a stack of sections, a ship you are shooting down its long axis - a rake puts damage into several sections at once, and its **total** can exceed what one round nominally carries. That is the trade, not a bug.

### Closing speed

Closing speed is how fast you and your target converge **along the round's line of flight**. Sideways motion counts for nothing, so a target circling you is not fleeing.

Both curves are anchored at **100 u/s**, a PDC round's muzzle speed - what it closes at when neither ship is going anywhere. At that speed the multiplier is exactly 1.0, so a station-keeping duel plays out on the weapon's own numbers. Charging raises it; running from your target lowers it, and a stern chase is a real penalty. Both are clamped at each end: a head-on charge at most **doubles** Kinetic damage and at most **trebles** Pierce depth, while the worst tail chase still leaves a quarter of a slug's punch and half a penetrator's reach.

The catalog ships two PDCs, **PDC Turret (Kinetic)** and **PDC Turret (Pierce)**, on the same mount with the same fire rate and magazine. The Pierce gun deals half the damage per hit. Mount one of each and the punch-versus-rake trade is the only thing you are feeling.

<figure class="figure">
    <!-- Capture: assets/wiki-combat-aftermath.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-combat-aftermath.png</span
        >
        <span class="figure__placeholder-note"
            >The same target after the blast: sections gone
            off the hull, debris still clearing.</span
        >
    </div>
</figure>

## How far a round travels

A round's type decides what happens after it hits something, and the two rules are different resources.

A **Kinetic** round carries its damage as a **budget**. A round that **destroys** what it hits spends only what that target had left and **carries the rest on** into whatever was behind it - so a 100-damage slug that kills a 20-point plate arrives at the next thing with 80. A round that fails to destroy its target is spent on it. Thin destructible cover is therefore a **cost** rather than a wall, and a slug can never deal more in total than it was fired with.

A **Pierce** round does not pay for travel out of its damage at all. It carries a separate **power** budget and spends that on **thickness**: crossing a section costs that section's **full health rating**, whether or not the round killed it and whether or not it was already damaged. So it crosses whatever it likes while power lasts, dealing its full damage to every layer, and its total damage happily exceeds what one round nominally carries.

Two things follow from pricing power on the rating rather than on what is left. Light plating is nearly free to rake through while a heavy hull block eats most of a round's power in one go - the spaced-armour intuition, intact. And softening a section with other fire does **not** open a cheaper hole through it, so there is no trick of chipping first and raking after.

Every rake also has a hard ceiling on how many sections one round may cross, so a round fired down the length of a lightly built ship cannot chain forever.

Nothing pierces an **indestructible** obstacle: an asteroid or a planetoid stops any round of any type at any speed. Torpedoes do not travel through anything either - they detonate.

## Blast and armour

A torpedo blast ignores all of this. It damages **everything inside its radius at once**, falling off with distance from the centre, with no line of fire and nothing to get through. Outer sections take more than inner ones simply because the warhead goes off outside the hull and is nearer to them.

That is deliberate: armour that a bullet cannot rake through is exactly what a torpedo is for. Which weapon section fires which control is per-ship and rebindable in the editor (see [Keybinds](../keybinds/)).

Ordnance is not free to shoot down, either. A warhead now carries enough hit points that no single PDC round can swat it - an intercept costs a short burst, not one lucky tap - while the siege bay's armoured torpedoes take sustained fire across the whole closing window.
