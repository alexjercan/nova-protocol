#!/usr/bin/env python3
"""Generate tiny placeholder WAV sound effects for Nova Protocol.

The game has no real audio yet. Until a sound designer supplies the final
assets (see assets/base/sounds/README.md), this writes a short, deterministic
placeholder for each gameplay cue so the game is audibly wired end to end and
runs out of the box. Each cue has its own character (noise burst, pitch sweep,
or steady tone) so they are distinguishable by ear while testing.

Run from the repo root:  python3 scripts/gen-placeholder-sounds.py

It uses only the Python standard library (wave, struct, math, random), so it
needs no ffmpeg/sox or third-party package. Overwrite the generated files with
real assets at the same paths and no code changes are needed.
"""

import math
import os
import random
import struct
import wave

SAMPLE_RATE = 44100

# Each cue is rendered by one of three synths, chosen to fit the event:
#   "noise"  - a filtered noise burst with a fast decay (impacts, gunfire).
#   "sweep"  - a sine gliding freq_start -> freq_end (launches, whooshes).
#   "tone"   - a steady two-oscillator drone, no decay (the looping engine hum).
#
# name -> (kind, freq_start, freq_end, duration_s, peak_amp 0..1)
SOUNDS = {
    # PDC/turret round: dry, short, quiet (it fires very often).
    "turret_fire": ("noise", 0.0, 0.0, 0.07, 0.22),
    # Torpedo launch: airy rising whoosh.
    "torpedo_launch": ("sweep", 240.0, 620.0, 0.34, 0.24),
    # Explosion / section destruction / detonation: noisy burst, fast decay.
    "explosion": ("noise", 0.0, 0.0, 0.45, 0.32),
    # Impact / damage tick: short low thud.
    "impact": ("sweep", 260.0, 120.0, 0.10, 0.26),
    # Thruster engine loop: a steady low hum, rendered to loop seamlessly (an
    # integer number of cycles at both partials, no fade envelope).
    "thruster_loop": ("tone", 70.0, 140.0, 1.0, 0.18),
    # New objective posted: short neutral blip.
    "objective_new": ("sweep", 520.0, 560.0, 0.12, 0.20),
    # Objective completed: rising fifth, reads as success.
    "objective_complete": ("sweep", 440.0, 660.0, 0.22, 0.22),
    # Radar lock acquired: quick rising chirp, higher and shorter than the
    # objective blip so the two never blur (once per radar gesture).
    "lock_on": ("sweep", 700.0, 980.0, 0.09, 0.22),
    # Lock cleared: the falling mirror of lock_on.
    "lock_off": ("sweep", 980.0, 640.0, 0.09, 0.20),
    # Weapons safety re-engaging (hot -> cold): dull low click, deliberately
    # unexciting - "the guns just went quiet".
    "safety_on": ("sweep", 320.0, 180.0, 0.06, 0.22),
    # Radar denied (computer grants no Lock capability): low flat buzz.
    "radar_deny": ("tone", 110.0, 112.0, 0.16, 0.20),
    # Salvage crate picked up: a light, bright rising "ding" - short and quiet,
    # its own register above the near-flat objective blip (520->560) and below
    # the radar lock chirp (700->980) so a pickup never blurs with either.
    "salvage_pickup": ("sweep", 640.0, 860.0, 0.10, 0.20),
    # Menu button press: a crisp, short rising UI click.
    "menu_select": ("sweep", 620.0, 720.0, 0.06, 0.18),
    # Pause/settings toggle: a soft, slightly falling two-state blip, lower and
    # gentler than the menu-select click so a toggle reads distinct from a press.
    "ui_toggle": ("sweep", 430.0, 380.0, 0.05, 0.16),
    # Turret dry-fire on an empty magazine: a dull, low descending click - a dead
    # trigger, clearly not the noisy `turret_fire` shot.
    "dry_fire": ("sweep", 300.0, 160.0, 0.06, 0.18),
    # Radar re-designation tick: a very short, quiet blip - subtler than the
    # `lock_on` acquire chirp because it can repeat across one held gesture.
    "radar_retarget": ("sweep", 600.0, 660.0, 0.045, 0.14),
}


def _clamp_sample(x):
    return max(-1.0, min(1.0, x))


def render(kind, f0, f1, duration, amp):
    """Render one cue to mono 16-bit PCM bytes.

    Deterministic: the noise RNG is seeded by the caller, and the tone uses an
    integer number of cycles so its ends meet for gapless looping.
    """
    total = int(SAMPLE_RATE * duration)
    frames = bytearray()

    if kind == "tone":
        # Steady drone: two sine partials, each snapped to a whole number of
        # cycles across the clip so the loop point is click-free. No decay.
        cyc0 = max(1, round(f0 * duration))
        cyc1 = max(1, round(f1 * duration))
        for i in range(total):
            t = i / total if total else 0.0
            sample = 0.7 * math.sin(2.0 * math.pi * cyc0 * t)
            sample += 0.3 * math.sin(2.0 * math.pi * cyc1 * t)
            frames += struct.pack("<h", int(_clamp_sample(amp * sample) * 32767.0))
        return bytes(frames)

    # One-shot cues: fast quadratic decay so they read as transients, with a
    # short linear fade-in to kill the initial click.
    fade = max(1, int(SAMPLE_RATE * 0.003))
    phase = 0.0
    for i in range(total):
        t = i / total if total else 0.0
        env = amp * (1.0 - t) ** 2
        if i < fade:
            env *= i / fade
        if kind == "noise":
            sample = env * random.uniform(-1.0, 1.0)
        else:  # "sweep"
            freq = f0 + (f1 - f0) * t
            phase += 2.0 * math.pi * freq / SAMPLE_RATE
            sample = env * math.sin(phase)
        frames += struct.pack("<h", int(_clamp_sample(sample) * 32767.0))
    return bytes(frames)


def write_wav(path, data):
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        w.writeframes(data)
    print("wrote", path)


def main():
    out_dir = os.path.join(os.path.dirname(__file__), "..", "assets", "base", "sounds")
    out_dir = os.path.normpath(out_dir)
    os.makedirs(out_dir, exist_ok=True)

    for name, (kind, f0, f1, duration, amp) in SOUNDS.items():
        # Seed per name so regenerating gives byte-identical output.
        random.seed(name)
        write_wav(os.path.join(out_dir, name + ".wav"), render(kind, f0, f1, duration, amp))


if __name__ == "__main__":
    main()
