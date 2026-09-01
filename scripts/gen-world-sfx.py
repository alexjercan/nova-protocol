#!/usr/bin/env python3
"""Offline renderer for Nova Protocol's WORLD sound effects.

The world voice, as specified in `tasks/20260824-125955/INVENTORY.md`: ORDINARY
game sounds. Combat in a vacuum would be silent and a silent fight is a boring
fight, so Nova's guns sound the way a film's guns sound - present, bright and
physical - and the game does not apologise for it.

(A "vacuum sounds" mode, where every cue is instead conducted through the
player's own hull or synthesized by the ship's computer as feedback, is a
possible FUTURE setting. It is deliberately not built. Every world sound is mod
content addressed by an `AssetRef`, so that mode is a second set of files
behind the same names whenever it is wanted, and nothing here has to change to
allow it.)

Every cue is built from the same three layers - which is just how a percussive
sound works, not a physics argument:

  1. TRANSIENT (0-8 ms)   the crack. Broadband and bright: this is where a gun
                          gets its edge and most of its identity.
  2. BODY (10-200 ms)     filtered noise carrying the mass, 80-800 Hz. The
                          chest punch.
  3. RING (up to ~400 ms) a few detuned modes, the hardware answering.

Mono, 44100 Hz, 16-bit PCM WAV, peak-normalized to -3 dBFS. Balance is NOT set
here: the per-cue volume constants in `nova_ship/src/ship_audio/mod.rs` do the
mixing.

A cue is designed for the RATE IT IS HEARD AT. The PDC authors 50 rounds a
second per muzzle, but its cue throttles to twenty and stays there - a held
loop at the true rate fuses into a buzz, and a burst of separable rounds reads
better than a saw. Twenty a second is the number its round is shaped against.

The INTERFACE voice is a different renderer with a different brief - the NOVA
OS family in `scripts/gen-nova-os-sfx.py`. Keeping the two voices disjoint is
what makes both legible, so do not add a terminal blip here.

DETERMINISM: every cue seeds its own generator from a hash of its NAME, so a
rerun is byte-identical AND adding a cue rewrites no other cue's bytes. That
is the one thing `gen-nova-os-sfx.py` got wrong (it draws from a single shared
stream in list order, so an insertion churns every later file).

Run:  nix develop --command python3 scripts/gen-world-sfx.py
      nix develop --command python3 scripts/gen-world-sfx.py --only pdc_gatling_fire
"""

import argparse
import hashlib
import math
import os
import struct
import wave

import numpy as np
from scipy import signal

SAMPLE_RATE = 44100
HEADROOM_DBFS = -3.0
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


# --- Foundations ----------------------------------------------------------


def rng_for(name):
    """A generator seeded from the cue's NAME, so cues are independent."""
    digest = hashlib.blake2b(name.encode("utf-8"), digest_size=8).digest()
    return np.random.default_rng(int.from_bytes(digest, "big"))


def n_samples(duration):
    return int(round(duration * SAMPLE_RATE))


def silence(duration):
    return np.zeros(n_samples(duration))


def white(duration, rng):
    return rng.standard_normal(n_samples(duration))


def loop_noise(duration, shape, rng):
    """Colored noise that is PERFECTLY periodic over `duration`.

    Built in the frequency domain - a magnitude spectrum from `shape(freqs)`
    with uniformly random phases, inverse-transformed - so the last sample
    joins the first with no seam at all. This is what the loops use; a
    time-domain crossfade always leaves a soft spot the ear finds after the
    third repeat.
    """
    n = n_samples(duration)
    freqs = np.fft.rfftfreq(n, 1.0 / SAMPLE_RATE)
    magnitude = shape(np.maximum(freqs, 1e-6))
    phases = rng.uniform(0.0, 2.0 * math.pi, len(freqs))
    phases[0] = 0.0
    spectrum = magnitude * np.exp(1j * phases)
    return np.fft.irfft(spectrum, n)


def env_ad(duration, attack, decay, curve=3.0):
    """Attack-decay envelope: a linear rise then a power-law fall to zero."""
    n = n_samples(duration)
    t = np.arange(n) / SAMPLE_RATE
    rise = np.clip(t / max(attack, 1e-6), 0.0, 1.0)
    fall = np.clip(1.0 - (t - attack) / max(decay, 1e-6), 0.0, 1.0) ** curve
    return rise * fall


def env_exp(duration, attack, tau):
    """Attack then EXPONENTIAL decay with time constant `tau`, ending clean."""
    n = n_samples(duration)
    t = np.arange(n) / SAMPLE_RATE
    rise = np.clip(t / max(attack, 1e-6), 0.0, 1.0)
    fall = np.exp(-np.maximum(t - attack, 0.0) / max(tau, 1e-6))
    # Force the tail to zero so a clip never ends on a step.
    fall *= np.clip((duration - t) / (0.15 * duration), 0.0, 1.0)
    return rise * fall


def bandpass(x, low, high, order=2):
    nyq = SAMPLE_RATE * 0.5
    sos = signal.butter(
        order, [max(low, 1.0) / nyq, min(high, nyq * 0.99) / nyq], btype="bandpass", output="sos"
    )
    return signal.sosfilt(sos, x)


def lowpass(x, cutoff, order=2):
    nyq = SAMPLE_RATE * 0.5
    sos = signal.butter(order, min(cutoff, nyq * 0.99) / nyq, btype="lowpass", output="sos")
    return signal.sosfilt(sos, x)


def highpass(x, cutoff, order=2):
    nyq = SAMPLE_RATE * 0.5
    sos = signal.butter(order, max(cutoff, 1.0) / nyq, btype="highpass", output="sos")
    return signal.sosfilt(sos, x)


def resonator(x, freq, decay, gain=1.0):
    """Excite one 2-pole resonance - a single mode of the metal.

    `decay` is the seconds the mode takes to fall 60 dB. Ringing NOISE through
    these is the physical order of events (something strikes, the structure
    answers) and it is why the result reads as a hull rather than as a synth
    patch.
    """
    r = math.exp(-6.91 / (max(decay, 1e-4) * SAMPLE_RATE))
    w = 2.0 * math.pi * freq / SAMPLE_RATE
    a = [1.0, -2.0 * r * math.cos(w), r * r]
    return gain * (1.0 - r) * signal.lfilter([1.0], a, x)


def modes(x, spec):
    """A bank of [`resonator`] modes summed - the structure's whole voice."""
    out = np.zeros(len(x))
    for freq, decay, gain in spec:
        out += resonator(x, freq, decay, gain)
    return out


def sweep(duration, f0, f1, curve=2.0):
    """A sine gliding f0 -> f1, the glide itself easing on `curve`."""
    n = n_samples(duration)
    t = np.arange(n) / SAMPLE_RATE
    shape = (t / max(duration, 1e-6)) ** curve
    freq = f0 + (f1 - f0) * shape
    return np.sin(2.0 * math.pi * np.cumsum(freq) / SAMPLE_RATE)


def partials(duration, spec):
    """A stack of steady partials, each breathing on its own slow LFO.

    Every frequency here must be a WHOLE number of cycles over `duration` or
    the loop will click; the caller keeps them integer Hz over a one-second
    bed. The per-partial LFO is what stops the stack reading as an organ chord:
    the harmonics drift against each other instead of sitting in lockstep.
    """
    t = np.arange(n_samples(duration)) / SAMPLE_RATE
    out = np.zeros(len(t))
    for freq, gain, lfo_hz, depth in spec:
        breath = 1.0 + depth * np.sin(2.0 * math.pi * lfo_hz * t)
        out += gain * breath * np.sin(2.0 * math.pi * freq * t)
    return out


def saturate(x, drive):
    """Soft clip. Weight, and the grit that stops a low thump reading as clean."""
    return np.tanh(x * drive) / math.tanh(drive)


def place(target, part, at):
    """Sum `part` into `target` starting at `at` seconds, clipped to fit."""
    start = n_samples(at)
    end = min(start + len(part), len(target))
    if start < len(target):
        target[start:end] += part[: end - start]
    return target


def pad(x, duration):
    """Fit `x` to exactly `duration`, zero-padding or truncating."""
    n = n_samples(duration)
    out = np.zeros(n)
    out[: min(n, len(x))] = x[: min(n, len(x))]
    return out


def normalize(x, dbfs=HEADROOM_DBFS):
    peak = np.max(np.abs(x))
    if peak <= 0.0:
        return x
    return x * (10.0 ** (dbfs / 20.0)) / peak


def write_wav(path, x):
    data = np.clip(x, -1.0, 1.0)
    pcm = (data * 32767.0).astype("<i2")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with wave.open(path, "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(SAMPLE_RATE)
        out.writeframes(pcm.tobytes())


# --- The cues -------------------------------------------------------------
#
# One function per cue. Each takes its own generator and returns a mono buffer;
# the recipe table below binds it to an output path and a duration.


def pdc_gatling_fire(rng):
    """One round from the rotary PDC.

    The gun authors 50 rounds a second per muzzle - 100 on a twin mount - but
    the cue throttles to twenty (`TURRET_FIRE_MIN_INTERVAL` in
    `nova_ship/src/ship_audio/mod.rs`), and twenty is where this is designed to
    sit. Rendering the true rate as a held loop was tried and rejected: at a
    10 ms period the rounds fuse into a buzz, and a burst of separable rounds
    reads better than a saw.

    So each round has to stand on its own. The front edge is hard, the low body
    decays fast enough to leave a gap before the next, and the identity lives
    in the top - the muzzle report and the rotary action, 2-9 kHz, which is
    what a PDC is recognised by.
    """
    duration = 0.06
    out = silence(duration)

    # Ignition: the primer. Very short, very bright - the click that puts a
    # hard edge on the front of the round.
    prime = white(0.008, rng) * env_exp(0.008, 0.00008, 0.0009)
    out = place(out, highpass(prime, 4200.0, order=3) * 0.34, 0.0)

    # The muzzle crack: a bright report with real energy from 1.5 kHz up,
    # saturated so it bites.
    crack = white(0.03, rng) * env_exp(0.03, 0.0002, 0.0026)
    crack = bandpass(crack, 1500.0, 8200.0, order=2)
    out = place(out, saturate(crack * 2.2, 1.8) * 0.52, 0.0004)

    # The body: the round leaving. The chest punch.
    body = white(0.045, rng) * env_exp(0.045, 0.0006, 0.0062)
    body = bandpass(body, 95.0, 450.0, order=3)
    out = place(out, saturate(body * 2.4, 2.2) * 1.05, 0.001)

    # The mechanism: the rotary action and the case clearing. Metallic, high,
    # and gone almost immediately.
    action = white(0.02, rng) * env_exp(0.02, 0.0001, 0.0018)
    zing = modes(
        action,
        [(2600.0, 0.020, 1.0), (4900.0, 0.012, 0.62), (7400.0, 0.008, 0.34)],
    )
    out = place(out, zing * 1.45, 0.0012)

    # The mount answering. Two short modes only - a third starts to sing.
    strike = white(0.03, rng) * env_exp(0.03, 0.0002, 0.003)
    ring = modes(strike, [(740.0, 0.030, 1.0), (1310.0, 0.020, 0.55)])
    out = place(out, ring * 1.5, 0.0015)

    return out


def railgun_fire(rng):
    """The spinal lance discharging.

    The loudest one-shot in the game and the only one that can afford to be:
    the lance fires once every twelve seconds, so it is punctuation, not
    texture. Three events in sequence - the capacitor dumping, the slug
    leaving, and the hull taking the recoil - which is also the order the
    gameplay does them in.
    """
    duration = 0.95
    out = silence(duration)

    # The capacitor bank dumping: an electrical snap, bright and instant.
    snap = white(0.05, rng) * env_exp(0.05, 0.0001, 0.004)
    out = place(out, highpass(snap, 2200.0) * 0.5, 0.0)
    out = place(out, resonator(snap, 3100.0, 0.05, 1.0) * 0.8, 0.0)

    # The slug leaving: a huge downward-swept low body, the actual shot.
    shot = white(0.4, rng) * env_exp(0.4, 0.001, 0.055)
    shot = bandpass(shot, 45.0, 300.0, order=3)
    shot += sweep(0.4, 190.0, 42.0, curve=1.3) * env_exp(0.4, 0.001, 0.045) * 0.7
    out = place(out, saturate(shot * 2.0, 2.6), 0.002)

    # The hull taking the recoil: long, low modes rolling away down the spine.
    groan = white(0.6, rng) * env_exp(0.6, 0.004, 0.14)
    groan = modes(
        groan,
        [(96.0, 0.55, 1.0), (173.0, 0.40, 0.7), (287.0, 0.26, 0.45), (438.0, 0.16, 0.25)],
    )
    out = place(out, groan * 2.6, 0.012)

    return out


def impact_kinetic(rng):
    """A kinetic round landing on plate.

    The most-heard event in a fight and the easiest to make tiring, so it is
    deliberately small: a strike and a short answer, no low-end weight to
    accumulate when a burst walks across a hull.
    """
    duration = 0.2
    out = silence(duration)

    # The strike, bright and instant - the spall coming off the plate.
    strike = white(0.02, rng) * env_exp(0.02, 0.0002, 0.0018)
    out = place(out, highpass(strike, 3000.0, order=3) * 0.5, 0.0)

    body = white(0.08, rng) * env_exp(0.08, 0.0005, 0.013)
    out = place(out, bandpass(body, 130.0, 620.0, order=3) * 1.5, 0.0)

    ring = modes(
        white(0.12, rng) * env_exp(0.12, 0.0002, 0.006),
        [
            (430.0, 0.11, 1.0),
            (960.0, 0.07, 0.6),
            (1580.0, 0.045, 0.35),
            (3400.0, 0.022, 0.22),
        ],
    )
    out = place(out, ring * 2.0, 0.001)

    return out


def destroy_section(rng):
    """A section failing: metal tearing, then the piece letting go.

    Not an explosion. There is no air to carry a blast wave, so what the pilot
    hears through the hull is structural - a tear, a collapse, and debris
    rattling off the plating on the way out.
    """
    duration = 0.9
    out = silence(duration)

    # The tear: ragged noise across the mids and into the top, so the failure
    # has an edge and not only weight.
    tear = white(0.16, rng) * env_exp(0.16, 0.0015, 0.030)
    out = place(out, saturate(bandpass(tear, 380.0, 4800.0, order=2) * 1.8, 1.6) * 0.7, 0.0)
    shear = white(0.09, rng) * env_exp(0.09, 0.0004, 0.011)
    out = place(out, highpass(shear, 3600.0, order=3) * 0.42, 0.0)

    # The collapse: the mass going.
    collapse = white(0.45, rng) * env_exp(0.45, 0.002, 0.075)
    collapse = bandpass(collapse, 55.0, 340.0, order=3)
    collapse += sweep(0.45, 150.0, 38.0, curve=1.6) * env_exp(0.45, 0.002, 0.06) * 0.6
    out = place(out, saturate(collapse * 1.9, 2.2), 0.010)

    # Debris off the plating: a scatter of short strikes, thinning out.
    for index in range(14):
        at = 0.05 + 0.55 * (index / 14.0) ** 1.7 + float(rng.uniform(0.0, 0.03))
        gain = float(rng.uniform(0.10, 0.34)) * (1.0 - index / 16.0)
        hit = white(0.05, rng) * env_exp(0.05, 0.0002, 0.004)
        hit = modes(
            hit,
            [
                (float(rng.uniform(600.0, 1900.0)), 0.05, 1.0),
                (float(rng.uniform(1900.0, 3600.0)), 0.03, 0.5),
            ],
        )
        out = place(out, hit * gain * 2.4, at)

    return out


def thruster_basic_loop(rng):
    """The main drive, running.

    A seamless one-second bed, and the thing it has to be is HEAVY. Two rounds
    of notes landed on the same word: the noise-only version read as floaty,
    and the old placeholder - a bare two-oscillator hum - was closer to right
    despite being cruder. So the bed is built the other way up now: a tonal
    spine carries it, low and steady like a reactor under load, and the
    turbulence is texture layered over the top rather than the whole substance.

    Every partial is an integer number of Hz, so it is a whole number of cycles
    over the loop and the seam stays silent. The noise is still synthesized in
    the frequency domain for the same reason.
    """
    duration = 1.0

    # The spine. 52 Hz reads as machinery under load; 26 underneath it is felt
    # more than heard and is where the weight comes from. Each partial breathes
    # on its own slow LFO so the stack never freezes into a chord.
    spine = partials(
        duration,
        [
            (26.0, 0.42, 2.0, 0.10),
            (52.0, 1.00, 3.0, 0.07),
            (104.0, 0.46, 5.0, 0.09),
            (156.0, 0.20, 7.0, 0.12),
            (208.0, 0.09, 4.0, 0.14),
        ],
    )
    # A little drive: the odd harmonics saturation adds are what separate a
    # reactor from an organ pipe.
    spine = saturate(spine * 0.75, 1.5)

    def spectrum(freq):
        # Turbulence, sitting UNDER the spine now. The broadband term above the
        # resonances stays small - letting it run is what made the first bed
        # hiss.
        rumble = 1.0 / (1.0 + (freq / 50.0) ** 2.8)
        throat = 0.40 / (1.0 + ((freq - 165.0) / 42.0) ** 2)
        plenum = 0.22 / (1.0 + ((freq - 322.0) / 78.0) ** 2)
        breath = 0.016 / (1.0 + (freq / 620.0) ** 2.4)
        return rumble + throat + plenum + breath

    bed = loop_noise(duration, spectrum, rng)
    bed = bed / (np.std(bed) + 1e-9)

    t = np.arange(n_samples(duration)) / SAMPLE_RATE
    return (spine + 0.34 * bed) * (1.0 + 0.05 * np.sin(2.0 * math.pi * 3.0 * t))


# name -> (renderer, output path relative to the repo root)
#
# The paths on the LEGACY names are deliberate for this first pass: those four
# files are already referenced by base content, so replacing them in place is
# audible with no code change. The authoring pass renames them to the cue names
# and retires the shared voices (see the inventory's production list).
CUES = {
    "pdc_gatling_fire": (pdc_gatling_fire, "assets/base/sounds/turret_fire.wav"),
    "railgun_fire": (railgun_fire, "assets/base/sounds/railgun_fire.wav"),
    "impact_kinetic": (impact_kinetic, "assets/base/sounds/impact.wav"),
    "destroy_section": (destroy_section, "assets/base/sounds/explosion.wav"),
    "thruster_basic_loop": (thruster_basic_loop, "assets/base/sounds/thruster_loop.wav"),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", action="append", help="render just this cue (repeatable)")
    args = parser.parse_args()

    wanted = args.only or sorted(CUES)
    for name in wanted:
        if name not in CUES:
            raise SystemExit(f"unknown cue '{name}'; known: {', '.join(sorted(CUES))}")
        render, relative = CUES[name]
        buffer = normalize(render(rng_for(name)))
        path = os.path.join(REPO_ROOT, relative)
        write_wav(path, buffer)
        print(f"{name:24s} -> {relative:44s} {len(buffer) / SAMPLE_RATE:.3f}s")


if __name__ == "__main__":
    main()
