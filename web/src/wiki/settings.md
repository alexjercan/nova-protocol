# Settings

The Settings menu is the same modal from **both** the main menu and the pause
menu, so you can change anything mid-run. Every choice is **remembered across
restarts** - saved to a config file on the desktop build and to browser storage
on the web.

<figure class="figure">
    <!-- Capture: assets/wiki-settings.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
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

A single **Low / Medium / High** preset trades visual richness for performance.
It does two things at once: it tunes the combat *juice*, and on the lower tiers
it drops the heavier effects for weak machines.

| Preset | Camera shake | Hit flashes | Particle bursts | 3D world resolution |
| --- | --- | --- | --- | --- |
| High | on | on | full | native |
| Medium | off | on | full | native |
| Low | off | off | not spawned | reduced, upscaled |

<details class="explain">
<summary>Show explanation</summary>

Low is **spawn-less**: torpedo and muzzle particle bursts and the railgun wake
are not created at all, rather than created and hidden. Low also takes no
transient lights, so the light a railgun slug carries with it is one of the
things it does not draw. It also renders the world at a **reduced
internal resolution** and upscales it to fill the window - a lever aimed at
fill-bound hardware (laptop iGPUs, phones). The HUD and menus stay crisp and
fully clickable; only the 3D world softens. On a strong discrete GPU the speed
win is small, so Low is a knob for the low end rather than a general speed-up.

</details>

## Controls

<figure class="figure">
    <!-- Capture: assets/wiki-controls.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag">Screenshot needed</span>
        <span class="figure__placeholder-name">assets/wiki-controls.png</span>
        <span class="figure__placeholder-note">The Controls tab open on the FLIGHT group: one row per action, with the keyboard keycap and the pad glyph in their own columns.</span>
    </div>
    <figcaption class="figure__caption">One group at a time, drawn as the buttons you press.</figcaption>
</figure>

The **Controls** tab is where you REBIND. It shows one binding group at a time -
FLIGHT, TARGETING, CAMERA, SCENARIO, SYSTEM, NOVA OS and the two viewer apps -
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
flight and the mates overlay in the ship computer, and only one of them is ever
listening.

Two rows are read-only, drawn greyed: <kbd>Esc</kbd> and the pad's pause chord.
They are the way out of every other screen, including this one.

A section's weapon or thruster trigger is not here - that is per ship, assigned
in the editor or in the ship computer's SHIP app.
