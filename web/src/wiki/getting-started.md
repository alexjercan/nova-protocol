# Your first flight

Nova Protocol is a build-and-fly space shooter. You take a modular ship into a scenario - an asteroid field with gravity wells, salvage, and hostile ships. Everything moves under real Newtonian physics - momentum persists and nothing dampens you, so you fly the ship, not a cursor. The flight computer can fly for you through real thrusters, but a manual burn or RCS input hands control back to you. This page is the whole first flight: how to launch, the core gestures, Basic Training beat by beat, and where to go next.

## Launch and start

The game boots into a main menu. **New Game** drops you into **Basic Training** - the Fleet gunnery range, flown in a ready-made armed trainer, so there is nothing to build first. The other doors can wait.

<details class="explain">
<summary>Show the full menu rundown</summary>

- **New Game** - drops you into **Basic Training**, the Fleet gunnery range, flown in a ready-made armed trainer, so there is nothing to build first.
- **Sandbox** - opens the ship editor so you can build a ship and test-fly it in a practice scenario.
- **Scenarios** - opens the complete scenario picker, every scenario your enabled mods ship included.
- **Mods** - opens the installed-mod and online-catalog browser.
- **Settings** - adjusts volume, graphics quality, and UI skin, and shows the control reference.
- **Exit** - quits (hidden in the browser build).

</details>

<figure class="figure">
    <!-- Capture: assets/tutorial-menu.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-menu.png</span>
        <span class="figure__placeholder-note">The main menu over its live backdrop: a section-built gunship holding station in a rock band, with the New Game / Sandbox / Scenarios / Mods / Settings / Exit options beside it.</span>
    </div>
    <figcaption class="figure__caption">The game boots into a main menu; New Game starts Basic Training.</figcaption>
</figure>

In any scenario, <kbd>Esc</kbd> pauses the game and gives you Resume / Retry (restart the current scenario) / Settings / Back to Main Menu / Exit.

Pick **New Game**. Basic Training teaches one gesture at a time and hands you each verb only when you reach the beat that needs it - so a key that answers with a deny buzz early on just is not unlocked yet. Each beat completes as soon as the range sees the gesture done.

## The first two minutes

Distances read in meters and kilometers and speeds in meters per second (`m/s`); see the [glossary](../glossary/).

- **Burn.** Hold <kbd>W</kbd> (or <kbd>Space</kbd>) for the main-drive burn and aim with the mouse. You point the hull, then thrust where the nose looks. The velocity sphere beside your ship shows where you are actually going.
- **Lock a target.** Hold <kbd>Ctrl</kbd> to sweep the radar; the hollow box shows what you are about to lock. Settle on it and hold steady for a moment - a short lock-on dwell has to fill (longer the farther the target is) before the lock latches, and sweeping off before then cancels it. A white crosshair is a travel (nav) lock; raise weapons first (see below) and it lands as a red combat lock instead.
- **GOTO.** With a lock, tap <kbd>G</kbd> and the autopilot flies you there - it burns over, flips, and coasts to a stop just off the target. Any manual input hands the ship straight back to you.
- **Raise weapons and fire.** Hold the right mouse button to raise weapons (combat stance); your reticle goes red and the ship is now "hot". Left mouse fires the turrets and launches torpedoes. A torpedo only launches while you hold a red combat lock.

That is the whole core loop: burn, lock, GOTO, shoot. Basic Training covers all of it on an armed trainer, ORBIT included.

## Basic Training, beat by beat

You open on the Fleet gunnery range in an exterior shot of **Trainer Seven**, an armed picket held on the line, while Range Control reads the qualification card: fly the pattern, take the flight computer out to the planetoid and back, put five target hulks on the scrap list, then deal with two live drones. The camera comes back with the helm, each objective arrives as a short amber notification, and one gold marker at a time keeps the current target in view. A 150 m/s manual-flight cap stays on for the whole card.

### Part 1 - The pattern

1. **Burn to mark ALPHA.** Hold <kbd>W</kbd> to burn and steer with the mouse. ALPHA sits dead ahead; flying into its ring completes the beat.
2. **Kill the drift.** Press <kbd>X</kbd> once and let STOP bring the trainer to rest. Hands off: the next lesson waits for the maneuver to finish, and a manual input takes the ship back before it is done.
3. **Slide across to BRAVO.** Hold <kbd>Shift</kbd> and move the mouse to translate without turning the hull - short taps, not a held push. The velocity ball on your HUD goes violet while the thrusters have the ship, and RCS tops out at 100 m/s in any direction.

### Part 2 - The flight computer

4. **Travel-lock the planetoid.** Keep weapons down, put the planetoid off the far corner in front of you and hold <kbd>Ctrl</kbd> until the white travel lock takes. A red lock means weapons were up: it is the gun's lock, and the computer flies only the white one.
5. **GOTO.** Press <kbd>G</kbd> and the autopilot flies the leg: it burns over, flips, and stops 500 m off the rock. Hands off until it is parked; a manual input takes the ship back, and the card waits for you to lock and press <kbd>G</kbd> again.
6. **ORBIT.** Parked inside the planetoid's pull, press <kbd>O</kbd>. The computer circularises and holds the orbit; the beat completes when it calls the orbit stable.
7. **Home to CHARLIE.** Mark CHARLIE goes up on the line. Travel-lock it - wait for it to come round if the planetoid is in the way - and press <kbd>G</kbd>. The computer drops the orbit on its own. Flying home by hand counts too: CHARLIE's ring closes the beat.

### Part 3 - The line

<figure class="figure">
    <!-- Capture: assets/tutorial-radar-lock.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/tutorial-radar-lock.png</span>
        <span class="figure__placeholder-note">The red combat crosshair mid-sweep while holding CTRL with weapons raised, with the lock brackets snapping onto Target 1.</span>
    </div>
    <figcaption class="figure__caption">Locking is deliberate: raise weapons, hold CTRL, and the radar locks whatever you look at.</figcaption>
</figure>

8. **Lock Target 1.** Hold the right mouse button to raise weapons, put Target 1 in front of you and hold <kbd>Ctrl</kbd> until the red combat lock takes. A white lock means you swept with weapons lowered: it is a travel lock, it feeds no gun, and Range Control asks you to sweep again.
9. **Put rounds into it.** Hold the left mouse button and the gun follows the lock. Keep the rounds on the hulk until it comes apart. A cadet who shoots Target 1 apart before the lock lands skips the lesson; the card moves on either way.
10. **Clear the line.** Four more hulks are marked. Work them in any order; the range tallies each one as it breaks up.

### Part 4 - Something that shoots back

11. **Two range drones go live.** They sat neutral through the whole card. Now they turn hostile, come for you, and shoot back. Defeat both - destroyed or disarmed, either counts. A crippled drone that coasts off the range counts too: Range Control calls the recovery tug, and you are not asked to chase it.
12. **Range is cold.** Range Control logs the qualification, welcomes you to the Fleet, and points you at the Scenarios board for your first posting. Breaking the trainer up, or losing its gun, ends the card with a Retry on the same range.

(Tap <kbd>Ctrl</kbd> to clear a lock at any time.)

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
