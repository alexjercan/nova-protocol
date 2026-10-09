# Gravity wells

Only a planet (or an invisible authored anchor) carries a gravity well - it is on rails, immovable. Everything else that can feel one is mobile: asteroids, ships and in-flight ordnance. Wells pull with real physics and never pull each other - the strength is authored so every well is escapable under main drive. Every ship feels a well, piloted or not: an unpiloted bystander falls in just like a crewed hull, and an asteroid falls in too - no rock is pinned in place by its own gravity anymore.

<figure class="figure">
    <!-- Capture: assets/wiki-gravity.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Diagram needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-gravity.png</span
        >
        <span class="figure__placeholder-note"
            >A cutter holding a clean ORBIT around an ice
            world: the flight computer's ring drawn across
            the body, the radius spoke out to the hull, and
            the debris ring beyond it.</span
        >
    </div>
</figure>

## The pull

A well accelerates you toward its center by an inverse-square law, `a = mu / r^2`. The mass parameter `mu` is the body's one authored gravity number - never your ship's mass: gravity is acceleration, so a stripped fighter and a laden hauler fall the same.

<div class="widget" data-widget="gravity-well">
<p>The pull runs a = mu / r^2: held at its surface value below the surface (no slingshots), a clean inverse square through the core, then smoothstepped to exactly zero across the outer 15% of the sphere of influence. The campaign inspection planetoid authors mass 27000 for a 3.29 km sphere of influence and a few tens of m/s^2 at its drawn surface; ORBIT trusts the ring band between 1.5x the surface and 90% of the fade start.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

Two rules tame the extremes of the inverse square:

- **Surface clamp** - just below the drawn surface (plus a 10 m margin) the pull is held at its surface value, so grazing the planet is a bump, not a singularity slingshot.
- **Faded edge** - across the outer 15% of the sphere of influence the pull follows a smoothstep down to exactly zero at the boundary, and stays zero beyond it, so there is no force discontinuity to bump across.

Who feels it: every ship root (player, AI, or an unpiloted bystander alike), every asteroid, torpedoes and turret rounds - a long shot visibly curves near a planetoid. Carved chunks and detached sections join the pull when their collision grace ends and they become dynamic. Wells never pull other wells.

</details>

## Sphere of influence

A well's reach follows from its mass alone: the sphere of influence is the distance at which the raw pull has decayed to a fixed cutoff of 2.5 m/s^2 - the body's drawn size never sets it, and because the pull is inverse-square, four times the mass buys only twice the reach. The campaign's inspection planetoid reaches 3.29 km; the lighter concealment planetoid it hides a warship behind reaches 2.83 km. Outside it, the well does not exist as far as your ship is concerned.

## How a generated body starts

An open-world sector places its own asteroids and unpiloted ships, and each one is given a real starting velocity rather than scripted motion. A body that starts inside one well's checked band - clear of the surface and fading edge, with its nominal circle within the owning sector - starts with a seeded tangent at circular speed. A single-well body whose nominal circle does not clear those checks starts on an outward escape path instead of falling from rest. A body with no well in reach coasts from rest. A pre-fix census of 512 deterministic seeds (64,000 cells) found 7 overlapping candidates among 95,248 placed mobile bodies. The generator retries each overlapping placement in a checked unique-well orbit band, then in well-free space, keeping the body's identity and count if a position fits. A placed escort and its hull move together to keep their separation. It does not choose the strongest well or coast from rest inside one. If no position fits, a named sector fault stops generation and streaming still fails. These velocities are starting conditions, not a promise that every orbit stays bounded forever - a collision or a shot can send a body anywhere. A mobile body belongs to whichever cell it is standing in now, not the one that spawned it. A cell that retires freezes its bodies in place, holding their position, velocity and damage, and a return thaws them where they froze: a frozen body does not move or take damage while its cell is off-window, so its orbital phase simply waits.

## The dominant well

Where two spheres of influence overlap, the pulls do not blend: you feel only the **dominant** well - the strongest at your position - and it keeps ownership until a challenger clearly beats it, so it does not flicker at the boundary.

<div class="widget" data-widget="dominant-well">
<p>Two overlapping wells: the ship feels only the stronger pull, and the incumbent holds until a challenger pulls more than 1.10x harder. Between those two thresholds sits a hysteresis window where ownership depends on which well had you last.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

The pick ranks wells by the pull each one exerts at your position - not by distance or mass - and the incumbent survives any challenger up to 1.10x its own pull. That 10% margin is what stops ownership chattering while you coast along a boundary.

The dominant well is exactly what the [ORBIT](../flight-autopilot/) autopilot circularizes around, flying a stable ring at orbital speed `v = sqrt(mu / r)`. That single mechanic is what turns "fly to a point" into "manage your orbit".

</details>
