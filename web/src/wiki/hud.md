# HUD

The heads-up display is diegetic - the instruments read the ship's real state, and every widget knows which visibility tier it belongs to so you can strip the screen down for a clean shot or a quiet cockpit.

<figure class="figure">
    <!-- Capture: assets/wiki-hud.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-hud.png</span
        >
        <span class="figure__placeholder-note"
            >The full HUD in flight: velocity sphere, speed
            + mode chips, keybind cluster, a lock reticle
            and the corner target inset, ideally lightly
            annotated.</span
        >
    </div>
</figure>

## Visibility tiers

Grave / tilde (or the gamepad Select button) cycles the whole display through three levels, in order:

- **All** - everything: instruments plus chrome (the learning aids and secondary overlays).
- **Minimal** - the flight and combat _instruments_ only; the chrome drops away.
- **None** - a clean screen for cinematic captures.

Every widget carries a tier: **Instrument** (velocity sphere, flight chips, autopilot marker, maneuver instruments, lead pips, lock crosshairs, the target inset - shown at All and Minimal) or **Chrome** (the keybind hint cluster, verb cues, the component-lock panel, edge indicators, objective markers - shown only at All).

## Flight readouts

The flight instruments sit around the ship, not in a status bar:

- **Velocity sphere** - an orbiting cone and shaded sphere driven by your linear velocity; white and blue in manual flight, cyan when the autopilot is flying. A yellow variant shows the local gravity pull, hidden in flat space.
- **Speed and mode chips** - a speed readout (`u/s`) always beside the sphere, and a mode chip reading `AP GOTO - BURN` (verb and phase: STOP/GOTO/ORBIT and ALIGN/BURN/HOLD) only while the autopilot is engaged.
- **ORBIT ring and radius spoke** - while you hold an orbit, a world-space ring marks the orbit plane and a thin spoke runs from the well to your ship with the current radius.
- **Keybind hint cluster** - a lower-left column of verb rows (STOP, GOTO, ORBIT, CANCEL, RADAR, COMPONENT) that only shows a row while its verb can actually do something, and pulses gold when a scenario wants you to use it.

## Locks and reticles

Locks are slot-coloured: a **white crosshair** is your travel (nav) lock, a **red reticle** is your combat lock. The combat reticle carries a readout riding its right edge - range to the target (`DST`), closing speed (`CLS`) and a health bar - plus a focus meter that fills as a fine-lock dwell accumulates. While you hold the radar gesture a hollow box shows, coloured by the slot it will land in; clearing a lock pops a brief "unlatch" ghost. See [Targeting & radar](../targeting-radar/).

## Target viewfinder

The corner inset renders a live, magnified 3D view of your combat lock through a second (offscreen) camera. Its frame glows hot-red while your weapons are hot and steel while safe, with corner ticks that appear only when hot; a caption names the target and its relation. Bodies that cannot be scoped (nav beacons) show a **NO-SIGNAL** panel instead. When the framed target dies the inset freezes the final pose for about two seconds - a **kill cam** - then closes. The fine-locked section glows in both the inset and the main view.

## The screen substrate

Every projected element - lock brackets, edge arrows, turret lead pips, objective markers - rides one shared system that anchors a UI node to a world point or entity, sizes it by fixed pixels or apparent (on-screen) size, and either hides off-screen targets or clamps them to the viewport edge with an arrow pointing back to them. Turret **lead pips** are small amber squares at each turret's computed intercept point, turning red when your weapons are hot.
