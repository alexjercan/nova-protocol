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

A turret is an **articulated mount** - a base, the joints that swivel and elevate it, and one or more barrels - that aims at the current combat lock with true intercept lead and fires bullet projectiles. Its coverage is bounded by its yaw and pitch limits, and its output by its fire rate; a mount with several muzzles aims and fires all of them at once.

Turrets draw their aim from the combat lock and prefer a fine-locked section if you have one, falling back to live structure and then the camera ray.

<figure class="figure">
    <!-- Capture: assets/loop-section-turret.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loop-section-turret.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: a turret slewing onto a crossing
            target and holding a tracer burst on the lead
            point.</span
        >
    </div>
</figure>

## Variants

Every shipped gun rides the same mount: yaw unlimited, pitch from 10 degrees below level to 90 above, slewing at 180 deg/s, one muzzle. Reach is muzzle speed times projectile lifetime. There are two turrets, and they differ only in the round they load - the ten per-craft `*_turret_*` modules were the same PDC on the same joint tree and have been removed; every craft mounts the shared one.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: shared mount turret_joint_tree :111-189 (yaw unlimited :142-143, pitch -10deg :157 to +90deg :158, slew PI :141); pdc_*_turret_section builder :215-290 (mass :228, health :229, fire rate 100 :259, muzzle 100 :270, lifetime 2.0 :277, magazine 500 :283, reload 3.0s/200 :285-286) with kinds and damage at :406-426 (kinetic 4.0 :414 via :45, pierce 2.0 :425 via :55). Every craft mounts the kinetic one: ships/shared.rs `module` and `placement`. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-turret.png</span></span></span><span class="catalog__title">Turret - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Damage</th><th>Type</th><th>Fire rate</th><th>Magazine</th><th>Recharge</th><th>Muzzle</th><th>Reach</th><th>Health</th><th>Mass</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-kinetic-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-pierce-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">2.0</td><td>Pierce</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td><td class="catalog__num">1.0</td></tr>
</tbody>
</table>
</div>

<!-- raider mount health: nova_authoring ships/shared.rs ENEMY_TURRET_HEALTH = 60 and the ShipGrade::Enemy SetHealth pass -->
Raider hulls mount the same two guns. The scavenger grade only lowers the mount's health to 60 (against 130 on a player hull), so an enemy's guns are quicker to shoot off - but every round they land hits exactly as hard as yours.
