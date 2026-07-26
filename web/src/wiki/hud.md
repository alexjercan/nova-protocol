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

Every widget carries a tier: **Instrument** (velocity sphere, flight chips, autopilot marker, maneuver instruments, lead pips, lock crosshairs, the target inset, allegiance markers - shown at All and Minimal) or **Chrome** (the keybind hint cluster, verb cues, the component-lock panel, edge indicators, objective markers - shown only at All).

## Flight readouts

The flight instruments sit around the ship, not in a status bar:

- **Velocity sphere** - an orbiting cone and shaded sphere driven by your linear velocity; white and blue in manual flight, cyan when the autopilot is flying. A yellow variant shows the local gravity pull, hidden in flat space.
- **Speed and mode chips** - a speed readout (`u/s`, units per second; see the [glossary](../glossary/)) always beside the sphere, and a mode chip reading `AP GOTO - BURN` (verb and phase: STOP/GOTO/ORBIT and ALIGN/BURN/HOLD) only while the autopilot is engaged.
- **ORBIT ring and radius spoke** - while you hold an orbit, a world-space ring marks the orbit plane and a thin spoke runs from the well to your ship with the current radius.
- **Keybind hint cluster** - a lower-left column of verb rows (STOP, GOTO, ORBIT, CANCEL, RADAR, COMPONENT) that only shows a row while its verb can actually do something, and pulses gold when a scenario wants you to use it.

## Locks and reticles

Locks are slot-coloured: a **white crosshair** is your travel (nav) lock, a **red reticle** is your combat lock. The combat reticle carries a readout riding its right edge - range to the target (`DST`), closing speed (`CLS`) and a health bar - plus a focus meter that fills as a fine-lock dwell accumulates. While you hold the radar gesture a hollow box shows, coloured by the slot it will land in; a **white ring fills clockwise** around the target while the lock-on dwell charges, and vanishes with a cue the instant the lock snaps (sweep off before it fills to cancel). Clearing a lock pops a brief "unlatch" ghost. See [Targeting & radar](../targeting-radar/).

## Allegiance markers

A small filled triangle floats above every ship in view, pointing down at the hull and coloured by its side: **green** for your allies (your AI wingmen included), **red** for hostiles, **grey** for neutral bystanders. Your own ship shows none - you know where you are. The triangle tracks each ship and hides when it leaves the screen (pointing at off-screen ships is the edge indicators' job); a ship that turns hostile mid-fight (a neutral hauler provoked into a threat) flips its marker red on the spot. It reads at a glance so a mixed brawl - wingmen and enemies tangled together - stays legible. See [Factions](../factions/).

## Target viewfinder

The corner inset renders a live, magnified 3D view of your combat lock through a second (offscreen) camera. Its frame glows hot-red while your weapons are hot and steel while safe, with corner ticks that appear only when hot; a caption names the target and its relation. Bodies that cannot be scoped (nav beacons) show a **NO-SIGNAL** panel instead. When the framed target dies the inset freezes the final pose for about two seconds - a **kill cam** - then closes. The fine-locked section glows in both the inset and the main view.

## Comms and objectives

Scenarios talk to you through a **comms stack**: speaker-attributed story cards (`OKONO > Strip it clean.`) that rise from the bottom-left like a chat transcript. Several cards can stay visible at once; newest sits at the bottom, older lines push upward and fade. Each card has a speaker icon slot, using authored scenario art when supplied and a cockpit placeholder otherwise, fades in with a soft blip, and leaves by timeout or explicit dismiss. Press <kbd>V</kbd> to dismiss the oldest visible card, or <kbd>B</kbd> to skip queued backlog into view. A newly posted **objective gets the cockpit moment**: it appears slightly rotated on the HUD, holds for a couple of seconds, then tucks up-and-right into the small **objective hint** in the top-right corner. That hint is deliberately terse - an objective glyph, the active-objective count, and a `TAB` affordance - so at a glance you know there is work and how to see it; the full list lives in the ship-computer drawer. A **completed** objective ghosts green as it fades. Everything clears with the scenario rather than lingering over the menu.

## The ship-computer drawer

Press <kbd>Tab</kbd> (or click the right stick on a gamepad) to open the **NOVA OS** ship-computer monitor. Opening it pauses the game and frees the mouse cursor, so you can read at your own pace; <kbd>Tab</kbd> again (or <kbd>Esc</kbd>) closes it and resumes. The old side panels are now one inset cockpit screen with dark casing, a green phosphor display, scanline/glass treatment and orange/yellow accents. Current objectives and the combined **Flight Log** still use live drawer data inside the monitor: comms transcript rows and objective received/completed rows interleave like compact terminal output, while active objectives remain readable in their own block. Long Flight Log and Objectives runs scroll inside the monitor instead of spilling over the screen. Future commands and apps will take over this same monitor rather than adding permanent drawer sections. The terse objective hint in the top-right corner is the in-flight cue that the drawer is there.

## The screen substrate

Every projected element - lock brackets, edge arrows, turret lead pips, objective markers - rides one shared system that anchors a UI node to a world point or entity, sizes it by fixed pixels or apparent (on-screen) size, and either hides off-screen targets or clamps them to the viewport edge with an arrow pointing back to them. Turret **lead pips** are small amber squares at each turret's computed intercept point, turning red when your weapons are hot.
