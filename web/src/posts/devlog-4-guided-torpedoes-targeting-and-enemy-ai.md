# Devlog #4: guided torpedoes, targeting and an enemy that fights back

The last three devlogs built the pieces of a space combat game: modular ships, objectives, and weapons that do damage. v0.4.0 is the release where those pieces finally point at each other and shoot. This is the biggest update so far, and it lands on four fronts at once: weapons that _find_ their target, a targeting layer that tells you what you are locked onto, an enemy with an actual brain, and a flight model that flies the hull honestly. On top of all of that, the game makes noise for the first time.

## Torpedoes that chase

In v0.3.0 the torpedo was a dumb projectile with a blast radius: aim, fire, hope. v0.4.0 turns it into a **guided weapon**. The torpedo steers toward its lock using **proportional navigation** - the same intercept law real missiles use, which turns toward where the target _will be_ rather than where it is. An angular lock-on aim-assist gets the shot pointed in the right direction on launch, an arming gate keeps it from detonating in your own lap, and a launch particle burst plus a blast-radius visual make the whole thing read at a glance.

The effect is that torpedoes stop being an area-denial gimmick and become a genuine threat you have to respect - both when you fire them and, later in this same release, when something fires them at you.

<figure class="figure">
    <!-- Capture: assets/devlog4-proportional-navigation.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Image needed</span
        >
        <span class="figure__placeholder-name"
            >assets/devlog4-proportional-navigation.png</span
        >
        <span class="figure__placeholder-note"
            >Diagram or in-game trail of a torpedo leading a
            moving target: line-of-sight rotation vs. the
            intercept course. A screenshot of a torpedo curving
            onto a target works too.</span
        >
    </div>
    <figcaption class="figure__caption">
        Proportional navigation: the torpedo turns toward where
        the target will be, not where it is.
    </figcaption>
</figure>

## A targeting arc that knows what you are looking at

A guided weapon needs something to guide toward, so v0.4.0 adds a real **targeting layer**. Ships inside your acquisition range are auto-acquired by their signature, and dwelling your focus on one promotes it to a full lock. From there you can drill in: a **per-section fine-lock** lets you pick out an individual component of the enemy hull, with aim-snap and cycling to move between them. The HUD grew a whole **substrate** to show all of this - screen-projected markers that anchor to entities or points, size themselves by apparent size, and clamp to the screen edge with arrows when the target goes off-screen, plus a locked-target readout for range, closing speed and health.

Turret fire plugs straight into this. The turret **auto-aims with true intercept lead**, solved in the shooter's own frame so a moving ship's rounds actually land, and it prefers the fine-locked section if you have one, falling back to live structure and then to the camera ray. Lead pips on the HUD show you where it is aiming.

## Friend, foe, or furniture

None of this means anything without a notion of sides, so v0.4.0 introduces a **faction and relation model**: hostile, neutral, or own. It drives who gets auto-acquired, whose allegiance a projectile carries (so your rounds do not collide with you, and shot-down torpedoes die whole and blast-free), and how a reticle is tinted. It is a small system that quietly makes the whole combat sandbox coherent.

## An enemy with a state machine

The headline feature: v0.4.0 ships the first **AI combat wave**, and the enemy is no longer a stationary target. Each AI ship runs a **behavior state machine** - Idle, Patrol, Engage, Evade, Retreat - and behaves differently in each. It flies autopilot patrol routes, keeps fire discipline, prioritizes point defense on inbound torpedoes, holds a standoff orbit and strafes, remembers threats and evades them, and launches its own enveloped torpedoes when it has a solution. The AI helm writes slewed absolute rotation commands, so it turns like a ship instead of snapping.

This is the moment Nova Protocol stopped being a shooting range and started being a fight.

<figure class="figure">
    <!-- Capture: assets/devlog4-ai-state-machine.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Image needed</span
        >
        <span class="figure__placeholder-name"
            >assets/devlog4-ai-state-machine.png</span
        >
        <span class="figure__placeholder-note"
            >State-transition diagram of the AI behavior states
            (Idle, Patrol, Engage, Evade, Retreat) with the
            edges that move between them. A debug-overlay
            screenshot showing an AI ship's current state also
            works.</span
        >
    </div>
    <figcaption class="figure__caption">
        The enemy runs a five-state behavior machine and picks
        its move from the state it is in.
    </figcaption>
</figure>

## Flying the hull honestly

Underneath the combat is a **flight-assist overhaul**. There is now an assisted velocity-hold mode (WASDQE nudges, an X brake latch, a soft speed cap) alongside a Z direct Newtonian mode for when you want raw thrust, all working against an RCS budget with a live FA and speed readout. The piece I am proudest of: the flight computer **balances thrust through the live center of mass**. It uses differential throttle to null the torque a burn would induce, and recruits off-axis thrusters for counter-torque, so an asymmetric or battle-damaged ship still flies straight instead of pinwheeling. Handling is now mass-legible: turn rate comes from the torque budget and live inertia, so a stripped ship snaps around and a heavy build lumbers.

All of this rides on the same foundation the game started from: a ship is not a monolithic model but a root entity with a handful of _section_ children - hull, controller, thruster, turret, torpedo bay - each carrying its own mass and health and contributing exactly one behavior. Because the flight computer drives those real actuators rather than scripting a position, it stays honest about the hull it is given.

That honesty is the whole point of the **diegetic autopilot**. When you press GOTO, ORBIT or STOP, the computer does not slide the ship to a destination - it drives the same controller and thrusters you would, so you watch the hull physically swing to a new heading and the exhaust plume light up as it burns:

- **GOTO** burns toward your current lock, flips at the arrival curve, and decelerates to rest at a standoff distance.
- **ORBIT** parks you into a stable circular orbit around the dominant gravity well.
- **STOP** faces retrograde and burns until you are at rest.

Because it flies through the real actuators, it is honest about the ship's limits - a sluggish, under-thrustered build takes longer to come around than a nimble one - and the moment you touch a control, the autopilot disengages and hands you back a ship that is already moving. ORBIT has something real to fly around, too: large asteroids carry gravity wells with genuine inverse-square pull, clamped near the surface and faded to nothing at the edge of their sphere of influence. That single mechanic is what turns "fly to a point" into "manage your orbit".

## Sound and fury

And for the first time, the game has **audio**. Placeholder SFX for explosions, impacts, turret fire, torpedo launches, and a throttle-tracking thruster loop, all with distance attenuation and throttling so a busy fight does not turn into noise. Alongside it, **combat juice**: a trauma-model camera shake and expanding hit and impact flash rings, also attenuated by distance. One hit plays exactly one cue - the audio and juice observers ignore damage-propagation re-entry - so the feedback stays crisp.

## Under the hood

A release this size came with plenty of plumbing. The integrity, health, blast and mesh-slicer systems now come from the shared `bevy-common-systems` crate instead of in-tree copies. Ships, asteroids and torpedoes interpolate between physics ticks so the camera stops twitching, and the chase camera anchors on the live center of mass. Blast damage reaches every body overlapping the blast, section overkill is absorbed instead of propagated, and a disabled controller stops torquing the hull. There is also a row of example test ranges (`06_torpedo_range`, `08_turret_range`, `10_gameplay`, `11_com_range`) with live tuning sliders and a headless screenshot smoke harness, plus a new CI workflow running fmt, clippy and the test suite on every push.

<p class="prose__meta">
    Shortly after, <strong>v0.4.1</strong> followed with a small
    release-flow fix (installing the macOS darwin std for the
    pinned nightly so the universal build succeeds) and a CI
    consolidation onto a single feature set.
</p>

## Where v0.4.0 lands

v0.4.0 is the release where the flight toy became a combat game: guided weapons, a targeting layer, an AI that fights back, and a flight model honest enough to make damaged ships handle differently - now with sound. Next up: making the player's relationship with all this targeting deliberate instead of automatic. Or you can just [go fly something](../../play/).
