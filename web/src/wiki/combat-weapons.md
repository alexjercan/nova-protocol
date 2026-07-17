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

A [turret](../sections/turret/) is an articulated mount that aims at your combat lock with **true intercept lead** - the solution is computed in the shooter's own frame, so a moving ship's rounds actually land - bounded by its yaw and pitch limits and fire rate. Its rounds are sensor projectiles: they deal damage on first contact with no physical shove, and they curve through [gravity wells](../gravity-wells/) like everything else. The point-defense cannon is tuned to chip a target down over a visible burst rather than delete it, and prioritizes shooting down inbound torpedoes.

## Torpedoes

A torpedo homes on the combat lock with **proportional-navigation** guidance - turning toward where the target will be - after an arming gate clears (a short time or distance from launch, so it cannot go off in your lap). It detonates on a proximity fuze and deals **blast (area) damage** that falls off from the center, so torpedoes are about zoning and catching clustered or fragile targets where turret fire is precise and pointed.

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

Every round carries a damage type, and each turret has a loaded-ammo slot that sets its rounds' type (the ammo readout is color-coded to match):

- **Kinetic** - a plain slug; the generalist, unmodified against everything.
- **Armor-piercing (AP)** - a dense penetrator, strong against armor and turrets.
- **EMP** - anti-electronics; wrecks controllers, barely dents hull.
- **Explosive** - concussive area damage (what torpedo blasts deal); shreds thrusters, bounces off turret armor.

## Section resistances

Damage is scaled by a resistance table keyed on the [section](../sections/) it hits. Kinetic is 1.0 everywhere (the feel baseline); the multipliers:

<table class="controls">
    <tr>
        <td><strong>Section</strong></td>
        <td>
            Kinetic &nbsp;/&nbsp; AP &nbsp;/&nbsp; EMP
            &nbsp;/&nbsp; Explosive
        </td>
    </tr>
    <tr>
        <td>Hull</td>
        <td>1.0 / 1.5 / 0.1 / 1.0</td>
    </tr>
    <tr>
        <td>Thruster</td>
        <td>1.0 / 0.75 / 0.25 / 1.5</td>
    </tr>
    <tr>
        <td>Controller</td>
        <td>1.0 / 1.0 / 3.0 / 1.0</td>
    </tr>
    <tr>
        <td>Turret</td>
        <td>1.0 / 1.75 / 1.5 / 0.5</td>
    </tr>
    <tr>
        <td>Torpedo bay</td>
        <td>1.0 / 1.0 / 1.25 / 1.25</td>
    </tr>
</table>

So EMP into a controller (3.0x) cripples a ship's steering, while the same round barely scratches its hull (0.1x); AP is the answer to turret armor. Which weapon section fires which control is per-ship and rebindable in the editor (see [Keybinds](../keybinds/)).
