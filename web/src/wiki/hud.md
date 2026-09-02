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
<p>The situations and what each brings up: an engaged autopilot adds the mode chip and (for GOTO and ORBIT) the destination marker, and grows the speed chip; a combat lock adds the red reticle with its DST/CLS readout and the target viewfinder, and inverts the dock's RADAR chip; hot weapons raise the ammo gauges, redden the lead pips and put up a railgun's bore sight; a nearly-dry or reloading group forces the gauges up on its own; Cinematic clears every element.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

- **Autopilot burn** - the mode chip comes up, the speed chip grows (it is the number you are flying by), and the dock lights the maneuver you are flying. GOTO and ORBIT also put the destination marker and its readout up; STOP flies without one - there is no destination to mark.
- **Combat lock** - the red reticle, its DST/CLS readout and the target viewfinder come up, and the RADAR chip inverts because the lock is the thing you would change.
- **Weapons hot** - the ammo gauges appear on your weapons and the lock readout grows. With the trigger down the reticle pulses. A hull carrying a railgun draws its [bore sight](#bore-sight).
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

Locks are slot-coloured: a **white crosshair** is your travel (nav) lock, a **red reticle** is your combat lock. The combat reticle carries a readout riding its right edge - range to the target (`DST`), closing speed (`CLS`) and a health bar (measured against the hull the target was BUILT with, so it falls as you take sections off and never refills) - plus a focus meter that fills as a fine-lock dwell accumulates. Lock an asteroid and the bar is simply absent: a rock has no health to draw, and how much of it is left is something you read off the rock itself. While you hold the radar gesture a hollow box shows, coloured by the slot it will land in; a **white ring fills clockwise** around the target while the lock-on dwell charges, and vanishes with a cue the instant the lock snaps (sweep off before it fills to cancel). Clearing a lock pops a brief "unlatch" ghost. See [Targeting & radar](../targeting-radar/).

## Bore sight

<!-- Behavior verified against crates/nova_hud/src/bore_sight.rs: gated on WeaponsHot; the trace walks sections through the same `pierce_remainder` the round uses; a MARK_RADIUS 0.55 ring per section it would destroy; LINE_RADIUS 0.03 thickened CHARGE_THICKEN 2.4x at full charge; EMPTY_ALPHA_SCALE 0.3 on an empty magazine. -->

A hull carrying a [railgun](../sections/railgun/) has one more instrument, because nothing else on the screen says where a hull is pointing: a thin **sight line** in the slug's own steel blue, out of the muzzle and ending exactly where the slug would end, with a **ring on every section that shot would destroy**. It is up whenever your weapons are hot, thickens as a charge runs, and stays up dimmed through the reload so the twelve seconds can be spent aiming.

<figure class="figure">
    <!-- Capture: assets/wiki-section-railgun-sight.png (shared with the Railgun page) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-railgun-sight.png</span
        >
        <span class="figure__placeholder-note"
            >The sight line out of the muzzle onto a corvette,
            a ring on each section the shot would gut.</span
        >
    </div>
</figure>

## Allegiance markers

A small filled triangle floats above every ship in view, pointing down at the hull and coloured by its side: **green** for your allies (your AI wingmen included), **red** for hostiles, **grey** for neutral bystanders. Your own ship shows none - you know where you are. The triangle tracks each ship and hides when it leaves the screen (pointing at off-screen ships is the edge indicators' job); a ship that turns hostile mid-fight (a neutral hauler provoked into a threat) flips its marker red on the spot. It reads at a glance so a mixed brawl - wingmen and enemies tangled together - stays legible. See [Factions](../factions/).

## Target viewfinder

The corner inset renders a live, magnified 3D view of your combat lock through a second (offscreen) camera. Its frame glows hot-red while your weapons are hot and steel while safe, with corner ticks that appear only when hot; a caption names the target and its relation. Bodies that cannot be scoped (nav beacons) show a **NO-SIGNAL** panel instead. When the framed target dies the inset freezes the final pose for about two seconds - a **kill cam** - then closes. The fine-locked section glows in both the inset and the main view.

## Comms and objectives

Scenarios talk to you through a **comms stack**: speaker-attributed story cards (`OKONO > Strip it clean.`) that rise from the bottom-left like a chat transcript, and a newly posted **objective** arrives the same way in the **objective stack** at the top of the screen - a column of amber chips, newest on top. Both are notifications, not standing lists: each leaves once it has been read, so a quiet cockpit stays quiet.

<details class="explain">
<summary>Show explanation</summary>

Several comms cards can stay visible at once; newest sits at the bottom, older lines push upward and fade. Each card has a speaker icon slot, using authored scenario art when supplied and a cockpit placeholder otherwise, fades in with a soft blip, and leaves by timeout or explicit dismiss.

An objective chip carries the objective itself behind a diamond; it pops the moment its objective posts and then breathes quietly, and it is read either after its dwell or the instant you open [NOVA OS](../nova-os/). For the standing list, type `objectives` in NOVA OS (or `log` for the combined comms/objective event history); in flight, the gold **objective markers** on the targets themselves are the lasting "go here" cue. A **completed** objective ghosts green as it fades. Everything clears with the scenario rather than lingering over the menu.

</details>

## The ship computer

Press <kbd>Tab</kbd> (or click the right stick on a gamepad) to open the **NOVA OS** ship-computer monitor: a real CRT terminal that pauses the game, frees the cursor, and answers `help`, `log`, `objectives`, `ship`, `map`, `clear` and `exit`. <kbd>Esc</kbd> (or `exit`) closes it and resumes flight. A posted objective's chip carries a `TAB` cue while it is up - the in-flight reminder that the computer is there.

The computer has [its own page](../nova-os/): the full command reference, the MAP and SHIP apps, rebinding a section's controls, and the monitor's own knobs and sounds.

## The screen substrate

Every projected element - lock brackets, edge arrows, turret lead pips, objective markers - rides one shared system that anchors a UI node to a world point or entity, sizes it by fixed pixels or apparent (on-screen) size, and either hides off-screen targets or clamps them to the viewport edge with an arrow pointing back to them. Turret **lead pips** are small amber squares at each turret's computed intercept point, turning red when your weapons are hot.
