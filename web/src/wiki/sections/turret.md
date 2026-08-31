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

## What it can bear on

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (turret_joint_tree :111-189: traverse limits None/None :142-143, elevation min -TURRET_DEPRESSION_LIMIT :157 where that constant is PI/18 at :84, elevation max FRAC_PI_2 :158, hinge speed PI rad/s :141,:150; muzzle_speed 100.0 :273 x projectile_lifetime 2.0 :280 = 200 u of reach, and crates/nova_ship/src/sections/turret_section/config.rs:124-126 states that product IS the reach) and crates/nova_ship/src/sections/turret_section/aim.rs (fire gate TURRET_ON_TARGET_RAD = 1.6 / 100 :19,:24,:47, doc'd 0.016 rad / 0.92 deg at :26-27; the reachability test is derived from the elevation hinge alone, arc.rs:46-102). -->

The mount turns all the way round, so nothing is out of reach sideways. What bounds it is the barrel's floor: it stops ten degrees below level, because below that it would be pointing back through the ship it is bolted to.

<div class="widget" data-widget="turret-arc">
<p>Traverse is unlimited and the barrel elevates from 10 degrees below level to straight up, so one mount covers 58.7 percent of the sky and the rest is a blind cone under its own keel. Both hinges turn at 180 deg/s at once, so a swing costs the larger of the two angles rather than their sum - a 90 degree traverse takes half a second, which is 50 rounds the gun does not fire while it is moving. Reach is 200 u: muzzle speed times how long a round lives, not an authored range.</p>
</div>

That blind cone is the whole reason [point defense](../../combat-weapons/#point-defense) is assigned per mount rather than per battery, and the reason a salvo arriving from one side meets fewer guns than one across the beam.

## Stowed between fights

<!-- Behavior verified against crates/nova_ship/src/sections/turret_section/stow.rs (deploy demand: weapons hot OR tracked target OR point-defense assignment, drive_turret_stow; settle STOW_SETTLE_SECONDS = 4.0; aim gate crates/nova_ship/src/sections/turret_section/aim.rs and fire gate firing.rs both skip a non-Deployed mount) and the authored travel times in crates/nova_authoring/src/base_content/sections/standard.rs pdc_stow_tracks (lift 0.9 s down / 0.35 s up, lids 0.5 s / 0.25 s). -->

A mount with nothing to do does not stand in the wind. Out of combat the barrel swings straight up, the assembly sinks into its housing, and two lid halves slide shut over it - the ship at rest reads as at rest. The gun comes back up the moment it is wanted: weapons hot, a live tracking target, or a point-defense assignment all deploy it, and it stows again only after the guns have been cold with nothing tracked for a few quiet seconds. Deploy is fast and stow is lazy, so a lull in the fight does not park your guns.

The deploy is not free. A stowed mount neither tracks nor fires until it is fully up - under a second, but a real window. Point defense assigns the mount well outside its own kill envelope, so an inbound torpedo meets a gun that is already firing; what the delay actually costs you is the ambush you spring with cold weapons and no lock, where the first trigger pull raises the guns instead of firing them.

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

Every craft mounts the same gun. There are two turrets in the catalog, they ride the identical mount, and the only thing that separates them is the round they load.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: shared mount turret_joint_tree :111-189 (yaw unlimited :142-143, pitch -10deg :157 to +90deg :158, slew PI :141); pdc_*_turret_section builder :215-290 (health :229, fire rate 100 :259, muzzle 100 :270, lifetime 2.0 :277, magazine 500 :283, reload 3.0s/200 :285-286) with kinds and damage at :406-426 (kinetic 4.0 :414 via :45, pierce 2.0 :425 via :55). Every craft mounts the kinetic one: ships/shared.rs `module` and `placement`. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-turret.png</span></span></span><span class="catalog__title">Turret - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Damage</th><th>Type</th><th>Fire rate</th><th>Magazine</th><th>Recharge</th><th>Muzzle</th><th>Reach</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-kinetic-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-pierce-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">2.0</td><td>Pierce</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td></tr>
</tbody>
</table>
</div>

<!-- raider mount health: nova_authoring ships/shared.rs ENEMY_TURRET_HEALTH = 60 and the ShipGrade::Enemy SetHealth pass -->
Raider hulls mount the same two guns. The scavenger grade only lowers the mount's health to 60 (against 130 on a player hull), so an enemy's guns are quicker to shoot off - but every round they land hits exactly as hard as yours.
