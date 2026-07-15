# Gravity wells

Large asteroids and planetoids carry gravity wells that pull ships, torpedoes and turret rounds with real physics. Wells never pull each other, and the strength is authored so every well is escapable under main drive.

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

A well accelerates you toward its center by an inverse-square law, `a = mu / r^2`, where `mu` comes from the body's authored surface gravity and radius (never your mass - gravity is acceleration, so a stripped fighter and a laden hauler fall the same). Two things tame the extremes:

- **Surface clamp** - just above the surface the pull is held at its surface value, so there are no singularity slingshots.
- **Faded edge** - across the outer ~15% of the sphere of influence the pull smoothly tapers to zero, and is exactly zero beyond it, so there is no force discontinuity at the boundary.

## Sphere of influence

Each well reaches out to a sphere of influence about eight times the body's radius - a 20 u rock pulls out to roughly 160 u. Outside it, the well does not exist as far as your ship is concerned.

## The dominant well

Where two spheres of influence overlap, the pulls do not blend: you feel only the **dominant** well - the strongest at your position - and it keeps ownership until a challenger clearly beats it (a ~10% hysteresis margin) so it does not flicker at the boundary. The dominant well is exactly what the [ORBIT](../flight-autopilot/) autopilot circularizes around, flying a stable ring at orbital speed `v = sqrt(mu / r)`. That single mechanic is what turns "fly to a point" into "manage your orbit".
