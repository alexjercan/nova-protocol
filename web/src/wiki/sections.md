# Ship sections

<figure class="figure">
    <!-- Capture: assets/wiki-sections.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-sections.png</span
        >
        <span class="figure__placeholder-note"
            >A built ship with its hull, controller,
            thruster, turret and torpedo-bay sections called
            out - ideally labelled or lightly
            exploded.</span
        >
    </div>
    <figcaption class="figure__caption">
        Every part of a ship is a section with one job,
        and the ship is what they add up to.
    </figcaption>
</figure>

A ship is an assembly of sections, each with one job: hull blocks carry the structure, a controller turns it, thrusters push it, and the turrets, torpedo bays and the railgun fight with it. There is no hidden ship underneath - what a hull can do is exactly what its parts can do, and what it loses when a part is shot off is exactly that part's job. How a whole ship holds together and comes apart is on [Ships & damage](../ships/). This chapter is the parts themselves, one page per kind.

<div id="wiki-children"></div>

## What every section shares

- **It is a body.** A section is its own rigid piece with its own mass, its own health and its own collider. Its mass is exactly the volume of the box it is hit on - nothing authors it - so a ship's mass is its shape, and a cargo hull is heavy because it is big.
- **It lives on a grid.** The build grid is 10 m cells. Most sections fill one; a torpedo tube fills two and the railgun three, and they weigh two and three cells to match. The scopes on these pages count in the same cells.
- **It bolts on through its faces.** Every section offers faces another can attach through, and the builder seats it so its working end points the right way: a hull block takes neighbours on any side, a thruster attaches by its forward face only so the plume is always clear of the ship.
- **It has one job, and keeps doing it.** A part does not degrade as it is hurt. A cracked, sparking drive pushes exactly as hard as a fresh one and a battered turret shoots as straight; what changes is what it looks like, until the moment it dies and the whole part comes off. [What damage looks like](../ships/#what-damage-looks-like) has the vocabulary.
- **Its variants share the kind.** Every kind ships in a handful of prototypes that differ in health and in the one stat the kind is about - the round a turret loads, the torpedo a bay carries, the grade of drive a hull needs. Each page lists its own at the foot.

## Reading a section page

Every page runs the same course: what the part does and the numbers that decide it, how it behaves in a fight, what it looks like as it is worn down, what it is like to face one on an enemy hull, and the shipped variants with their stats. Where a rule is worth playing with, the page carries a scope - the turret's arc, the thruster's mass and pull, the railgun's corridor - with the shipped numbers as the defaults.

## The catalog at a glance

The whole shipped catalog at a glance; every child page carries the full per-kind stats. A section weighs the space it fills, so the unit cells all weigh the same and health and the kind stat are what separate them. What breaks the pattern is the mounts that are not unit cells: a PDC turret is a half-cell cube at an eighth of the mass, a torpedo bay a two-cell tube, a railgun a three-cell spine, and the two large drives are 18 and 75 cells of block.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: reinforced hull health 200 :596; basic thruster 70 at 1.0 :611-622,:642; vector thruster 480 at 9.0 over 3x3x2 :663-667; capital thruster 1250 at 25.0 over 5x5x3 :677-691; basic controller :701 (max_torque 1501 :718); light hull 60 :744-750; cargo hull 200 :769; tank hull 200 :790; pdc_turret_prototype :414 (health 130 :429,:32) with gatling call sites :791-815 at 100/s :67 and twin call sites :816-839 at half per muzzle :75; torpedo bay builder :1144 with call sites :842-861; heavy_torpedo_section :994; both lances from railgun_lance_prototype with their grades at the two call sites in standard_section_prototypes. -->
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
<tr><td>Thruster</td><td><span class="catalog__name">Vector Thruster Section</span><span class="catalog__id">vector_thruster_section</span></td><td class="catalog__num">480</td><td class="catalog__num">9.0 thrust, 3x3x2</td></tr>
<tr><td>Thruster</td><td><span class="catalog__name">Capital Thruster Section</span><span class="catalog__id">capital_thruster_section</span></td><td class="catalog__num">1250</td><td class="catalog__num">25.0 thrust, 5x5x3</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">4.0 Kinetic at 100/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">2.0 Pierce at 100/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">Twin PDC Turret (Kinetic)</span><span class="catalog__id">pdc_twin_kinetic_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">4.0 Kinetic at 2 x 50/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">Twin PDC Turret (Pierce)</span><span class="catalog__id">pdc_twin_pierce_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">2.0 Pierce at 2 x 50/s</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Torpedo Bay (Serpent)</span><span class="catalog__id">torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">750 blast / 300 m</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Torpedo Bay (Lance)</span><span class="catalog__id">lance_torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">750 blast / 300 m</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Siege Torpedo Bay Section<span class="catalog__flag">experimental</span></span><span class="catalog__id">heavy_torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">2000 blast / 450 m</td></tr>
<tr><td>Railgun</td><td><span class="catalog__name">Railgun Lance</span><span class="catalog__id">railgun_lance_section</span></td><td class="catalog__num">180</td><td class="catalog__num">300 Pierce / 1800 power</td></tr>
<tr><td>Railgun</td><td><span class="catalog__name">Siege Railgun Lance<span class="catalog__flag">experimental</span></span><span class="catalog__id">siege_railgun_lance_section</span></td><td class="catalog__num">180</td><td class="catalog__num">500 Pierce / 360,000 power</td></tr>
</tbody>
</table>
</div>
