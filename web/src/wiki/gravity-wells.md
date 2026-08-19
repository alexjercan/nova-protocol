# Gravity wells

Large asteroids and planetoids carry gravity wells that pull ships, torpedoes and turret rounds with real physics. Wells never pull each other, and the strength is authored so every well is escapable under main drive. Only PILOTED ships feel a well - yours and the AI's; an unpiloted ship (a scripted bystander with no drive to resist the pull) floats where it sits rather than falling in.

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
            >A diagram of a well: surface-clamp core,
            inverse-square falloff, and the faded outer edge
            of the sphere of influence - or a ship on a
            clean ORBIT around a planetoid.</span
        >
    </div>
</figure>

## The pull

A well accelerates you toward its center by an inverse-square law, `a = mu / r^2`. The mass parameter `mu` is the body's one authored gravity number - never your ship's mass: gravity is acceleration, so a stripped fighter and a laden hauler fall the same.

<div class="widget" data-widget="gravity-well">
<p>The pull runs a = mu / r^2: held at its surface value below the rock (no slingshots), a clean inverse square through the core, then smoothstepped to exactly zero across the outer 15% of the sphere of influence. The Shakedown planetoid authors mass 27000 for a 329 u sphere of influence and a few u/s^2 at its drawn surface; ORBIT trusts the ring band between 1.5x the surface and 90% of the fade start.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

Two rules tame the extremes of the inverse square:

- **Surface clamp** - just below the drawn surface (plus a 1 u margin) the pull is held at its surface value, so grazing the rock is a bump, not a singularity slingshot.
- **Faded edge** - across the outer 15% of the sphere of influence the pull follows a smoothstep down to exactly zero at the boundary, and stays zero beyond it, so there is no force discontinuity to bump across.

Who feels it: piloted ships (player and AI alike), torpedoes and turret rounds - a long shot visibly curves near a planetoid. Section debris skips gravity, and an unpiloted scripted ship floats rather than falls. Wells never pull other wells.

</details>

## Sphere of influence

A well's reach follows from its mass alone: the sphere of influence is where the raw pull has decayed to a fixed cutoff (0.25 u/s^2), so `soi = sqrt(mu / 0.25)` - the body's drawn size never sets it. The Shakedown planetoid reaches 329 u, about 3.3 km on the HUD; the Final Tally anchorage rock (mass 45000) reaches 424 u. Outside it, the well does not exist as far as your ship is concerned.

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
