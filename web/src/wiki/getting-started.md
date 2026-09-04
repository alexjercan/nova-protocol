# Your first flight

Nova Protocol is a build-and-fly space shooter. You take a modular ship into a scenario - an asteroid field with gravity wells, salvage, and hostile ships. Everything moves under real Newtonian physics - momentum persists and nothing dampens you, so you fly the ship, not a cursor. The flight computer can fly for you through real thrusters, but a manual burn or RCS input hands control back to you. This page is the whole first flight: how to launch, the core gestures, First Shift beat by beat, and where to go next.

## Launch and start

The game boots into a main menu. **New Game** drops you into **First Shift** - the campaign opening, flown in a ready-made maintenance cutter, so there is nothing to build first. The other doors can wait.

<details class="explain">
<summary>Show the full menu rundown</summary>

- **New Game** - drops you into **First Shift**, the campaign opening, flown in a ready-made ship, so there is nothing to build first.
- **Sandbox** - opens the ship editor so you can build a ship and test-fly it in a practice scenario.
- **Scenarios** - opens the complete scenario picker.
- **Mods** - opens the installed-mod and online-catalog browser.
- **Settings** - adjusts volume, graphics quality, and UI skin, and shows the control reference.
- **Exit** - quits (hidden in the browser build).

</details>

<figure class="figure">
    <!-- Capture: assets/tutorial-menu.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-menu.png</span>
        <span class="figure__placeholder-note">The main menu with its live ambient backdrop (an AI ship flying a thruster orbit) and the New Game / Sandbox / Scenarios / Mods / Settings / Exit options.</span>
    </div>
    <figcaption class="figure__caption">The game boots into a main menu; New Game starts First Shift.</figcaption>
</figure>

In any scenario, <kbd>Esc</kbd> pauses the game and gives you Resume / Retry (restart the current scenario) / Settings / Back to Main Menu / Exit.

Pick **New Game**. First Shift teaches one gesture at a time and hands you each verb only when you reach the beat that needs it - so a key that answers with a deny buzz early on just is not unlocked yet. Each beat completes the instant the gesture lands.

## The first two minutes

Distances read in meters and kilometers and speeds in meters per second (`m/s`); see the [glossary](../glossary/).

- **Burn.** Hold <kbd>W</kbd> (or <kbd>Space</kbd>) for the main-drive burn and aim with the mouse. You point the hull, then thrust where the nose looks. The velocity sphere beside your ship shows where you are actually going.
- **Lock a target.** Hold <kbd>Ctrl</kbd> to sweep the radar; the hollow box shows what you are about to lock. Settle on it and hold steady for a moment - a short lock-on dwell has to fill (longer the farther the target is) before the lock latches, and sweeping off before then cancels it. A white crosshair is a travel (nav) lock; raise weapons first (see below) and it lands as a red combat lock instead.
- **GOTO.** With a lock, tap <kbd>G</kbd> and the autopilot flies you there - it burns over, flips, and coasts to a stop just off the target. Any manual input hands the ship straight back to you.
- **Raise weapons and fire.** Hold the right mouse button to raise weapons (combat stance); your reticle goes red and the ship is now "hot". Left mouse fires the turrets and launches torpedoes. A torpedo only launches while you hold a red combat lock.

That is the whole core loop: burn, lock, GOTO, shoot. First Shift walks you through the flying half of it - the cutter it hands you carries no gun, so the last gesture waits for a ship that does (the [sandbox](#the-sandbox) range, or a scenario that arms you).

## First Shift, beat by beat

You open on the flank of the industrial carrier **Meridian**, undocked and drifting, while the Deck Chief clears you for an ordinary day's work on the rock plate. You are the cutter's captain and you have a crew: the copilot in the seat beside you and the engineer down the back are the other two voices. The chief's lines carry the story; each objective arrives as a short amber notification a beat later, and one gold marker at a time keeps the current target in view.

### Part 1 - Burn, then trim

1. **Burn to the work mark, then stop.** Hold <kbd>W</kbd> to burn. On arrival, hold <kbd>X</kbd> until STOP has brought Cutter fully to rest; the next lesson waits for the physical maneuver to finish. A gentle 150 m/s manual-flight cap remains active for the whole shift.
2. **Fly the four-mark trim box.** Your fine thrusters unlock here, in open space, with nothing to hit. The camera first shows all four marks together, then returns control and highlights only the current corner. Hold <kbd>Shift</kbd> and move the mouse to translate without turning the ship - short taps, not a held push. The velocity ball on your HUD goes violet while the thrusters have the ship, and RCS runs out at 100 m/s in any direction. Fly across to A, up to B, back across to C, then down to D where you started.

### Part 2 - The plate

3. **Recover the crates, one at a time.** The first sits on the plate's open edge where the rocks are sparse; the second is well inside them, where they are not. Fly through a crate to collect it. The next one is not shown until the one before it is aboard.

### Part 3 - Lock and let the computer fly

<figure class="figure">
    <!-- Capture: assets/tutorial-radar-lock.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-radar-lock.png</span>
        <span class="figure__placeholder-note">The white NAV crosshair mid-sweep while holding CTRL, with the lock brackets snapping onto the first transit mark.</span>
    </div>
    <figcaption class="figure__caption">Radar locking is deliberate: hold CTRL and the radar locks whatever you look at.</figcaption>
</figure>

4. **Lock the transit mark.** Your targeting computer comes online. Hold <kbd>Ctrl</kbd> on the mark out west until the white NAV lock sticks - holding <kbd>Ctrl</kbd> sweeps and live-locks whatever your look ray is on.
5. **Press GOTO.** With the mark locked, press <kbd>G</kbd> and let the computer fly the leg. It burns over, flips, and coasts to a stop just off the mark. The temporary mark remains until the maneuver has settled. Then the chief gives you a second one and says almost nothing about it - the same two keys, on your own.

### Part 4 - The detour

<figure class="figure">
    <!-- Capture: assets/tutorial-orbit.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-orbit.png</span>
        <span class="figure__placeholder-note">The ship flying a clean ORBIT circle around the planetoid, with the ORBIT radius spoke shown on the HUD.</span>
    </div>
    <figcaption class="figure__caption">Press ORBIT near a gravity well and the ship flies itself into a clean circle.</figcaption>
</figure>

6. **Take the crew up on it.** Nobody assigned this. The copilot points out the survey body sitting there with a crate still outstanding, and the engineer is not going to talk you out of it. GOTO across.
7. **Press ORBIT and hold it.** Press <kbd>O</kbd> and the ship parks itself into a clean circle around the well. Hold it for about thirteen seconds - long enough for the crew to enjoy it, and for the Meridian to notice.
8. **Get back on the plate.** Meridian Control has been watching. The carrier lifts within the hour, the last crate is still out there, and finishing is what the bonus is for. Fly back to the work site.

### Part 5 - What you cannot stop

9. **Collect the last crate, then run home.** The sheet is clean and the chief calls you in. Hold on the outer mark three kilometres off the Meridian - the last GOTO of the shift, and the only thing the rest of the chapter is waiting for.
10. **A warship comes out from behind the large planetoid.** It crosses into plain sight, closes to a firing position, turns its whole hull onto your carrier, walks six siege torpedoes out of its bays and fires both railguns together. It takes over a minute to arrive and there is nothing here to win. The game means it.
11. **The staged shots hold your controls.** The camera takes five shots across three short cinematic intervals, including the RCS briefing. Flight, look, stance, and weapon input pause while a shot owns the view; pause and menu controls still work. The camera and controls return after the briefing, for the warship's long middle leg, and before the aftermath. The script never flies Cutter for you.
12. **Keep the channel open.** Nobody answers. What finally does is an automated distress beacon, and that is the hand-off: the victory screen continues straight into **Second Shift**, where you go back to the same belt for what is left of the ship.

(Tap <kbd>Ctrl</kbd> to clear a nav lock at any time.)

## The sandbox

<figure class="figure">
    <!-- Capture: assets/wiki-sandbox-range.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/wiki-sandbox-range.png</span>
        <span class="figure__placeholder-note">The sandbox free-flight range from the player ship: the rock belts ahead, target hulks to port, and the F1 back-to-editor objective chip visible.</span>
    </div>
    <figcaption class="figure__caption">The sandbox range: nothing to win, nothing to lose but the ship.</figcaption>
</figure>

**Sandbox** opens the ship editor. Build a hull, press **Play**, and you launch into a free-flight range with nothing to win and nothing to lose but the ship:

- **Rock belts**: a shallow one off to starboard, and a deeper wall of bigger rocks straight ahead.
- **Target hulks** off to port - inert bare-hull wrecks that never shoot back. Practice fire.
- **Pickets** further out. They sit neutral and ignore you until you paint one with a combat lock or fly too close, and then they fight. There is no way to un-wake one.
- **Beacons** that swap the sky as you pass through them - one out, one back.
- A **planetoid** off to port with a real gravity well. It is far enough that you have to go looking for it.

Nothing here ends. The objective on your HUD just names the way out: <kbd>F1</kbd> returns to the editor at any time, and if you die the overlay offers **Retry** on the same range with the same ship.

## Where to go next

That is everything you need to get off the launch pad. The rest of the wiki is the full reference:

- [Flight & autopilot](../flight-autopilot/) - how ships move and what GOTO, ORBIT and STOP each do.
- [Gravity wells](../gravity-wells/) - the pull every body in a scene exerts, and how to fly it.
- [Ships & damage](../ships/) - what a hull is made of and how it comes apart.
- [Targeting & radar](../targeting-radar/) - deliberate locking, stances, and per-section fine-lock.
- [Combat](../combat-weapons/) - the engagement ladder and the rules every weapon shares.
- [Ship sections](../sections/) - what each part of a ship does, one page per part.
- [Keybinds](../keybinds/) - the complete control reference for keyboard and gamepad.
- [Glossary](../glossary/) - the recurring terms and units in one place.

<p style="margin-top: 2.5em"><a href="../../play/" class="btn btn--primary">Launch the game</a></p>
