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

A cue is designed for the RATE IT IS HEARD AT, not for solo listening. The PDC
runs at 100 rounds a second, so the round is shaped to stack into a buzz at a
10 ms period; auditioning it alone says almost nothing about it.

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


def place_wrapped(target, part, at):
    """Sum `part` in at `at` seconds, WRAPPING past the end of `target`.

    What makes a fire loop seamless: a round late in the loop spills its tail
    onto the front, exactly as the round before the loop point would have.
    """
    start = n_samples(at) % len(target)
    index = (np.arange(len(part)) + start) % len(target)
    np.add.at(target, index, part)
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


PDC_RATE = 100.0
"""Rounds per second for the rotary PDC - the rate the gun actually runs at."""


def _pdc_core(rng):
    """The part of a PDC round that REPEATS: the low body and the mount ring.

    Held identical from round to round on purpose. A gatling's buzz is a
    periodic waveform at the fire rate, and re-randomizing the dominant low
    layer every round replaces that periodicity with broadband noise - which
    is exactly what the first fire loop did, and why it rattled instead of
    buzzing.
    """
    duration = 0.06
    out = silence(duration)

    body = white(0.045, rng) * env_exp(0.045, 0.0006, 0.0062)
    body = bandpass(body, 95.0, 450.0, order=3)
    out = place(out, saturate(body * 2.4, 2.2) * 1.05, 0.001)

    strike = white(0.03, rng) * env_exp(0.03, 0.0002, 0.003)
    ring = modes(strike, [(740.0, 0.030, 1.0), (1310.0, 0.020, 0.55)])
    out = place(out, ring * 1.5, 0.0015)

    return out


def _pdc_edge(rng, tint=0.0):
    """The part of a PDC round that VARIES: primer, muzzle crack, mechanism.

    THIS is the high end the first pass was missing, and it is what a PDC is
    recognised by - the report and the rotary action, 2-9 kHz. It carries
    little energy, so varying it per round textures the buzz without breaking
    the periodicity the body establishes. `tint` shifts it a few percent and is
    driven from the round index, so the loop stays deterministic.
    """
    duration = 0.06
    out = silence(duration)
    shift = 1.0 + 0.06 * tint

    # Ignition: the primer. Very short, very bright - the click that lets the
    # ear place each round inside the buzz.
    prime = white(0.008, rng) * env_exp(0.008, 0.00008, 0.0009)
    out = place(out, highpass(prime, 4200.0 * shift, order=3) * 0.34, 0.0)

    # The muzzle crack: a bright report with real energy from 1.5 kHz up,
    # saturated so it bites.
    crack = white(0.03, rng) * env_exp(0.03, 0.0002, 0.0026)
    crack = bandpass(crack, 1500.0 * shift, 8200.0 * shift, order=2)
    out = place(out, saturate(crack * 2.2, 1.8) * 0.52, 0.0004)

    # The mechanism: the rotary action and the case clearing. Metallic, high,
    # and gone almost immediately.
    action = white(0.02, rng) * env_exp(0.02, 0.0001, 0.0018)
    zing = modes(
        action,
        [
            (2600.0 * shift, 0.020, 1.0),
            (4900.0 * shift, 0.012, 0.62),
            (7400.0 * shift, 0.008, 0.34),
        ],
    )
    out = place(out, zing * 1.45, 0.0012)

    return out


def pdc_gatling_fire(rng):
    """One round, for a single shot and for the tail of a burst.

    Auditioning it alone says little: the gun runs at [`PDC_RATE`], so what a
    player hears is [`pdc_gatling_loop`]. This exists for the ragged edges of a
    burst, where individual rounds ARE separable.
    """
    return _pdc_core(rng) + _pdc_edge(rng)


def pdc_gatling_loop(rng):
    """The PDC firing, as a seamless loop at [`PDC_RATE`].

    A gun that runs at 100 rounds a second cannot be a hundred one-shots a
    second - that is a hundred audio entities a second, and the game's cue
    throttles to twenty, which is why the gun currently rattles rather than
    buzzes. A loop held while the trigger is down is the standard answer and
    the only one that reaches the real rate.

    The period is an EXACT whole number of rounds (44100 / 100 = 441 samples,
    no remainder) and every round's tail wraps onto the front, so there is no
    seam to find.
    """
    rounds = 20
    duration = rounds / PDC_RATE
    out = silence(duration)
    core = _pdc_core(rng)
    for index in range(rounds):
        # A slow waver over the loop rather than a per-round lottery: the gun
        # is a machine, and it should sound like one running, not like twenty
        # different guns.
        tint = math.sin(2.0 * math.pi * index / rounds)
        gain = 0.94 + 0.06 * math.cos(2.0 * math.pi * 3.0 * index / rounds)
        round_ = (core + _pdc_edge(rng, tint)) * gain
        out = place_wrapped(out, round_, index / PDC_RATE)
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

    A seamless one-second bed. Spectral synthesis, not a two-oscillator tone:
    a drive is broadband turbulence with structure resonating in it, and a tone
    reads as a synthesizer the moment it is held for more than a second. The
    slow wobble is an INTEGER number of cycles over the loop so it survives
    repeating.
    """
    duration = 1.0

    def spectrum(freq):
        # A drive is a low mass of turbulence with the throat and plenum
        # resonating in it. The broadband term above them is kept DELIBERATELY
        # small - the first pass let it run and the bed read as hiss rather
        # than as an engine.
        rumble = 1.0 / (1.0 + (freq / 58.0) ** 2.6)
        throat = 0.30 / (1.0 + ((freq - 165.0) / 42.0) ** 2)
        plenum = 0.16 / (1.0 + ((freq - 322.0) / 78.0) ** 2)
        breath = 0.013 / (1.0 + (freq / 620.0) ** 2.4)
        return rumble + throat + plenum + breath

    bed = loop_noise(duration, spectrum, rng)
    bed = saturate(bed / (np.std(bed) + 1e-9) * 0.24, 1.4)

    # One slow wobble, an integer number of cycles over the loop, so the bed
    # breathes instead of sitting perfectly still. Two of them beat against
    # each other into something the ear tracks; one does not.
    t = np.arange(n_samples(duration)) / SAMPLE_RATE
    return bed * (1.0 + 0.06 * np.sin(2.0 * math.pi * 3.0 * t))


# name -> (renderer, output path relative to the repo root)
#
# The paths on the LEGACY names are deliberate for this first pass: those four
# files are already referenced by base content, so replacing them in place is
# audible with no code change. The authoring pass renames them to the cue names
# and retires the shared voices (see the inventory's production list).
CUES = {
    "pdc_gatling_fire": (pdc_gatling_fire, "assets/base/sounds/turret_fire.wav"),
    "pdc_gatling_loop": (pdc_gatling_loop, "assets/base/sounds/pdc_gatling_loop.wav"),
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
