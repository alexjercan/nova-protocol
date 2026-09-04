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

<!-- Values: crates/nova_authoring/src/base_content/sections/standard.rs railgun_lance_section (charge 1.5 :928, slug 15,000 m/s :929 x 1.2 s :971 = 18 km, 300 Pierce :943, 1800 power :949, rake 10 m :968, one shell :982 back in 12 s :987); PDC reach 1,000 m/s :486 x 2.0 s :493 = 2 km; torpedo reach is the along-the-line table at the head of ordnance.rs:13-21 over the bay's 100 s lifetime standard.rs:1180; the raider orbit is AI_STANDOFF_RANGE plus its band - engine world units, measured against avian - which is 1 to 1.25 km, crates/nova_ship/src/input/ai/maneuver.rs:36-48. -->

The three families are three **reaches**, and the ladder is the shape of every fight: torpedoes at the long end, the railgun in the middle, guns close in. Reach is never an authored number - it is muzzle speed times how long the round lives - and so is the time a shot takes to arrive, which is what actually separates them.

<div class="widget" data-widget="weapon-reach">
<p>A PDC round lives two seconds at 1,000 m/s, so it reaches 2 km. A railgun slug lives 1.2 s at 15,000 m/s and reaches 18 km, arriving anywhere inside that in about a second. A torpedo cruises at 320 to 350 m/s for a hundred seconds, so it reaches about 29 to 31 km - and takes half a minute to cover 10 km. Nothing can shoot a round or a slug down; a torpedo costs the defender a hundred to four hundred rounds of point defense.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

Enemy gunships close to about a kilometer and fight there, inside everyone's guns. The railgun's reach is nine times that, which is its whole reason to exist: it is the weapon that answers a hostile still burning in, the one that outranges every mount on the ship carrying it, and the only one of the three that arrives before its target can react - a slug is unanswerable in flight, so the charge is the only window the other side gets.

A torpedo owns the long end but pays for it in time. Fired at the edge of its reach a Serpent is a minute and a half out, and every one of those seconds is a second the defender's mounts are working on it. That is why the torpedo fight is about saturation, the railgun fight is about the line, and the gun fight is about closing.

</details>

- **Turrets** are the close fight: a mount leads its target and chips it down over a burst, and everyone's guns reach the same 2 km. The [Turret](../sections/turret/) page has the mount's arc, its barrel discipline and how it picks a torpedo to shoot down.
- **Torpedoes** home on the lock and burst against the skin. The Serpent weaves through point defense and the Lance runs straight and fast; the [Torpedo bay](../sections/torpedo-bay/) page runs both in and puts the warhead through a hull.
- **The railgun** is a spinal gun the hull aims: a 1.5 second charge, a slug at 15,000 m/s, and a corridor three cells wide through whatever stands in the line. The [Railgun](../sections/railgun/) page has the scope for what one shot takes out.

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
  meters of the last one joins that crater; a round further off opens its own.
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

Both curves are anchored at **1,000 m/s**, a PDC round's muzzle speed - what it closes at when neither ship is going anywhere. At that speed the multiplier is exactly 1.0, so a station-keeping duel plays out on the weapon's own numbers. Charging raises it; running from your target lowers it, and a stern chase is a real penalty. Both are clamped at each end: a head-on charge at most **doubles** Kinetic damage and at most **trebles** Pierce depth, while the worst tail chase still leaves a quarter of a slug's punch and half a penetrator's reach. A railgun slug leaves at 15,000 m/s, fifteen times the anchor, so it sits at the Pierce ceiling whatever the two ships are doing - closing speed is not a lever on the railgun.

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

## Magazines

Every weapon in the game refills. A magazine is a **rate limit**, not a budget: no ship is ever left alive with nothing to fight with. What a weapon imposes instead is a rhythm, and it is the same rule for all of them - a batch lands only after a whole quiet interval, and every shot that lands restarts that interval.

The level is diegetic, on a gauge riding the weapon itself: a **ring** on each turret that drains as it fires, and a **row of pips** on the torpedo bay, one per loaded torpedo. While a weapon stays quiet the incoming batch pulses above the solid live rounds and brightens toward completion, and the gauge stays visible until it lands. (Some tutorial or sandbox ships fly with unlimited ammo, and then carry no gauge at all.)

The six are the torpedo bay's real weapon. Fired together they are a salvo the defender's guns cannot answer all of, and that is what makes a torpedo fight an attrition fight: a salvo costs the defender rounds whether or not it connects, and those rounds come back at a rate too. Patience wins nothing for either side. Torpedoes get through by arriving faster than the guns can answer, not by outlasting them.

The [Turret](../sections/turret/#trigger-discipline) page has the scope: set a burst and a pause and read what a gun, a bay and a railgun actually hold.

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
