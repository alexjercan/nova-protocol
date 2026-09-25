# Docking port

<figure class="figure">
    <!-- Capture: assets/wiki-section-docking.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-docking.png</span
        >
        <span class="figure__placeholder-note"
            >Two hulls held nose to nose on their docking
            ports, both sleeves reached out across the gap
            between the two port faces.</span
        >
    </div>
</figure>

A docking port is a **sealed hatch**: one cell of collar with a graphite sleeve behind it, and nothing proud of the cell until it is used. Lock the ship you want, bring your port face up to its port face, and <kbd>D</kbd> clamps the two hulls together. The sleeves then reach across the gap and bridge it.

What a dock is NOT is a merge. The two ships stay two ships: two hulls, two pilots, two sets of controls, and nothing pooled between them. The clamp holds the pose you met in and does no more than that. One ship at a time flies the pair: the other ship's pilot until you take the **helm** with <kbd>H</kbd>, and you until you hand it back. <kbd>D</kbd> again lets go.

<!-- Values from crates/nova_authoring/src/base_content/sections/docking_port.rs: health 90 (DOCKING_BASE_HEALTH), capture_distance 10 m, capture_angle 15 deg, maximum_relative_speed 5 m/s, maximum_relative_angular_speed 5 rad/s, DOCK_TUBE_TRAVEL 0.5 cells over 1.2 s out and 0.8 s back. -->

| The docking port at a glance | |
|---|---|
| Reach | a **10 m** face gap: one cell between the two port faces |
| Facing | the two ports within **15 degrees** of directly opposed |
| Speed | under **5 m/s** of closing speed between the two hulls |
| Roll | **ignored** - the ports are cylinders, any twist docks |
| Sleeve | **5 m** out, in 1.2 s, and back in 0.8 s |
| Health | **90**: the lightest part on a hull |

## Flying the approach

<!-- Behavior verified against crates/nova_ship/src/sections/docking_section/port.rs (the envelope, measured between retracted faces; pairs graded on the stricter of the two ports) and crates/nova_ship/src/input/player/flight_rig.rs (the DOCK press). -->

Docking is one key, and everything before it is flying. Lock the other ship the way you lock anything - the dock is offered against your **travel lock**, not against whatever is nearest - and then close the gap yourself. [RCS](../../flight-autopilot/#rcs-fine-docking-thrusters) is the trim the last hundred meters wants: it pushes the hull along its own axes without turning it, which is exactly the motion an approach needs and the one a main drive cannot make.

Inside 80 m the **docking sight** draws itself: a flat cross over each port face, and a line running between the two faces with a tick on it for every cell of capture distance left. Square the two crosses up, stand the line perpendicular to them, then close the gap. The crosses turn green when the facing is inside the envelope, and the line turns green when the gap and the closing speed are. Three green and the chip lights.

Nothing flies the roll for you, and nothing is going to. Lining a port up on the roll axis by hand is the hard part of an approach, and it is meant to be.

The DOCK chip lights on the [keybind dock](../../hud/) only while a pair of ports is genuinely eligible, so the chip is the readout: if it is dark, something in the envelope is still wrong. Three things have to be true at once.

- **The gap.** Ten meters or less, measured **face to face** between the two retracted ports - not between the two ships, and not between their centres. A port buried behind a wing measures from its own face like any other.
- **The facing.** The two ports have to look at each other within fifteen degrees. Roll does not count: the collar is a circle, so meeting one upside down is the same as meeting it level, and the clamp simply keeps whatever twist you arrived with.
- **The speed.** Under five meters a second of RELATIVE motion, and the same for relative spin. Absolute speed is nothing here: two ships running together at 2 km/s are standing still as far as the other one is concerned.

You do not choose the ports. The game pairs the nearest eligible ones on the two hulls and picks the same pair every time for the same approach, so a ship carrying four ports docks on the one you flew to.

## What the clamp holds

<!-- Behavior verified against crates/nova_ship/src/sections/docking_section/connection.rs (one FixedJoint with an explicit anchor and basis, no velocity written; DockingConnection.helm starts Neutral; park_suppressed_docked_helms) and crates/nova_ship/src/sections/docking_section/assembly.rs (the driver's wrench split over both roots from the combined mass, centre of mass and inertia). -->

The clamp is a single joint between the two hulls, made the instant you press the key, and it holds the **pose you met in** - the gap you crossed at, the angle you came in on, the roll you happened to carry. Nothing is snapped straight and nothing is tidied up.

Your drift comes with you. A dock never brings either ship to a halt: two hulls that met while coasting keep coasting, together, and a pair caught in a well falls the way one hull would. The clamp only takes away the motion of one hull relative to the OTHER.

A docked pair flies as **one body**, and exactly one ship flies it. Every dock starts **neutral**: the other ship keeps the helm, its autopilot and its orders fly the pair, and your drive, your helm, your RCS and your maneuver keys stand down. RCS mode ends too, even with <kbd>Shift</kbd> still held: release it and press it again once you have the helm. The mode chip reads **NEUTRAL**, and the blocked chips stay on the dock, dark.

<kbd>H</kbd> takes the helm, and the chip reads **RELEASE HELM**. Now your throttle, your RCS, your mouse and <kbd>G</kbd>, <kbd>O</kbd> and <kbd>X</kbd> fly the pair, and the other ship stands down: its orders pause where they are, and pick up again when you hand the helm back with <kbd>H</kbd>. Whoever flies, the pair is flown from its combined mass and balance point: the turn is shared over both hulls, the drive is trimmed about the pair's balance point, and the autopilot brakes for the whole pair, so it turns and stops like one ship rather than one hull dragging the other. Only the flying ship's engines burn. <kbd>Z</kbd> still drops a maneuver without letting go of the clamp. Your guns, your radar and your point defense are untouched either way.

If the game cannot measure the pair, neither ship flies it and the mode chip reads **HELM FAULT** until it can. <kbd>H</kbd> cannot take the helm then, and its chip reads **HELM FAULT** too, dark. The blocked chips leave the dock until the pair measures. If you held the helm when the fault came, <kbd>H</kbd> still hands it back, but you cannot take it again until the pair measures. <kbd>G</kbd> on the map and `map goto` refuse with the fault. The clamp holds, and <kbd>D</kbd> still lets go; the next dock starts clear.

Only then do the sleeves move. Both ports reach 5 m out over about a second, which bridges a full cell of gap; at a shorter gap the two sleeves simply overlap inside each other and read as one tunnel. They are skin, not structure: the clamp is already holding before they start, and an extended sleeve never touches the other ship.

## Letting go

<!-- Behavior verified against crates/nova_ship/src/sections/docking_section/connection.rs: on_docking_release_request is ungated and addressed to a ship; release_connection removes DockedShip and DockedAssembly from both roots; plus a destroyed port and a destroyed ship. -->

**<kbd>D</kbd> again lets go**, the same way <kbd>O</kbd> again drops an orbit. Either ship can press it, and neither needs the other's permission - the ship you docked with can leave while you are still sitting there. Each ship flies itself again the moment the clamp goes, and the helm goes with it: the next dock starts neutral.

A dock has no timer and no drift. A pair stays docked indefinitely, because nothing you do at the controls can end it - only the verb can. A maneuver does not end it either: [ORBIT](../../flight-autopilot/#the-autopilot-flies-the-hull), GOTO and STOP fly the whole pair.

The last two endings are damage. Shoot the port off either hull, or kill either ship, and the clamp goes with it; the surviving port stows its sleeve and is free to dock again. A port is the lightest thing on a hull at 90 health, so a dock in a firefight is a dock on borrowed time.

## Variants

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/docking_port.rs and assets/base/sections/base.content.ron. Mounted by crates/nova_authoring/src/base_content/ships/block.rs: port_collar on the workship, both frame tenders and the line warship; starboard_collar on the line warship. -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-docking.png</span></span></span><span class="catalog__title">Docking - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Gap</th><th>Facing</th><th>Closing speed</th><th>Sleeve</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-docking-port-section.png</span></span></span></td><td><span class="catalog__name">Docking Port Section</span><span class="catalog__id">docking_port_section</span></td><td class="catalog__num">10 m</td><td class="catalog__num">15 deg</td><td class="catalog__num">5 m/s</td><td class="catalog__num">5 m / 1.2 s</td><td class="catalog__num">90</td></tr>
</tbody>
</table>
</div>

One port ships. The Utility Workship and both Frame Tenders carry one on the port flank, and the Line Warship carries one on each flank. Bolt one onto any other hull in the ship editor to fly a ship that can dock, or fly the `docking_approach` example, which hands you a tender built for the job. It takes neighbours on every face except the one it docks through - that face is the hatch. See [Ship sections for mods](../../../create/sections/#docking) for the numbers a mod can change, the capture envelope among them, and [Events](../../../create/events/#docking-lifecycle) for the two edges a scenario scripts a dock with.
