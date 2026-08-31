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

The torpedo bay fires **guided torpedoes** that home on the combat lock with proportional-navigation guidance and deal **blast (area-of-effect) damage** on detonation. An arming gate keeps a torpedo from going off in your own lap.

A torpedo is a section like any other, so it inherits the same physics and mounting rules as the rest of the build. Where turret fire is precise and pointed, torpedoes are about zoning and catching clustered or fragile targets.

**A torpedo is dropped, not fired.** The bay kicks it clear on a cold charge and the drive catches about six tenths of a second later, a few body lengths out - lighting a torch inside the hull that launched it is how a ship kills itself. For that window the torpedo is cargo: no thrust, no guidance, no fuze, and nothing that can be hit. It cannot be shot down, and it cannot touch the ship it is leaving.

**The warhead bursts just before contact.** Against a locked ship or rock the fuze fires about thirty metres from the nearest part of that body. That clears the physical torpedo while landing almost full pressure on the plating. A torpedo with nothing left to touch - one sent at a bare point in space, or one whose target dies mid-flight - bursts a half-radius out instead.

A blast then works from the outside in. A section that survives shields everything behind it; a destroyed section lets 65 percent of the remaining pressure continue. One hit can gut a thin craft, while a deep capital loses a local breach instead of its whole hull. Later hits travel farther through the opened hole.

The bay itself cracks and sparks as it is worn down, and keeps launching until it dies.

## The two run-ins

A bay is loaded with a **torpedo type**, and the editor offers two: **Torpedo Bay (Serpent)** and **Torpedo Bay (Lance)**. Same tube, same warhead, same blast, same six-round rack. The only thing that differs is the run-in, and each flies in its own colour so what is inbound is readable at a glance.

<!-- Every figure in the widget is the measured table at the head of crates/nova_authoring/src/base_content/sections/ordnance.rs:13-21 (cruise caps, weave half-angles, rounds one PDC spends, where it kills each, time over a 300 u run-in, speed along the line), plus the closing rates against a 25 u/s runner at ordnance.rs:23-25. Authored sources: Lance LANCE_MAX_SPEED 35.0 ordnance.rs:49 with weave 0.0 :72-73; Serpent max_speed 32.0 / weave_angle 0.44 / weave_rate 1.4 at crates/nova_ship/src/sections/torpedo_section/mod.rs:347-349; the 11.1 u swing at that angle and rate is measured in the weave_rate doc, mod.rs:293-303. Weave taper: full beyond 3x blast radius, zero at 0.5x, crates/nova_ship/src/sections/torpedo_section/projectile.rs:330-334. -->

<div class="widget" data-widget="torpedo-run">
<p>Over a 300 u run-in a Lance arrives in 9.10 seconds and a Serpent in 9.78 - the weave is paid for in cruise speed, 32 u/s against 35. Against one stock point-defense mount that price buys a great deal: the PDC spends 116 rounds on the Lance and kills it 114 u short of its target, then spends 390 on the Serpent and only catches it 40 u out, on the doorstep. The corkscrew tapers to nothing over the last stretch, so a Serpent that survives still arrives dead on the aim point.</p>
</div>

So the Serpent is what you fire at something that shoots back, and the Lance at something that cannot - against a target running at the player's speed cap the Lance closes half again as fast. See [Combat & weapons](../../combat-weapons/#torpedoes) for how the two types read in flight.

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

## Variants

The two editor bays share every number but the torpedo type; the capital-grade siege bay is scene dressing that never reaches the gallery. Ordnance hp is what point defense has to shoot through per torpedo.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: torpedo_bay_prototype :614-693 (health :628, blast radius 30 :650, blast damage 750 :658, projectile_health 10 :668, rack 6 :676, rearm 10s/1 :688-689) with Serpent/Lance call sites :528-549; heavy_torpedo_section :550-601 (hide_in_editor :567, blast 2000 :588, radius 45 :587, projectile_health 5000 :596, no magazine :598-599, Breaker :597). Torpedo types: sections/ordnance.rs lance :62-75 (cruise 35 :68, no weave :72-73), breaker :95-108 (cruise 70 :104); Serpent defaults crates/nova_ship/src/sections/torpedo_section/mod.rs:341-351 (cruise 32 :347, weave :348-349). CargoB pods: ships/cargo_b.rs:37,:47 (health 350), torpedo kind ships/shared.rs:259-301, _lance variants shared.rs:388-400. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-torpedo-bay.png</span></span></span><span class="catalog__title">Torpedo bay - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Torpedo</th><th>Cruise</th><th>Blast</th><th>Radius</th><th>Rack</th><th>Rearm</th><th>Ordnance hp</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Torpedo Bay (Serpent)</span><span class="catalog__id">torpedo_section</span></td><td>Serpent (weaves)</td><td class="catalog__num">32 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">100</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-lance-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Torpedo Bay (Lance)</span><span class="catalog__id">lance_torpedo_section</span></td><td>Lance (straight)</td><td class="catalog__num">35 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">100</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-heavy-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Siege Torpedo Bay Section<span class="catalog__flag">hidden</span></span><span class="catalog__id">heavy_torpedo_section</span></td><td>Breaker (weaves)</td><td class="catalog__num">70 u/s</td><td class="catalog__num">2000</td><td class="catalog__num">45 u</td><td class="catalog__num">no magazine</td><td class="catalog__num">-</td><td class="catalog__num">5000</td><td class="catalog__num">100</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-pod.png</span></span></span></td><td><span class="catalog__name">CargoB // Pod</span><span class="catalog__id">cargob_pod_port + cargob_pod_starboard</span></td><td>Serpent (weaves)</td><td class="catalog__num">32 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">350</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-pod-lance.png</span></span></span></td><td><span class="catalog__name">CargoB // Pod (Lance)</span><span class="catalog__id">cargob_pod_port_lance + cargob_pod_starboard_lance</span></td><td>Lance (straight)</td><td class="catalog__num">35 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">350</td></tr>
</tbody>
</table>
</div>
