# Devlog #5: radar locking, a tutorial, and a home on the web

v0.4.0 gave Nova Protocol combat, an AI to fight, and a lot of automatic targeting. v0.5.0 takes that targeting and hands the controls back to the player - deliberately. It also does the thing every project eventually has to do: it teaches a new player how to play, and it puts up a front door on the web. So this release lands on locking, a tutorial, damage with texture, and a proper home page.

## Locking is now a decision

The biggest change in feel: all the passive auto-targeting from the last release is gone, replaced by **deliberate radar locking**. You hold CTRL to sweep, and the radar live-locks whatever you are looking at. Your _stance_ picks the slot: lowered gives a white NAV crosshair that feeds GOTO, raised gives a red combat reticle that feeds guns, torpedoes and fine-lock. Tapping CTRL clears the lock in stages - combat first, then nav - and a lock sticks until the target dies, leaves range, or goes cold. Crucially, LOCK is now framed as a _ship-computer capability_, the same way GOTO is: it is something the ship does for you, not a magic HUD overlay.

The lock language got clarified to match: a red bracket is a combat lock, white is a nav lock, and the old relation tints and reticle pips were retired. Turrets hold the combat lock even while you aim manually.

<figure class="figure">
    <!-- Capture: assets/devlog5-radar-stance-slots.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Image needed</span
        >
        <span class="figure__placeholder-name"
            >assets/devlog5-radar-stance-slots.png</span
        >
        <span class="figure__placeholder-note"
            >Side-by-side of the two locks: weapons-lowered
            white NAV crosshair vs. weapons-raised red combat
            reticle, each mid-sweep with CTRL held.</span
        >
    </div>
    <figcaption class="figure__caption">
        Your stance picks the slot: white NAV lock lowered, red
        combat lock raised.
    </figcaption>
</figure>

## The target viewfinder

To make a lock feel like something you _have_, v0.5.0 adds a **target viewfinder**: a corner inset that renders a live, magnified 3D view of your combat lock through a second camera. It shows a red armed frame while your weapons are hot, a NO-SIGNAL panel for bodies that cannot be scoped, and - my favorite touch - a roughly two-second freeze-frame **kill cam** when the target dies. The fine-locked section glows in both the main view and the inset, so you always know exactly what you are shooting at.

<figure class="figure">
    <!-- Capture: assets/devlog5-target-viewfinder.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Image needed</span
        >
        <span class="figure__placeholder-name"
            >assets/devlog5-target-viewfinder.png</span
        >
        <span class="figure__placeholder-note"
            >Full-frame gameplay showing the corner viewfinder
            inset with its red armed frame and the fine-locked
            section glowing. A kill-cam freeze-frame is an even
            better shot if you can catch one.</span
        >
    </div>
    <figcaption class="figure__caption">
        The corner viewfinder renders a live magnified view of
        your combat lock, right through to the kill cam.
    </figcaption>
</figure>

## Teaching the first twelve minutes

New Game now starts the **Shakedown Run**, a roughly twelve-beat tutorial that walks a new pilot through the whole vocabulary: burn, freelook, stop, salvage, GOTO, coasting through a gravity well, ORBIT, radar lock, a live-fire rehearsal on a derelict, and finally a real scavenger fight. Each beat teaches exactly one gesture and completes the instant that gesture lands, so it never blocks a player who already knows what they are doing. Building this shook out a long tail of playtest fixes - park points sized to their beacon triggers, orbit-hold completion, scavenger spawn timing and a combat leash, invulnerable planetoids, and readable objective pacing.

## Damage with a grain

Combat picked up some texture too. Damage is now **typed** - Kinetic, AP, EMP, Explosive - and checked against **per-section resistance tables**, so what you shoot matters as much as where you shoot it. Each turret carries a loaded-ammo slot that sets its rounds' type, with a color-coded ammo readout. The point-defense cannon was also retuned to actually be point defense: per-hit damage dropped from 20 to 4, so the stream chips a target down over a visible burst instead of deleting it in one frame.

## A game that boots into a menu

v0.5.0 is also the release where Nova Protocol starts to feel like a shipped game rather than a scene loader. It boots into a **main menu** with a live ambient backdrop - an AI ship flying a thruster-driven orbit - and there is an ESC pause menu, all living in a new `nova_menu` crate. The HUD learned **visibility levels** (grave/tilde cycles ALL, MINIMAL, NONE) for when you want a clean view. And the flight readouts went diegetic: speed and engaged-mode chips sit beside the velocity sphere, ORBIT shows a radius spoke, and the keybind cluster is now contextual, showing a verb's row only when it can actually do something.

Objectives got easier to follow, with a gold marker chip carrying live distance to the current target, glowing salvage crates, keybind emphasis pulses, and a completion chime. Underneath, the scenario system gained new primitives - nav beacons, salvage crates with authorable radar signatures, despawn-by-id, and events like OnOrbit, OnTravelLock and OnCombatLock - which is what made the Shakedown Run expressible as data.

## A home on the web

Finally, the thing you are reading this on. v0.5.0 ships a **web landing site** (the `web/` directory: TypeScript, Webpack and Tailwind), and the GitHub Pages deploy now serves it at the root with the playable game tucked under `/play/`. This devlog, the tutorial, and the wiki all live here now.

<p class="prose__meta">
    A day later, <strong>v0.5.1</strong> cleaned up two
    web-build issues that only bit the shipped WASM app: a
    render-target view-format override that WebGL2 does not
    support, and skybox cubemap meta-loader settings that had
    silently stopped applying - both of which could crash the
    game on New Game or Play.
</p>

## Where v0.5.0 lands

v0.5.0 turns automatic targeting into a deliberate skill, wraps it in a viewfinder and a kill cam, teaches a newcomer how to fly through the Shakedown Run, gives combat a typed-damage grain, and finally gives the whole project a front door. Next up: more of the game behind that door. Or you can just [go fly something](../../play/).
