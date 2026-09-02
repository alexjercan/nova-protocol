# Commands

The ship computer speaks a second language. Press <kbd>:</kbd> anywhere - the main menu, the ship editor, mid-flight, on the pause screen - and the same CRT opens on the **command shell**: one prompt that can read the run, change your settings, and, once you deliberately arm it, cheat.

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
            >':' over live flight: the same monitor, the
            COMMANDS header, and the introduction typing
            itself out against the world it just froze.</span
        >
    </div>
</figure>

## Getting in and out

<!-- ':' gesture and its guards: crates/nova_menu/src/pause.rs:108-140.
     'commands' builtin: crates/nova_os/src/command.rs:178-182.
     Shell state (two transcripts, two histories): crates/nova_os/src/terminal/state.rs:80-120, 401-430. -->

| You do | What happens |
| --- | --- |
| <kbd>:</kbd> | The monitor opens straight on the command shell. It needs no ship, so it works in the menu and the editor too. |
| Type `commands` at the NOVA OS prompt | Switch shells without closing the picture. Inside an app, you land on the NOVA OS prompt first. |
| <kbd>Esc</kbd> | Climb one level: back to the NOVA OS prompt you came from, or out of the computer. |
| Type `close` | Power the monitor off and return to whatever was underneath. |
| Type `clear` | Wipe the screen back to the introduction, read against the world as it is now. |

<details class="explain">
<summary>Show explanation</summary>

It is one monitor with two languages, not two overlays. The casing, the glass, the phosphor, the scrollback, <kbd>Tab</kbd> completion, the history keys and <kbd>PgUp</kbd>/<kbd>PgDn</kbd> are the terminal's and are shared. What each shell owns is its own vocabulary, its own prompt (`nova>` against `cmd>`), its own transcript and its own history - so switching back and forth loses nothing, and never replays the power-on animation.

Opening the command shell freezes the game exactly as NOVA OS does: the clocks stop, the cursor is freed, and <kbd>Esc</kbd> belongs to the shell rather than the pause menu. Switching between the two shells does not unpause and re-pause; the monitor owns the freeze for as long as it is open. The one place it will not open is mid-load, where there is nothing yet to inspect.

</details>

## What the introduction tells you

```text
NOVA OS v0.13.0 // COMMANDS
POST ......... command shell / ok
CORE ......... local game runtime / attached
REGISTRY ..... 27 commands / ready
WORLD ........ shakedown_run / paused
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
| `clear` | restore this shell's introduction |
| `close` | close the terminal and return to what was underneath |
| `scenario load <id>` | abandon this attempt and load a fresh scenario |

### Read-only

| command | what it does |
| --- | --- |
| `status` | a compact run and world summary |
| `scenario` | the current scenario, its state and its outcome |
| `ships` | live ships by id |
| `ship <id>` | one ship: side, hull, sections, speed cap, magazines |
| `sections <ship-id>` | that ship's sections, with health and ammunition |
| `section <id>` | one section |
| `objectives` | the open objectives |
| `variables` | the scenario's variables |
| `variable <name>` | one of them |
| `bindings [action]` | every input action and what it is bound to, or one of them |
| `settings` | every current setting |
| `cheats status` | whether cheats are armed, and whether the run is marked |

### Settings

One rule: the command **without** a value prints the setting, the command **with** one changes it. A change here is the same change the settings menu makes, and it is saved the same way.

| command | what it does |
| --- | --- |
| `graphics [low\|medium\|high]` | the graphics-quality preset |
| `volume [master\|music\|world\|interface [0..1]]` | one mixer channel |
| `window [windowed\|borderless]` | the window mode |
| `bind <action> <source>` | rebind one action, e.g. `bind novaos_toggle F1` |
| `bind reset <action>` | put an action back on its default |

### Cheats

Every one of these is refused until you run `cheats enable`. That command is the deliberate act, and it marks the run the moment it succeeds - there is no command that unmarks it.

| command | what it does |
| --- | --- |
| `cheats enable` | arm cheats and mark this run, one way |
| `ammo infinite <ship-id> <on\|off>` | unlimited ammunition on one ship's weapons |
| `ammo refill <ship-id>` | top up every finite magazine on a ship |
| `ammo refill section <section-id>` | top up one magazine |
| `speed-cap <ship-id> <m/s\|off>` | change or remove a ship's manual speed cap |

<details class="explain">
<summary>Show explanation</summary>

<!-- The mark: crates/nova_gameplay/src/cheats.rs:43-63 (arm() is one-way, begin_new_run clears).
     Arming checked once, on the class: crates/nova_console/src/dispatch.rs.
     Restoration rule: SuspendedSectionAmmo, crates/nova_ship/src/sections/ammo.rs:98. -->

**Why the mark is one-way.** A run that was ever armed was never clean, whether or not you went on to use a cheat. Marking at the moment you ask, rather than at the moment you benefit, is what makes the mark honest - and it gives the refusal something true to say rather than pretending the command does not exist.

**Why a fresh scenario clears it.** A new attempt is a new run. `scenario load <id>` abandons the current attempt without giving it an outcome and without advancing campaign progress, then resets both the arming and the mark.

**What `ammo infinite off` gives back.** Turning it on suspends the magazine; turning it off restores the authored capacity **full**, and re-seeds the reload from the section's own configuration. It does not try to remember the count you had when you switched it on - a number from before the cheat is not a number the run earned either way, and a full magazine is the state you can reason about.

**This is not the scenario language.** A scenario author has a much larger vocabulary (spawning, despawning, allegiance, forced launches, outcomes). None of it is reachable from this prompt. The catalog above is the whole public surface, and a new scenario action does not appear here unless somebody deliberately adds it.

</details>

## Driving it from outside

<!-- The wire's command lane: crates/nova_channel/src/protocol.rs, apply.rs. -->

The development process channel carries the same lines. `{"tick":120,"command":"graphics low"}` is the text you would have typed, resolved by the same parser and run by the same dispatcher, and the acknowledgement names the command, its class and its result. There is no separate wire vocabulary to learn or to drift.
