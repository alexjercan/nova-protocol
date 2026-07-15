# Flight & autopilot

Manual flight in Nova Protocol is fully **Newtonian**: momentum persists, nothing dampens you, and the only assist is the autopilot itself - which flies the same real controller and thrusters you do. There is no flight-assist toggle and no separate RCS budget; how a ship handles falls out of its mass and the thrusters bolted to it.

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

You point the hull by mouse or stick - the controller section turns the ship toward your aim - and hold <kbd>W</kbd> (or the burn trigger) for an analog main-drive burn. The main drive is the sum of the thrusters that point forward; inputs spool up and down smoothly rather than snapping. A ship can carry an optional soft **speed cap** (used by training legs like the Shakedown's 25 u/s starter governor - `u/s` is units per second; see the [glossary](../glossary/)): the burn tapers to zero over the last stretch before the cap, along the burn axis only, so a held throttle levels off instead of accelerating forever. Turning and braking are never capped.

## Balancing thrust through the hull

Because thrusters sit wherever you bolted them, an off-center burn would spin the ship. The flight computer prevents that: it sets each engine's throttle to deliver the commanded forward thrust while cancelling the twist through the live center of mass, recruiting off-axis thrusters purely for counter-torque when the firing set cannot balance itself. An asymmetric or battle-damaged ship still flies straight (any tiny leftover spin is mopped up by the steering) - see [Thruster](../sections/thruster/).

## Mass-legible handling

Turn rate is derived from the ship's torque budget and its live rotational inertia, then clamped into a sane band (roughly 10 to 240 degrees per second). A stripped-down hull snaps around; a heavy, fully-built ship lumbers. Lose sections and the handling changes with the mass - so how a ship flies is legible from how it is built.

## The autopilot flies the hull

The autopilot verbs are the assist. Each writes to the _same_ actuators you use - the controller's rotation command and the thrusters' throttle - so you watch the hull physically swing and the plume light up; there are no invisible forces. Any manual input (a thruster key, a burn, a rotation, or CANCEL) disengages it instantly and hands you back a ship that is already moving.

- **GOTO** - burns toward your current nav lock, flips at the arrival curve, and decelerates to rest at a standoff (about 50 u plus the target's radius, measured from the surface, kept outside a torpedo's blast radius). It tracks a drifting target.
- **ORBIT** - circularizes and station-keeps around the dominant [gravity well](../gravity-wells/), holding a stable ring at orbital speed (`v = sqrt(mu / r)`) with micro-burns. It never self-completes - it holds until you break away.
- **STOP** - flips to retrograde and burns until you are at rest, budgeting for the local gravity pull along your velocity.

Because the autopilot flies through the real actuators, it is honest about the ship's limits - an under-thrustered build takes longer to come around than a nimble one - and it will refuse a maneuver it cannot physically achieve (for example, ORBIT will not engage a well with no stable band). See [Keybinds](../keybinds/) for the verb keys.
