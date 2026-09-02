# Your first flight

Nova Protocol is a build-and-fly space shooter. You take a modular ship into a scenario - an asteroid field with gravity wells, salvage, and hostile ships. Everything moves under real Newtonian physics - momentum persists and nothing dampens you, so you fly the ship, not a cursor. The flight computer can fly for you through real thrusters, but a manual burn or RCS input hands control back to you. This page is the whole first flight: how to launch, the core gestures, the Shakedown Run beat by beat, and where to go next.

## Launch and start

The game boots into a main menu. **New Game** drops you into the **Shakedown Run** - a guided tutorial scenario with a ready-made ship, so there is nothing to build first. The other doors can wait.

<details class="explain">
<summary>Show the full menu rundown</summary>

- **New Game** - drops you into the **Shakedown Run**, a guided tutorial scenario with a ready-made ship, so there is nothing to build first.
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
    <figcaption class="figure__caption">The game boots into a main menu; New Game starts the Shakedown Run.</figcaption>
</figure>

In any scenario, <kbd>Esc</kbd> pauses the game and gives you Resume / Retry (restart the current scenario) / Settings / Back to Main Menu / Exit.

Pick **New Game**. The Shakedown Run teaches one gesture at a time and hands you each verb only when you reach the beat that needs it - so a key that answers with a deny buzz early on just is not unlocked yet. Each beat completes the instant the gesture lands.

## The first two minutes

Distances read in meters and kilometers and speeds in meters per second (`m/s`); see the [glossary](../glossary/).

- **Burn.** Hold <kbd>W</kbd> (or <kbd>Space</kbd>) for the main-drive burn and aim with the mouse. You point the hull, then thrust where the nose looks. The velocity sphere beside your ship shows where you are actually going.
- **Lock a target.** Hold <kbd>Ctrl</kbd> to sweep the radar; the hollow box shows what you are about to lock. Settle on it and hold steady for a moment - a short lock-on dwell has to fill (longer the farther the target is) before the lock latches, and sweeping off before then cancels it. A white crosshair is a travel (nav) lock; raise weapons first (see below) and it lands as a red combat lock instead.
- **GOTO.** With a lock, tap <kbd>G</kbd> and the autopilot flies you there - it burns over, flips, and coasts to a stop just off the target. Any manual input hands the ship straight back to you.
- **Raise weapons and fire.** Hold the right mouse button to raise weapons (combat stance); your reticle goes red and the ship is now "hot". Left mouse fires the turrets and launches torpedoes. A torpedo only launches while you hold a red combat lock.

That is the whole core loop: burn, lock, GOTO, shoot. The Shakedown Run walks you through each beat in order.

## The Shakedown Run, beat by beat

You open drifting off the dock while Capt. Halloran briefs you through the comms stack - the run's first conversation, and the campaign's first look at your own voice. Her lines carry the story; each objective arrives as a short amber notification, while gold markers keep the current target in view. She keeps talking you through the run, with each line followed by breathing room before the next objective.

### Part 1 - Burn and look

1. **Burn to Beacon 1.** Hold <kbd>W</kbd> to burn, tap <kbd>X</kbd> to STOP. There is a gentle speed cap here that lifts once you arrive.
2. **Find Beacon 2.** It is off your beam. Hold <kbd>Alt</kbd> to free-look around and spot it, then burn over.
3. **Recover 3 supply crates** from the debris cluster - fly through each one to collect it.

### Part 2 - Lock and let the computer fly

<figure class="figure">
    <!-- Capture: assets/tutorial-radar-lock.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-radar-lock.png</span>
        <span class="figure__placeholder-note">The white NAV crosshair mid-sweep while holding CTRL, with the lock brackets snapping onto a beacon.</span>
    </div>
    <figcaption class="figure__caption">Radar locking is deliberate: hold CTRL and the radar locks whatever you look at.</figcaption>
</figure>

4. **Lock Beacon 3.** Your targeting computer comes online. Hold <kbd>Ctrl</kbd> on the beacon until the white NAV lock sticks. Radar locking is deliberate now: holding <kbd>Ctrl</kbd> sweeps and live-locks whatever your look ray is on.
5. **Press GOTO.** With the beacon locked, press <kbd>G</kbd> and let the computer fly you there.
6. **On to Beacon 4.** A new waypoint appears - lock it and press <kbd>G</kbd> again.

### Part 3 - Gravity and orbit

<figure class="figure">
    <!-- Capture: assets/tutorial-orbit.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-orbit.png</span>
        <span class="figure__placeholder-note">The ship flying a clean ORBIT circle around the planetoid, with the ORBIT radius spoke shown on the HUD.</span>
    </div>
    <figcaption class="figure__caption">Press ORBIT near a gravity well and the ship flies itself into a clean circle.</figcaption>
</figure>

7. **Coast.** You drift into the planetoid's pull - cut the burn and let gravity carry you.
8. **Press ORBIT.** Press <kbd>O</kbd> and the ship parks itself into a clean orbit around the well.
9. **Break away.** Press <kbd>Z</kbd> to cancel the autopilot and burn clear.

### Part 4 - First blood

<figure class="figure">
    <!-- Capture: assets/tutorial-combat-lock.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-combat-lock.png</span>
        <span class="figure__placeholder-note">Weapons raised (RMB): the red combat reticle locked on the derelict hulk, with the corner target viewfinder inset showing the magnified target.</span>
    </div>
    <figcaption class="figure__caption">Raise your weapons with RMB and the radar commits to a red combat lock; the viewfinder shows it live.</figcaption>
</figure>

10. **Paint the derelict hulk.** Hold the right mouse button to raise your weapons, keep <kbd>Ctrl</kbd> on the hulk, and watch the corner viewfinder. Raised weapons make the radar commit to a _red_ combat lock instead of the white nav one.
11. **Open fire.** Locked on, fire the turrets with the left mouse button.
12. **Drive off the scavenger.** Destroying the hulk draws a scavenger picking through your debris field - put it down.

Finish that and the Shakedown is complete: the victory screen offers to continue straight into **Broadside**, chapter two - a hauler's distress call, an ambush, and the gang's torpedo gunship. (Tap <kbd>Ctrl</kbd> to clear the combat lock; tap it again to clear the nav lock.)

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
