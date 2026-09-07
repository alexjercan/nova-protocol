# Hull

<figure class="figure">
    <!-- Capture: assets/icon-hull.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-hull.png</span
        >
        <span class="figure__placeholder-note"
            >A hull section on a ship, ideally showing where
            other sections bolt onto it.</span
        >
    </div>
</figure>

The hull is the ship's **passive structure and armor**. It provides hitpoints and the connective backbone every other section mounts to. A hull has no active behavior of its own - it is what the rest of the ship is built around, and what keeps taking hits while the working sections keep doing their jobs.

Because health is per-section, a hull soaks damage locally: a hit on one side chews through the sections there while the far side stays intact, so where a shot lands matters, not just whether it landed.

A hull cell keeps the shape it was built in. What it does instead is **crack**: dark fractures spreading as its own health falls, glowing through when it is about to fail, burnt out cold when it dies - and then the whole cell leaves at once. It throws no sparks, because a hull has nothing in it to short out.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-hull.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-hull.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: identical hull cells step from
            clean plating through dark cracks, hot fractures,
            and final failure.</span
        >
    </div>
</figure>

## Variants

Four unit-cell hulls ship. The cargo and tank cells match the reinforced cell stat for stat today; what they buy is a hull that reads as what it carries, and the light cell is the scavenger grade at a third of the health. A hull weighs the space it fills, exactly: a section's mass is the volume of the box it is hit on, and no section is denser than another. Health is authored; the mass carrying it is not.

That is why the health column below is not the order you want when you are choosing what to bolt on - and it is not only a hull question, because everything else you bolt on is weighed the same way.

<!-- Stats verified against crates/nova_ship/src/sections/base_section.rs (density 1 and not authorable, so mass IS the authored box :470-471; the unit-cube collider a section with no authored collider falls back to :79-85) and the authored boxes themselves: crates/nova_authoring/src/base_content/sections/standard.rs (reinforced_hull_section health 200 :596; light_hull_section health 60 :744-750; cargo_hull_section and tank_hull_section health 200 :769,:790 - all four with no authored collider, so the unit-cube fallback; capital_thruster_section 1250 over a 5x5x3 box :677-691; vector_thruster_section 480 over 3x3x2 :663-667; siege_railgun_lance_section 180 over LANCE_CELLS 1x1x3 :321,:923-926; pdc_kinetic_turret_section 130 over a 0.5-cell cube :91,:434,:446-448; heavy_torpedo_section 100 over BAY_CELLS 1x1x2 :318,:952-960; basic_controller_section 100 and basic_thruster_section 70, both unit cubes :611-622,:701). -->

<div class="widget" data-widget="hull-armour">
<p>Ranked by health the Capital Thruster Section (1250) looks like the best armour in the catalog and the PDC Turret Section (130) like the worst. The drive is a 5x5x3 box - 75 of mass - and the mount is a 0.5-cell cube at 0.125, so per unit of mass it is 17 against 1040 and the ranking is exactly backwards. Across the nine prototypes here the spread runs a factor of 62, which the health column alone never shows.</p>
</div>

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (reinforced :596, light :744-750, cargo :769, tank :790). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-hull.png</span></span></span><span class="catalog__title">Hull - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-reinforced-hull-section.png</span></span></span></td><td><span class="catalog__name">Reinforced Hull Section</span><span class="catalog__id">reinforced_hull_section</span></td><td class="catalog__num">200</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-light-hull-section.png</span></span></span></td><td><span class="catalog__name">Light Hull Section</span><span class="catalog__id">light_hull_section</span></td><td class="catalog__num">60</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargo-hull-section.png</span></span></span></td><td><span class="catalog__name">Cargo Hull Section</span><span class="catalog__id">cargo_hull_section</span></td><td class="catalog__num">200</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-tank-hull-section.png</span></span></span></td><td><span class="catalog__name">Tank Hull Section</span><span class="catalog__id">tank_hull_section</span></td><td class="catalog__num">200</td></tr>
</tbody>
</table>
</div>

Enemy-grade ships thin these same cells down with per-ship health modifications rather than separate prototypes.
