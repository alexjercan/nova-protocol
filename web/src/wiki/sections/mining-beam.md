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
            >The line warship's bow emitter deployed, its green
            beam running straight out to a locked ore rock about
            58 m ahead and ending on the rock's face.</span
        >
    </div>
</figure>

A mining beam is a **one-cell emitter** behind two sliding doors. Lock a rock, point the emitter at it and hold its key: the doors part, the tip runs out, and once a second the beam cuts the rock where it hits. The ore it cuts free leaves the rock as canisters, and a [cargo intake](../cargo-intake/) takes them into your hold.

The beam does not aim. It runs straight out of the emitter's face, so the **ship** aims it, the way a railgun is aimed. The lock only says which rock you mean.

<!-- Values from crates/nova_authoring/src/base_content/sections/mining_beam.rs: health 90 (MINING_BEAM_BASE_HEALTH), TRACK_SECONDS 0.3 per leg, reach 100 m, pulse_interval_seconds 1.0, carve_radius_cells 1.5 of the rock's field, whose cell is FIELD_CELL_WORLD 0.5 units, about 5 m (crates/nova_scenario/src/objects/asteroid_carve.rs). Per-section key: `input_mapping` entry -> `SpaceshipMiningInputBinding` in crates/nova_ship/src/input/player/weapons.rs; the line warship's `mining_beam` takes V in crates/nova_authoring/src/base_content/scenarios/open_world.rs `weapon_bindings`; the editor default V with no pad button in crates/nova_editor/src/placement.rs `default_binds`. -->

| The mining beam at a glance | |
|---|---|
| Key | its own, held: <kbd>V</kbd> on the line warship |
| Deploy | doors **0.3 s**, then the tip **0.3 s** |
| Reach | **100 m** from the emitter face, straight ahead of it |
| Rate | a pulse as soon as it is out, then **one a second** |
| Cut | a sphere about **7.5 m** in radius where the beam hits |
| Health | **90** |

<!-- Widget values: crates/nova_authoring/src/base_content/sections/mining_beam.rs:101-103 (reach 100 m, pulse 1 s, carve_radius_cells 1.5); FIELD_CELL_WORLD 0.5 units in crates/nova_scenario/src/objects/asteroid_carve.rs:108 (1.5 cells ~ 7.5 m); CARGO_CANISTER_MAX_MASS_G 200 kg in crates/nova_gameplay/src/inventory.rs:532; MINED_CANISTER_OFFSET 10 m and MINED_CANISTER_SPEED 2 m/s in crates/nova_scenario/src/mining.rs:93,97,895-925; check order and inclusive reach :160-173, :449-486; refusal is an info log only :434-435; canisters after the remesh :804-841. -->

<div class="widget" data-widget="mining-beam-checks">
<p>A pulse runs its checks in order and stops at the first failure: a travel lock, on an asteroid, of a kind that holds ore, with its nearest point within 100 m of the emitter face, and on the line straight out of that face within 100 m. A refused pulse shows nothing on the HUD. A pulse that passes cuts a sphere about 7.5 m in radius where the beam meets solid material; the first pulse into an untouched rock cuts nothing. Ore leaves only after the rock's remesh lands, as canisters of up to 200 kg, each born 10 m off the cut and drifting off it at 2 m/s.</p>
</div>

## Cutting ore

<!-- Behavior verified against crates/nova_ship/src/sections/mining_section.rs (hold to deploy, doors then tip, tip then doors on release, reverse mid-travel) and crates/nova_scenario/src/mining.rs (the MiningRefusalType checks in order, silent refusal, MiningBeamHit drawn only while every check passes, the first pulse into an untouched rock seeds its field and cuts nothing). -->

1. **Lock the rock.** Hold the radar on it with weapons lowered, so it takes your **travel lock**. See [Targeting & radar](../../targeting-radar/#stances-and-slots).
2. **Point the emitter at it.** The Line Warship's emitter sits on the starboard top deck at the bow and looks forward, so put your nose on the rock and close to inside 100 m.
3. **Hold the beam's key.** The doors part and the tip runs out. Once the tip is out, the beam fires at once and then once a second for as long as you hold the key.
4. **Let go.** The tip runs in first, then the doors shut. Pressing or letting go halfway reverses from wherever the parts are.

Every pulse checks five things, in order: your ship has a travel lock; the lock is a rock; the rock holds ore; the rock is within 100 m of the emitter face; the line straight out of that face meets it within 100 m. A pulse that fails any of them does nothing at all: no beam, no sound, no cut. So **no beam means a check is failing** - most often the nose is off the rock or the rock is too far. Nothing else blocks the beam: it is cast against the locked rock alone.

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
            canisters earlier cuts freed and into the locked
            rock, and on release the tip runs in and the doors
            shut.</span
        >
    </div>
</figure>

## Where the ore goes

<!-- Ore by kind: ore_for_asteroid_kind in crates/nova_scenario/src/mining.rs. One item per field corner cut, paid once the rock's remesh lands; canisters of at most CARGO_CANISTER_MAX_MASS_G 200 kg (crates/nova_gameplay/src/inventory.rs), born MINED_CANISTER_OFFSET 1 unit (10 m) off the last cut along its outward normal and drifting at MINED_CANISTER_SPEED 0.2 units/s (2 m/s) plus the rock's own motion there. Ore mass 10 kg each: ItemType::mass_g. Open-world rock kinds: ROCK_KINDS in crates/nova_world_base/src/clusters.rs. -->

What a rock is made of decides the ore. Every ore weighs 10 kg, so one canister carries up to 20.

| Rock kind | Ore |
| --- | --- |
| rock | Stone ore |
| metal | Iron ore |
| ice | Water ice |
| carbon | Carbon ore |
| plain | none: the beam does not fire on it |

The ore leaves the rock once the cut is drawn, packed into canisters of up to 200 kg. They are born one at a time, 10 m off the last cut and drifting away from it at 2 m/s on top of the rock's own motion, and the next waits until the space there is clear. A rock that is cut away entirely still lets its last canisters go. Fly your [cargo intake](../cargo-intake/#taking-a-canister-in) onto a canister to take it in. A saved Open World keeps a canister as you left it; any other scenario keeps it only for the session.

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
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/mining_beam.rs and assets/base/sections/base.content.ron. Mounted by crates/nova_authoring/src/base_content/ships/block.rs: `mining_beam` at the line warship's bow cell (1, 1, -7). -->
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

One emitter ships. The Line Warship you fly in New Game carries it on the starboard top deck at its bow. Bolt one onto any other hull in the ship editor; it takes neighbours on every face except the emitter face. The Ship pane lists it as `MNG`. See [Ship sections for mods](../../../create/sections/#mining) for the numbers a mod can change.
