#!/usr/bin/env python3
"""Offline renderer for the NOVA OS terminal sound cues.

This mirrors the WebAudio synth recipes in the `Sound` IIFE of
web/design/nova_os_terminal_poc.html and bakes each cue into a
`assets/sounds/nova_*.wav` file so the game can play the same chrome
SFX without a live WebAudio graph.

Two primitives are reimplemented by hand:
  - tone(freq, dur, gain, type, slideTo, delay): one oscillator with an
    optional exponential pitch slide and an exponential attack/decay gain
    envelope (matching the PoC's setValueAtTime/exponentialRampToValueAtTime).
  - noise(dur, freq, q, gain, type): seeded white noise through an RBJ
    biquad (bandpass / lowpass / highpass) with the same envelope shape.

DETERMINISM: the noise RNG is seeded with a FIXED seed (SEED below) so
running the script twice produces byte-identical WAVs. CI regenerates and
diffs the outputs, so the render must be reproducible.

Dependencies: Python STDLIB only (wave, struct, math, array, random). The
DSP (oscillators, biquad, envelopes, normalization) is implemented by hand
so there is no numpy dependency -> zero-dep, fully deterministic.

Run:  python3 scripts/gen-nova-os-sfx.py
  (or: nix develop --command python3 scripts/gen-nova-os-sfx.py)
"""

import array
import math
import os
import random
import struct
import wave

# --- Global format / tuning constants -------------------------------------

SAMPLE_RATE = 44100
SEED = 0x4E4F5641  # "NOVA" - fixed noise seed for deterministic renders.

# The PoC ran everything through a master gain of 0.3. We keep that so the
# baked cues sit at the same relative loudness the recipe gains imply, then
# peak-normalize each cue (except the bed) to the headroom below.
MASTER_GAIN = 0.3

# Peak-normalize cues to about -3 dBFS so nothing clips and cues aren't harsh.
HEADROOM_DBFS = -3.0

# Envelope timings from the PoC (seconds).
TONE_ATTACK = 0.006
NOISE_ATTACK = 0.004
ENV_FLOOR = 0.0001  # exponential ramps start/end near-zero, never true zero.

# Ambient bed tuning (see recipe below). NOT peak-normalized - stays quiet.
BED_DURATION = 2.0            # exactly 2.000 s -> integer cycles, click-free loop
BED_WHINE_FREQ = 7860.0       # 15720 whole cycles in 2 s
BED_WHINE_AMP = 0.014
BED_MAINS_FREQ = 50.0         # 100 whole cycles in 2 s
BED_MAINS_AMP = 0.05
BED_MAINS_LOWPASS = 140.0
BED_GAIN = 1.0                # gentle constant gain, no fade (a fade breaks the loop)


# --- DSP helpers ----------------------------------------------------------

def _exp_env(n, sr, dur, gain, attack):
    """Exponential attack->decay gain envelope, sample by sample.

    Mirrors WebAudio: ramp ENV_FLOOR->gain over `attack`, then gain->ENV_FLOOR
    over the remainder up to `dur`. Exponential ramps are linear in log space.
    """
    env = [0.0] * n
    a_samples = max(1, int(round(attack * sr)))
    d_samples = max(1, int(round(dur * sr)) - a_samples)
    log_floor = math.log(ENV_FLOOR)
    log_gain = math.log(gain)
    for i in range(n):
        if i < a_samples:
            frac = i / a_samples
            env[i] = math.exp(log_floor + (log_gain - log_floor) * frac)
        else:
            j = i - a_samples
            if j >= d_samples:
                env[i] = ENV_FLOOR
            else:
                frac = j / d_samples
                env[i] = math.exp(log_gain + (log_floor - log_gain) * frac)
    return env


def _osc_sample(kind, phase):
    """One oscillator sample for a phase in [0, 1)."""
    if kind == "sine":
        return math.sin(2.0 * math.pi * phase)
    if kind == "square":
        return 1.0 if phase < 0.5 else -1.0
    if kind == "triangle":
        # 0->1->-1->0 triangle, peak +-1.
        return 4.0 * abs(phase - 0.5) - 1.0
    if kind == "sawtooth":
        return 2.0 * phase - 1.0
    raise ValueError("unknown osc type: %s" % kind)


def tone(freq, dur, gain, kind="square", slide_to=0.0, delay=0.0,
         sr=SAMPLE_RATE):
    """Render a single oscillator with optional exponential pitch slide.

    Returns a (start_sample, samples[]) pair; the caller mixes it in at the
    given delay offset.
    """
    n = max(1, int(round(dur * sr)))
    env = _exp_env(n, sr, dur, gain, TONE_ATTACK)
    out = [0.0] * n
    phase = 0.0
    log_f0 = math.log(freq)
    log_f1 = math.log(slide_to) if slide_to else log_f0
    for i in range(n):
        if slide_to:
            frac = i / n
            f = math.exp(log_f0 + (log_f1 - log_f0) * frac)
        else:
            f = freq
        out[i] = _osc_sample(kind, phase) * env[i]
        phase += f / sr
        phase -= math.floor(phase)
    start = int(round(delay * sr))
    return start, out


def _biquad_coeffs(kind, freq, q, sr):
    """RBJ audio-EQ cookbook coefficients for bandpass/lowpass/highpass."""
    w0 = 2.0 * math.pi * freq / sr
    cos_w0 = math.cos(w0)
    sin_w0 = math.sin(w0)
    alpha = sin_w0 / (2.0 * q)

    if kind == "bandpass":
        # Constant 0 dB peak gain (BPF, "csg" form).
        b0 = alpha
        b1 = 0.0
        b2 = -alpha
    elif kind == "lowpass":
        b0 = (1.0 - cos_w0) / 2.0
        b1 = 1.0 - cos_w0
        b2 = (1.0 - cos_w0) / 2.0
    elif kind == "highpass":
        b0 = (1.0 + cos_w0) / 2.0
        b1 = -(1.0 + cos_w0)
        b2 = (1.0 + cos_w0) / 2.0
    else:
        raise ValueError("unknown filter type: %s" % kind)

    a0 = 1.0 + alpha
    a1 = -2.0 * cos_w0
    a2 = 1.0 - alpha
    return (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)


def noise(dur, freq, q, gain, kind="bandpass", rng=None, sr=SAMPLE_RATE):
    """Render a seeded white-noise burst through an RBJ biquad."""
    n = max(1, int(round(dur * sr)))
    env = _exp_env(n, sr, dur, gain, NOISE_ATTACK)
    b0, b1, b2, a1, a2 = _biquad_coeffs(kind, freq, q, sr)
    out = [0.0] * n
    x1 = x2 = y1 = y2 = 0.0
    for i in range(n):
        x0 = rng.random() * 2.0 - 1.0  # white noise in [-1, 1)
        y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2
        x2, x1 = x1, x0
        y2, y1 = y1, y0
        out[i] = y0 * env[i]
    return 0, out


# --- Mixing / output ------------------------------------------------------

def mix(parts):
    """Sum a list of (start_sample, samples[]) into one buffer, * MASTER_GAIN."""
    total = 0
    for start, samples in parts:
        total = max(total, start + len(samples))
    buf = [0.0] * total
    for start, samples in parts:
        for i, s in enumerate(samples):
            buf[start + i] += s
    return [s * MASTER_GAIN for s in buf]


def peak_normalize(buf, dbfs=HEADROOM_DBFS):
    peak = max((abs(s) for s in buf), default=0.0)
    if peak <= 0.0:
        return buf
    target = 10.0 ** (dbfs / 20.0)
    scale = target / peak
    return [s * scale for s in buf]


def write_wav(path, buf, sr=SAMPLE_RATE):
    """Write a mono 16-bit PCM WAV, clamping to [-1, 1]."""
    pcm = array.array("h")
    for s in buf:
        v = s
        if v > 1.0:
            v = 1.0
        elif v < -1.0:
            v = -1.0
        pcm.append(int(round(v * 32767.0)))
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())


def render_bed(sr=SAMPLE_RATE):
    """Quiet, seamlessly loopable tube hum. NOT peak-normalized.

    Pure phase functions of the sample index so sample 0 and sample N wrap
    seamlessly; a 140 Hz one-pole lowpass shapes the 50 Hz sawtooth. The
    lowpass is warmed up over one full loop so its state matches at the wrap.
    """
    n = int(round(BED_DURATION * sr))

    # 140 Hz one-pole lowpass coefficient.
    rc = 1.0 / (2.0 * math.pi * BED_MAINS_LOWPASS)
    dt = 1.0 / sr
    alpha = dt / (rc + dt)

    def saw(i):
        phase = (BED_MAINS_FREQ * i / sr) % 1.0
        return 2.0 * phase - 1.0

    # Prime the lowpass by running a full loop so its warm state wraps cleanly.
    lp = 0.0
    for i in range(n):
        lp += alpha * (saw(i) - lp)

    buf = [0.0] * n
    for i in range(n):
        lp += alpha * (saw(i) - lp)
        whine = math.sin(2.0 * math.pi * BED_WHINE_FREQ * i / sr)
        buf[i] = (whine * BED_WHINE_AMP + lp * BED_MAINS_AMP) * BED_GAIN * MASTER_GAIN
    return buf


# --- The cue recipes (mirrors the PoC) ------------------------------------

def build_cues(rng):
    """Return {name: buffer} for every peak-normalized cue.

    Each cue is the sum of its primitives; noise() draws from the shared
    seeded rng in the fixed order below so the render is deterministic.
    """
    cues = {}

    cues["nova_key"] = peak_normalize(mix([
        noise(0.028, 2100, 1.4, 0.16, "bandpass", rng),
        tone(160, 0.03, 0.05, "triangle"),
    ]))

    cues["nova_back"] = peak_normalize(mix([
        noise(0.03, 1500, 1.2, 0.15, "bandpass", rng),
    ]))

    cues["nova_enter"] = peak_normalize(mix([
        noise(0.05, 900, 0.9, 0.24, "bandpass", rng),
        tone(120, 0.06, 0.08, "triangle"),
    ]))

    cues["nova_ok"] = peak_normalize(mix([
        tone(1180, 0.05, 0.055, "square"),
    ]))

    cues["nova_error"] = peak_normalize(mix([
        tone(220, 0.09, 0.09, "square"),
        tone(165, 0.13, 0.09, "square", slide_to=0, delay=0.10),
    ]))

    cues["nova_tick"] = peak_normalize(mix([
        noise(0.02, 3200, 3, 0.2, "bandpass", rng),
    ]))

    cues["nova_coil"] = peak_normalize(mix([
        tone(118, 0.55, 0.1, "sine", slide_to=42),
        tone(59, 0.5, 0.07, "triangle", slide_to=30),
        noise(0.25, 260, 0.7, 0.06, "lowpass", rng),
    ]))

    cues["nova_powerup"] = peak_normalize(mix([
        noise(0.06, 700, 0.8, 0.3, "lowpass", rng),
        tone(90, 0.12, 0.1, "triangle", delay=0.02),
        tone(2400, 0.6, 0.05, "sine", slide_to=7860, delay=0.12),
        noise(0.4, 3000, 0.6, 0.05, "highpass", rng),
    ]))

    cues["nova_powerdown"] = peak_normalize(mix([
        tone(7860, 0.45, 0.06, "sine", slide_to=240),
        noise(0.09, 520, 0.8, 0.28, "lowpass", rng),
        tone(70, 0.16, 0.1, "triangle", delay=0.05),
    ]))

    return cues


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    out_dir = os.path.join(root, "assets", "sounds")
    os.makedirs(out_dir, exist_ok=True)

    # Single shared, fixed-seed RNG -> deterministic across runs.
    rng = random.Random(SEED)

    cues = build_cues(rng)
    cues["nova_bed"] = render_bed()  # quiet, not normalized

    for name in sorted(cues):
        path = os.path.join(out_dir, name + ".wav")
        write_wav(path, cues[name])
        print("wrote %s (%d frames)" % (path, len(cues[name])))


if __name__ == "__main__":
    main()
