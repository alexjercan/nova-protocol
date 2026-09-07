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

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_thruster_section magnitude 1.0 :642, no authored collider :611-622 so the unit cube default applies) and crates/nova_ship/src/sections/ (thruster magnitude is an IMPULSE PER FIXED TICK handed to avian with no dt, thruster_section.rs:276-295 and :370-373, at Bevy's own 64 Hz :289-292; density 1 and not authorable, base_section.rs:470-471; unit-cube collider default base_section.rs:79-85). Hull masses are the summed volumes of each ship's own authored cells in assets/base/ships/base.content.ron: block_skiff (18 light cells, 1 computer, 2 drives), block_cutter (23 reinforced, 1, 2) and block_tug (38 light, 1, 2) - the three base hulls that fly the SAME drive fit, so only their plate count separates the curves. A thruster's magnitude is an engine impulse, so the widget's m/s^2 crosses back through METERS_PER_UNIT 10.0, crates/nova_events/src/units.rs:32. -->

<div class="widget" data-widget="thruster-mass">
<p>Every drive here pushes with the same 1.0, and all three of these salvage hulls fly on exactly two of them. The skiff weighs 21.00 and pulls 61 m/s^2; the tug weighs 41.00 and pulls 31 m/s^2, on the same pair of engines. Nothing authors that gap - a section's mass is exactly the volume of the box it is hit on, so it is the plate count and nothing else. Bolting basic drives on closes it, with a tapering return: each one is a unit of mass as well as a unit of push, and no stack of them passes 640 m/s^2 - what a single drive would do carrying only itself.</p>
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

Three drive grades ship, and they are priced by the space they take: the basic drive is a unit cube pushing 1.0, the vector drive is a 3x3x2 block pushing 9.0, and the capital drive is 5x5x3 pushing 25.0. Thrust per unit of mass FALLS as they grow - 1.0, then 0.5, then 0.33 - so a large drive buys a large hull fewer mounts to defend and a shorter drive bay, not a better deal. All three bolt on by their forward face only, plume clear of the ship.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_thruster_section health THRUSTER_BASE_HEALTH :622, magnitude 1.0 :642, single forward socket :630-638, no authored collider so the unit cube; vector_thruster_section 3x3x2, health 480, magnitude 9.0 :663-667 through large_thruster_prototype :542 which authors the cuboid collider :551-553; capital_thruster_section 5x5x3, health 1250, magnitude 25.0 :677-691). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-thruster.png</span></span></span><span class="catalog__title">Thruster - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Box</th><th>Thrust</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-basic-thruster-section.png</span></span></span></td><td><span class="catalog__name">Basic Thruster Section</span><span class="catalog__id">basic_thruster_section</span></td><td class="catalog__num">1x1x1</td><td class="catalog__num">1.0</td><td class="catalog__num">70</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-vector-thruster-section.png</span></span></span></td><td><span class="catalog__name">Vector Thruster Section</span><span class="catalog__id">vector_thruster_section</span></td><td class="catalog__num">3x3x2</td><td class="catalog__num">9.0</td><td class="catalog__num">480</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-capital-thruster-section.png</span></span></span></td><td><span class="catalog__name">Capital Thruster Section</span><span class="catalog__id">capital_thruster_section</span></td><td class="catalog__num">5x5x3</td><td class="catalog__num">25.0</td><td class="catalog__num">1250</td></tr>
</tbody>
</table>
</div>

