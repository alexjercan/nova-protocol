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

<figure class="figure">
    <!-- Capture: assets/loop-section-hull.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loop-section-hull.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: turret fire chewing through hull
            sections on one side while the far side stays
            intact.</span
        >
    </div>
</figure>

## Variants

Two unit-cell hulls ship, and the semantic craft parts - noses, tails, wings and pods - are hulls too: passive structure with a health rating and nothing else. Every shipped section masses 1.0, so health is the number that separates them.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (reinforced_hull_section mass :291, health :292; light_hull_section mass :431, health :434) and crates/nova_authoring/src/base_content/ships/ (semantic-part mass shared.rs:204; racer.rs:41,:51 wings 180, :61 nose 120, :71 tail 120; cargo_a.rs:44,:54 pods 350, :64 nose 180, :74 tail 150; cargo_b.rs:57 nose 180, :67 tail 150). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-hull.png</span></span></span><span class="catalog__title">Hull - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Health</th><th>Mass</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-reinforced-hull-section.png</span></span></span></td><td><span class="catalog__name">Reinforced Hull Section</span><span class="catalog__id">reinforced_hull_section</span></td><td class="catalog__num">200</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-light-hull-section.png</span></span></span></td><td><span class="catalog__name">Light Hull Section</span><span class="catalog__id">light_hull_section</span></td><td class="catalog__num">60</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-wing.png</span></span></span></td><td><span class="catalog__name">Racer // Wing</span><span class="catalog__id">racer_wing_port + racer_wing_starboard</span></td><td class="catalog__num">180</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-nose.png</span></span></span></td><td><span class="catalog__name">Racer // Nose</span><span class="catalog__id">racer_nose</span></td><td class="catalog__num">120</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-tail.png</span></span></span></td><td><span class="catalog__name">Racer // Tail</span><span class="catalog__id">racer_tail</span></td><td class="catalog__num">120</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-pod.png</span></span></span></td><td><span class="catalog__name">CargoA // Pod</span><span class="catalog__id">cargoa_pod_port + cargoa_pod_starboard</span></td><td class="catalog__num">350</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-nose.png</span></span></span></td><td><span class="catalog__name">CargoA // Nose</span><span class="catalog__id">cargoa_nose</span></td><td class="catalog__num">180</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-tail.png</span></span></span></td><td><span class="catalog__name">CargoA // Tail</span><span class="catalog__id">cargoa_tail</span></td><td class="catalog__num">150</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-nose.png</span></span></span></td><td><span class="catalog__name">CargoB // Nose</span><span class="catalog__id">cargob_nose</span></td><td class="catalog__num">180</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-tail.png</span></span></span></td><td><span class="catalog__name">CargoB // Tail</span><span class="catalog__id">cargob_tail</span></td><td class="catalog__num">150</td><td class="catalog__num">1.0</td></tr>
</tbody>
</table>
</div>

Enemy-grade ships thin these same parts down with per-ship health modifications rather than separate prototypes.
