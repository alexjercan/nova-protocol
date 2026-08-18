# HUD

The heads-up display is diegetic - the instruments read the ship's real state - and contextual: elements surface while their situation is live and settle back when it passes, so a quiet cruise has a quiet screen without you managing it.

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
            + mode chips, the keybind icon dock, a lock reticle
            and the corner target inset, ideally lightly
            annotated.</span
        >
    </div>
</figure>

## What is on screen, and when

The HUD is CONTEXTUAL: it shows you what the situation calls for and gets out of the way when it does not. Idle cruise keeps a quiet screen - the velocity sphere, your speed, the dock's few live verbs, the always-on ship markers and the status bar - and everything else arrives with its moment.

<div class="widget" data-widget="hud-context">
<p>The situations and what each brings up: an engaged autopilot adds the mode chip and (for GOTO and ORBIT) the destination marker, and grows the speed chip; a combat lock adds the red reticle with its DST/CLS readout and the target viewfinder, and inverts the dock's RADAR chip; hot weapons raise the ammo gauges and redden the lead pips; a nearly-dry or reloading group forces the gauges up on its own; Cinematic clears every element.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

- **Autopilot burn** - the mode chip comes up, the speed chip grows (it is the number you are flying by), and the dock lights the maneuver you are flying. GOTO and ORBIT also put the destination marker and its readout up; STOP flies without one - there is no destination to mark.
- **Combat lock** - the red reticle, its DST/CLS readout and the target viewfinder come up, and the RADAR chip inverts because the lock is the thing you would change.
- **Weapons hot** - the ammo gauges appear on your weapons and the lock readout grows. With the trigger down the reticle pulses.
- **Low ammo or reloading** - a nearly-dry group (a quarter magazine or less) pulses amber and forces the gauges up on its own, even with the safety on: a dry magazine is news before you pull the trigger. An active reload holds them up the same way, with its own pulse.
- **A posted objective** - a chip carrying the objective itself pops into the stack at the top of the screen the moment it posts, like a chat notification, and then keeps a slow breath. The chip is a notification: it leaves once you have read it - after a dwell, or the moment you open NOVA OS.
- **An incoming transmission** - the comms card arrives grown and settles while it holds.

</details>

Grave / tilde (or the gamepad Select button) toggles the whole display between two levels:

- **On** - the contextual HUD above.
- **Cinematic** - a clean screen for captures and quiet flying.

There is no third "minimal" level any more: showing everything all the time is what made a manual detail dial useful, and the contextual rules do that job continuously instead.

Every widget still declares its kind - **Instrument** (velocity sphere, flight chips, autopilot marker, maneuver instruments, lead pips, lock crosshairs, allegiance markers), **Chrome** (the keybind dock, verb cues, the component-lock panel, edge indicators, objective markers, the target inset) or **Status** (the fps/version bar) - and all of them clear at Cinematic.

## Flight readouts

The flight instruments sit around the ship, not in a status bar:

- **Velocity sphere** - an orbiting cone and shaded sphere driven by your linear velocity; white and blue in manual flight, cyan when the autopilot is flying, violet while you hold RCS fine adjust (the violet wins over the cyan). A yellow variant shows the local gravity pull, hidden in flat space.
- **Speed and mode chips** - a speed readout (`m/s`, metres per second; see the [glossary](../glossary/)) always beside the sphere, and a mode chip reading `AP GOTO - BURN` (verb and phase: STOP/GOTO/ORBIT and ALIGN/BURN/HOLD) only while the autopilot is engaged.
- **ORBIT ring and radius spoke** - while you hold an orbit, a world-space ring marks the orbit plane and a thin spoke runs from the well to your ship with the current radius.
- **Keybind dock** - a row of icon chips along the bottom of the screen showing the flight verbs you can use RIGHT NOW, drawn from STOP, GOTO, ORBIT, CANCEL, RADAR, COMPONENT and RCS. A verb that would do nothing at this moment is not on the dock at all, so the row grows and shrinks with the situation instead of parking a wall of dead keys under your ship. Each chip shows the real KEYCAP for the key that drives it plus the verb word, in full phosphor - or inverted while that verb is what the ship is doing (an engaged ORBIT keeps its chip even though you can no longer start one). A chip pulses gold when a scenario wants you to use it, and a spotlight will show a chip that has not lit up yet - that is how a tutorial points at a key before you can press it. The anchored **verb cues** are the same chip parked on the thing you would act on - the ORBIT keycap on a gravity well, the GOTO keycap on your aim lock.

## Locks and reticles

Locks are slot-coloured: a **white crosshair** is your travel (nav) lock, a **red reticle** is your combat lock. The combat reticle carries a readout riding its right edge - range to the target (`DST`), closing speed (`CLS`) and a health bar (measured against the hull the target was BUILT with, so it falls as you take sections off and never refills) - plus a focus meter that fills as a fine-lock dwell accumulates. While you hold the radar gesture a hollow box shows, coloured by the slot it will land in; a **white ring fills clockwise** around the target while the lock-on dwell charges, and vanishes with a cue the instant the lock snaps (sweep off before it fills to cancel). Clearing a lock pops a brief "unlatch" ghost. See [Targeting & radar](../targeting-radar/).

## Allegiance markers

A small filled triangle floats above every ship in view, pointing down at the hull and coloured by its side: **green** for your allies (your AI wingmen included), **red** for hostiles, **grey** for neutral bystanders. Your own ship shows none - you know where you are. The triangle tracks each ship and hides when it leaves the screen (pointing at off-screen ships is the edge indicators' job); a ship that turns hostile mid-fight (a neutral hauler provoked into a threat) flips its marker red on the spot. It reads at a glance so a mixed brawl - wingmen and enemies tangled together - stays legible. See [Factions](../factions/).

## Target viewfinder

The corner inset renders a live, magnified 3D view of your combat lock through a second (offscreen) camera. Its frame glows hot-red while your weapons are hot and steel while safe, with corner ticks that appear only when hot; a caption names the target and its relation. Bodies that cannot be scoped (nav beacons) show a **NO-SIGNAL** panel instead. When the framed target dies the inset freezes the final pose for about two seconds - a **kill cam** - then closes. The fine-locked section glows in both the inset and the main view.

## Comms and objectives

Scenarios talk to you through a **comms stack**: speaker-attributed story cards (`OKONO > Strip it clean.`) that rise from the bottom-left like a chat transcript, and a newly posted **objective** arrives the same way in the **objective stack** at the top of the screen - a column of amber chips, newest on top. Both are notifications, not standing lists: each leaves once it has been read, so a quiet cockpit stays quiet.

<details class="explain">
<summary>Show explanation</summary>

Several comms cards can stay visible at once; newest sits at the bottom, older lines push upward and fade. Each card has a speaker icon slot, using authored scenario art when supplied and a cockpit placeholder otherwise, fades in with a soft blip, and leaves by timeout or explicit dismiss. Press <kbd>V</kbd> to dismiss the oldest visible card, or <kbd>B</kbd> to pull queued backlog into view once the visible stack is full.

An objective chip carries the objective itself behind a diamond; it pops the moment its objective posts and then breathes quietly, and it is read either after its dwell or the instant you open NOVA OS. For the standing list, type `objectives` in NOVA OS (or `log` for the combined comms/objective event history); in flight, the gold **objective markers** on the targets themselves are the lasting "go here" cue. A **completed** objective ghosts green as it fades. Everything clears with the scenario rather than lingering over the menu.

</details>

## The ship computer

Press <kbd>Tab</kbd> (or click the right stick on a gamepad) to open the **NOVA OS** ship-computer monitor: a real CRT terminal that pauses the game, frees the cursor, and answers `help`, `log`, `objectives`, `ship`, `map`, `clear` and `exit`. <kbd>Esc</kbd> (or `exit`) closes it and resumes flight.

<details class="explain">
<summary>Show explanation</summary>

The old side panels are now one inset cockpit screen with dark casing, a rounded green phosphor display, and a real CRT tube: the terminal is rendered to an offscreen image and shown through a single screen shader, so the bright green text blooms into a soft phosphor halo and the whole picture bows with gentle barrel curvature, over subtle square grain, soft scanlines, a glass sheen and an edge vignette. Opening the computer blooms the raster on from a single scan line and closing collapses it to a dying dot before flight resumes. Live ship-name topbar status, footer hints and orange/yellow accents complete the panel.

The command line sits in a dark input strip above the screen and shows a fish-style inline completion that continues on the same line as you type; <kbd>Tab</kbd> completes commands, and mistypes get close-match (`did you mean`) suggestions. It starts with a boot welcome block and keeps command history and cursor editing. Command output scrolls to the bottom automatically; `clear` restores the welcome block instead of leaving the screen blank. `log` prints comms plus objective events, and `objectives` prints the active mission list.

`ship view` prints live player-ship section status including weapons, thrusters, ammo where present, and critical or neutralized state; bare `ship` opens a **schematic viewer app** that swallows the monitor and renders your ship as an orbitable green-phosphor 3D schematic, its sections labelled with short codes (`HULL-1`, `PDC-1`, `TRB-1`) you can select by clicking a block or cycling with <kbd>[</kbd>/<kbd>]</kbd>. Sections are addressed by those codes from the CLI too: `ship section <id>` shows one section's detail, and `ship reload <id>` / `ship repair <id>` act on it (also on the <kbd>L</kbd>/<kbd>P</kbd> keys inside the app; Tab completes the code). A bindable selected section also shows its current inputs; <kbd>B</kbd> captures one replacement keyboard key or mouse button, while <kbd>Esc</kbd> cancels. Several sections can share one input; reserved flight controls remain unavailable.

`map view` prints local-space contacts as the same fixed-width KIND/LABEL/INFO table, each contact carrying a short unique label (`SELF` for your ship, then `HOST-1`, `ALLY-1`, `OBJ-1`, `AST-1`) with its range and bearing; `map goto <label>` flies the ship to that contact by engaging the flight autopilot (Tab completes the label). Bare `map` opens the schematic local-space minimap app, where the same labels ride on each contact blip and <kbd>G</kbd> sets GOTO on the selected one. Apps swallow this same monitor while they run and hand it back on <kbd>Esc</kbd>, rather than adding permanent side panels. A posted objective's chip carries a `TAB` cue while it is up, which is the in-flight reminder that the computer is there.

</details>

## The screen substrate

Every projected element - lock brackets, edge arrows, turret lead pips, objective markers - rides one shared system that anchors a UI node to a world point or entity, sizes it by fixed pixels or apparent (on-screen) size, and either hides off-screen targets or clamps them to the viewport edge with an arrow pointing back to them. Turret **lead pips** are small amber squares at each turret's computed intercept point, turning red when your weapons are hot.
