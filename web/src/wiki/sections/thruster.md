# Thruster

<figure class="figure">
    <!-- Capture: assets/icon-thruster.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-thruster.png</span
        >
        <span class="figure__placeholder-note"
            >A thruster burning, exhaust plume lit, ideally
            on a multi-thruster build.</span
        >
    </div>
</figure>

The thruster produces **forward thrust** and drives the exhaust plume. Throttle is analog, so a build's thrust authority comes from each thruster's magnitude and how many you fit - more thrusters, more push.

A thruster bolts on by its forward end and by that end only. The rest of it is barrel, nozzle bell and exhaust, so the builder always seats it nose-in with the plume clear of the ship - you choose the face it grows from, not which way round it sits.

The flight computer balances thrust through the ship's live center of mass, recruiting off-axis thrusters for counter-torque, so an asymmetric or battle-damaged thruster layout still flies straight instead of pinwheeling.

A hurt drive **looks** hurt. Past about a third of its health gone it cracks and throws sparks, and its exhaust runs short and guttering instead of steady. It never guts all the way out, because a dead plume means a shut-down drive and this one is not shut down: a damaged thruster delivers exactly the push a fresh one does. The plume tells you what a chaser has already taken off it, not what it can still do.

<figure class="figure">
    <!-- Capture: assets/loop-section-thruster.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loop-section-thruster.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: throttle rolling up, plumes
            brightening, the ship pulling away.</span
        >
    </div>
</figure>

## Variants

Every shipped drive pushes with the same 1.0 magnitude - a build's thrust authority comes from how many you fit, not which. All bolt on by their forward face only, plume clear of the ship.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_thruster_section mass :309, health :312, thrust magnitude 1.0 :332, single forward socket :324-328) and crates/nova_authoring/src/base_content/ships/ (semantic-part mass shared.rs:204, thruster kind magnitude 1.0 shared.rs:227; racer.rs:21,:31 engines 70; cargo_a.rs:24,:34 engines 70; cargo_b.rs:17,:27 engines 70). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-thruster.png</span></span></span><span class="catalog__title">Thruster - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Thrust</th><th>Health</th><th>Mass</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-basic-thruster-section.png</span></span></span></td><td><span class="catalog__name">Basic Thruster Section</span><span class="catalog__id">basic_thruster_section</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-engine.png</span></span></span></td><td><span class="catalog__name">Racer // Engine</span><span class="catalog__id">racer_engine_port + racer_engine_starboard</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-engine.png</span></span></span></td><td><span class="catalog__name">CargoA // Engine</span><span class="catalog__id">cargoa_engine_port + cargoa_engine_starboard</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-engine.png</span></span></span></td><td><span class="catalog__name">CargoB // Engine</span><span class="catalog__id">cargob_engine_port + cargob_engine_starboard</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td><td class="catalog__num">1.0</td></tr>
</tbody>
</table>
</div>
