# Settings

The Settings menu is the same modal from **both** the main menu and the pause
menu, so you can change anything mid-run. Every choice is **remembered across
restarts** - saved to a config file on the desktop build and to browser storage
on the web.

<figure class="figure">
    <!-- Capture: assets/wiki-settings.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot</span>
        <span class="figure__placeholder-name">assets/wiki-settings.png</span>
        <span class="figure__placeholder-note">The Settings modal open over the main menu on its Audio tab: the tab bar, the four volume sliders and the Back button.</span>
    </div>
    <figcaption class="figure__caption">One Settings modal, reachable from the main menu and the pause menu.</figcaption>
</figure>

## Audio

Four draggable sliders, top to bottom. Each shows its level as a whole percent
and takes effect live as you drag.

| Slider | What it moves |
| --- | --- |
| Master | Everything at once. The one control most players ever touch. |
| Interface | Menu clicks, HUD ticks, the editor and the flight computer - the sounds that come from the cockpit, not from the world. |
| World | The engine hum, weapons, hits, the rocks breaking up - everything out there, heard at the distance it happens. |
| Music | Reserved. The slider moves and the setting is saved, but the game ships no music yet. |

Master multiplies the other three, so a track you have turned down stays down
when you raise Master.

## Graphics quality

The **Preset** row offers **Low / Medium / High**; the four lever rows under
it are what a preset sets. Move any lever on its own and the preset row reads
**Custom**; land the levers back on a tier's values and it reads that tier
again. Every row applies live.

| Row | Choices | What it moves |
| --- | --- | --- |
| Preset | Low, Medium, High, Custom | All four levers at once. Custom is read-only: it means the levers match no tier. |
| Particles | Off, On | Torpedo and muzzle bursts, hit sparks and the railgun wake. Off spawns none of them. |
| Resolution | 50%, 70%, 85%, 100% | The internal resolution the 3D world is drawn at before it is upscaled to the window. The HUD and menus stay crisp. |
| Flash lights | Off, 3, 6 | How many detonations and slugs may light the hull at once. |
| Camera shake | Off, On | The shake on hits and detonations. |

| Preset | Camera shake | Flash lights | Particles | Resolution |
| --- | --- | --- | --- | --- |
| High | on | 6 | on | 100% |
| Medium | off | 3 | on | 100% |
| Low | off | off | off | 70% |

<details class="explain">
<summary>Show explanation</summary>

Particles Off is **spawn-less**: the bursts and the wake are not created at
all, rather than created and hidden, which is the cut the frame-time baseline
measures as a real combat cost. A lower **Resolution** renders the world
smaller and upscales it to fill the window - a lever aimed at fill-bound
hardware (laptop iGPUs, phones). The HUD and menus stay crisp and fully
clickable; only the 3D world softens. On a strong discrete GPU the win is
small, so it is a knob for the low end rather than a general speed-up. **Flash
lights** are paid per pixel they touch, so the step from six to three is a
real one on a busy salvo and costs nothing on a single hit.

</details>

## Window

The desktop build opens **borderless fullscreen**, filling the monitor it
starts on. The **Windowed** row on the Graphics tab gives the desk back, and
whichever you pick is remembered - an install that has already chosen keeps its
choice.

A window will not be dragged narrower than **640x600**. The Settings panel is
the widest thing the game draws, and below that size there is no layout left to
read it in.

**VSync** sits under the same heading. On, the default, presents each frame
on the display's refresh; Off presents as soon as a frame is drawn, for a
player measuring frame times or chasing input latency, at the cost of tearing.

The web build has neither row: the game fits the canvas the page gives it and
the browser presents on its own schedule. The panels themselves are sized as a
share of the window, so a narrow browser column still gets a readable Settings
modal with its Back button on screen.

## Pause when unfocused

Alt-tab, or clicking away from the game, leaves the run **going**: the fight
does not wait. Switch **Pause when unfocused** to **On** under **Focus** on the
Graphics tab and losing the window opens the ordinary pause menu instead.
Coming back never resumes by itself; **Resume** is yours. It works the same in
a browser tab.

## Controls

<figure class="figure">
    <!-- Capture: assets/wiki-controls.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot</span>
        <span class="figure__placeholder-name">assets/wiki-controls.png</span>
        <span class="figure__placeholder-note">The Controls tab open on the FLIGHT group: one row per action, with the keyboard keycap and the pad glyph in their own columns.</span>
    </div>
    <figcaption class="figure__caption">One group at a time, drawn as the buttons you press.</figcaption>
</figure>

The **Controls** tab is where you REBIND. It shows one binding group at a time -
FLIGHT, TARGETING, CAMERA, SCENARIO, SYSTEM, INTERFACE, MAP, SHIP and
INVENTORY -
with the keyboard and gamepad columns side by side, drawn as the keycaps and pad
glyphs you actually press. The same table is laid out on the
[Keybinds](../keybinds/) page.

Click a chip and it prompts `PRESS A KEY`; the next key, mouse button or pad
button you press becomes that column's whole binding. <kbd>Esc</kbd> backs out,
which is why no row can be bound to it, and **Reset Defaults** puts the whole
table back.

A press something else could answer at the same instant is refused by name -
`W is already bound to Main Drive` - and the chip stays armed for another try.
Sharing a key across screens is fine and deliberate: <kbd>G</kbd> is GO TO in
flight and the mates overlay on the interface's Ship pane, and only one of them is ever
listening.

Two rows are read-only, drawn greyed: <kbd>Esc</kbd> and the pad's pause chord.
They are the way out of every other screen, including this one.

A section's weapon, thruster or mining beam key is not here - that is per
ship, assigned in the editor or on the interface's Ship pane. From the pause
menu, a flight row cannot take a key that a section of your ship holds:
`V is already bound to the ship's mining_beam section`.

### Mouse

**MOUSE** is a group in the same list, and the one page with nothing to rebind
on it: three sliders and the **Invert Y** switch, and no **Reset Defaults** -
that button is a keybinding operation and would not put a sensitivity back.

| Slider | Range | Default | What it scales |
| --- | --- | --- | --- |
| Look | 100-300% | 200% | Ship steering, free look and turret aim |
| RCS | 100-500% | 100% | Mouse-driven translation, the docking fine-adjust |
| Free Camera | 100-300% | 200% | Mouse look on the free (WASD) camera |

Each `100%` is that slider's OWN baseline, not a shared number - the three gains
behind them differ by two orders of magnitude, so a single scale would be
unreadable. Only the mouse is affected; the gamepad sticks and the keyboard have
their own curves and are untouched.

**Invert Y** turns mouse look's pitch the other way: push the mouse forward to
look down. It applies to the Look path only - ship steering, free look and
turret aim. The free (WASD) camera and RCS translation keep their direction.

A slider or switch moved from the PAUSE menu takes effect on the ship you are
already flying - there is no reload and no ship to respawn.

<details class="explain">
<summary>Why Look 200% is the new default</summary>

Mouse look shipped before this page existed, on one fixed gain. That gain is
what `300%` is now, and it was too fast for most pointers - so `100%` is a third
of it and the default sits at `200%`, about two thirds of what mouse look used
to feel like. If the old speed was right for you, put Look at `300%`.

</details>

## Interface

| Row | Choices | What it moves |
| --- | --- | --- |
| Theme | Phosphor, Hardware, and any theme an enabled mod declares | The look of every panel, button and readout. A theme that fails to load says so under the row and the game falls back to Phosphor. |
| UI scale | 75%, 100%, 125%, 150% | How large the interface draws. Panels are capped at a share of the window, so a larger scale still keeps them on screen. |
| Training prompt | On, Off | The Basic Training offer in the main menu's corner. |
| Field notes | On, Off | The handbook hint card beside it. |
