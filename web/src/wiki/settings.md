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
        <span class="figure__placeholder-note">The Settings modal open over the main menu: the master volume slider, the Low/Medium/High graphics preset selector, and the keybind reference panel.</span>
    </div>
    <figcaption class="figure__caption">One Settings modal, reachable from the main menu and the pause menu.</figcaption>
</figure>

## Audio

A draggable **master volume** slider scales all game audio at once - the engine
hum, weapons, radar ticks and the world's impact sounds. It takes effect live as
you drag.

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

Low is **spawn-less**: torpedo and muzzle particle bursts are not created at
all, rather than created and hidden. It also renders the world at a **reduced
internal resolution** and upscales it to fill the window - a lever aimed at
fill-bound hardware (laptop iGPUs, phones). The HUD and menus stay crisp and
fully clickable; only the 3D world softens. On a strong discrete GPU the speed
win is small, so Low is a knob for the low end rather than a general speed-up.

</details>

## Controls reference

The **Controls** panel is a read-only reference of the current keyboard and
gamepad bindings - flight, targeting, camera and pause - the same reference laid
out on the [Keybinds](../keybinds/) page. It is there to check a binding without
leaving the game; the bindings themselves are fixed (weapon fire is the one thing
you assign, per section, in the editor).
