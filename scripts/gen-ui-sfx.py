#!/usr/bin/env python3
"""Offline renderer for Nova Protocol's INTERFACE and AVIONICS sound cues.

Two voices, one recipe book, because they are the same instrument in two rooms.

INTERFACE (`assets/sounds/`) is engine chrome: menus, the editor, objectives,
alerts. The eleven `nova_*` files baked by `scripts/gen-nova-os-sfx.py` are the
standard and are NOT touched here - this renders the rest of the family to
match them. That means their primitives: a square or triangle oscillator with
an exponential pitch slide, a short noise blip through one resonant band, and
the WebAudio attack/decay envelope ([`nova_sfx.env_ui`]) that is most of why
they read as one set.

AVIONICS (`assets/base/sounds/`) is the cockpit: lock, radar, safety, dry
magazine, and the two threat alarms. It is world CONTENT, so a mod can reship it, but it is an INSTRUMENT
and not machinery - a lock tone is the ship telling you something, not a thing
happening in space. So it is this recipe darkened: lower, shorter, with a
little of the world voice's metal ring underneath. That ring is the only
borrowing between the two renderers and it is deliberate.

Some cues RHYME on purpose. `editor_deny` and `radar_deny` are both a beating
pair of low squares, separated by pitch and by the avionics ring, because "no"
should be recognisable as "no" wherever a player meets it.

Every cue is peak-normalized to -3 dBFS like the rest, so the balance BETWEEN
cues is not set here - `UI_SFX_FILES` volumes and the per-cue constants in
`nova_ship/src/ship_audio/mod.rs` do that. Relative gains WITHIN a cue are the
only loudness decision this file makes.

Mono, 44100 Hz, 16-bit PCM WAV.

DETERMINISM: every cue seeds its own generator from a hash of its NAME, so a
rerun is byte-identical and adding a cue rewrites no other cue's bytes.

Run:  nix develop --command python3 scripts/gen-ui-sfx.py
      nix develop --command python3 scripts/gen-ui-sfx.py --only menu_select
"""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from nova_sfx import (  # noqa: E402  (path set up just above)
    bandpass,
    env_exp,
    env_ui,
    highpass,
    lowpass,
    modes,
    osc,
    peaking,
    place,
    render_all,
    silence,
    white,
)

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The NOVA OS envelope timings, kept exactly so a new cue sits in the family.
TONE_ATTACK = 0.006
NOISE_ATTACK = 0.004


# --- The two primitives ---------------------------------------------------


def tone(duration, freq, gain, shape="sine", slide_to=None, attack=TONE_ATTACK):
    """One oscillator with an optional pitch slide - the family's voice."""
    return osc(duration, freq, shape, slide_to=slide_to) * env_ui(duration, attack) * gain


def blip(duration, freq, q, gain, rng, kind="bandpass", attack=NOISE_ATTACK):
    """A noise burst through one filter - the family's consonant.

    `q` shapes the band for "bandpass"; the two rolloff kinds take `freq` as a
    corner and ignore it, which is the same thing the PoC's biquads did with a
    Q on a lowpass.
    """
    x = white(duration, rng)
    if kind == "lowpass":
        x = lowpass(x, freq, order=2)
    elif kind == "highpass":
        x = highpass(x, freq, order=2)
    else:
        x = peaking(x, freq, q)
    return x * env_ui(duration, attack) * gain


def instrument_ring(duration, rng, spec, gain):
    """A short metallic answer - the avionics voice's one borrowed layer.

    This is what separates a cockpit instrument from a menu button: the panel
    it is mounted in answers the tone. Kept short and quiet; let it run and the
    cue stops being chrome.
    """
    excite = white(duration, rng) * env_exp(duration, 0.0003, duration * 0.12)
    return modes(excite, spec) * gain


# --- Interface: menus -----------------------------------------------------


def menu_select(rng):
    """Commit. The one cue in the set that rises without arriving anywhere."""
    out = silence(0.075)
    out = place(out, blip(0.030, 2400.0, 1.6, 0.55, rng), 0.0)
    out = place(out, tone(0.050, 700.0, 0.30, "square", slide_to=1050.0), 0.004)
    out = place(out, tone(0.060, 150.0, 0.22, "triangle"), 0.0)
    return out


def menu_back(rng):
    """Leave. `menu_select` played backwards in every way that matters."""
    out = silence(0.08)
    out = place(out, blip(0.032, 1500.0, 1.2, 0.50, rng), 0.0)
    out = place(out, tone(0.055, 900.0, 0.28, "square", slide_to=560.0), 0.004)
    out = place(out, tone(0.050, 120.0, 0.18, "triangle"), 0.0)
    return out


def menu_focus(rng):
    """The cursor moving. Fires more than anything else in the game, so it is
    the shortest cue with the least in it - one blip and a hint of pitch."""
    out = silence(0.03)
    out = place(out, blip(0.022, 3000.0, 3.0, 0.50, rng), 0.0)
    out = place(out, tone(0.020, 1900.0, 0.10, "sine"), 0.0)
    return out


def ui_toggle(rng):
    """A setting changing state: two pips, low then high.

    Two discrete pips rather than `menu_select`'s slide, because a toggle went
    from one place to another and a slide is a journey.
    """
    out = silence(0.075)
    out = place(out, blip(0.020, 2200.0, 2.4, 0.28, rng), 0.0)
    out = place(out, tone(0.028, 620.0, 0.32, "square"), 0.0)
    out = place(out, tone(0.032, 880.0, 0.30, "square"), 0.030)
    return out


def ui_tick(rng):
    """A slider detent. The smallest sound the game makes."""
    out = silence(0.018)
    out = place(out, blip(0.014, 3600.0, 4.0, 0.60, rng), 0.0)
    out = place(out, tone(0.012, 2400.0, 0.12, "square"), 0.0)
    return out


# --- Interface: objectives and alerts -------------------------------------


def objective_new(rng):
    """A new objective. Attention, not alarm: it rises and stops."""
    out = silence(0.30)
    out = place(out, blip(0.050, 700.0, 0.9, 0.20, rng), 0.0)
    out = place(out, tone(0.090, 880.0, 0.30, "square"), 0.0)
    out = place(out, tone(0.140, 1320.0, 0.26, "square"), 0.085)
    out = place(out, tone(0.100, 165.0, 0.16, "triangle"), 0.0)
    return out


def objective_complete(rng):
    """Done. Three steps up, and the last one is held - the arrival
    `objective_new` deliberately withholds."""
    out = silence(0.42)
    out = place(out, blip(0.060, 900.0, 0.8, 0.16, rng), 0.0)
    out = place(out, tone(0.080, 784.0, 0.30, "square"), 0.0)
    out = place(out, tone(0.080, 1046.0, 0.28, "square"), 0.075)
    out = place(out, tone(0.220, 1568.0, 0.30, "square"), 0.150)
    out = place(out, tone(0.160, 196.0, 0.18, "triangle"), 0.150)
    return out


def objective_fail(rng):
    """Failed. The mirror of complete: three steps down, and the last one
    keeps falling instead of landing."""
    out = silence(0.5)
    out = place(out, blip(0.090, 420.0, 0.7, 0.20, rng, kind="lowpass"), 0.0)
    out = place(out, tone(0.100, 466.0, 0.30, "square"), 0.0)
    out = place(out, tone(0.120, 370.0, 0.28, "square"), 0.095)
    out = place(out, tone(0.280, 233.0, 0.32, "square", slide_to=147.0), 0.200)
    out = place(out, tone(0.240, 98.0, 0.22, "triangle"), 0.200)
    return out


def comms_line(rng):
    """The squelch before someone speaks.

    Band-limited to 320-3200 Hz at the end, which is the whole identity: it is
    not a menu sound that happens to precede dialogue, it is a RADIO. Squelch
    open, carrier, squelch closed.
    """
    out = silence(0.22)
    out = place(out, blip(0.035, 2600.0, 1.4, 0.35, rng), 0.0)
    out = place(out, tone(0.100, 1180.0, 0.24, "square"), 0.020)
    out = place(out, tone(0.100, 1770.0, 0.16, "square"), 0.020)
    out = place(out, blip(0.070, 3200.0, 0.9, 0.22, rng, kind="highpass"), 0.130)
    return bandpass(out, 320.0, 3200.0, order=2)


def warn_lock(rng):
    """Someone has locked you. Three fast pips on a detuned pair.

    The 24 Hz beat between the two squares is what makes it read as an ALARM
    rather than a notification - a clean tone at this pitch sounds like a menu.
    """
    out = silence(0.55)
    for at in (0.0, 0.14, 0.28):
        out = place(out, tone(0.075, 1480.0, 0.32, "square"), at)
        out = place(out, tone(0.075, 1504.0, 0.20, "square"), at + 0.002)
        out = place(out, blip(0.020, 2600.0, 2.0, 0.12, rng), at)
    return out


def warn_hull(rng):
    """The hull is critical. Everything `warn_lock` is, an octave down and
    half the speed - slower is more serious, which is the opposite of how
    alarms usually escalate and is why this one lands."""
    out = silence(0.78)
    for at in (0.0, 0.24, 0.48):
        out = place(out, tone(0.140, 330.0, 0.34, "square", slide_to=247.0), at)
        out = place(out, tone(0.140, 82.0, 0.24, "triangle"), at)
        out = place(out, blip(0.050, 500.0, 0.8, 0.16, rng, kind="lowpass"), at)
    return out


# --- Interface: the editor ------------------------------------------------


def editor_place(rng):
    """A part seating in the grid. The heaviest cue in the interface voice -
    it is the one moment the editor has real mass."""
    out = silence(0.09)
    out = place(out, blip(0.030, 1200.0, 1.4, 0.50, rng), 0.0)
    out = place(out, tone(0.070, 220.0, 0.34, "triangle"), 0.0)
    out = place(out, tone(0.045, 660.0, 0.20, "square", slide_to=880.0), 0.004)
    return out


def editor_remove(rng):
    """A part lifting out. Lighter than placing and falling instead of
    rising, so the pair reads as a direction."""
    out = silence(0.08)
    out = place(out, blip(0.028, 1700.0, 1.6, 0.45, rng), 0.0)
    out = place(out, tone(0.055, 520.0, 0.24, "square", slide_to=390.0), 0.003)
    out = place(out, tone(0.050, 165.0, 0.18, "triangle"), 0.0)
    return out


def editor_rotate(rng):
    """A rotation detent. `ui_tick` with mass - the same gesture on a part
    rather than on a slider."""
    out = silence(0.035)
    out = place(out, blip(0.022, 2200.0, 3.0, 0.55, rng), 0.0)
    out = place(out, tone(0.026, 880.0, 0.16, "triangle"), 0.0)
    return out


def editor_deny(rng):
    """That placement is illegal. A beating pair of low squares - 8 Hz apart,
    which is a rasp rather than a chord."""
    out = silence(0.16)
    out = place(out, blip(0.050, 600.0, 0.9, 0.18, rng, kind="lowpass"), 0.0)
    out = place(out, tone(0.130, 138.0, 0.36, "square"), 0.0)
    out = place(out, tone(0.130, 146.0, 0.22, "square"), 0.0)
    return out


# --- Avionics -------------------------------------------------------------


def lock_on(rng):
    """The targeting radar taking hold. Two steps up, and the panel rings."""
    out = silence(0.16)
    out = place(out, tone(0.050, 740.0, 0.30, "square"), 0.0)
    out = place(out, tone(0.090, 1110.0, 0.30, "square"), 0.048)
    out = place(
        out,
        instrument_ring(0.10, rng, [(2400.0, 0.05, 1.0), (3900.0, 0.03, 0.4)], 0.50),
        0.048,
    )
    out = place(out, tone(0.060, 185.0, 0.16, "triangle"), 0.0)
    return out


def lock_off(rng):
    """The lock dropping. No ring: the instrument stopped, it did not act."""
    out = silence(0.14)
    out = place(out, tone(0.050, 1110.0, 0.26, "square"), 0.0)
    out = place(out, tone(0.085, 740.0, 0.28, "square", slide_to=555.0), 0.045)
    out = place(out, tone(0.050, 148.0, 0.14, "triangle"), 0.0)
    return out


def radar_deny(rng):
    """The radar refusing - nothing in the cone, or nothing it will take.

    `editor_deny`'s rhyme, moved up into the cockpit's register and given the
    panel ring so it belongs to the instruments rather than to the menus.
    """
    out = silence(0.19)
    out = place(out, blip(0.055, 700.0, 0.9, 0.18, rng, kind="lowpass"), 0.0)
    out = place(out, tone(0.150, 196.0, 0.34, "square"), 0.0)
    out = place(out, tone(0.150, 207.0, 0.20, "square"), 0.0)
    out = place(out, instrument_ring(0.09, rng, [(1420.0, 0.04, 1.0)], 0.28), 0.0)
    return out


def radar_retarget(rng):
    """Cycling to the next contact. Fires on every tap of the target key, so
    it is the avionics answer to `menu_focus`: almost nothing."""
    out = silence(0.045)
    out = place(out, blip(0.026, 2800.0, 3.2, 0.55, rng), 0.0)
    out = place(out, tone(0.030, 1480.0, 0.14, "square"), 0.0)
    return out


def safety_on(rng):
    """The ship going cold: weapons safed, systems lowered. A descent that
    settles, with the panel answering once as it seats."""
    out = silence(0.22)
    out = place(out, blip(0.050, 380.0, 0.8, 0.22, rng, kind="lowpass"), 0.0)
    out = place(out, tone(0.100, 420.0, 0.26, "square", slide_to=210.0), 0.0)
    out = place(out, tone(0.160, 105.0, 0.24, "triangle"), 0.010)
    out = place(out, instrument_ring(0.10, rng, [(620.0, 0.06, 1.0)], 0.35), 0.008)
    return out


def ammo_dry(rng):
    """The magazine is empty.

    Two pips down, and the ring is the MECHANISM rather than the panel - high
    and short, the same 1.8-3 kHz metal the world voice's `pdc_dry_fire` uses.
    The two cues fire at nearly the same moment and are meant to be heard as
    one event from two places: the gun outside, the gauge inside.
    """
    out = silence(0.24)
    out = place(out, tone(0.060, 660.0, 0.28, "square"), 0.0)
    out = place(out, tone(0.100, 440.0, 0.30, "square"), 0.060)
    out = place(
        out,
        instrument_ring(0.08, rng, [(1850.0, 0.025, 1.0), (3100.0, 0.015, 0.5)], 0.90),
        0.060,
    )
    out = place(out, tone(0.080, 110.0, 0.16, "triangle"), 0.060)
    return out


# name -> (renderer, output path relative to the repo root)
#
# The split is OWNERSHIP, not voice: `assets/sounds/` is engine chrome the game
# ships and keys by `UiSfx`, `assets/base/sounds/` is mod content addressed by
# an `AssetRef`. The avionics cues live in the second because a total
# conversion should be able to reship a cockpit.
CUES = {
    "menu_select": (menu_select, "assets/sounds/menu_select.wav"),
    "menu_back": (menu_back, "assets/sounds/menu_back.wav"),
    "menu_focus": (menu_focus, "assets/sounds/menu_focus.wav"),
    "ui_toggle": (ui_toggle, "assets/sounds/ui_toggle.wav"),
    "ui_tick": (ui_tick, "assets/sounds/ui_tick.wav"),
    "objective_new": (objective_new, "assets/sounds/objective_new.wav"),
    "objective_complete": (objective_complete, "assets/sounds/objective_complete.wav"),
    "objective_fail": (objective_fail, "assets/sounds/objective_fail.wav"),
    "comms_line": (comms_line, "assets/sounds/comms_line.wav"),
    "warn_lock": (warn_lock, "assets/base/sounds/warn_lock.wav"),
    "warn_hull": (warn_hull, "assets/base/sounds/warn_hull.wav"),
    "editor_place": (editor_place, "assets/sounds/editor_place.wav"),
    "editor_remove": (editor_remove, "assets/sounds/editor_remove.wav"),
    "editor_rotate": (editor_rotate, "assets/sounds/editor_rotate.wav"),
    "editor_deny": (editor_deny, "assets/sounds/editor_deny.wav"),
    "lock_on": (lock_on, "assets/base/sounds/lock_on.wav"),
    "lock_off": (lock_off, "assets/base/sounds/lock_off.wav"),
    "radar_deny": (radar_deny, "assets/base/sounds/radar_deny.wav"),
    "radar_retarget": (radar_retarget, "assets/base/sounds/radar_retarget.wav"),
    "safety_on": (safety_on, "assets/base/sounds/safety_on.wav"),
    "ammo_dry": (ammo_dry, "assets/base/sounds/ammo_dry.wav"),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", action="append", help="render just this cue (repeatable)")
    args = parser.parse_args()
    render_all(CUES, args.only or sorted(CUES), REPO_ROOT)


if __name__ == "__main__":
    main()
