# Thruster

<figure class="figure">
    <!-- Capture: assets/icon-thruster.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
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

The thruster produces **forward thrust** and drives the exhaust plume. Throttle is analog, so a build's thrust authority comes from each thruster's magnitude and how many you fit - more thrusters, more push. What that push is _worth_ is the other half of it: a section weighs the box it is hit on, so a roomy hull is a slow one and no part of it says so anywhere. The skin plates that clad every base hull add weight of their own, so a clad ship is heavier than its sections alone.

<!-- Stats verified against crates/nova_authoring/src/base_content/sections/thruster.rs (basic_thruster_section magnitude 1.0, no authored collider so the unit cube default applies) and crates/nova_ship/src/sections/ (thruster magnitude is an IMPULSE PER FIXED TICK handed to avian with no dt, thruster_section.rs:484-497 and :620, at Bevy's own 64 Hz :497; density 1 and not authorable, base_section.rs:559; skin plates at SKIN_DENSITY 0.25, crates/nova_ship/src/sections/shell_skin.rs:96, on every base hull, crates/nova_authoring/src/base_content/ships/mod.rs:303, and inside the ComputedMass flight divides by, crates/nova_ship/src/flight/authority.rs:153; unit-cube collider default base_section.rs:79-85). Structure masses are the summed volumes of each ship's own authored cells, skin excluded, in assets/base/ships/base.content.ron: training_picket (21 sections: 16 light cells, 1 computer, 2 basic drives, 1 PDC mount, 1 docking port; mass 20.125), mining_tug (28: 19 reinforced, 2 truss, 1 computer, 1 mining beam, 1 cargo intake 3x2x1, 3 basic drives, 1 docking port; mass 33) and interceptor (38: 30 reinforced, 1 computer, 2 PDC mounts, 1 cargo intake 3x2x1, 3 basic drives, 1 docking port; mass 41.25). No three base hulls share a drive fit: the tug and the interceptor fly the SAME three basic drives, so only what sits behind the drives separates their curves, and the picket flies two. A thruster's magnitude is an engine impulse, so the widget's m/s^2 crosses back through METERS_PER_UNIT 10.0, crates/nova_events/src/units.rs:32. -->

<div class="widget" data-widget="thruster-mass">
<p>Every drive here is a basic drive pushing the same 1.0. The Mining Tug and the Interceptor both fly on three of them. The tug's structure weighs 33.00, good for at most 58 m/s^2; the interceptor's weighs 41.25, good for at most 47 m/s^2, on the same three engines. The Training Picket has one drive fewer on a 20.13 frame: 64 m/s^2. Nothing authors that gap - a section's mass is the volume of the box it is hit on. These are upper bounds: the scope counts structure only, and the skin plates on a base hull make it heavier and slower. Bolting basic drives on closes the gap, with a tapering return: each one is a unit of mass as well as a unit of push, and no stack of them passes 640 m/s^2 - what a single drive would do carrying only itself.</p>
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
            >Loop capture</span
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

Three drive grades ship, and they are priced by the space they take: the basic drive is a unit cube pushing 1.0, the vector drive is a 3x3x2 block pushing 9.0, and the capital drive is 5x5x3 pushing 75.0. Thrust per unit of mass is 1.0, then 0.5, then 1.0 again: the vector drive buys a hull fewer mounts to defend and a shorter drive bay, and the capital drive moves the largest hulls on one bell. All three bolt on by their forward face only, plume clear of the ship.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/thruster.rs (basic_thruster_section health THRUSTER_BASE_HEALTH, magnitude 1.0, single forward socket, no authored collider so the unit cube; vector_thruster_section 3x3x2, health 480, magnitude 9.0 through large_thruster_prototype, which authors the cuboid collider; capital_thruster_section 5x5x3, health 1250, magnitude 75.0). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-thruster.png</span></span></span><span class="catalog__title">Thruster - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Box</th><th>Thrust</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-basic-thruster-section.png</span></span></span></td><td><span class="catalog__name">Basic Thruster Section</span><span class="catalog__id">basic_thruster_section</span></td><td class="catalog__num">1x1x1</td><td class="catalog__num">1.0</td><td class="catalog__num">70</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-vector-thruster-section.png</span></span></span></td><td><span class="catalog__name">Vector Thruster Section</span><span class="catalog__id">vector_thruster_section</span></td><td class="catalog__num">3x3x2</td><td class="catalog__num">9.0</td><td class="catalog__num">480</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-capital-thruster-section.png</span></span></span></td><td><span class="catalog__name">Capital Thruster Section</span><span class="catalog__id">capital_thruster_section</span></td><td class="catalog__num">5x5x3</td><td class="catalog__num">75.0</td><td class="catalog__num">1250</td></tr>
</tbody>
</table>
</div>

