# Torpedo bay

<figure class="figure">
    <!-- Capture: assets/icon-torpedo-bay.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-torpedo-bay.png</span
        >
        <span class="figure__placeholder-note"
            >A torpedo launching from the bay, guidance
            trail curving onto a target.</span
        >
    </div>
</figure>

The torpedo bay fires **guided torpedoes**: they home on the combat lock and deal **blast (area-of-effect) damage** on detonation, so where turret fire is precise and pointed, torpedoes are about zoning and catching clustered or fragile targets. A torpedo is a section like any other, so it inherits the same physics and mounting rules as the rest of the build. The bay itself cracks and sparks as it is worn down, and keeps launching until it dies.

## Shut until it fires

<!-- Behavior verified against crates/nova_ship/src/sections/torpedo_section/bay.rs (a launch is refused while the MuzzleDoor cue is short of 1.0 :207-217; drive_muzzle_doors opens only on a held trigger with the safety off and rounds racked :539-568; a launch pins the door open for ignition_delay + MUZZLE_DOOR_LINGER = 0.9 s :487-517) and the authored track in crates/nova_authoring/src/base_content/sections/standard.rs (open 0.25 s, close 0.7 s) plus the 1x1x2 footprint and sockets there (:975-978, collider :319-359 path). -->

The tube fills two cells of the build - twice a unit section's volume, and twice the mass - and its muzzle is sealed by a six-petal iris that opens for a launch and for nothing else. Holding the trigger folds the petals out and the ejection waits for them: a quarter second on the first shot, then the rest of the salvo leaves on cadence through the held-open door. Release the trigger and it winks shut again.

The iris is honest. A safed bay keeps it shut and so does an empty one, so an open iris on another hull is never decoration - something is about to come out of it.

## How a torpedo runs in

**A torpedo is dropped, not fired.** The bay kicks it clear on a cold charge and the drive catches about six tenths of a second later, a few body lengths out - lighting a torch inside the hull that launched it is how a ship kills itself. For that window the torpedo is cargo: no thrust, no guidance, no fuze, and nothing that can be hit. It cannot be shot down, and it cannot touch the ship it is leaving.

**Then it homes.** Once the drive catches, the torpedo steers on the combat lock with **proportional-navigation** guidance - turning toward where the target will be - after an arming gate clears: a short time or distance from launch, so it cannot go off in your lap. It curves through [gravity wells](../../gravity-wells/) like everything else, and from that moment it is a body the other side can shoot at. Not cheaply: a warhead carries more hit points than the hardest single PDC round can deliver, so an intercept costs a short burst rather than one lucky tap, and the siege bay's armoured torpedoes take sustained fire across the whole closing window. How a defending battery picks and holds its torpedoes is on the [Turret](../turret/#point-defense) page.

<!-- projectile_health 10.0, sized above the hardest single PDC round (4.0 authored x the 2.0 Kinetic speed ceiling): crates/nova_authoring/src/base_content/sections/standard.rs:583 with the reasoning at :580-582; siege bay 5000.0 at :509. -->

**The warhead bursts just before contact.** Against a locked ship or rock the fuze fires about thirty meters from the nearest part of that body's skin. That near-contact margin keeps the physical torpedo from becoming a dud on the hull, while the warhead still delivers almost all its rated pressure and puts the crater on the target. Fire one at a bare point in space instead - a scripted volley, or a torpedo whose target dies mid-flight - and it bursts a half-radius short, because there is no surface left to reach.

## The two run-ins

A bay is loaded with a **torpedo type**, and the editor offers two: **Torpedo Bay (Serpent)** and **Torpedo Bay (Lance)**. Same tube, same warhead, same blast, same six-round rack. The only thing that differs is the run-in, and each flies in its own colour so what is inbound is readable at a glance.

**Serpent** - the assault torpedo. Once armed it runs in on a **terminal weave**: a slow corkscrew laid over the guidance solution rather than instead of it. Point defense fires on a lead solution - where the torpedo will be when the round gets there - and a weaving torpedo is never quite there, so the defender spends roughly three times the ammunition on the same intercept and only kills it on the doorstep. The weave fades out on the final approach, and the torpedo arrives dead on the aim point - which is also the defender's cleanest shot at it. It pays for that with **speed**: the Serpent cruises slower, so it takes noticeably longer to arrive and gains ground on a fleeing target far more slowly.

**Lance** - the bombardment torpedo. No weave at all: the bare intercept, flown straight, and the faster cruise of the two. It is the cheaper one to shoot down and does not pretend otherwise, which is exactly why it is what you fire at something that cannot answer: it gets there sooner, and against a ship running away it closes half again as fast as a Serpent. A defender meeting Lances is a defender whose point defense works.

<!-- Every figure in the widget is the measured table at the head of crates/nova_authoring/src/base_content/sections/ordnance.rs:13-21 (cruise caps, weave half-angles, rounds one PDC spends, where it kills each, time over a 300 world unit (3 km) run-in, speed along the line), plus the closing rates against a runner at 25 world units per second (250 m/s) at ordnance.rs:23-25. Authored sources: Lance LANCE_MAX_SPEED 35.0 ordnance.rs:49 with weave 0.0 :72-73; Serpent max_speed 32.0 / weave_angle 0.44 / weave_rate 1.4 at crates/nova_ship/src/sections/torpedo_section/mod.rs:347-349; the 11.1 world unit (111 m) swing at that angle and rate is measured in the weave_rate doc, mod.rs:293-303. Weave taper: full beyond 3x blast radius, zero at 0.5x, crates/nova_ship/src/sections/torpedo_section/projectile.rs:330-334. -->

<div class="widget" data-widget="torpedo-run">
<p>Over a 3 km run-in a Lance arrives in 9.10 seconds and a Serpent in 9.78 - the weave is paid for in cruise speed, 320 m/s against 350. Against one stock point-defense mount that price buys a great deal: the PDC spends 116 rounds on the Lance and kills it 1.14 km short of its target, then spends 390 on the Serpent and only catches it 400 m out, on the doorstep. The corkscrew tapers to nothing over the last stretch, so a Serpent that survives still arrives dead on the aim point.</p>
</div>

So the Serpent is what you fire at something that shoots back, and the Lance at something that cannot - against a target running at the player's speed cap the Lance closes half again as fast. Which type is in the tubes is the difference between a salvo that costs a defender a magazine and one that costs it four. The campaign uses that on purpose: the first gunship that fires torpedoes at you fires Lances, and the flagship at the end fires Serpents.

<figure class="figure">
    <!-- Capture: assets/wiki-combat-torpedo.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-combat-torpedo.png</span
        >
        <span class="figure__placeholder-note"
            >A salvo in flight: launch burst at the bay,
            drive plumes curving onto the lock.</span
        >
    </div>
</figure>

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-torpedo-bay.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-torpedo-bay.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: the straight Lance and weaving
            Serpent draw their distinct paths across the same
            firing lane.</span
        >
    </div>
</figure>

## What a warhead does to a hull

A torpedo blast reaches every collider inside its visible radius. Distance from the blast to that collider's centre sets the free pressure: full at the centre, falling linearly to zero at the edge.

Ship structure then gives that pressure something to travel through. For each section, the game traces a line from the blast centre. A section that survives its incoming pressure stops the wave and shields every section behind it. A section the wave destroys lets 65 percent continue. Existing holes cost nothing, so repeated hits drive a breach deeper into the same place rather than spreading damage evenly over the hull.

A warhead is charged for the material it really destroys, and it marks the hull **once** however many sections it reached. Two torpedoes on one flank are two wounds; one torpedo over forty sections is still one.

Only actual ship sections consume penetration. Cladding and fixtures still take pressure, and sections can shield them, but a thin decorative plate does not attenuate pressure travelling toward something behind it. Several warheads detonating in one fixed tick see the same initial structure: they can combine to remove the outer layer, but cannot use the new hole until the next tick.

<div class="widget" data-widget="blast-layers">
<p>Worked example: the standard torpedo warhead (750 blast, 300 m radius) against three light hull layers at 100, 120 and 140 m destroys all three and still puts about 96 hp into a section at 160 m. Drop the blast to 200 and the third layer holds, shielding everything behind it.</p>
</div>

The scope spaces its layers out so you can read the falloff. A real torpedo goes off **against the skin**, so the outer sections of the ship it hits sit at the sharp end of that curve, not out at 100 m.

<figure class="figure">
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/torpedo-blast.webm</span
        >
        <span class="figure__placeholder-note"
            >The real thing: a Serpent's detonation blowing the
            outer layers off a corvette.</span
        >
    </div>
</figure>

The result scales through geometry rather than a ship-size bonus. A thin small craft can still be gutted by one direct hit. A deep capital loses a local bite - the sections around the impact, not the ship - and keeps fighting until later hits open it farther or push it below structural collapse. A section destroyed at any structural depth leaves a real hole. If that cut disconnects the graph, the controller-bearing hull keeps ship identity and healthy detached components drift away as inert, damageable wrecks.

<figure class="figure">
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/spine-cut.webm</span
        >
        <span class="figure__placeholder-note"
            >A cut that disconnects the graph: the engine block
            severs and drifts free as a wreck.</span
        >
    </div>
</figure>

## Variants

The two editor bays share every number but the torpedo type; the capital-grade siege bay is scene dressing that never reaches the gallery. Ordnance hp is what point defense has to shoot through per torpedo.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: torpedo_bay_prototype :959 (health :972, blast radius 30 :1014, blast damage 750 :1015, projectile_health 10 :1032, rack 6 :1040, rearm 10s/1 :1051-1053) with Serpent/Lance call sites :842-861; heavy_torpedo_section :864 (hide_in_editor :879, radius 45 :911, blast 2000 :912, projectile_health 5000 :920, no magazine :922-923, Breaker). Torpedo types: sections/ordnance.rs lance :62-75 (cruise 35 :68, no weave :72-73), breaker :95-108 (cruise 70 :104); Serpent defaults crates/nova_ship/src/sections/torpedo_section/mod.rs:341-351 (cruise 32 :347, weave :348-349). CargoB pods: ships/cargo_b.rs:37,:47 (health 350), torpedo kind ships/shared.rs:259-301, _lance variants shared.rs:388-400. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-torpedo-bay.png</span></span></span><span class="catalog__title">Torpedo bay - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Torpedo</th><th>Cruise</th><th>Blast</th><th>Radius</th><th>Rack</th><th>Rearm</th><th>Ordnance hp</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Torpedo Bay (Serpent)</span><span class="catalog__id">torpedo_section</span></td><td>Serpent (weaves)</td><td class="catalog__num">320 m/s</td><td class="catalog__num">750</td><td class="catalog__num">300 m</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">100</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-lance-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Torpedo Bay (Lance)</span><span class="catalog__id">lance_torpedo_section</span></td><td>Lance (straight)</td><td class="catalog__num">350 m/s</td><td class="catalog__num">750</td><td class="catalog__num">300 m</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">100</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-heavy-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Siege Torpedo Bay Section<span class="catalog__flag">hidden</span></span><span class="catalog__id">heavy_torpedo_section</span></td><td>Breaker (weaves)</td><td class="catalog__num">700 m/s</td><td class="catalog__num">2000</td><td class="catalog__num">450 m</td><td class="catalog__num">no magazine</td><td class="catalog__num">-</td><td class="catalog__num">5000</td><td class="catalog__num">100</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-pod.png</span></span></span></td><td><span class="catalog__name">CargoB // Pod</span><span class="catalog__id">cargob_pod_port + cargob_pod_starboard</span></td><td>Serpent (weaves)</td><td class="catalog__num">320 m/s</td><td class="catalog__num">750</td><td class="catalog__num">300 m</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">350</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-pod-lance.png</span></span></span></td><td><span class="catalog__name">CargoB // Pod (Lance)</span><span class="catalog__id">cargob_pod_port_lance + cargob_pod_starboard_lance</span></td><td>Lance (straight)</td><td class="catalog__num">350 m/s</td><td class="catalog__num">750</td><td class="catalog__num">300 m</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">350</td></tr>
</tbody>
</table>
</div>
