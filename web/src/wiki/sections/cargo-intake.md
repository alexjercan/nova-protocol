# Cargo intake

<figure class="figure">
    <!-- Capture: assets/wiki-section-cargo-intake.png (producer: mine_and_sell) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-cargo-intake.png</span
        >
        <span class="figure__placeholder-note"
            >A line warship's dorsal intake rising under a
            drifting ore canister, its accordion door folded
            open and the cyan pickup sight drawn from the door
            to the canister.</span
        >
    </div>
</figure>

A cargo intake is a **hold mouth**: a 30 x 20 m box, three cells across, two high and one deep, with an accordion door on one face. Bring the door to a canister and the intake takes the whole canister into your hold. Pick an item in the Inventory pane and the intake drops it out through the same door, packed into canisters.

The intake does not reach out or pull. A canister goes in only when it touches the thin slab in front of the opening, so a pickup is flying: you move the door onto the canister.

<!-- Values from crates/nova_authoring/src/base_content/sections/cargo_intake.rs: health 90 (CARGO_INTAKE_BASE_HEALTH), INTAKE_CELLS 3 x 2 x 1, door open_seconds 1.2, detection_range 40 m, capture_gap 1 m, aperture 22.2 x 15.3 m, eject_speed 3 m/s. Pickup sight range CARGO_PICKUP_SIGHT_RANGE 200 m in crates/nova_hud/src/pickup_sight.rs. -->

| The cargo intake at a glance | |
|---|---|
| Size | **30 x 20 x 10 m**: three cells across, two high, one deep |
| Opening | **22.2 x 15.3 m**, centred on the door face |
| Door | opens for a canister within **40 m** in front of it, in **1.2 s** |
| Take | any part of a canister within **1 m** in front of the opening |
| Drop | one canister at a time, at **3 m/s** away from your ship |
| Sight | drawn for a canister within **200 m** of your ship |
| Health | **90** |

<!-- Widget values: crates/nova_authoring/src/base_content/sections/cargo_intake.rs:23 (INTAKE_CELLS 3 x 2 x 1), :111-112 (detection_range 40 m, capture_gap 1 m), :117-119 (aperture 22.2 x 15.3 m, eject_speed 3 m/s); crates/nova_ship/src/sections/cargo_intake_section.rs:72 (CARGO_CANISTER_SIZE 9.4 x 5.8 x 5.8 m), :453-459 (door detection by the canister centre from the face centre, front half only), :486-495 (take: contact penetration >= 0 and room in the hold), :499-503 (birth 2.9 + 1 + 0.5 = 4.4 m off the face). -->

<div class="widget" data-widget="cargo-intake-take">
<p>The intake's door face is 30 by 20 m, and the clear opening in it is 22.2 by 15.3 m, wider than a 9.4 by 5.8 by 5.8 m canister is long. The door opens for a canister whose centre is in front of the face and within 40 m of the face centre, so an end-on canister whose near end is inside 40 m can still leave the door shut. The intake takes the whole canister when part of it is in contact with the 1 m slab in front of the opening and the hold has room for it. A dropped canister is born 4.4 m off the face and leaves at 3 m/s.</p>
</div>

## Taking a canister in

<!-- Behavior verified against crates/nova_ship/src/sections/cargo_intake_section.rs `run_cargo_intakes` (the trigger slab, whole-canister take, room check, a drop born outside the slab) and crates/nova_hud/src/pickup_sight.rs (sight target order). -->

| Canister | What the intake does |
| --- | --- |
| In front of the door and within 40 m of it | Opens the door. |
| Any part of it within 1 m in front of the 22.2 x 15.3 m opening | Takes the whole canister into your hold at once, at any speed and any angle, open door or not. |
| Touching your ship anywhere else | Is not taken. |
| Heavier than your hold has room for | Stays out, whole. |
| Just dropped | Leaves without being taken. It is taken back only if it turns and comes within 1 m of the opening again. |

With a canister within 200 m of your ship, the **pickup sight** draws a cross on your intake's face and a line to the canister, with no lock needed. A travel-locked canister in range takes the line first; otherwise the canister nearest an intake does. Fly until the line stands perpendicular to the intake cross, then close the gap. The sight stays cyan while closing. The take sound and the canister going through the door confirm a take; the sight goes with the canister. An unavailable intake, or no canister within 200 m, draws no sight.

The take ignores speed, but a canister is a free body: one that meets your hull beside the opening is knocked aside, not taken. Come up under it door first and gently.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-cargo-intake.webm (producer: mine_and_sell) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-cargo-intake.webm</span
        >
        <span class="figure__placeholder-note"
            >The line warship's dorsal intake closing on a
            drifting ore canister: the canister reaches the
            open door, is gone into the hold, and the door
            folds shut.</span
        >
    </div>
</figure>

## Dropping cargo

<!-- Behavior verified against crates/nova_ship/src/sections/cargo_intake_section.rs `run_cargo_intakes`: the queue drains only with the door fully open and no canister within CARGO_CANISTER_SIZE.length() (12.5 m) of the birth point, 4.4 m off the face; velocity is the ship's at that point plus eject_speed. -->

Undocked, the Inventory pane's **Jettison** packs an item into canisters queued on your intake; [Jettison and pickup](../../interface/#jettison-and-pickup) has the form. Once you close the interface, the door folds open and the canisters leave through it one at a time, in the order you queued them, at 3 m/s relative to your ship. Each is born 4.4 m out from the door, and the next waits until no canister is within 12.5 m of that point. Canisters still waiting are lost with the intake.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-cargo-jettison.webm (producer: lesson_cargo) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-cargo-jettison.webm</span
        >
        <span class="figure__placeholder-note"
            >A jettison seen from beside the hull: the intake's
            door folds open and a canister of hull plates leaves
            it, drifting clear of the ship.</span
        >
    </div>
</figure>

## Canisters

<!-- Values from crates/nova_gameplay/src/inventory.rs CARGO_CANISTER_MAX_MASS_G 200 kg; crates/nova_hud/src/cargo_canister_chips.rs CARGO_TAG_RANGE 100 m; canister health and lock rules in crates/nova_ship/src/sections/cargo_intake_section.rs `cargo_canister`. -->

A canister holds at most 200 kg across all its stacks. Within 100 m of your ship, each canister carries an amber tag that reads what it holds, such as `4 Hull plate`, or `Mixed cargo` for multiple item types, with its total mass in kg. The tag hides while the canister is off-screen. A canister has 20 HP; at zero it and its contents are destroyed, not picked up. Canisters can be designated with a travel lock but cannot enter the combat target slot. Canisters are not saved: they go when the scenario ends, and a jettisoned stack returns to your hold when the scenario loads again.

Canisters come from two places: your own jettison, and a [mining beam](../mining-beam/) cutting ore out of a rock.

## Variants

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/cargo_intake.rs and assets/base/sections/base.content.ron. Mounted by crates/nova_authoring/src/base_content/ships/block.rs: `cargo_intake` on the line warship's top deck, door up. -->
<div class="catalog__head"><span class="catalog__title">Cargo intake - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Size</th><th>Opening</th><th>Door zone</th><th>Take gap</th><th>Drop speed</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargo-intake-section.png</span></span></span></td><td><span class="catalog__name">Cargo Intake Section</span><span class="catalog__id">cargo_intake_section</span></td><td class="catalog__num">3x2x1</td><td class="catalog__num">22.2 x 15.3 m</td><td class="catalog__num">40 m</td><td class="catalog__num">1 m</td><td class="catalog__num">3 m/s</td><td class="catalog__num">90</td></tr>
</tbody>
</table>
</div>

One intake ships. The Line Warship you fly in New Game carries it on its top deck, aft of the dorsal guns, door up. Bolt one onto any other hull in the ship editor; it takes neighbours on every face except the door. See [Ship sections for mods](../../../create/sections/#cargo-intake) for the numbers a mod can change.
