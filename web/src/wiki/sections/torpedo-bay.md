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

**The warhead touches before it bursts.** Against a locked ship or rock the fuze fires about a metre off the nearest part of that body, so the pressure lands at full strength on the plating rather than short of it. A torpedo with nothing left to touch - one sent at a bare point in space, or one whose target dies mid-flight - bursts a half-radius out instead.

A blast then works from the outside in. A section that survives shields everything behind it; a destroyed section lets 65 percent of the remaining pressure continue. One hit can gut a thin craft, while a deep capital loses a local breach instead of its whole hull. Later hits travel farther through the opened hole.

The bay itself cracks and sparks as it is worn down, and keeps launching until it dies.

A bay is loaded with a **torpedo type**, and the editor offers two: **Torpedo Bay (Serpent)** and **Torpedo Bay (Lance)**. Same tube, same warhead, same blast, same six-round rack - the only difference is the run-in. A Serpent corkscrews, so a defender spends roughly three times the rounds stopping it and only kills it on its own doorstep, and it pays for that by cruising slower. A Lance flies the bare intercept at the faster cruise, arriving sooner on the shortest path there is - and a defender's guns kill it comfortably short of what it was aimed at. Each flies in its own colour, so what is inbound is readable at a glance. See [Combat & weapons](../../combat-weapons/) for the trade in full.

<figure class="figure">
    <!-- Capture: assets/loop-section-torpedo-bay.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loop-section-torpedo-bay.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: a salvo leaving the bay, one
            torpedo weaving through point-defense fire onto
            the lock.</span
        >
    </div>
</figure>

## Variants

The two editor bays share every number but the torpedo type; the capital-grade siege bay is scene dressing that never reaches the gallery. Ordnance hp is what point defense has to shoot through per torpedo.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: torpedo_bay_prototype :614-693 (mass :626, health :628, blast radius 30 :650, blast damage 750 :658, projectile_health 10 :668, rack 6 :676, rearm 10s/1 :688-689) with Serpent/Lance call sites :528-549; heavy_torpedo_section :550-601 (hide_in_editor :567, blast 2000 :588, radius 45 :587, projectile_health 5000 :596, no magazine :598-599, Breaker :597). Torpedo types: sections/ordnance.rs lance :62-75 (cruise 35 :68, no weave :72-73), breaker :95-108 (cruise 70 :104); Serpent defaults crates/nova_ship/src/sections/torpedo_section/mod.rs:341-351 (cruise 32 :347, weave :348-349). CargoB pods: ships/cargo_b.rs:37,:47 (health 350), torpedo kind ships/shared.rs:259-301, _lance variants shared.rs:388-400. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-torpedo-bay.png</span></span></span><span class="catalog__title">Torpedo bay - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Torpedo</th><th>Cruise</th><th>Blast</th><th>Radius</th><th>Rack</th><th>Rearm</th><th>Ordnance hp</th><th>Health</th><th>Mass</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Torpedo Bay (Serpent)</span><span class="catalog__id">torpedo_section</span></td><td>Serpent (weaves)</td><td class="catalog__num">32 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">100</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-lance-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Torpedo Bay (Lance)</span><span class="catalog__id">lance_torpedo_section</span></td><td>Lance (straight)</td><td class="catalog__num">35 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">100</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-heavy-torpedo-section.png</span></span></span></td><td><span class="catalog__name">Siege Torpedo Bay Section<span class="catalog__flag">hidden</span></span><span class="catalog__id">heavy_torpedo_section</span></td><td>Breaker (weaves)</td><td class="catalog__num">70 u/s</td><td class="catalog__num">2000</td><td class="catalog__num">45 u</td><td class="catalog__num">no magazine</td><td class="catalog__num">-</td><td class="catalog__num">5000</td><td class="catalog__num">100</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-pod.png</span></span></span></td><td><span class="catalog__name">CargoB // Pod</span><span class="catalog__id">cargob_pod_port + cargob_pod_starboard</span></td><td>Serpent (weaves)</td><td class="catalog__num">32 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">350</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-pod-lance.png</span></span></span></td><td><span class="catalog__name">CargoB // Pod (Lance)</span><span class="catalog__id">cargob_pod_port_lance + cargob_pod_starboard_lance</span></td><td>Lance (straight)</td><td class="catalog__num">35 u/s</td><td class="catalog__num">750</td><td class="catalog__num">30 u</td><td class="catalog__num">6</td><td class="catalog__num">1 / 10 s</td><td class="catalog__num">10</td><td class="catalog__num">350</td><td class="catalog__num">1.0</td></tr>
</tbody>
</table>
</div>
