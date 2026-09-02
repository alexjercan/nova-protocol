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

The thruster produces **forward thrust** and drives the exhaust plume. Throttle is analog, so a build's thrust authority comes from each thruster's magnitude and how many you fit - more thrusters, more push. What that push is _worth_ is the other half of it: a section weighs the box it is hit on, so a roomy hull is a slow one and no part of it says so anywhere.

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_thruster_section magnitude 1.0 :354, no authored collider :337 so the unit cube default applies) and crates/nova_ship/src/sections/ (thruster magnitude is an IMPULSE PER FIXED TICK handed to avian with no dt, thruster_section.rs:276-295 and :370-373, at Bevy's own 64 Hz :289-292; density 1 and not authorable, base_section.rs:376; unit-cube collider default base_section.rs:79-85). Hull masses are the summed volumes of the craft's own authored boxes: crates/nova_authoring/src/base_content/ships/racer.rs:13-88 with the base assembly taking the meshed seven :107-115, cargo_a.rs:16-96, cargo_b.rs:9-82, all built through shared.rs:44-50,:235. A thruster's magnitude is an engine impulse, so the widget's m/s^2 crosses back through METERS_PER_UNIT 10.0, crates/nova_events/src/units.rs:32. -->

<div class="widget" data-widget="thruster-mass">
<p>Every shipped drive pushes with the same 1.0 and all three shipped hulls carry two of them. The civilian yacht weighs 8.28 and pulls 155 m/s^2; the torpedo hauler weighs 18.95 and pulls 68 m/s^2. Nothing authors that gap - a section's mass is exactly the volume of the box it is hit on, so it is the shape of the craft and nothing else. Bolting basic drives on closes it, with a tapering return: each one is a unit of mass as well as a unit of push, and no stack of them passes 640 m/s^2 - what a single drive would do carrying only itself.</p>
</div>

A thruster bolts on by its forward end and by that end only. The rest of it is barrel, nozzle bell and exhaust, so the builder always seats it nose-in with the plume clear of the ship - you choose the face it grows from, not which way round it sits.

## Balancing thrust through the hull

Because thrusters sit wherever you bolted them, an off-center burn would spin the ship. The flight computer prevents that: it sets each engine's throttle to deliver the commanded forward thrust while cancelling the twist through the live center of mass, recruiting off-axis thrusters purely for counter-torque when the firing set cannot balance itself. An asymmetric or battle-damaged ship still flies straight - any tiny leftover spin is mopped up by the steering - so a drive shot off one flank costs you push, not control.

## What a hurt drive tells you

A hurt drive **looks** hurt. Past about a third of its health gone it cracks and throws sparks, and its exhaust runs short and guttering instead of steady. It never guts all the way out, because a dead plume means a shut-down drive and this one is not shut down: a damaged thruster delivers exactly the push a fresh one does. The plume tells you what a chaser has already taken off it, not what it can still do.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-thruster.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-thruster.webm</span
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
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_thruster_section health :312, thrust magnitude 1.0 :332, single forward socket :324-328) and crates/nova_authoring/src/base_content/ships/ (thruster kind magnitude 1.0 shared.rs:227; racer.rs:21,:31 engines 70; cargo_a.rs:24,:34 engines 70; cargo_b.rs:17,:27 engines 70). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-thruster.png</span></span></span><span class="catalog__title">Thruster - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Thrust</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-basic-thruster-section.png</span></span></span></td><td><span class="catalog__name">Basic Thruster Section</span><span class="catalog__id">basic_thruster_section</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-engine.png</span></span></span></td><td><span class="catalog__name">Racer // Engine</span><span class="catalog__id">racer_engine_port + racer_engine_starboard</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-engine.png</span></span></span></td><td><span class="catalog__name">CargoA // Engine</span><span class="catalog__id">cargoa_engine_port + cargoa_engine_starboard</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-engine.png</span></span></span></td><td><span class="catalog__name">CargoB // Engine</span><span class="catalog__id">cargob_engine_port + cargob_engine_starboard</span></td><td class="catalog__num">1.0</td><td class="catalog__num">70</td></tr>
</tbody>
</table>
</div>
