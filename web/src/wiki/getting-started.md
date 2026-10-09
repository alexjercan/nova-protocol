# Your first flight

Nova Protocol is a build-and-fly space shooter. You take a modular ship into a scenario with asteroids, salvage, hostile ships, and planets that provide static gravity wells. Everything moves under real Newtonian physics - momentum persists and nothing dampens you, so you fly the ship, not a cursor. The flight computer can fly for you through real thrusters, but a manual burn or RCS input hands control back to you. This page is the whole first flight: how to launch, the core gestures, Basic Training beat by beat, and where to go next.

## Launch and start

The game boots into a main menu. On a first launch a card in the bottom-left corner offers you **Basic Training** straight away, with **Open lessons** beside it and **Not now** to put the card away for good. **Start Basic Training** puts you on the Fleet gunnery range in a ready-made armed trainer, so there is nothing to build first. The other doors can wait.

The card is a one-time offer, not the entrance: the menu's **Lessons** row always opens the handbook, whose first row is **Basic Training**, and **Settings > Interface > Training prompt** switches the corner back on if you want it.

Under it sits a **field note** - one short fact taken from the handbook, with **Open lesson** to read the page it came from and **Don't show again** to stop the notes appearing on the menu. It changes each time you come back to the menu, and the same notes fill the slot at the bottom of every loading screen, which that switch does not touch. While a practice range loads, the note is about what that range teaches. **Settings > Interface > Field notes** switches the menu card back on.

<details class="explain">
<summary>Show the full menu rundown</summary>

- **New Game** - opens the world setup: a name for the world (desktop build only) and a seed (a fresh one each time, or type your own), **Randomize**, **Create** and **Cancel**. **Create** puts you in a line warship in an open world of sparse asteroid clusters, planetoids and [generated ships](../ships/#generated-ships), generated from the seed and streamed in around you as you fly. The same seed gives the same world on the same build. On the desktop build the world saves itself as you play, and the pause menu's Retry becomes **Load last save**; see [Saved worlds](#saved-worlds) below. The web build keeps no saved worlds.
- **Load** - desktop build only: lists your saved world folders and resumes the one you pick, or shows why it cannot. **Delete** removes the one you pick after you confirm.
- **Sandbox** - opens the ship editor so you can build a ship and test-fly it in a practice scenario.
- **Lessons** - opens the training handbook. Its first row, **Basic Training**, starts the first-flight course. Under it is one screen per topic, with a demonstration, the actions it uses under your own bindings, a link into this manual, and often a focused range to fly. The handbook keeps track of what you have done: a lesson you open is marked **read**, and one is marked **done** only when you win a scenario that teaches it - finishing Basic Training or the topic's own practice range. Both are kept between sessions, in their own file beside your settings.
- **Scenarios** - opens the complete scenario picker, every scenario your enabled mods ship included.
- **Mods** - opens the installed-mod and online-catalog browser. A mod whose content will not load or fails its checks is switched off for you and named in one **MODS DISABLED** notice on the front door; its files stay installed, so you can update, remove or switch it back on from this screen.
- **Settings** - adjusts volume, graphics quality, window mode and the UI theme, and shows the control reference. The desktop build opens borderless fullscreen; **Windowed** is one click away.
- **Exit** - quits (hidden in the browser build).

</details>

<figure class="figure">
    <!-- Capture: assets/tutorial-menu.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot</span>
        <span class="figure__placeholder-name">assets/tutorial-menu.png</span>
        <span class="figure__placeholder-note">The main menu over its live backdrop: a section-built gunship holding station in a rock band, with the New Game / Sandbox / Scenarios / Mods / Settings / Exit options beside it.</span>
    </div>
    <figcaption class="figure__caption">The game boots into a main menu; New Game opens the world setup.</figcaption>
</figure>

In any scenario, <kbd>Esc</kbd> pauses the game and gives you Resume / Retry (restart the current scenario) / Settings / Back to Main Menu / Exit.

## Saved worlds

<!-- Route: crates/nova_world_base/src/save/mod.rs (`create_world`, `open_world`,
     `list_worlds`, `delete_world`, `world_slug`, `WorldRefusal`), save/session.rs
     (`WorldSaveSession`, `SaveReason`, death at :316-347), save/transients.rs
     (`FrozenTransient`, `WORLD_RESUME_SECONDS_MAX` 240 s); crates/nova_menu/src/
     world_setup.rs (Create), load_screen.rs (Load), save_status.rs (status
     line), leave.rs (leave overlay, Retry), outcome.rs (Defeat Load last
     save); crates/nova_authoring/src/base_content/scenarios/open_world.rs
     (the death Defeat); crates/nova_assets/src/storage.rs
     `worlds_root`; crates/nova_core/src/loading_screen.rs (resume progress
     line). -->

On the desktop build, New Game saves what you play; the web build keeps no saved worlds, and the main menu has no **Load** button.

Each world is a folder named after it, under this system's data folder, inside `nova-protocol/worlds` (for example `~/.local/share/nova-protocol/worlds` on Linux, `~/Library/Application Support/nova-protocol/worlds` on macOS, or `%APPDATA%\nova-protocol\worlds` on Windows).

**Create** takes a name of 1 to 32 letters, digits, spaces, `_` or `-`, and the seed field above it. A name already taken, or one this system cannot make a folder for, is refused under the field and nothing starts. The game takes the world's first save as soon as the world comes up.

The game saves when you cross into a new sector, and again when you leave: **Back to Main Menu**, **Exit** and the window's close button each wait on that save before they act. A status line at the top right reads `World saved`, `Saving world...`, `Waiting to save: <reason>` or `SAVE FAILED: <reason>` for the open world's last attempt. Leaving shows an overlay while its save runs; a failed leave save offers **Try again** or **Leave without saving**, which keeps the last good save and leaves anyway.

Dying ends the run with a **DEFEAT** panel: "Your ship was destroyed." On the desktop build it offers **Load last save** and **Main Menu**, and <kbd>Enter</kbd> goes to the menu; the web build offers **Main Menu** alone. Dying writes nothing - the last good save stays on disk - and a way out after a death writes nothing either. The pause menu's Retry reads **Load last save** instead of restarting too: it reopens the world from disk rather than the seed.

### What a Load brings back

A Load restores your ship (pose, motion, sections, damage, plates, ammo, reloads, hold, credits, key bindings and the chase-camera zoom), every frozen off-window sector, and every live sector in the streamed window as it was at the last good save - including a turret round, a torpedo, a rock chunk cut off a rock, a detached hull piece and a shed plate or decor fixture still in flight, each resuming on its own remaining lifetime. A visual-only effect, such as a spark or a muzzle flash, is not saved and does not come back.

A Load that cannot bring its sectors and transients back within 240 seconds is refused, and the loading screen shows `RESTORING SECTORS <live> / <desired>` while it waits.

### A refused Load

The Load screen lists every world folder and shows why one cannot be opened, on its row:

| Reason | Shown as |
| --- | --- |
| Another game has it open | `open in another game` |
| Saved with another save layout | `save format <found>; this build reads <current>` |
| Saved with other content | `content changed; saved with <mods>` |
| Save file missing, corrupt or of another generation | `unreadable: <reason>` |
| A file operation failed | the raw I/O error |
| The sectors or transients did not come back in time | `the world did not come back within 240 s: <reason>` |
| A saved reference matches two live bodies, or a saved style is not in this game's content | the reason, at once |

### Deleting a world

Select a world on the Load screen and press **Delete**, then **Delete world** to confirm or **Cancel** to keep it. Delete works on a world that cannot load, too. It removes only the files the game writes (`world.ron`, `world.lock`, the `state.<n>.ron` files and the temp files of an interrupted save), then the empty folder. There is no undo.

Delete removes nothing and shows `Cannot delete <folder>: <reason>` when another game has the world open, when the folder is gone, is a link or has a name no world gives, or when it holds any other file or folder: move that out first. If a removal fails part way, the world lists as unreadable and cannot load; press **Delete** again to remove the rest.

For your first flight, pick **Start Basic Training** on the card, the **Basic Training** row at the top of **Lessons**, or **Basic Training** in **Scenarios**. It teaches one gesture at a time and hands you each verb only when you reach the beat that needs it - so a key that answers with a deny buzz early on just is not unlocked yet. Each beat completes as soon as the range sees the gesture done.

## The first two minutes

Distances read in meters and kilometers and speeds in meters per second (`m/s`); see the [glossary](../glossary/).

- **Burn.** Hold <kbd>W</kbd> (or <kbd>Space</kbd>) for the main-drive burn and aim with the mouse. You point the hull, then thrust where the nose looks. The velocity sphere beside your ship shows where you are actually going.
- **Lock a target.** Hold <kbd>Ctrl</kbd> to sweep the radar; the hollow box shows what you are about to lock. Settle on it and hold steady for a moment - a short lock-on dwell has to fill (longer the farther the target is) before the lock latches, and sweeping off before then cancels it. A white crosshair is a travel (nav) lock; raise weapons first (see below) and it lands as a red combat lock instead.
- **GOTO.** With a lock, tap <kbd>G</kbd> and the autopilot flies you there - it burns over, flips, and coasts to a stop just off the target. <kbd>Z</kbd>, a burn on <kbd>W</kbd> or <kbd>G</kbd> again hands the ship straight back to you; the mouse does not, because while the computer flies your look input only swings the camera.
- **Raise weapons and fire.** Hold the right mouse button to raise weapons (combat stance); your reticle goes red and the ship is now "hot". Left mouse fires the turrets and launches torpedoes. A torpedo only launches while you hold a red combat lock.

That is the whole core loop: burn, lock, GOTO, shoot. Basic Training covers all of it on an armed trainer, ORBIT included.

## Basic Training, beat by beat

You open on the Fleet gunnery range in an exterior shot of **Trainer Seven**, an armed picket held on the line, while Range Control reads the qualification card: fly the pattern, take the flight computer out to the planetoid and back, put five target hulks on the scrap list, then deal with two live drones. The camera comes back with the helm, each objective arrives as a short amber notification, and one gold marker at a time keeps the current target in view.

### Part 1 - The pattern

1. **Burn to mark ALPHA.** Hold <kbd>W</kbd> to burn and steer with the mouse. ALPHA sits dead ahead; flying into its ring completes the beat.
2. **Kill the drift.** Press <kbd>X</kbd> once and let STOP brake the trainer down. Hands off the throttle: the next lesson waits for the maneuver to finish, and a burn or <kbd>Z</kbd> takes the ship back before it is done. The mouse is safe - it only moves the camera while STOP flies.
3. **Slide across to BRAVO.** Hold <kbd>Shift</kbd> and move the mouse to translate without turning the hull - short taps, not a held push. The velocity ball on your HUD goes violet while the thrusters have the ship, and ten violet pips under the speed chip show how much RCS is left. Let go and they refill.

### Part 2 - The flight computer

4. **Travel-lock the planetoid.** Keep weapons down, put the planetoid off the far corner in front of you and hold <kbd>Ctrl</kbd> until the white travel lock takes. A red lock means weapons were up: it is the gun's lock, and the computer flies only the white one.
5. **GOTO.** Press <kbd>G</kbd> and the autopilot flies the leg: it burns over, flips, and stops 500 m off the rock. Hands off until it is parked; a manual input takes the ship back, and the card waits for you to lock and press <kbd>G</kbd> again.
6. **ORBIT.** Parked inside the planetoid's pull, press <kbd>O</kbd>. The computer circularises and holds the orbit; the beat completes when it calls the orbit stable.
7. **Home to CHARLIE.** Mark CHARLIE goes up on the line. Travel-lock it - wait for it to come round if the planetoid is in the way - and press <kbd>G</kbd>. The computer drops the orbit on its own. Flying home by hand counts too: CHARLIE's ring closes the beat.

### Part 3 - The line

<figure class="figure">
    <!-- Capture: assets/tutorial-radar-lock.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot</span>
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
        <span class="figure__placeholder-tag">Screenshot</span>
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

## Mining and cargo

<!-- Route verified against crates/nova_authoring/src/base_content/scenarios/open_world.rs (the line warship and its stock), crates/nova_authoring/src/base_content/ships/block.rs (the warship's collars, dorsal intake and bow emitter), crates/nova_world_base/src/sector_ships.rs (`ship_stock`, `ship_credits`; every generated ship starts with goods and credits) and crates/nova_world/src/streaming.rs `spawn_sector_ship` (derelicts are lootable; a living generated ship trades its own stock and credits). -->

The line warship New Game gives you carries a docking port on each shoulder, a cargo intake on its top deck and a mining beam at its bow. Its hold is what keeps it fighting: a [repair](../interface/#the-ship) spends hull plates, and every idle [reload](../combat-weapons/#magazines) spends ammunition. Press <kbd>Tab</kbd> and step to the **Inventory** pane to see what you carry.

1. **Take from a wreck.** Each generated derelict wreck carries its own mixed goods - it may hold no hull plates at all - and a share of its credits. Travel-lock one, line one of your ports up with one of its ports and press <kbd>D</kbd> to [dock](../sections/docking/#flying-the-approach). In the Inventory pane, click an item and **Confirm** to [take](../interface/#take-and-give) it, or click **Take credits** to take its whole balance in one click. <kbd>D</kbd> again lets go.
2. **Mine a rock.** Travel-lock a rock, put your nose on it inside 100 m and hold <kbd>V</kbd>. The [mining beam](../sections/mining-beam/#cutting-ore) cuts it once a second, and the ore drifts off the rock in canisters.
3. **Pick up a canister.** Fly your top-deck [cargo intake](../sections/cargo-intake/#taking-a-canister-in) up onto a canister, door first. It goes into your hold whole.
4. **Drop cargo.** Undocked, click an item in the Inventory pane and **Confirm** a [Jettison](../interface/#jettison-and-pickup). Close the pane and the intake drops it as canisters, which you can pick up again.

Ore sells to a docked ship that trades: a living generated ship carries its own goods and credits, so Buy and Sell work once you dock with a calm one you have not neutralized. Mined ore stays in your hold until you find one.

## Where to go next

That is everything you need to get off the launch pad. The rest of the wiki is the full reference:

- [Flight & autopilot](../flight-autopilot/) - how ships move and what GOTO, ORBIT and STOP each do.
- [Gravity wells](../gravity-wells/) - how static wells on planets and anchors affect mobile asteroids, ships, rounds, and torpedoes.
- [Ships & damage](../ships/) - what a hull is made of and how it comes apart.
- [Targeting & radar](../targeting-radar/) - deliberate locking, stances, and per-section fine-lock.
- [Combat](../combat-weapons/) - the engagement ladder and the rules every weapon shares.
- [Ship sections](../sections/) - what each part of a ship does, one page per part.
- [Keybinds](../keybinds/) - the complete control reference for keyboard and gamepad.
- [Glossary](../glossary/) - the recurring terms and units in one place.

<p style="margin-top: 2.5em"><a href="../../play/" class="btn btn--primary">Launch the game</a></p>
