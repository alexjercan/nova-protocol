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

Four unit-cell hulls ship, and the modelled craft parts a mod brings - noses, tails, wings and pods - are hulls too: passive structure with a health rating and nothing else. The cargo and tank cells match the reinforced cell stat for stat today; what they buy is a hull that reads as what it carries. A hull weighs the space it fills, exactly: a section's mass is the volume of the box it is hit on, and no part is denser than another. Health is authored; the mass carrying it is not.

That is why the health column below is not the order you want when you are choosing what to bolt on.

<!-- Stats verified against crates/nova_ship/src/sections/base_section.rs (density 1 and not authorable, so mass IS the authored box :376; the unit-cube collider a part with no authored collider falls back to :79-85) and the authored boxes themselves: crates/nova_authoring/src/base_content/sections/standard.rs (reinforced_hull_section health 200 :584; light_hull_section health 60 :732; cargo_hull_section and tank_hull_section health 200 :757,:778 - all four with no authored collider, so the unit-cube fallback) and the modelled craft parts in webmods/the-ledger/ledger_sections.content.ron (the racer, cargoa and cargob hull prototypes, which The Ledger brings with it). -->

<div class="widget" data-widget="hull-armour">
<p>Ranked by health, The Ledger's CargoA nose (180) looks like better armour than its Racer tail (120). It is 2.50 of mass against the tail's 0.88, so per unit of mass it is 72 against 136 - barely half as good. Across the twelve hull parts here the spread runs from the CargoA pod at 216 per mass down to the scavenger-grade light cell at 60, a factor of 3.6 that the health column alone never shows.</p>
</div>

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (reinforced :584, light :732, cargo :757, tank :778) and webmods/the-ledger/ledger_sections.content.ron (racer wings 180, nose 120, tail 120; cargoa pods 350, nose 180, tail 150; cargob nose 180, tail 150). -->
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
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-wing.png</span></span></span></td><td><span class="catalog__name">Racer // Wing</span><span class="catalog__id">racer_wing_port + racer_wing_starboard</span></td><td class="catalog__num">180</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-nose.png</span></span></span></td><td><span class="catalog__name">Racer // Nose</span><span class="catalog__id">racer_nose</span></td><td class="catalog__num">120</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-tail.png</span></span></span></td><td><span class="catalog__name">Racer // Tail</span><span class="catalog__id">racer_tail</span></td><td class="catalog__num">120</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-pod.png</span></span></span></td><td><span class="catalog__name">CargoA // Pod</span><span class="catalog__id">cargoa_pod_port + cargoa_pod_starboard</span></td><td class="catalog__num">350</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-nose.png</span></span></span></td><td><span class="catalog__name">CargoA // Nose</span><span class="catalog__id">cargoa_nose</span></td><td class="catalog__num">180</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-tail.png</span></span></span></td><td><span class="catalog__name">CargoA // Tail</span><span class="catalog__id">cargoa_tail</span></td><td class="catalog__num">150</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-nose.png</span></span></span></td><td><span class="catalog__name">CargoB // Nose</span><span class="catalog__id">cargob_nose</span></td><td class="catalog__num">180</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-tail.png</span></span></span></td><td><span class="catalog__name">CargoB // Tail</span><span class="catalog__id">cargob_tail</span></td><td class="catalog__num">150</td></tr>
</tbody>
</table>
</div>

The four cells are base content. The craft rows below them are The Ledger's: a mod that brings modelled craft brings their part prototypes with it, and base references none of them.

Enemy-grade ships thin these same parts down with per-ship health modifications rather than separate prototypes.
