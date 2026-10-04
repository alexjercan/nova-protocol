# Commands

Press <kbd>:</kbd> anywhere - the main menu, the ship editor, mid-flight, on the pause screen, over the [interface](../interface/) - and the ship's CRT monitor opens on **NOVA COMMANDS**: one prompt that can read the run, change your settings, print the flight log, and, once you deliberately arm it, cheat.

<figure class="figure">
    <!-- Capture: assets/loops/command-shell-open.webm -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/command-shell-open.webm</span
        >
        <span class="figure__placeholder-note"
            >':' over live flight: the monitor, the
            NOVA COMMANDS header, the introduction typing itself out
            against the world it just froze, and 'ships'
            answered from the paused run.</span
        >
    </div>
</figure>

## Getting in and out

<!-- ':' gesture and its guards: crates/nova_menu/src/pause.rs `open_command_shell`
     (not mid-load, not while a settings chip or a ship section is capturing a key).
     Back-out owner (Esc, Start): crates/nova_interface/src/terminal/input.rs
     `close_surface_from_menu_keys`; the `close` command: same file, pending close.
     Return target: `NovaOsCloseTransition::return_to`, set in `open_command_shell`. -->

| You do | What happens |
| --- | --- |
| <kbd>:</kbd> | The monitor opens on the prompt. It needs no ship, so it works in the menu and the editor too. Close it and you are back on exactly what it covered - the menu, the pause screen, the interface pane or your flight. |
| <kbd>Esc</kbd> (or gamepad Start) | Power the monitor off and return to what was underneath. |
| Type `close` | The same animated power-off. |
| Click the **PWR** button | The same again, on the monitor's chin. |
| <kbd>Tab</kbd> | Complete the command. It does not close the monitor. |
| Type `clear` | Wipe the screen back to the introduction, read against the world as it is now. |

<details class="explain">
<summary>Show explanation</summary>

<!-- Freeze: crates/nova_menu/src/lib.rs OnEnter/OnExit(Interface | Commands),
     `hold_clocks_for_interface` (Playing only). -->

Opened over the game, the monitor freezes it: the clocks stop, the cursor is freed, and <kbd>Esc</kbd> belongs to the monitor rather than the pause menu. Opened over the interface, it shares the interface's freeze - the world never runs a frame between the two, and closing the monitor puts you back on the same pane with its camera and selection intact. Opened on the main menu it stops nothing - the backdrop behind it keeps flying, because that is a view and not a game you are being kept out of - but the monitor is still the front-most thing on the screen, so the menu buttons under it wait until you close it.

Two places refuse the key. Mid-load there is nothing yet to inspect. While a ship section or a settings control waits for you to press a key to bind, <kbd>:</kbd> is the key being bound, so it does not open the monitor.

While the monitor is open, nothing you type reaches flight or the pane behind it.

</details>

## The prompt

<!-- Prompt "cmd> ": crates/nova_command/src/terminal/state.rs `PROMPT_PREFIX`.
     History cap 200 (`MAX_HISTORY`) and scrollback cap 500 (`MAX_SCROLLBACK_ROWS`):
     crates/nova_command/src/terminal/state.rs. Completion, ghost and hint line:
     crates/nova_command/src/terminal/{edit,view}.rs. Wheel and PgUp/PgDn:
     crates/nova_interface/src/terminal/input.rs. -->

| Key | At the prompt |
| --- | --- |
| <kbd>Tab</kbd> | Complete the command - repeated presses cycle the matches. |
| <kbd>Enter</kbd> | Run the line. |
| <kbd>Up</kbd> / <kbd>Down</kbd> | Walk the command history (the last 200 lines). |
| <kbd>PgUp</kbd> / <kbd>PgDn</kbd>, or the wheel | Scroll the scrollback (it keeps 500 rows). |
| <kbd>Esc</kbd> | Close the monitor. |

The prompt reads `cmd>`. As you type, a dim ghost continues the line toward the nearest command, and a hint line under the prompt tracks what you have: an unknown word turns the input red and offers a `did you mean` suggestion, and a wrong argument shows the command's usage. <kbd>Tab</kbd> completes command names, subcommands, and the live ship and section ids an argument wants.

## What the introduction tells you

```text
NOVA OS v0.13.0 // COMMANDS
POST ......... command shell / ok
CORE ......... local game runtime / attached
REGISTRY ..... 28 commands / ready
WORLD ........ tutorial / paused
CHEATS ....... disabled / run clean
Hint: type `help` and press Enter.
```

`WORLD` names what you opened over - a scenario id, `main menu`, or `no scenario` - and whether it is running. `CHEATS` is the line that matters: `disabled / run clean` while your run still counts, `enabled / run marked` once it does not. The header carries the same fact in the top right, where `CHEATS: OFF` turns amber and reads `CHEATS: ON`.

## The commands

Every command belongs to one of four classes, and the class is the whole permission model.

| class | what it may do | marks your run |
| --- | --- | --- |
| utility | control the shell; abandon one scenario for another | no |
| readonly | look at the world and never touch it | no |
| setting | change the same saved settings the menu changes | no |
| cheat | change the live world; refused until armed | arming does, once |

### Utility

| command | what it does |
| --- | --- |
| `help [command]` | this text, or one command's usage, class, arguments and examples |
| `commands [class]` | the whole catalog, or one class of it |
| `clear` | restore the introduction |
| `close` | close the terminal and return to what was underneath |
| `scenario load <id>` | abandon this attempt and load a fresh scenario |

### Read-only

| command | what it does |
| --- | --- |
| `status` | a compact run and world summary |
| `scenario` | the current scenario, its state and its outcome |
| `ships` | live ships by id |
| `ship <id>` | one ship: side, hull, sections, magazines |
| `sections <ship-id>` | that ship's sections, with health and ammunition |
| `section <ship-id> <section-id>` | one section of one ship |
| `objectives` | the open objectives |
| `log` | the flight log: comms, objective updates and ship events |
| `variables` | the scenario's variables |
| `variable <name>` | one of them |
| `bindings [action]` | every input action by its qualified name, such as `flight.mine`, and what it is bound to, or one of them |
| `settings` | every current setting |
| `cheats status` | whether cheats are armed, and whether the run is marked |

### The flight log

<!-- `log` rows: crates/nova_console/src/inspect.rs `log`. Log model and its
     sources (story feed, objective list, `CombatLockDropped`):
     crates/nova_interface/src/terminal/flight_log.rs. Lock-drop wording:
     `combat_lock_drop_line`. -->

`log` prints the flight log, oldest first. Each row carries its index and a label for its kind:

| Row | Meaning |
| --- | --- |
| `0001 COMMS OKONO > Strip it clean.` | A comms line, with its speaker. |
| `0002 OBJ + Strip it clean.` | An objective posted. |
| `0003 OBJ x Strip it clean.` | An objective completed. |
| `0004 SYS ! Combat lock lost: target out of lock range.` | Your ship's own report. Today that is a dropped combat lock, with its reason: target gone, out of range, behind cover, or no longer hostile. |

The log is empty until something happens. It answers the question a player asks afterwards - "why did I lose my lock?" - without putting anything on screen mid-fight.

### Settings

One rule: the command **without** a value prints the setting, the command **with** one changes it. A change here is the same change the settings menu makes, and it is saved the same way.

| command | what it does |
| --- | --- |
| `graphics [low\|medium\|high]` | the graphics-quality preset |
| `volume [master\|music\|world\|interface [0..1]]` | one mixer channel |
| `window [windowed\|borderless]` | the window mode |
| `bind <action> <source>` | rebind one action by its bare name, e.g. `bind interface_toggle F1` |
| `bind reset <action>` | put an action back on its default |

### Cheats

Every one of these is refused until you run `cheats enable`. That command is the deliberate act, and it marks the run the moment it succeeds - there is no command that unmarks it.

| command | what it does |
| --- | --- |
| `cheats enable` | arm cheats and mark this run, one way |
| `ammo infinite <ship-id> <on\|off>` | unlimited ammunition on one ship's weapons |
| `ammo refill <ship-id>` | top up every finite magazine on a ship |
| `ammo refill section <ship-id> <section-id>` | top up one magazine |
| `item give <ship-id> <item-id> <quantity>` | add items to one ship's inventory, e.g. `item give player_spaceship HullPlate 12` |

<details class="explain">
<summary>Show explanation</summary>

<!-- The mark: crates/nova_gameplay/src/cheats.rs:43-63 (arm() is one-way, begin_new_run clears).
     Arming checked once, on the class: crates/nova_console/src/dispatch.rs.
     Restoration rule: SuspendedSectionAmmo, crates/nova_ship/src/sections/ammo.rs:98.
     `item give`: crates/nova_console/src/cheats.rs `item_give`; item masses:
     crates/nova_gameplay/src/inventory.rs `ItemType::mass_g`. -->

**Why the mark is one-way.** A run that was ever armed was never clean, whether or not you went on to use a cheat. Marking at the moment you ask, rather than at the moment you benefit, is what makes the mark honest - and it gives the refusal something true to say rather than pretending the command does not exist.

**Why a fresh scenario clears it.** A new attempt is a new run. `scenario load <id>` abandons the current attempt without giving it an outcome and without advancing campaign progress, then resets both the arming and the mark.

**What `ammo infinite off` gives back.** Turning it on suspends the magazine; turning it off restores the authored capacity **full**, and re-seeds the reload from the section's own configuration. It does not try to remember the count you had when you switched it on - a number from before the cheat is not a number the run earned either way, and a full magazine is the state you can reason about.

**The `ammo` cheats do not touch your hold.** The ordinary idle reload spends matching ammunition from the ship's own inventory (see [Magazines](../combat-weapons/#magazines)); both `ammo` cheats bypass that and act on the magazine directly, spending and returning nothing from stock.

**`item give` fills the hold instead.** The item id is the name a scenario file writes: `HullPlate`, `PdcRound`, `RailSlug`, `Torpedo`, `StoneOre`, `IronOre`, `WaterIce`, `CarbonOre`, `Rations` or `SalvagedParts`, in that exact case. The quantity is a whole number above zero, and the items weigh what they always weigh (a hull plate 10 kg, a PDC round 0.2 kg, a rail slug 20 kg, a torpedo 150 kg). The whole quantity fits the ship's free hold or none of it is added: with 280 kg free, 29 hull plates are refused whole. An unknown ship, item or quantity adds nothing either. The idle reload then loads from the new stock as usual.

**This is not the scenario language.** A scenario author has a much larger vocabulary (spawning, despawning, allegiance, forced launches, outcomes). None of it is reachable from this prompt. The catalog above is the whole public surface, and a new scenario action does not appear here unless somebody deliberately adds it.

</details>

## The monitor

<figure class="figure">
    <!-- Capture: assets/wiki-command-shell.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-command-shell.png</span
        >
        <span class="figure__placeholder-note"
            >The whole NOVACRT 9000 at the prompt: casing,
            vents, chin controls, a lit scrollback with the
            introduction, a command answer and a did-you-mean
            suggestion.</span
        >
    </div>
</figure>

<!-- Chin controls: crates/nova_interface/src/terminal/casing.rs. Detents:
     terminal/style.rs. SND mute gate: terminal/sound.rs, dark bulb casing.rs.
     PWR close + orange LED: terminal/shell.rs. Settings persist via the menu
     settings store: terminal/components.rs `NovaOsMonitorSettings`,
     crates/nova_menu/src/settings_store.rs. -->

The monitor is a physical one - a NOVACRT 9000, per its brand plate - and its chin controls are real. They are clicked directly (everything above them, on the glass, is clicked *through* the curved picture), and their settings persist across sessions:

| Control | What it does |
| --- | --- |
| **BRIGHT** knob | Four detents of picture brightness, from dim to blazing. |
| **SCAN** knob | Four detents of scanline strength, from clean to heavy. |
| **SND** toggle | Mutes every monitor sound; the indicator bulb goes dark. |
| **PWR** button | Powers the monitor off - the LED flashes orange while the picture collapses. |

<details class="explain">
<summary>Show explanation</summary>

<!-- CRT composite + pointer forwarding: crates/nova_interface/src/terminal/crt.rs.
     Shader physics: assets/shaders/nova_os_crt.wgsl (hum bar, mains flicker,
     retrace beam, power collapse, degauss). Casing detail: terminal/casing.rs.
     Sounds: terminal/sound.rs, crates/nova_gameplay/src/audio/mod.rs. -->

The picture is a real tube, not a flat overlay: the terminal renders offscreen and is shown through one screen shader, so the green glyphs bloom into a phosphor halo, the image bows with barrel curvature under scanlines, grain and an edge vignette, a hum bar drifts, the mains flicker breathes, and a retrace beam sweeps by every few seconds. Your mouse works through all of it: clicks on the glass are mapped through the same curvature, so the control you see under the cursor is the one you hit.

The monitor sounds like it looks, when SND is up: a key tick per character, a heavier clack on Enter, an ok blip when a command runs and an error buzz when it does not, a completion tick per Tab step, the power sweep up and down, and a low ambient bed humming the whole time it is on. Around the glass, the casing carries its bezel, corner screws, a vent grille and the spec line `P22 GREEN PHOSPHOR . 15 IN . TYPE CQ-4`.

</details>

## Driving it from outside

<!-- The wire's command lane: crates/nova_channel/src/protocol.rs, apply.rs. -->

The development process channel carries the same lines. `{"tick":120,"command":"graphics low"}` is the text you would have typed, resolved by the same parser and run by the same dispatcher, and the acknowledgement names the command, its class and its result. There is no separate wire vocabulary to learn or to drift.
