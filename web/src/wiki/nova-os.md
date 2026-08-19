# NOVA OS

Every ship carries a second seat of control: **NOVA OS**, the ship computer. Press <kbd>Tab</kbd> in flight and a CRT monitor takes the whole screen - the world freezes while you read, type and click, and flight resumes the moment the picture powers back down.

<figure class="figure">
    <!-- Capture: assets/loops/nova-os-open.webm -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/nova-os-open.webm</span
        >
        <span class="figure__placeholder-note"
            >Tab: the raster blooms on from a single scan
            line, the boot report types itself out, and the
            close collapses the picture to a dying dot.</span
        >
    </div>
</figure>

## Opening and closing

<!-- Open gestures + player-ship guard: crates/nova_os_ui/src/terminal/input.rs:48-74.
     Back-out gestures: input.rs:83-146 (Esc/Start :101-104, Shift+Esc :132-137,
     Ctrl+C / Ctrl+[ :107-109,138-141, one-level Escape :142-145).
     exit command: crates/nova_os/src/command.rs:173, input.rs:332-335.
     PWR button: crates/nova_os_ui/src/terminal/shell.rs:72-79. -->

| You do | What happens |
| --- | --- |
| <kbd>Tab</kbd> (or click the right stick) | The monitor powers on. It needs a live ship to be the computer of; it will not open over the pause menu. |
| <kbd>Esc</kbd> (or gamepad Start) | Back out one level: a running app returns to the prompt; the prompt powers the monitor off. |
| <kbd>Shift</kbd>+<kbd>Esc</kbd> | Power off from anywhere, even inside an app. |
| <kbd>Ctrl</kbd>+<kbd>C</kbd> or <kbd>Ctrl</kbd>+<kbd>[</kbd> | Leave the running app, keep the computer on. |
| Type `exit`, or click the **PWR** button | The same animated power-off. |

<details class="explain">
<summary>Show explanation</summary>

<!-- Freeze semantics: crates/nova_gameplay/src/lib.rs:133-144 (clocks stopped,
     cursor freed, no pause menu); clocks: crates/nova_menu/src/pause.rs:80-86;
     cursor release + keep-free guard: pause.rs:99-116; Esc cannot reach the
     pause menu while open: pause.rs:56-66; frozen through the close animation:
     crates/nova_os_ui/src/terminal/shell.rs:558-602.
     Weapons/input gated: crates/nova_ship/src/lib.rs:121-144.
     HUD hides (exempt widgets aside): crates/nova_hud/src/lib.rs:444-462.
     Objective chips marked read: crates/nova_hud/src/objective_stack.rs:321-333.
     Tab is completion, pad button closes: input.rs:69-71,243-255.
     Scrollback and a running app persist across closes: shell.rs:126-128,223-224. -->

While the computer is open the game is frozen: the clocks stop, so combat, physics, AI and every projectile hold mid-frame, and a held trigger cannot fire into the frozen world. The mouse cursor is freed and *stays* free - you can point, read and type at your own pace. It is a pause without a pause menu; <kbd>Esc</kbd> backs out of the computer instead of opening the menu, and flight only resumes once the picture has fully collapsed.

<kbd>Tab</kbd> does not close the computer - inside, it is the completion key. On a gamepad the same right-stick click that opened it closes it.

Opening has side effects on the HUD: the flight instruments hide behind the monitor, and any posted objective chips are marked as read (the standing list is `objectives`). Closing leaves the session in place - reopen and the scrollback, your command history, even a running app are exactly where you left them.

</details>

## The terminal

<!-- Footer hint set: crates/nova_os/src/app.rs:15-25. History cap 200:
     crates/nova_os/src/terminal/edit.rs:23. Scrollback cap 500:
     crates/nova_os/src/terminal/state.rs:16,253-262. Wheel scroll:
     crates/nova_os_ui/src/terminal/input.rs:435-465. -->

| Key | At the prompt |
| --- | --- |
| <kbd>Tab</kbd> | Complete the command - repeated presses cycle the matches. |
| <kbd>Enter</kbd> | Run the line. |
| <kbd>Up</kbd> / <kbd>Down</kbd> | Walk the command history (the last 200 lines). |
| <kbd>PgUp</kbd> / <kbd>PgDn</kbd>, or the wheel | Scroll the scrollback (it keeps 500 rows). |
| <kbd>Esc</kbd> | Close the computer. |

<details class="explain">
<summary>Show explanation</summary>

<!-- Prompt "nova>": crates/nova_os/src/terminal/edit.rs:18,112-115.
     Completion cycle + list: edit.rs:246-262; candidates incl. live section /
     contact codes: edit.rs:271-317. Ghost suffix: crates/nova_os/src/terminal/view.rs:212-227.
     did-you-mean (edit distance <= 2): edit.rs:199-220, crates/nova_os/src/shell.rs:246-252.
     Arity rejections + usage: edit.rs:176-198, shell.rs:34-42.
     Live hint line: edit.rs:362-402. Red prompt when invalid:
     crates/nova_os_ui/src/terminal/shell.rs:545-552. -->

The prompt reads `nova>`. As you type, a dim ghost continues the line toward the nearest command, and a hint line under the prompt tracks what you have: an unknown word turns the input red and offers `did you mean map?`, a wrong argument shows the command's usage. Mistyped submits answer in kind - `command not found: mpa`, the same `did you mean` suggestion, and a pointer to `help`. <kbd>Tab</kbd> completes command names, subcommands, and even the live section and contact codes an argument wants (`ship repair hu` completes to `HULL-3`).

<!-- Welcome rows verbatim: crates/nova_os/src/terminal/view.rs:14-41; unread
     line: view.rs:56-68; staggered reveal: shell.rs:247-262; clear reprints:
     crates/nova_os/src/terminal/state.rs:415-420; live objective completion
     rows: crates/nova_os_ui/src/terminal/flight_log.rs:102-131. -->

The first open of a session boots with a report, revealed a line at a time:

```
NOVA OS v<version>
POST ......... flight computer / ok
CORE ......... 64K static / ok
DISPLAY ...... green phosphor crt / warm
LINK ......... cockpit bus / local
Hint: type `help` and press Enter.
```

Reopening after events happened in flight adds an unread count with the latest headline - `3 unread events. OBJ x Strip it clean. - try 'log'.` - and `clear` wipes the scrollback back to this report rather than to a blank screen. The terminal also talks on its own: an objective completing while you sit at the prompt prints its `OBJ x` row into the scrollback immediately.

</details>

## Command reference

<!-- The full registered command set. Core builtins:
     crates/nova_os/src/command.rs:166-175. Map tree:
     crates/nova_os_ui/src/map/mod.rs:96-113. Ship tree:
     crates/nova_os_ui/src/ship/mod.rs:142-169. Dispatch classes (print vs app
     takeover vs ship action): crates/nova_os/src/shell.rs:50-80. -->

<div class="widget" data-widget="nova-os-surfaces">
<p>Every command lands on one of three surfaces. Most print into the terminal scrollback: <code>help</code>, <code>log</code>, <code>objectives</code>, <code>clear</code>, <code>version</code>, <code>ship view</code>, <code>map view</code> and <code>ship section</code>. Two hand the whole screen to an app - <code>map</code> and <code>ship</code> - and swap the footer hint row to that app's keys under a breadcrumb like <code>NOVA OS // APPS / MAP</code>. The rest act on the live ship and print the result: <code>map goto</code> engages the autopilot, <code>ship reload</code> and <code>ship repair</code> service a section. <code>exit</code> powers the monitor off.</p>
</div>

| Command | What it does |
| --- | --- |
| `help` | Prints the command list. Every command also answers `<command> help` and `-h`. |
| `log` | Prints the flight log: comms lines (`COMMS OKONO > ...`) and objective events (`OBJ +` posted, `OBJ x` completed). |
| `objectives` | Prints the active objectives. |
| `clear` | Clears the scrollback back to the boot report. |
| `version` | Prints the version banner. |
| `exit` | Powers the computer off and returns to flight. |
| `map` | Opens the **MAP** app. |
| `map view` | Prints the local-space contact table. |
| `map goto <label>` | Engages the flight autopilot toward a contact - the burn continues after the computer closes. |
| `ship` | Opens the **SHIP** app. |
| `ship view` | Prints the ship status table: every section with HP, ammo, and any `[critical]` / `[neutralized]` flag. |
| `ship section <id>` | Prints one section's detail: kind, integrity bar, status, ammo. |
| `ship reload <id>` | Reloads a weapon section - `reloaded PDC-1: ammo 6/6`. |
| `ship repair <id>` | Repairs a section - `repaired HULL-3: integrity restored to 100 HP`. |

<details class="explain">
<summary>Show explanation</summary>

<!-- log/objectives row shapes: crates/nova_os_ui/src/terminal/content.rs:98-137,403-413.
     version output: crates/nova_os/src/terminal/view.rs:169-180.
     help shape: view.rs:79-104; per-command help + universal help/version
     sub-verbs: view.rs:128-166, crates/nova_os/src/shell.rs:105-113,218-225.
     Longest-name matching: shell.rs:189-244.
     launching row: crates/nova_os/src/terminal/edit.rs:126-138.
     ship action results/errors: crates/nova_os_ui/src/ship/sections.rs:384-415,495-507,536-586.
     map goto results/errors: crates/nova_os_ui/src/map/app.rs:33-101.
     Actions instant and free today: sections.rs:512-513,533-535. -->

Output is honest terminal text: `log` numbers its entries (`0001 COMMS OKONO > Strip it clean.`), `version` signs off with `cockpit link nominal - (c) Nova Dynamics, all reactors reserved`, and launching an app prints `launching map ...` before the screen hands over. Multi-word commands resolve longest name first, so `map view` is its own command, not `map` with an argument.

The acting verbs answer with what actually happened, or with why not: a hull section refuses `reload` (`reload: HULL-3 is a hull section, no ammo feed`), a healthy section refuses `repair`, and a bad code lists the codes that exist. `map goto SELF` politely declines to fly you to yourself. Reloads and repairs are instant and free today; that may change.

</details>

## Apps

An app swallows the whole monitor: the header breadcrumb switches to `APPS / MAP` or `APPS / SHIP`, an amber `[ ESC ]` control appears beside it, the footer hints swap to the app's keys, and the tube thumps through a brief degauss shimmer. The terminal scrollback is untouched underneath - leaving the app restores it exactly.

<!-- Breadcrumb: crates/nova_os_ui/src/terminal/content.rs:45-55. [ ESC ] control:
     crates/nova_os_ui/src/terminal/spawn.rs:372-393. Degauss + coil on app
     switch: crates/nova_os_ui/src/terminal/shell.rs:129-180. Scrollback
     preserved: crates/nova_os/src/terminal/state.rs:422-435. Hints:
     crates/nova_os_ui/src/map/mod.rs:70-80, crates/nova_os_ui/src/ship/mod.rs:82-94. -->

| App | Launch word | The screen | Its keys |
| --- | --- | --- | --- |
| **MAP** | `map` | A schematic 3D minimap of local space: distance rings, a hub, and every contact as a labelled blip. | <kbd>WASD</kbd> move, <kbd>Q</kbd>/<kbd>E</kbd> turn, <kbd>R</kbd>/<kbd>F</kbd> tilt, right-drag look, wheel zoom, <kbd>[</kbd>/<kbd>]</kbd> cycle, <kbd>G</kbd> goto, <kbd>T</kbd> reset |
| **SHIP** | `ship` | A schematic 3D viewer of your own hull: one block per section, status blips riding on top, an inspector panel beside. | <kbd>Q</kbd>/<kbd>E</kbd> turn, <kbd>R</kbd>/<kbd>F</kbd> tilt, right-drag look, wheel zoom, <kbd>[</kbd>/<kbd>]</kbd> select, <kbd>G</kbd> mates, <kbd>L</kbd> reload, <kbd>P</kbd> repair, <kbd>B</kbd> rebind, <kbd>T</kbd> reset |

### The map

<figure class="figure">
    <!-- Capture: assets/wiki-nova-os-map.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-nova-os-map.png</span
        >
        <span class="figure__placeholder-note"
            >The MAP app mid-fight: rings, a selected hostile
            blip with its amber ring, the readout line showing
            range and bearing.</span
        >
    </div>
</figure>

<!-- Contact kinds + codes: crates/nova_os_ui/src/map/contacts.rs:28-36,59-69.
     Readout format + notes: contacts.rs:49-57,126-147. Blips + selection:
     crates/nova_os_ui/src/map/scene.rs:231-259,413-449,468-541. G = GOTO with
     flash: scene.rs:369-383. Persists after close:
     crates/nova_os_ui/src/map/app.rs:69-84. -->

Every contact carries a short label you can select, read, and hand to `map goto`:

| Label | Contact |
| --- | --- |
| `SELF` | Your own ship. |
| `ALLY-1` | A friendly ship. |
| `HOST-1` | A hostile ship - its blip pulses. |
| `OBJ-1` | A mission objective. |
| `AST-1` | Terrain: an asteroid mass. |

<details class="explain">
<summary>Show explanation</summary>

Click a blip (or cycle with <kbd>[</kbd>/<kbd>]</kbd>) and the readout fills in: `HOSTILE HOST-1 / Raider - range 412 m, bearing 214 mark +12. Hostile contact.` Selecting re-centres the map once; <kbd>T</kbd> re-frames it. <kbd>G</kbd> engages the flight autopilot toward the selection and flashes `GOTO SET: Raider` - the burn keeps flying after you close the computer, so the map is a real navigation console, not a picture of one. `map view` prints the same contacts as an aligned `KIND LABEL INFO` table, own ship first, then nearest first.

</details>

### The ship

<figure class="figure">
    <!-- Capture: assets/wiki-nova-os-ship.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-nova-os-ship.png</span
        >
        <span class="figure__placeholder-note"
            >The SHIP app: green schematic blocks, section
            blips with integrity bars, the inspector panel on
            a damaged section.</span
        >
    </div>
</figure>

<!-- Codes + glyphs: crates/nova_os_ui/src/ship/sections.rs:31-52. Blocks do not
     encode status; blips do: crates/nova_os_ui/src/ship/mod.rs:14-25,
     crates/nova_os_ui/src/ship/scene.rs:648-705. Status words + thresholds:
     sections.rs:205-214. Inspector: crates/nova_os_ui/src/ship/app.rs:68-131,
     sections.rs:427-444. -->

Sections are addressed by short codes, stable for the whole session and shared with the CLI verbs:

| Code | Section | Blip glyph |
| --- | --- | --- |
| `HULL-1` | Hull plating | `#` |
| `THR-1` | Thruster | `>` |
| `CTL-1` | Controller | `@` |
| `PDC-1` | Turret | `T` |
| `TRB-1` | Torpedo bay | `^` |

<details class="explain">
<summary>Show explanation</summary>

The blocks are the shape of your ship - a dim green fill in a bright outline per section, with a gap so neighbours read apart. Status lives on the blips and in the inspector, not in the block colour: each blip carries its glyph and code, an integrity bar whose width is HP and whose colour is status (`nominal`, `degraded`, `critical`, `neutralized`), and ammo pips on weapons. Select a section by clicking its blip or cycling <kbd>[</kbd>/<kbd>]</kbd>; the inspector fills with its kind, an ASCII integrity meter (`integrity: 41% [####------]`), status, ammo and current bindings, with `P Repair`, `L Reload` and `B Rebind` buttons that do exactly what the keys do. <kbd>G</kbd> overlays the structural mates - which sections hold which.

</details>

### Rebinding a section

<!-- Arm: crates/nova_os_ui/src/ship/scene.rs:481-492 (bindable sections only);
     prompt text: scene.rs:805-809. Capture one key/button:
     crates/nova_os_ui/src/ship/rebind.rs:43-52; Esc cancels: rebind.rs:33-38
     (Escape swallowed: crates/nova_os_ui/src/terminal/input.rs:110-112);
     reserved flight controls refused: rebind.rs:8-13,64-70 with the list in
     crates/nova_ship/src/input/player/hints.rs:164-194; sharing allowed:
     rebind.rs:126-150; success note: rebind.rs:96. -->

Your weapon and thruster sections fire on rebindable inputs, and the SHIP app is where you rebind them:

1. Select a thruster, turret or torpedo bay and press <kbd>B</kbd> (or click `B Rebind`).
2. The panel arms: `PRESS A KEY OR MOUSE BUTTON - ESC CANCELS`.
3. The next key or mouse button you press becomes that section's whole binding: `Bound engine_port to LMB`.

A reserved flight control is refused on the spot - `SPACE is already used by flight control: burn` - and the capture stays armed for another try. Several sections may share one input (one key can fire every tube), and <kbd>Esc</kbd> backs out with `Rebind cancelled` without leaving the app.

## The monitor

<figure class="figure">
    <!-- Capture: assets/wiki-nova-os-terminal.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-nova-os-terminal.png</span
        >
        <span class="figure__placeholder-note"
            >The whole NOVACRT 9000 at the prompt: casing,
            vents, chin controls, a lit scrollback with the
            boot report and a did-you-mean suggestion.</span
        >
    </div>
</figure>

<!-- Chin controls: crates/nova_os_ui/src/terminal/casing.rs:417-596. Detents:
     crates/nova_os_ui/src/terminal/style.rs:74-87. SND mute gate:
     crates/nova_os_ui/src/terminal/sound.rs:29-40, dark bulb casing.rs:663-671.
     PWR close + orange LED: crates/nova_os_ui/src/terminal/shell.rs:72-79,106-118.
     Settings persist via the menu settings store:
     crates/nova_os_ui/src/terminal/components.rs:132-202. -->

The computer is a physical monitor - a NOVACRT 9000, per its brand plate - and its chin controls are real. They are clicked directly (everything above them, on the glass, is clicked *through* the curved picture), and their settings persist across sessions:

| Control | What it does |
| --- | --- |
| **BRIGHT** knob | Four detents of picture brightness, from dim to blazing. |
| **SCAN** knob | Four detents of scanline strength, from clean to heavy. |
| **SND** toggle | Mutes every computer sound; the indicator bulb goes dark. |
| **PWR** button | Powers the monitor off - the LED flashes orange while the picture collapses. |

<details class="explain">
<summary>Show explanation</summary>

<!-- CRT composite + pointer forwarding: crates/nova_os_ui/src/terminal/crt.rs:1-9,
     115-125, 271-344. Shader physics: assets/shaders/nova_os_crt.wgsl:53-78,
     146-186, 195-218 (hum bar, mains flicker, retrace beam); power collapse:
     nova_os_crt.wgsl:113-125; degauss: nova_os_crt.wgsl:57-61,127-133.
     Casing detail: crates/nova_os_ui/src/terminal/casing.rs:19-284,381-400.
     Sounds: crates/nova_os_ui/src/terminal/sound.rs:45-99,
     crates/nova_gameplay/src/audio/mod.rs:80-170. -->

The picture is a real tube, not a flat overlay: the terminal renders offscreen and is shown through one screen shader, so the green glyphs bloom into a phosphor halo, the image bows with barrel curvature under scanlines, grain and an edge vignette, a hum bar drifts, the mains flicker breathes, and a retrace beam sweeps by every few seconds. Launching or leaving an app fires a degauss - a brief horizontal shear and flash with a coil thump. Your mouse works through all of it: clicks on the glass are mapped through the same curvature, so the blip you see under the cursor is the blip you hit.

The monitor sounds like it looks, when SND is up: a key tick per character, a heavier clack on Enter, an ok blip when a command runs and an error buzz when it does not, a completion tick per Tab step, the power sweep up and down, and a low ambient bed humming the whole time it is on. Around the glass, the casing carries its bezel, corner screws, a vent grille and the spec line `P22 GREEN PHOSPHOR . 15 IN . TYPE CQ-4`.

</details>

## In the WFC arena

<!-- The arena is a developer example, not shipped content: Cargo.toml example
     entry, examples/playable/wfc_arena.rs:70-80. NOVA OS freezes the match
     and closing resumes it: examples/playable/wfc_arena/result.rs:425-446;
     Escape stays with NOVA OS while open: examples/playable/wfc_arena/pause.rs:51.
     Rebinds survive restart / return-to-lobby for the same hull:
     examples/playable/wfc_arena/lobby.rs:531-533,559-568,608-633. -->

The WFC arena - the developer match bench that fields wave-function-collapse ships, run from a source checkout with `cargo run --example wfc_arena` - carries the full computer when you fly a `player` slot. Opening NOVA OS mid-match freezes the entire fight, both teams hold in place while you read the map or service a section, and closing it resumes the brawl where it stood; the arena's own Escape pause menu waits its turn while the computer is open. Rebinds you make in the SHIP app stick to that hull through a match restart and a return to the lobby - reroll the seed and the new hull starts fresh.
