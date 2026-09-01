#!/usr/bin/env python3
"""Shared DSP toolkit for Nova Protocol's sound renderers.

Two renderers stand on this module and they render two deliberately disjoint
voices - `gen-world-sfx.py` (machinery: guns, hulls, drives) and
`gen-ui-sfx.py` (chrome: menus, alerts, cockpit instruments). What they share
is arithmetic, not taste: filters, envelopes, resonators, oscillators and the
WAV writer. Every decision about how a cue SOUNDS lives in the renderer.

`gen-nova-os-sfx.py` does not use this. It is stdlib-only by design and it
renders the eleven `nova_*` files that are already the interface standard;
rebuilding it on numpy would churn accepted bytes for no gain.

Sits next to its callers, and Python puts a script's own directory on the
import path, so a sibling `import nova_sfx` resolves with no packaging.

DETERMINISM: callers seed PER CUE through [`rng_for`], so a rerun is
byte-identical and adding a cue rewrites no other cue's bytes.
"""

import hashlib
import math
import os
import wave

import numpy as np
from scipy import signal

SAMPLE_RATE = 44100

# Peak-normalize cues to about -3 dBFS: nothing clips, and the per-cue volume
# constants in `nova_ship/src/ship_audio/mod.rs` do the actual mixing.
HEADROOM_DBFS = -3.0


# --- Sources ---------------------------------------------------------------


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
    # Bin 0 is DC, not a frequency. Every sensible `shape` peaks at its low
    # end, so leaving it in hands the bed a large constant offset that eats
    # headroom and can step the signal when a looping voice starts.
    magnitude[0] = 0.0
    phases = rng.uniform(0.0, 2.0 * math.pi, len(freqs))
    spectrum = magnitude * np.exp(1j * phases)
    return np.fft.irfft(spectrum, n)


def osc(duration, freq, wave_shape="sine", slide_to=None, curve="exp"):
    """One oscillator, optionally gliding to `slide_to` over its whole length.

    `curve` picks how the glide is heard: "exp" is constant musical interval
    per second, which is what a pitch slide sounds like to an ear and what the
    NOVA OS recipes use; "lin" is constant Hz per second, for a mechanism
    spinning up rather than a note bending.
    """
    n = n_samples(duration)
    t = np.arange(n) / SAMPLE_RATE
    if slide_to is None or abs(slide_to - freq) < 1e-9:
        track = np.full(n, float(freq))
    elif curve == "lin":
        track = np.linspace(freq, slide_to, n)
    else:
        start, end = max(float(freq), 1e-3), max(float(slide_to), 1e-3)
        track = start * (end / start) ** (t / max(duration, 1e-9))
    phase = 2.0 * math.pi * np.cumsum(track) / SAMPLE_RATE
    if wave_shape == "square":
        return np.sign(np.sin(phase))
    if wave_shape == "triangle":
        return 2.0 / math.pi * np.arcsin(np.sin(phase))
    if wave_shape == "saw":
        return 2.0 * ((phase / (2.0 * math.pi)) % 1.0) - 1.0
    return np.sin(phase)


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


# --- Envelopes -------------------------------------------------------------


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


def env_ui(duration, attack=0.006, floor=1e-4):
    """The NOVA OS envelope: exponential up, then exponential down to nothing.

    A straight port of what a WebAudio `exponentialRampToValueAtTime` pair
    does, because that shape is a large part of why the terminal cues read as
    one family. Anything joining that family wants this and not [`env_exp`].
    """
    n = max(n_samples(duration), 2)
    rise = max(1, n_samples(attack))
    out = np.empty(n)
    up = np.arange(min(rise, n)) / rise
    out[: len(up)] = floor ** (1.0 - up)
    if n > rise:
        down = np.arange(n - rise) / max(n - rise - 1, 1)
        out[rise:] = floor**down
    return out - floor


# --- Filters ---------------------------------------------------------------


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


def peaking(x, freq, q, gain=1.0):
    """A single resonant band pulled out of `x` - one RBJ bandpass, scaled.

    This is the interface voice's colouring tool. It is not [`resonator`]: it
    shapes a sound that is already there, where a resonator MAKES one ring.
    """
    w = 2.0 * math.pi * min(freq, SAMPLE_RATE * 0.45) / SAMPLE_RATE
    alpha = math.sin(w) / (2.0 * max(q, 1e-3))
    b = [alpha, 0.0, -alpha]
    a = [1.0 + alpha, -2.0 * math.cos(w), 1.0 - alpha]
    return gain * signal.lfilter(b, a, x)


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


def saturate(x, drive):
    """Soft clip. Weight, and the grit that stops a low thump reading as clean."""
    return np.tanh(x * drive) / math.tanh(drive)


# --- Assembly --------------------------------------------------------------


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


def render_all(cues, wanted, repo_root):
    """Render `wanted` cue names from a `{name: (fn, path)}` table.

    Both renderers share a CLI shape, so they share its body: seed from the
    name, normalize, write, and report what landed where.
    """
    for name in wanted:
        if name not in cues:
            raise SystemExit(f"unknown cue '{name}'; known: {', '.join(sorted(cues))}")
        render, relative = cues[name]
        buffer = normalize(render(rng_for(name)))
        write_wav(os.path.join(repo_root, relative), buffer)
        print(f"{name:24s} -> {relative:46s} {len(buffer) / SAMPLE_RATE:.3f}s")
