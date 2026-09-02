# Ship sections

A ship is an assembly of sections, each with one job. How they hold together, how a hull collapses and what damage looks like is on [Ships & damage](../ships/). This chapter is the parts themselves: pick a section for what it does, how it behaves, its numbers, and what it is like to face one.

<div id="wiki-children"></div>

## The catalog at a glance

The standard unit-cell catalog at a glance - every child page carries the full per-kind stats, plus the per-craft semantic parts (noses, wings, pods, fuselages). A section weighs the space it fills, so the unit cells all weigh the same and health and the kind stat are what separate them - the torpedo bays and the railgun are the exceptions, a two-cell tube and a three-cell spine at two and three times the mass.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: reinforced hull health 200 :584; basic thruster :611; basic controller :687 (max_torque 1501 :704); light hull 60 :732; cargo hull 200 :757; tank hull 200 :778; pdc_turret_prototype :414 (health 130 :429,:32) with gatling call sites :791-815 at 100/s :67 and twin call sites :816-839 at half per muzzle :75; torpedo bay builder :959 with call sites :842-861; heavy_torpedo_section :864. -->
<table>
<thead>
<tr><th>Kind</th><th>Variant</th><th>Health</th><th>Signature stat</th></tr>
</thead>
<tbody>
<tr><td>Hull</td><td><span class="catalog__name">Reinforced Hull Section</span><span class="catalog__id">reinforced_hull_section</span></td><td class="catalog__num">200</td><td>structure only</td></tr>
<tr><td>Hull</td><td><span class="catalog__name">Light Hull Section</span><span class="catalog__id">light_hull_section</span></td><td class="catalog__num">60</td><td>structure only</td></tr>
<tr><td>Hull</td><td><span class="catalog__name">Cargo Hull Section</span><span class="catalog__id">cargo_hull_section</span></td><td class="catalog__num">200</td><td>structure only</td></tr>
<tr><td>Hull</td><td><span class="catalog__name">Tank Hull Section</span><span class="catalog__id">tank_hull_section</span></td><td class="catalog__num">200</td><td>structure only</td></tr>
<tr><td>Controller</td><td><span class="catalog__name">Basic Controller Section</span><span class="catalog__id">basic_controller_section</span></td><td class="catalog__num">100</td><td class="catalog__num">1501 torque</td></tr>
<tr><td>Thruster</td><td><span class="catalog__name">Basic Thruster Section</span><span class="catalog__id">basic_thruster_section</span></td><td class="catalog__num">70</td><td class="catalog__num">1.0 thrust</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">4.0 Kinetic at 100/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">2.0 Pierce at 100/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">Twin PDC Turret (Kinetic)</span><span class="catalog__id">pdc_twin_kinetic_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">4.0 Kinetic at 2 x 50/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">Twin PDC Turret (Pierce)</span><span class="catalog__id">pdc_twin_pierce_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">2.0 Pierce at 2 x 50/s</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Torpedo Bay (Serpent)</span><span class="catalog__id">torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">750 blast / 30 u</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Torpedo Bay (Lance)</span><span class="catalog__id">lance_torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">750 blast / 30 u</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Siege Torpedo Bay Section<span class="catalog__flag">hidden</span></span><span class="catalog__id">heavy_torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">2000 blast / 45 u</td></tr>
<tr><td>Railgun</td><td><span class="catalog__name">Railgun Lance</span><span class="catalog__id">railgun_lance_section</span></td><td class="catalog__num">180</td><td class="catalog__num">300 Pierce / 1800 power</td></tr>
</tbody>
</table>
</div>
