# Flight & autopilot

Manual flight in Nova Protocol is fully **Newtonian**: momentum persists, nothing dampens you, and the only assist is the autopilot itself - which flies the same real controller and thrusters you do. There is no flight-assist toggle; how a ship handles falls out of its mass and the thrusters bolted to it. The one exception is **RCS**, an optional fine-translation mode a scenario can grant for close-in docking (see [RCS](#rcs-fine-docking-thrusters) below).

<figure class="figure">
    <!-- Capture: assets/wiki-flight.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-flight.png</span
        >
        <span class="figure__placeholder-note"
            >A ship mid-burn with its plume lit and the
            velocity sphere reading, ideally during a GOTO
            flip so the diegetic autopilot is visible.</span
        >
    </div>
</figure>

## Manual flight

You point the hull by mouse or stick - the controller section turns the ship toward your aim - and hold <kbd>W</kbd> (or the burn trigger) for an analog main-drive burn. The main drive is the sum of the thrusters that point forward; inputs spool up and down smoothly rather than snapping. A ship can carry an optional soft **speed cap** (used by training legs like the Shakedown's 250 m/s starter governor - `m/s` is metres per second; see the [glossary](../glossary/)): the burn tapers to zero over the last stretch before the cap, along the burn axis only, so a held throttle levels off instead of accelerating forever. Turning and braking are never capped.

## Balancing thrust through the hull

Because thrusters sit wherever you bolted them, an off-center burn would spin the ship. The flight computer prevents that: it sets each engine's throttle to deliver the commanded forward thrust while cancelling the twist through the live center of mass, recruiting off-axis thrusters purely for counter-torque when the firing set cannot balance itself. An asymmetric or battle-damaged ship still flies straight (any tiny leftover spin is mopped up by the steering) - see [Thruster](../sections/thruster/).

## Size decides the handling

<!-- Stats verified against crates/nova_events/src/scale.rs (LOAD_LIMIT 8 * 9.81 :23, METERS_PER_UNIT 10.0 :14) and crates/nova_ship/src/physics/attitude.rs (the two ceilings :70-86, the arm and what shortens it :131-164). "About two seconds": a bang-bang 180 is 2 * sqrt(pi / ceiling) (guidance.rs:302-303), and the shipped corvette's 2.76 u arm (attitude.rs:138) gives 78.48 / 27.6 = 2.84 rad/s2, so 2.10 s. -->

How hard a hull turns is the lower of two limits: what its flight computers twist it with against the mass they swing, and what its own metal survives. Hull takes 8 G, measured out at the furthest section from the centre of mass, so a long ship on a long arm has the gentler limit. Nothing authors the result - it falls out of the hull you built. A small craft whips around; a hauler handles like the freighter it is.

Every shipped craft is held by its structure rather than by its computers. Two things come with that: a hull that has lost its nose turns sharper than it did intact, because the arm got shorter, and a hull already deep in a hard turn has less authority left to tighten with.

A hull can mount several [controllers](../sections/controller/) and their torque adds, but on a structure-bound craft that buys no extra turn rate - only a hull heavy enough to run its computers out first gains from a stack. What stacking always buys is precision: the hull starts braking its turn earlier and stops on the commanded heading instead of swinging back.

## The autopilot flies the hull

The autopilot verbs are the assist. Each writes to the _same_ actuators you use - the controller's rotation command and the thrusters' throttle - so you watch the hull physically swing and the plume light up; there are no invisible forces. Any manual input (a thruster key, a burn, a rotation, or CANCEL) disengages it instantly and hands you back a ship that is already moving.

- **GOTO** - burns toward your current nav lock, flips at the arrival curve, and decelerates to rest at a standoff (about 500 m plus the target's radius, measured from the surface, kept outside a torpedo's blast radius). It tracks a drifting target.
- **ORBIT** - circularizes and station-keeps around the dominant [gravity well](../gravity-wells/), holding a stable ring at orbital speed (`v = sqrt(mu / r)`) with micro-burns. It never self-completes - it holds until you break away.
- **STOP** - flips to retrograde and burns until you are at rest, budgeting for the local gravity pull along your velocity.

<div class="widget" data-widget="goto-verb">
<p>One GOTO leg by the numbers: the drive burns out while the arrival envelope allows, swings retrograde one flip early (the envelope budgets the coast the flip costs), brakes at 85% of what the drive can do down to a 1.5 u/s approach floor, and eases the last stretch onto the 50 u standoff with the fine RCS jets. A longer leg just means a higher peak speed - the flip always lands the braking ramp on the standoff.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

What the scope calls the "arrival envelope" is the controller's one rule: at any distance it caps closing speed at what a flip-and-brake from here can still cancel. The flip is scheduled early by the time the swing itself takes - the shipped craft come around 180 degrees in about two seconds, a heavier hull takes longer, and whatever that coast costs is budgeted into the envelope, so the brake ramp starts aligned instead of late. Braking keeps a 15% authority margin in reserve, holds a small minimum approach speed so the last stretch never crawls, and hands the final meters to RCS: arrivals settle, they do not pulse the main drive on the spot.

The same machinery answers why the autopilot is honest about the ship's limits: everything above is computed from the live hull - its real mass, its real thrusters, its real flight computer. An under-thrustered build gets a shallower envelope and an earlier flip, not invisible help. And it will refuse a maneuver it cannot physically achieve: ORBIT disengages rather than fly a well with no stable band between its surface clearance and its fade band.

</details>

See [Keybinds](../keybinds/) for the verb keys.

<figure class="figure">
    <!-- Capture: assets/loops/goto-arrival.webm -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/goto-arrival.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop of a GOTO arrival: the flip to
            retrograde, the braking burn, and the ship easing
            to rest at the standoff on RCS.</span
        >
    </div>
    <figcaption class="figure__caption">The whole verb is visible on the hull: flip, brake, settle.</figcaption>
</figure>

## RCS: fine docking thrusters

For the last few meters of an approach - where a main-drive burn is too coarse - a ship can carry **RCS** (a reaction-control system): hold <kbd>Shift</kbd> and steer with the mouse (lateral and fore/aft) and the scroll wheel (up and down) to nudge the ship straight along its own axes, with **no rotation**. On a gamepad the same gesture is one thumb: click the left stick to engage, then push it to translate. While you hold it the helm and camera hold still so you can concentrate on the translation, the [velocity sphere](../hud/) turns violet, and a soft burn loop plays.

RCS is a **trim, not free thrust**: each ship-local axis caps at a gentle speed (about 20 m/s), so it eases you into position rather than replacing the main drive.

<details class="explain">
<summary>Show explanation</summary>

The cap is per ship-local axis, relative to whatever the maneuver is holding as its reference: push an axis you are already coasting at the cap and nothing happens, while the opposite direction still slows you. The push is a single linear impulse through the center of mass, so RCS never rotates the hull, and the last 4 m/s before the cap taper off rather than hitting a wall. The autopilot uses the same thrusters under the hood - **GOTO** and **STOP** settle their arrival on RCS, braking with the fine jets in the last stretch so a ship eases to a stop instead of pulsing on the spot.

</details>

RCS is a controller verb granted per ship, like the autopilot verbs, and the mainline campaign flies with it **withheld** - the RCS chip only appears in the keybind dock when a scenario grants it, so you know when it is available.
