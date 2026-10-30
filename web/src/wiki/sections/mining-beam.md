# Mining beam

<figure class="figure">
    <!-- Capture: assets/wiki-section-mining-beam.png (producer: screenshot_mining_beam) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-mining-beam.png</span
        >
        <span class="figure__placeholder-note"
            >The mining tug's bow emitter deployed, its green
            beam running straight out to an ore rock about
            58 m ahead and ending on the rock's face.</span
        >
    </div>
</figure>

A mining beam is a **one-cell emitter** behind two sliding doors. Point it at a rock and hold its key: the doors part, the tip runs out, and once a second the beam cuts the rock where it hits. No travel lock is needed. The ore it cuts free leaves the rock as canisters, and a [cargo intake](../cargo-intake/) takes them into your hold.

The beam does not aim. It runs straight out of the emitter's face, so the **ship** aims it, the way a railgun is aimed. If the ray meets more than one rock, it reaches the nearest one first.

<!-- Values from crates/nova_authoring/src/base_content/sections/mining_beam.rs: health 90 (MINING_BEAM_BASE_HEALTH), TRACK_SECONDS 0.3 per leg, reach 100 m, pulse_interval_seconds 1.0, carve_radius_cells 1.5 of the rock's field, whose cell is FIELD_CELL_WORLD 0.5 units, about 5 m (crates/nova_scenario/src/objects/asteroid_carve.rs). Per-section key: `input_mapping` entry -> `SpaceshipMiningInputBinding` in crates/nova_ship/src/input/player/weapons.rs; the mining tug's `mining_beam` takes V in crates/nova_authoring/src/base_content/scenarios/season_one/stage.rs (Kaveri); the editor default V with no pad button in crates/nova_editor/src/placement.rs `default_binds`. -->

| The mining beam at a glance | |
|---|---|
| Key | its own, held: <kbd>V</kbd> on the mining tug |
| Deploy | doors **0.3 s**, then the tip **0.3 s** |
| Reach | **100 m** from the emitter face, straight ahead of it |
| Rate | a pulse as soon as it is out, then **one a second** |
| Cut | a sphere about **7.5 m** in radius where the beam hits |
| Health | **90** |

<!-- Widget values: crates/nova_authoring/src/base_content/sections/mining_beam.rs:101-103 (reach 100 m, pulse 1 s, carve_radius_cells 1.5); FIELD_CELL_WORLD 0.5 units in crates/nova_scenario/src/objects/asteroid_carve.rs:108 (1.5 cells ~ 7.5 m); CARGO_CANISTER_MAX_MASS_G 1,000 kg in crates/nova_gameplay/src/inventory.rs; MINED_CANISTER_OFFSET 10 m and MINED_CANISTER_SPEED 2 m/s in crates/nova_scenario/src/mining.rs:93,97,895-925; direct nearest-asteroid aim and ore check in MiningRefusalType and aim_beam; a refusal is an info log and the authored refusal_sound at the emitter face in play_mining_pulse_sfx; canisters after the remesh :804-841. -->

<div class="widget" data-widget="mining-beam-checks">
<p>The emitter ray checks the nearest asteroid it intersects within 100 m, with no travel lock required. A barren asteroid in front of an ore rock refuses the pulse rather than letting the beam pass through it. A refused pulse shows nothing on the HUD. A pulse that passes cuts a sphere about 7.5 m in radius where the beam meets solid material; the first pulse into an untouched rock cuts nothing. Ore leaves only after the rock's remesh lands, as canisters of up to 1,000 kg, each born 10 m off the cut and drifting off it at 2 m/s.</p>
</div>

## Cutting ore

<!-- Behavior verified against crates/nova_ship/src/sections/mining_section.rs (hold to deploy, doors then tip, tip then doors on release, reverse mid-travel) and crates/nova_scenario/src/mining.rs (nearest intersected asteroid within reach, refusal on barren rock, one refusal_sound per refused pulse at the emitter face, MiningBeamHit drawn only while every check passes, the first pulse into an untouched rock seeds its field and cuts nothing). -->

1. **Point the emitter at a rock.** The Mining Tug's emitter sits at the tip of its bow boom and looks forward, so put your nose on the rock and close to inside 100 m. A travel lock is optional and does not choose the mining target.
2. **Hold the beam's key.** The doors part and the tip runs out. Once the tip is out, the beam fires at once and then once a second for as long as you hold the key.
3. **Let go.** The tip runs in first, then the doors shut. Pressing or letting go halfway reverses from wherever the parts are.

The emitter ray takes the nearest asteroid it meets within 100 m of its face, then checks that the rock holds ore. A missed ray or barren rock draws no beam and cuts nothing; it plays one refusal sound at the emitter, once a pulse while you hold the key. So **no beam and a refusal sound mean a check is failing** - the nose may be off the rock, the rock too far, or the nearest rock barren. Other bodies do not block this asteroid-only ray.

A pulse that passes draws the beam to the hit, flares it, throws sparks and plays a pulse sound at the rock. The first pulse into a rock nobody has cut yet only readies it and takes nothing; the cut starts with the next pulse. A pulse into a crater an earlier pulse or a weapon already emptied finds nothing to cut and frees no ore, so walk the beam onto fresh rock.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-mining-beam.webm (producer: screenshot_mining_beam) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-mining-beam.webm</span
        >
        <span class="figure__placeholder-note"
            >The bow emitter through a whole cycle: the doors
            part, the tip runs out, the beam pulses through the
            canisters earlier cuts freed and into the
            rock, and on release the tip runs in and the doors
            shut.</span
        >
    </div>
</figure>

## Where the ore goes

<!-- Ore by kind: ore_for_asteroid_kind in crates/nova_scenario/src/mining.rs. One item per field corner cut, paid once the rock's remesh lands; canisters of at most CARGO_CANISTER_MAX_MASS_G 1,000 kg (crates/nova_gameplay/src/inventory.rs), born MINED_CANISTER_OFFSET 1 unit (10 m) off the last cut along its outward normal and drifting at MINED_CANISTER_SPEED 0.2 units/s (2 m/s) plus the rock's own motion there. Ore mass 10 kg each: mass_g in crates/nova_authoring/src/base_content/items.rs `item_catalog`. Open-world rock kinds: ROCK_KINDS in crates/nova_world_base/src/clusters.rs. -->

What a rock is made of decides the ore. Every ore weighs 10 kg, so one canister carries up to 100.

| Rock kind | Ore |
| --- | --- |
| rock | Stone ore |
| metal | Iron ore |
| ice | Water ice |
| carbon | Carbon ore |
| plain | none: the beam does not fire on it |

The ore leaves the rock once the cut is drawn, packed into canisters of up to 1,000 kg. They are born one at a time, 10 m off the last cut and drifting away from it at 2 m/s on top of the rock's own motion, and the next waits until the space there is clear. A rock that is cut away entirely still lets its last canisters go. Fly your [cargo intake](../cargo-intake/#taking-a-canister-in) onto a canister to take it in. A saved Open World keeps a canister as you left it; any other scenario keeps it only for the session.

<figure class="figure">
    <!-- Capture: assets/wiki-section-mining-beam-carve.png (producer: screenshot_mining_beam) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-mining-beam-carve.png</span
        >
        <span class="figure__placeholder-note"
            >The same rock after the beam is released and every
            canister has left it: a crater cut into the face
            where the beam hit.</span
        >
    </div>
</figure>

Every rock in the New Game world holds ore. Ore sells only to a docked ship that trades - see [Buy and sell](../../interface/#buy-and-sell) - and the New Game world has no such ship yet, so for now mined ore stays in your hold.

## Variants

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/mining_beam.rs and assets/base/sections/base.content.ron. Mounted by crates/nova_authoring/src/base_content/ships/block.rs: `mining_beam` at the mining tug's boom tip, cell (0, 0, -6). -->
<div class="catalog__head"><span class="catalog__title">Mining beam - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Reach</th><th>Rate</th><th>Cut</th><th>Deploy</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-mining-beam-section.png</span></span></span></td><td><span class="catalog__name">Mining Beam Section</span><span class="catalog__id">mining_beam_section</span></td><td class="catalog__num">100 m</td><td class="catalog__num">1 pulse/s</td><td class="catalog__num">1.5 field cells</td><td class="catalog__num">0.6 s</td><td class="catalog__num">90</td></tr>
</tbody>
</table>
</div>

One emitter ships. The Mining Tug carries it at the tip of its bow boom. Bolt one onto any other hull in the ship editor; it takes neighbours on every face except the emitter face. The Ship pane lists it as `MNG`. See [Ship sections for mods](../../../create/sections/#mining) for the numbers a mod can change.
