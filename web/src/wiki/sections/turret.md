# Turret

<figure class="figure">
    <!-- Capture: assets/icon-turret.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-turret.png</span
        >
        <span class="figure__placeholder-note"
            >A turret tracking a target, barrel led ahead of
            it, aim pip on the intercept point.</span
        >
    </div>
</figure>

A turret is an **articulated mount** - a base, the joints that swivel and elevate it, and one or more barrels - that aims at the current combat lock with true intercept lead and fires bullet projectiles. Its output is bounded by its fire rate, and a mount with several muzzles aims and fires all of them at once.

Turrets draw their aim from the combat lock and prefer a fine-locked section if you have one, falling back to live structure and then the camera ray.

## Aiming with lead

The mount aims at your combat lock with **true intercept lead** - the solution is computed in the shooter's own frame, so a moving ship's rounds actually land - bounded by its yaw and pitch limits and fire rate. Its rounds are sensor projectiles: they deal damage on contact with no physical shove, they carry on through what they hit by the rules of their [damage type](../../combat-weapons/#damage-types), and they curve through [gravity wells](../../gravity-wells/) like everything else. The point-defense cannon is tuned to chip a target down over a visible burst rather than delete it, and prioritizes shooting down inbound torpedoes. A mount can carry **more than one barrel** - a twin-barrel PDC aims and fires every muzzle it has, two offset streams that share the turret's one magazine and its total fire rate (see [Variants](#variants)).

## Barrel discipline

A gun fires only while its barrel is actually **on** the point it is aiming at.
The tolerance is what a round can still hit: about a degree, which is a
corvette's beam at a kilometer. So a mount shoots while it is tracking and
**holds while it is slewing**, and two things follow from that. Wrenching the
ship around mid-burst stops the guns until the barrels catch up. And a mount
that cannot train on your target at all - the port gun ordered onto something
off the starboard quarter, or anything under the keel - simply holds, while the
mounts that CAN bear keep shooting. It is your ammunition either way; the rounds
a gun does not spend are the ones that were going to miss.

## What it can bear on

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (turret_joint_tree :206: traverse limits None/None :282-283, elevation min -TURRET_DEPRESSION_LIMIT to FRAC_PI_2 :298-299 where that constant is PI/18 at :104, hinge speed PI rad/s :281,:291; pdc_turret_prototype :424: muzzle_speed 1,000 m/s :486 x projectile_lifetime 2.0 :493 = 2 km of reach, and crates/nova_ship/src/sections/turret_section/config.rs:133-137 states that product IS the reach) and crates/nova_ship/src/sections/turret_section/aim.rs (fire gate TURRET_ON_TARGET_RAD = HULL_HIT_RADIUS / CLOSE_ENGAGEMENT_RANGE :47 - a ratio of two engine world-unit lengths, 16 m across a 1 km range, so it is dimensionless and unchanged - the per-muzzle gate :72, doc'd 0.92 deg; the reachability test is derived from the elevation hinge alone, arc.rs:46-102). -->

The mount turns all the way round, so nothing is out of reach sideways. What bounds it is the barrel's floor: it stops ten degrees below level, because below that it would be pointing back through the ship it is bolted to.

<div class="widget" data-widget="turret-arc">
<p>Traverse is unlimited and the barrel elevates from 10 degrees below level to straight up, so one mount covers 58.7 percent of the sky and the rest is a blind cone under its own keel. Both hinges turn at 180 deg/s at once, so a swing costs the larger of the two angles rather than their sum - a 90 degree traverse takes half a second, which is 50 rounds the gun does not fire while it is moving. Reach is 2 km: muzzle speed times how long a round lives, not an authored range.</p>
</div>

That blind cone is the whole reason [point defense](#point-defense) is assigned per mount rather than per battery, and the reason a salvo arriving from one side meets fewer guns than one across the beam.

## Reach and closing speed

A gun is a **short-range** weapon. A round is not tracked forever: it expires after a couple of seconds, which puts a PDC slug's reach at about **2 km**. Everyone shoots the same gun, so a raider's rounds reach exactly as far as yours - what a scavenger-grade mount gives up is toughness, not range. Enemy ships know it - they close to roughly **1 km** and fight there, and they hold fire until they are inside their own reach, so a hostile burning toward you is not being polite, it is out of range. Closing speed moves the number in both directions: rounds inherit the ship that fired them, so a charge carries them further and running from your target cuts what they can reach along with what they hit for. [Combat](../../combat-weapons/#closing-speed) puts the multipliers on that, and the [engagement ladder](../../combat-weapons/#three-reaches) sets the gun's 2 km beside the other two families.

## How far a round travels

A round's type decides what happens after it hits something, and the two rules are different resources.

A **Kinetic** round carries its damage as a **budget**. A round that **destroys** what it hits spends only what that target had left and **carries the rest on** into whatever was behind it - so a 100-damage slug that kills a 20-point plate arrives at the next thing with 80. A round that fails to destroy its target is spent on it. Thin destructible cover is therefore a **cost** rather than a wall, and a slug can never deal more in total than it was fired with.

A **Pierce** round does not pay for travel out of its damage at all. It carries a separate **power** budget and spends that on **thickness**: crossing a section costs that section's **full health rating**, whether or not the round killed it and whether or not it was already damaged. So it crosses whatever it likes while power lasts, dealing its full damage to every layer, and its total damage happily exceeds what one round nominally carries.

Two things follow from pricing power on the rating rather than on what is left. Light plating is nearly free to rake through while a heavy hull block eats most of a round's power in one go - the spaced-armour intuition, intact. And softening a section with other fire does **not** open a cheaper hole through it, so there is no trick of chipping first and raking after.

Every gun rake also has a hard ceiling on how many sections one round may cross - six - so a round fired down the length of a lightly built ship cannot chain forever. The railgun's slug is the one exception: it has no layer cap at all, and power alone decides where it stops.

<div class="widget" data-widget="round-travel">
<p>Worked example: five light hull sections at 60 hp each. A 100-damage kinetic slug at 1,000 m/s destroys the first section and hits the second for 40; at 2,000 m/s it punches twice as hard and destroys three. A pierce dart deals its full damage to every section it crosses: a crossing costs 60 of its 300 power at 1,000 m/s (five sections deep), only 20 at 3,000 m/s - but never more than six sections.</p>
</div>

Nothing pierces a rock while its collider remains: an asteroid or a planetoid stops any round of any type at any speed. What a round does to a rock instead is take a bite out of it (see [Shooting rock](../../../combat-weapons/#shooting-rock)); an invulnerable planetoid does not even do that. Torpedoes do not travel through anything either - they detonate.

## Trigger discipline

A magazine is a rate limit, not a budget - every weapon refills, and the rule is the same for all three (see [Magazines](../../combat-weapons/#magazines)). What that rhythm is worth is easiest to read on the gun, with the bay and the railgun beside it for scale.

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (PDC ammo_capacity 500 :503, reload delay 3.0 :505 / amount 200 :506, fire_rate 100; bay ammo_capacity 6 :1226, reload delay 10.0 :1238 / amount 1, fire_rate 1.0; railgun ammo_capacity 1 :982, reload delay 12.0 :987 / amount 1, charge 1.5 :928) and crates/nova_ship/src/sections/ammo.rs (a successful shot resets the clock :136, a whole batch lands at the delay :171-174, clamped at capacity :156, empty pulls never reset :134). The sustained column is sections/mod.rs:202's own formula, amount / (delay + batch fire time). -->

| Weapon | Magazine | Cyclic rate | One batch | Quiet, empty to full | Sustained |
| --- | --- | --- | --- | --- | --- |
| PDC turret | 500 rounds | 100 /s | 200 rounds per 3 s | 9 s | 40 rounds/s |
| Torpedo bay | 6 torpedoes | 1 /s | 1 torpedo per 10 s | 60 s | 0.09 /s |
| Railgun | 1 shell | one per 1.5 s charge | 1 shell per 12 s | 12 s | 0.07 /s |

<div class="widget" data-widget="ammo-rhythm">
<p>A PDC turret holds 500 rounds and spends them at 100 a second, so a held trigger runs it dry in five seconds. It gets 200 back for every three seconds it stays quiet - all at once, or not at all: a pause a tick short of three seconds returns nothing, and any shot that lands starts the three seconds again. Firing each batch as it arrives sustains 40 rounds a second against a cyclic 100. A torpedo bay works the same way at a different scale: six torpedoes, one back per ten quiet seconds, a full minute from empty to a fresh rack. A railgun is the rule at its simplest: one shell, twelve quiet seconds, and the charge on top - a shot every thirteen and a half.</p>
</div>

## Point defense

Every gun runs its own point defense, and each mount picks its
OWN inbound torpedo rather than the whole battery swinging onto one. That is not
a fairness rule, it is geometry: a turret is bolted to a hull and cannot depress
its barrel back through its own ship, so a mount handed a torpedo coming in
under the keel would sit there contributing nothing while a torpedo it could
have hit flew past. A mount is only ever given something it can actually bear
on; the fire splitting across a salvo falls out of that.

Mounts also **hold** what they are tracking. Slewing takes real time, so a gun
stays on its torpedo until that torpedo dies, drifts out of its arc, or
something far more urgent arrives - a battery that re-decided every moment would
spend the whole engagement swinging and hit nothing.

So a salvo arriving from one side, or from below a hull, meets only the mounts that can actually train on it - the band above is exactly the band one gun defends. What your own idle mounts do with that, and when the flight computer is allowed to work them for you, is on [Combat](../../combat-weapons/#point-defense).

## Stowed between fights

<!-- Behavior verified against crates/nova_ship/src/sections/turret_section/stow.rs (deploy demand: weapons hot OR tracked target OR point-defense assignment, drive_turret_stow; settle STOW_SETTLE_SECONDS = 4.0; aim gate crates/nova_ship/src/sections/turret_section/aim.rs and fire gate firing.rs both skip a non-Deployed mount) and the authored travel times in crates/nova_authoring/src/base_content/sections/standard.rs pdc_stow_tracks (lift 0.9 s down / 0.35 s up, lids 0.5 s / 0.25 s). -->

A mount with nothing to do does not stand in the wind. Out of combat the barrel swings straight up, the assembly sinks into its housing, and two lid halves slide shut over it - the ship at rest reads as at rest. The gun comes back up the moment it is wanted: weapons hot, a live tracking target, or a point-defense assignment all deploy it, and it stows again only after the guns have been cold with nothing tracked for a few quiet seconds. Deploy is fast and stow is lazy, so a lull in the fight does not park your guns.

The deploy is not free. A stowed mount neither tracks nor fires until it is fully up - under a second, but a real window. Point defense assigns the mount well outside its own kill envelope, so an inbound torpedo meets a gun that is already firing; what the delay actually costs you is the ambush you spring with cold weapons and no lock, where the first trigger pull raises the guns instead of firing them.

<figure class="figure">
    <!-- Capture: assets/wiki-section-turret-twin.png (producer: screenshot_section_weapons) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-turret-twin.png</span
        >
        <span class="figure__placeholder-note"
            >The twin mount deployed over its open housing,
            both barrels reading side by side.</span
        >
    </div>
</figure>

A beaten mount cracks and, past about a third of its health gone, throws sparks - but it loses nothing of itself and it shoots exactly as well as it did new. A turret that had been eaten away would be answering "how is that still firing?" with "it is not, really", and it is. It stops when it dies, and not before.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-turret.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-turret.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: a turret slewing onto a crossing
            target and holding a tracer burst on the lead
            point.</span
        >
    </div>
</figure>

## Variants

Four turrets ship on two mounts. The gatlings put one barrel on the compact assembly every craft carries; the twins put two barrels on a broader one. A twin's tubes each fire at half the gatling's cadence, so the pair costs the same total rate and drains the magazine no faster - the trade is two offset streams instead of one dense one. Within each mount the only thing that separates the pair is the round it loads. The Pierce guns deal half the damage per hit, so mount a Kinetic and a Pierce and the punch-versus-rake trade on [Combat](../../combat-weapons/#damage-types) is the only thing you are feeling.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: shared joint tree turret_joint_tree :199 (yaw unlimited :221-222, pitch -10deg to +90deg :291-292, slew PI :274,:284); pdc_turret_prototype :414 (health TURRET_BASE_HEALTH 130 :429,:32, muzzle 100 :473, lifetime 2.0 :480, magazine 500 :486, reload 3.0s/200 :487-489) with call sites :791-839 - gatlings on gatling_art :142 at GATLING_FIRE_RATE 100 :67, twins on twin_art :162 (two muzzles at x +-0.12) at TWIN_FIRE_RATE = half per muzzle :75; damage kinetic 4.0 :52, pierce 2.0 :62. Every craft mounts the kinetic gatling: base hulls seat it on a face, and a mod's modelled craft name it at their mount points. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-turret.png</span></span></span><span class="catalog__title">Turret - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Damage</th><th>Type</th><th>Fire rate</th><th>Magazine</th><th>Recharge</th><th>Muzzle</th><th>Reach</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-kinetic-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">1,000 m/s</td><td class="catalog__num">2 km</td><td class="catalog__num">130</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-pierce-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">2.0</td><td>Pierce</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">1,000 m/s</td><td class="catalog__num">2 km</td><td class="catalog__num">130</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-twin-kinetic-turret-section.png</span></span></span></td><td><span class="catalog__name">Twin PDC Turret (Kinetic)</span><span class="catalog__id">pdc_twin_kinetic_turret_section</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">2 x 50/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">1,000 m/s</td><td class="catalog__num">2 km</td><td class="catalog__num">130</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-twin-pierce-turret-section.png</span></span></span></td><td><span class="catalog__name">Twin PDC Turret (Pierce)</span><span class="catalog__id">pdc_twin_pierce_turret_section</span></td><td class="catalog__num">2.0</td><td>Pierce</td><td class="catalog__num">2 x 50/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">1,000 m/s</td><td class="catalog__num">2 km</td><td class="catalog__num">130</td></tr>
</tbody>
</table>
</div>

<!-- raider mount health: 60, a SetHealth modification on the mount rather than a separate prototype -->
Raider hulls mount the same guns from the same catalog. The scavenger grade only lowers the mount's health to 60 (against 130 on a player hull), so an enemy's guns are quicker to shoot off - but every round they land hits exactly as hard as yours.
