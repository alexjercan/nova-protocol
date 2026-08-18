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

Every shipped gun rides the same mount: yaw unlimited, pitch from 10 degrees below level to 90 above, slewing at 180 deg/s, one muzzle. Reach is muzzle speed times projectile lifetime; the ten per-craft `*_turret_*` modules are catalog-only - the same PDC on the same joint tree, kept for mods.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: shared mount turret_joint_tree :98-176 (yaw unlimited :129-130, pitch -10deg :144 to +90deg :145, slew PI :128); pdc_*_turret_section builder :202-279 (mass :214, health :215, fire rate 100 :248, muzzle 100 :259, lifetime 2.0 :266, magazine 500 :272, reload 3.0s/200 :274-275) with kinds and damage at :507-527 (kinetic 4.0 :515 via :44, pierce 2.0 :526 via :54); better_turret_section :377-422 (health :383, fire rate :397, muzzle :401, lifetime :403, damage :408, magazine :417, reload :421-422); light_turret_section :448-503 (health :456, fire rate :472, muzzle :479, lifetime :487, damage 3.825 derived :491, magazine :499, reload :502-503). Per-craft turret modules: ships/shared.rs:303-341 (player/raider stat split) and hide_in_editor shared.rs:214. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-turret.png</span></span></span><span class="catalog__title">Turret - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Damage</th><th>Type</th><th>Fire rate</th><th>Magazine</th><th>Recharge</th><th>Muzzle</th><th>Reach</th><th>Health</th><th>Mass</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-kinetic-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-pdc-pierce-turret-section.png</span></span></span></td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">2.0</td><td>Pierce</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-better-turret-section.png</span></span></span></td><td><span class="catalog__name">Better Turret Section</span><span class="catalog__id">better_turret_section</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-light-turret-section.png</span></span></span></td><td><span class="catalog__name">Light Turret Section</span><span class="catalog__id">light_turret_section</span></td><td class="catalog__num">3.825</td><td>Kinetic</td><td class="catalog__num">25/s</td><td class="catalog__num">150</td><td class="catalog__num">60 / 3 s</td><td class="catalog__num">60 u/s</td><td class="catalog__num">180 u</td><td class="catalog__num">60</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-craft-turret-module.png</span></span></span></td><td><span class="catalog__name">Per-craft mount (player grade)<span class="catalog__flag">catalog-only</span></span><span class="catalog__id">racer/cargoa/cargob *_turret_* (port + starboard)</span></td><td class="catalog__num">4.0</td><td>Kinetic</td><td class="catalog__num">100/s</td><td class="catalog__num">500</td><td class="catalog__num">200 / 3 s</td><td class="catalog__num">100 u/s</td><td class="catalog__num">200 u</td><td class="catalog__num">130</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-craft-turret-module-light.png</span></span></span></td><td><span class="catalog__name">Per-craft mount (raider grade)<span class="catalog__flag">catalog-only</span></span><span class="catalog__id">racer/cargoa *_turret_*_light</span></td><td class="catalog__num">3.825</td><td>Kinetic</td><td class="catalog__num">25/s</td><td class="catalog__num">150</td><td class="catalog__num">60 / 3 s</td><td class="catalog__num">60 u/s</td><td class="catalog__num">180 u</td><td class="catalog__num">60</td><td class="catalog__num">1.0</td></tr>
</tbody>
</table>
</div>

The Light Turret's 3.825 is a derived number - the representative kinetic damage of its 0.05-mass round at its 60 u/s muzzle speed - not a hand-tuned value.
