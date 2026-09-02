# Combat

Three weapon families - turrets, torpedoes and the spinal railgun - feed one typed-damage model, so what you shoot matters as much as where you shoot it. This chapter is the rules every weapon shares: who reaches whom, cover, what a round does inside a hull, magazines and point defense. What each weapon is and how it behaves lives on its own page - [Turret](../sections/turret/), [Torpedo bay](../sections/torpedo-bay/) and [Railgun](../sections/railgun/) - and which weapon fires on which control is per ship and rebindable in the editor (see [Keybinds](../keybinds/)).

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

## Three reaches

<!-- Values: crates/nova_authoring/src/base_content/sections/standard.rs railgun_lance_section (charge 1.5 :927, slug 1500 u/s :928 x 1.2 s :970 = 1800 u, 300 Pierce :942, 1800 power :948, rake 1.0 :967, one shell :981 back in 12 s :986); PDC reach 100 u/s :485 x 2.0 s :492; torpedo reach is the along-the-line table at the head of ordnance.rs:13-21 over the bay's 100 s lifetime standard.rs:1190; the raider orbit is AI_STANDOFF_RANGE 100 u plus a 25 u band, crates/nova_ship/src/input/ai/maneuver.rs:36-48. -->

The three families are three **reaches**, and the ladder is the shape of every fight: torpedoes at the long end, the railgun in the middle, guns close in. Reach is never an authored number - it is muzzle speed times how long the round lives - and so is the time a shot takes to arrive, which is what actually separates them.

<div class="widget" data-widget="weapon-reach">
<p>A PDC round lives two seconds at 100 u/s, so it reaches 200 u. A railgun slug lives 1.2 s at 1500 u/s and reaches 1800 u, arriving anywhere inside that in about a second. A torpedo cruises at 32 to 35 u/s for a hundred seconds, so it reaches about 2900 to 3100 u - and takes half a minute to cover 1000 u. Nothing can shoot a round or a slug down; a torpedo costs the defender a hundred to four hundred rounds of point defense.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

Enemy gunships close to about 100 u - a kilometre - and fight there, inside everyone's guns. The railgun's reach is nine times that, which is its whole reason to exist: it is the weapon that answers a hostile still burning in, the one that outranges every mount on the ship carrying it, and the only one of the three that arrives before its target can react - a slug is unanswerable in flight, so the charge is the only window the other side gets.

A torpedo owns the long end but pays for it in time. Fired at the edge of its reach a Serpent is a minute and a half out, and every one of those seconds is a second the defender's mounts are working on it. That is why the torpedo fight is about saturation, the railgun fight is about the line, and the gun fight is about closing.

</details>

- **Turrets** are the close fight: a mount leads its target and chips it down over a burst, and everyone's guns reach the same 200 u. The [Turret](../sections/turret/) page has the mount's arc, its barrel discipline and how it picks a torpedo to shoot down.
- **Torpedoes** home on the lock and burst against the skin. The Serpent weaves through point defense and the Lance runs straight and fast; the [Torpedo bay](../sections/torpedo-bay/) page runs both in and puts the warhead through a hull.
- **The railgun** is a spinal gun the hull aims: a 1.5 second charge, a slug at 1500 u/s, and a corridor three cells wide through whatever stands in the line. The [Railgun](../sections/railgun/) page has the scope for what one shot takes out.

<figure class="figure">
    <!-- Capture: assets/wiki-combat-railgun.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-combat-railgun.png</span
        >
        <span class="figure__placeholder-note"
            >A railgun shot mid-fight: the steel-blue slug
            crossing the gap, a corridor opening through a
            raider's flank and the far-side sections
            bursting out.</span
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
the pressure, not just a bullet sponge. Two caveats: a rock is cover that
**wears out where it is hit** - see below - and intangible volumes (beacon
rings, trigger zones) stop nothing. Point-defense is the exception - a turret
swatting an inbound torpedo keeps firing no matter what is in the way.

## Shooting rock

A hit on an asteroid takes a real bite out of it. The hole is in the rock's
shape, not painted on it: the silhouette changes, the collider changes with it,
and it is still there when you come back. A rock has no health bar, because it
has no health - what is left of the rock IS what is left of the rock.

Three things follow, and all of them are things you do with the trigger.

- **Hold it and the same hole deepens.** A round that lands within about ten
  metres of the last one joins that crater; a round further off opens its own.
  So the hole follows your aim instead of piling into the first place you shot.
- **Rocks take real time.** Material costs the same everywhere - a pebble and a
  planetoid give up the same amount to the same round - so a small rock is
  seconds of fire and a big one is minutes. Sustained fire is the only way
  through; a torpedo bowls out a large bite in one go.
- **Bite deep enough and pieces come off.** Anything a crater cuts loose becomes
  its own tumbling body, carrying the drift and spin the rock had. Everything
  smaller goes out as dust. Cut a rock in half across a firing line and you have
  opened it, not cleared it.

An **invulnerable** planetoid does none of this. It is scenery, it never wears,
and its gravity well cannot be shot away.

## Damage types

Every round carries a damage type, and each turret has a loaded-ammo slot that sets its rounds' type. The type is visible on the outside of the fight as well as inside the numbers: the ammo readout is color-coded to it, and the round in flight is modelled and lit to match - a Kinetic slug is a stubby amber tracer, a Pierce round a longer, finer steel-blue dart. Close in you can see the difference in shape; across an engagement the color is what you read. A damage type is **not** a damage multiplier - the same round deals the same number to a hull, a thruster or a turret. What a type changes is how the round **travels** through what it hits, which is something you can watch happen:

- **Kinetic** - the punch. The hardest single hit, and closing fast makes it harder still. It carries on only through what it **destroys**, spending its damage as it goes, and stops dead at anything it fails to kill.
- **Pierce** - the rake. Lower damage per hit, but dealt **in full to every section it crosses**, alive or dead, and never worn down by depth. Closing fast buys it more **depth**, never more damage. A [railgun](../sections/railgun/) slug is this rule at its ceiling, with one thing a gun round never has: a sphere trailing its tip that widens the cut into a corridor, paid for out of the same depth budget.
- **Explosive** - the torpedo's blast. Area pressure falling off from the centre, with no speed term. Ship sections shield what is behind them; a destroyed section transmits 65 percent of the pressure that reached it.

So the two guns answer different problems. Against one thin target the slug wins outright. Against something deep - a stack of sections, a ship you are shooting down its long axis - a rake puts damage into several sections at once, and its **total** can exceed what one round nominally carries. That is the trade, not a bug.

### Closing speed

Closing speed is how fast you and your target converge **along the round's line of flight**. Sideways motion counts for nothing, so a target circling you is not fleeing.

Both curves are anchored at **100 u/s**, a PDC round's muzzle speed - what it closes at when neither ship is going anywhere. At that speed the multiplier is exactly 1.0, so a station-keeping duel plays out on the weapon's own numbers. Charging raises it; running from your target lowers it, and a stern chase is a real penalty. Both are clamped at each end: a head-on charge at most **doubles** Kinetic damage and at most **trebles** Pierce depth, while the worst tail chase still leaves a quarter of a slug's punch and half a penetrator's reach. A railgun slug leaves at 1500 u/s, fifteen times the anchor, so it sits at the Pierce ceiling whatever the two ships are doing - closing speed is not a lever on the railgun.

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

Every gun rake also has a hard ceiling on how many sections one round may cross - six - so a round fired down the length of a lightly built ship cannot chain forever. The railgun's slug is the one exception: it has no layer cap at all, and power alone decides where it stops.

<div class="widget" data-widget="round-travel">
<p>Worked example: five light hull sections at 60 hp each. A 100-damage kinetic slug at 100 u/s destroys the first section and hits the second for 40; at 200 u/s it punches twice as hard and destroys three. A pierce dart deals its full damage to every section it crosses: a crossing costs 60 of its 300 power at 100 u/s (five sections deep), only 20 at 300 u/s - but never more than six sections.</p>
</div>

Nothing pierces a rock while its collider remains: an asteroid or a planetoid stops any round of any type at any speed. What a round does to a rock instead is take a bite out of it (see [Shooting rock](#shooting-rock)); an invulnerable planetoid does not even do that. Torpedoes do not travel through anything either - they detonate.

## Ammo & reloading

Every weapon in the game refills. A magazine is a **rate limit**, not a budget: no ship is ever left alive with nothing to fight with. What a weapon imposes instead is a rhythm, and it is the same rule for all of them - a batch lands only after a whole quiet interval, and every shot that lands restarts that interval.

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (PDC ammo_capacity 500 :502, reload delay 3.0 :504 / amount 200, fire_rate 100; bay ammo_capacity 6 :1225, reload delay 10.0 :1237 / amount 1, fire_rate 1.0; railgun ammo_capacity 1 :981, reload delay 12.0 :986 / amount 1, charge 1.5 :927) and crates/nova_ship/src/sections/ammo.rs (a successful shot resets the clock :136, a whole batch lands at the delay :171-174, clamped at capacity :156, empty pulls never reset :134). The sustained column is sections/mod.rs:202's own formula, amount / (delay + batch fire time). -->

| Weapon | Magazine | Cyclic rate | One batch | Quiet, empty to full | Sustained |
| --- | --- | --- | --- | --- | --- |
| PDC turret | 500 rounds | 100 /s | 200 rounds per 3 s | 9 s | 40 rounds/s |
| Torpedo bay | 6 torpedoes | 1 /s | 1 torpedo per 10 s | 60 s | 0.09 /s |
| Railgun | 1 shell | one per 1.5 s charge | 1 shell per 12 s | 12 s | 0.07 /s |

<div class="widget" data-widget="ammo-rhythm">
<p>A PDC turret holds 500 rounds and spends them at 100 a second, so a held trigger runs it dry in five seconds. It gets 200 back for every three seconds it stays quiet - all at once, or not at all: a pause a tick short of three seconds returns nothing, and any shot that lands starts the three seconds again. Firing each batch as it arrives sustains 40 rounds a second against a cyclic 100. A torpedo bay works the same way at a different scale: six torpedoes, one back per ten quiet seconds, a full minute from empty to a fresh rack. A railgun is the rule at its simplest: one shell, twelve quiet seconds, and the charge on top - a shot every thirteen and a half.</p>
</div>

The level is diegetic, on a gauge riding the weapon itself: a **ring** on each turret that drains as it fires, and a **row of pips** on the torpedo bay, one per loaded torpedo. While a weapon stays quiet the incoming batch pulses above the solid live rounds and brightens toward completion, and the gauge stays visible until it lands. (Some tutorial or sandbox ships fly with unlimited ammo, and then carry no gauge at all.)

The six are the torpedo bay's real weapon. Fired together they are a salvo the defender's guns cannot answer all of, and that is what makes a torpedo fight an attrition fight: a salvo costs the defender rounds whether or not it connects, and those rounds come back at a rate too. Patience wins nothing for either side. Torpedoes get through by arriving faster than the guns can answer, not by outlasting them.

## Point defense

A mount defends itself. Every gun runs its own point defense, picks its own inbound torpedo from the ones it can actually bear on, and holds it until that torpedo dies or leaves its arc; the [Turret](../sections/turret/#point-defense) page has the geometry. What it means for an attacker is **saturate a facing**: torpedoes arriving from one side, or from below a hull, meet only the mounts that can train on them.

### Your own battery

Your flight computer works the guns you are NOT using. There is no toggle and no
key: while you hold no combat lock and your weapons are lowered, the computer
may put your idle PDCs onto inbound torpedoes and fire them - the weapons safety
does not stop it, because the safety is your trigger discipline and this is not
your trigger. A thin line runs from each mount the computer is working to the
torpedo it picked, so you can see what the ship took and what it chose.

**You always win the argument.** Lock a target, or raise the weapons, and every
mount is yours that instant - the computer drops its claim and lets go of the
trigger it was holding. Let go again and the battery comes back to the computer
after a short pause, so a tap-clear on the way to your next lock does not make
the mounts swing away and back.

Scenarios can take the capability away: a hull whose flight computer does not
grant point defense answers a salvo only by hand.

A **neutralized** hull does not defend itself at all - take a ship's last gun or its flight computer and it is out of the fight for good. [Ships & damage](../ships/#taking-a-ship-apart) has the rule.
