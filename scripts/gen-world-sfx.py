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

A cue is designed for the RATE IT IS HEARD AT. The gatling PDC authors 100
rounds a second out of its one muzzle (the twin authors 50 per muzzle, for the
same total), but the cue throttles to twenty and stays there: a held loop at
the true rate fuses into a buzz, and a burst of separable rounds reads better
than a saw. Twenty a second is the number its round is shaped against.

Cues that answer the same event on different HARDWARE are separated by pitch,
not by decoration, because pitch is what survives a firefight: the drives run
34 / 52 / 78 Hz from capital to basic to vector, and the guns run heavy to
light the same way. That ladder is the reason a pilot can tell what is shooting
without looking.

The INTERFACE and AVIONICS voices are a different renderer with a different
brief - `scripts/gen-ui-sfx.py`, joining the NOVA OS family baked by
`scripts/gen-nova-os-sfx.py`. Keeping the voices disjoint is what makes all of
them legible, so do not add a terminal blip here.

DETERMINISM: every cue seeds its own generator from a hash of its NAME, so a
rerun is byte-identical AND adding a cue rewrites no other cue's bytes. That
is the one thing `gen-nova-os-sfx.py` got wrong (it draws from a single shared
stream in list order, so an insertion churns every later file).

Run:  nix develop --command python3 scripts/gen-world-sfx.py
      nix develop --command python3 scripts/gen-world-sfx.py --only pdc_gatling_fire
"""

import argparse
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from nova_sfx import (  # noqa: E402  (path set up just above)
    SAMPLE_RATE,
    bandpass,
    env_ad,
    env_exp,
    highpass,
    loop_noise,
    lowpass,
    modes,
    n_samples,
    osc,
    partials,
    place,
    render_all,
    resonator,
    saturate,
    silence,
    sweep,
    white,
)

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


# --- Guns -----------------------------------------------------------------
#
# One function per cue. Each takes its own generator and returns a mono buffer;
# the recipe table at the bottom binds it to an output path.


def pdc_gatling_fire(rng):
    """One round from the rotary PDC.

    The gatling authors 100 rounds a second out of its one muzzle, and the
    twin 50 per muzzle for the same total - but the cue throttles to twenty (`TURRET_FIRE_MIN_INTERVAL` in
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


def pdc_twin_fire(rng):
    """One round from the twin mount - the heavier of the two PDCs.

    Same family, one size up, and the size is carried by PITCH: the body sits
    at 70-380 Hz where the gatling's sits at 95-450, and the mount rings a
    third lower. Nothing about the recipe is decorated to make it "big"; it is
    the same round, slower and lower, which is what a bigger gun is.
    """
    duration = 0.085
    out = silence(duration)

    prime = white(0.009, rng) * env_exp(0.009, 0.0001, 0.0011)
    out = place(out, highpass(prime, 3600.0, order=3) * 0.38, 0.0)

    crack = white(0.04, rng) * env_exp(0.04, 0.00025, 0.0034)
    crack = bandpass(crack, 1100.0, 7000.0, order=2)
    out = place(out, saturate(crack * 2.0, 1.8) * 0.66, 0.0005)

    # The heavier charge: lower, and it takes longer to get out of the way.
    body = white(0.07, rng) * env_exp(0.07, 0.0008, 0.0105)
    body = bandpass(body, 70.0, 380.0, order=3)
    out = place(out, saturate(body * 2.4, 2.2) * 1.25, 0.0012)

    action = white(0.028, rng) * env_exp(0.028, 0.00012, 0.0026)
    zing = modes(
        action,
        [(2100.0, 0.026, 1.0), (3900.0, 0.016, 0.60), (6200.0, 0.010, 0.30)],
    )
    out = place(out, zing * 1.75, 0.0016)

    strike = white(0.05, rng) * env_exp(0.05, 0.00025, 0.0045)
    ring = modes(strike, [(560.0, 0.042, 1.0), (980.0, 0.027, 0.55)])
    out = place(out, ring * 1.6, 0.002)

    return out


def pdc_dry_fire(rng):
    """The trigger pulled on an empty gun.

    Everything that makes a shot is missing on purpose: no primer, no report,
    no body. What is left is the mechanism cycling into nothing, which is the
    whole message - the gun WORKED and there was no round in it.
    """
    duration = 0.05
    out = silence(duration)

    sear = white(0.005, rng) * env_exp(0.005, 0.0001, 0.0006)
    out = place(out, highpass(sear, 2500.0, order=3) * 0.6, 0.0)

    clack = white(0.03, rng) * env_exp(0.03, 0.00012, 0.0022)
    out = place(
        out,
        modes(clack, [(1850.0, 0.013, 1.0), (3200.0, 0.008, 0.55), (5400.0, 0.005, 0.30)]) * 2.0,
        0.0006,
    )

    # A little mass under it. The mechanism is heavy even when the gun is not
    # firing, and without this the cue reads as a UI click rather than a gun.
    mass = white(0.025, rng) * env_exp(0.025, 0.0004, 0.0032)
    out = place(out, bandpass(mass, 200.0, 520.0, order=3) * 0.55, 0.001)

    return out


def _stow_servo(rng, duration, freq_from, freq_to):
    """The housing's motor running - shared by the two stow cues.

    Open and close are the SAME machine going two ways, so they share the
    servo and differ only in which direction its whine travels and how hard
    the panel lands. Rendering them independently would have made two
    mechanisms out of one.
    """
    motor = white(duration, rng) * env_ad(duration, 0.03, duration - 0.03, curve=0.6)
    motor = bandpass(motor, 260.0, 1500.0, order=2)
    whine = osc(duration, freq_from, "saw", slide_to=freq_to, curve="lin")
    whine = lowpass(whine, 2200.0) * env_ad(duration, 0.04, duration - 0.04, curve=0.5)
    return motor * 0.55 + whine * 0.30


def pdc_stow_open(rng):
    """The PDC's housing opening: latch, servo, and the lid hitting its stop."""
    duration = 0.55
    out = silence(duration)

    latch = white(0.03, rng) * env_exp(0.03, 0.0003, 0.004)
    out = place(out, modes(latch, [(1700.0, 0.030, 1.0), (2900.0, 0.020, 0.5)]) * 1.6, 0.0)

    out = place(out, _stow_servo(rng, 0.34, 172.0, 214.0), 0.03)

    # Arriving at the stop, not seating: an open lid is caught, so this is
    # softer and rings longer than the close.
    stop = white(0.14, rng) * env_exp(0.14, 0.0006, 0.016)
    out = place(out, bandpass(stop, 90.0, 420.0, order=3) * 1.5, 0.40)
    out = place(
        out,
        modes(stop, [(330.0, 0.10, 1.0), (680.0, 0.06, 0.5), (1420.0, 0.035, 0.28)]) * 1.7,
        0.401,
    )

    return out


def pdc_stow_close(rng):
    """The housing closing: the servo runs down and the panel SEATS."""
    duration = 0.5
    out = silence(duration)

    out = place(out, _stow_servo(rng, 0.32, 208.0, 164.0), 0.0)

    # Seating is a harder event than the open's stop - metal on metal with the
    # motor still pushing - so it is shorter, lower and more damped.
    seat = white(0.16, rng) * env_exp(0.16, 0.0004, 0.014)
    out = place(out, saturate(bandpass(seat, 70.0, 360.0, order=3) * 1.8, 2.0) * 1.6, 0.34)
    out = place(out, modes(seat, [(295.0, 0.07, 1.0), (610.0, 0.04, 0.45)]) * 1.5, 0.341)

    lock = white(0.04, rng) * env_exp(0.04, 0.0002, 0.003)
    out = place(out, modes(lock, [(2050.0, 0.018, 1.0), (3400.0, 0.011, 0.5)]) * 1.3, 0.40)

    return out


def bay_door(rng):
    """The muzzle iris: a small servo and six petals seating not quite together.

    The scatter on the petals is the whole point. Six ticks on a grid reads as
    a synthesizer; six ticks that arrive within a few milliseconds of each
    other reads as six pieces of hardware.
    """
    duration = 0.4
    out = silence(duration)

    servo = white(0.22, rng) * env_ad(0.22, 0.02, 0.20, curve=0.8)
    out = place(out, bandpass(servo, 420.0, 2600.0, order=2) * 0.45, 0.0)
    whine = osc(0.22, 640.0, "triangle", slide_to=780.0, curve="lin")
    out = place(out, whine * env_ad(0.22, 0.03, 0.19, curve=0.7) * 0.22, 0.0)

    for index in range(6):
        at = 0.17 + index * 0.016 + float(rng.uniform(0.0, 0.010))
        tick = white(0.04, rng) * env_exp(0.04, 0.0002, 0.0035)
        tick = modes(
            tick,
            [
                (float(rng.uniform(1400.0, 2600.0)), 0.020, 1.0),
                (float(rng.uniform(3000.0, 4600.0)), 0.012, 0.5),
            ],
        )
        out = place(out, tick * float(rng.uniform(0.6, 1.0)) * 1.4, at)

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


def railgun_charge(rng):
    """The capacitor bank filling - a LOOP, held for the whole charge.

    A half-second bed rather than the drives' full second, because this one is
    meant to be played at a RISING RATE: the charge is three seconds long and
    the gun has to sound like it is approaching something, which is playback
    speed tracking `progress`, not a longer file. Every partial is a multiple
    of 2 Hz and every LFO is even, so a whole number of cycles fits the loop at
    any speed and the seam stays silent.

    It is a coil tone with a mains buzz under it, deliberately NOT a rising
    whine on its own: a bare sweep reads as a synthesizer warming up, and the
    harmonics are what make it read as a bank of hardware taking a load.
    """
    duration = 0.5

    core = partials(
        duration,
        [
            (146.0, 1.00, 4.0, 0.06),
            (292.0, 0.52, 6.0, 0.08),
            (438.0, 0.26, 10.0, 0.10),
            (584.0, 0.13, 14.0, 0.12),
            (876.0, 0.07, 8.0, 0.16),
        ],
    )
    core = saturate(core * 0.8, 1.7)

    def spectrum(freq):
        hum = 0.50 / (1.0 + (freq / 120.0) ** 2.2)
        coil = 0.30 / (1.0 + ((freq - 1460.0) / 420.0) ** 2)
        arc = 0.04 / (1.0 + (freq / 3200.0) ** 2.0)
        return hum + coil + arc

    bed = loop_noise(duration, spectrum, rng)
    bed = bed / (np.std(bed) + 1e-9)

    t = np.arange(n_samples(duration)) / SAMPLE_RATE
    return (core + 0.30 * bed) * (1.0 + 0.06 * np.sin(2.0 * math.pi * 8.0 * t))


def railgun_reload(rng):
    """A shell going into the lance: breech, rail, seat, lock.

    Twelve seconds of reload is a long silence to leave a pilot in, and this
    is the cue that tells them the gun is coming back. It is written as four
    separable events for exactly that reason - it should be possible to hear
    how far through it is.
    """
    duration = 1.0
    out = silence(duration)

    hiss = white(0.30, rng) * env_ad(0.30, 0.006, 0.29, curve=1.6)
    out = place(out, bandpass(hiss, 700.0, 4200.0, order=2) * 0.40, 0.0)

    slide = white(0.34, rng) * env_ad(0.34, 0.05, 0.29, curve=1.0)
    out = place(out, bandpass(slide, 150.0, 900.0, order=2) * 0.55, 0.16)

    # The shell seating: the biggest event here, and the one that means loaded.
    seat = white(0.30, rng) * env_exp(0.30, 0.0008, 0.030)
    out = place(out, bandpass(seat, 70.0, 380.0, order=3) * 1.7, 0.50)
    out = place(
        out,
        modes(seat, [(210.0, 0.16, 1.0), (470.0, 0.10, 0.55), (880.0, 0.06, 0.30)]) * 1.9,
        0.501,
    )

    for index, at in enumerate((0.70, 0.78)):
        clack = white(0.06, rng) * env_exp(0.06, 0.0002, 0.005)
        ring = modes(clack, [(1250.0, 0.035, 1.0), (2400.0, 0.020, 0.5)])
        out = place(out, ring * (1.5 - 0.4 * index), at)

    return out


# --- Ordnance -------------------------------------------------------------


def torpedo_launch(rng):
    """A torpedo leaving the tube: gas shoves it out, then the motor catches.

    Two events, and the gap between them is the cue. A launch that fires its
    motor instantly reads as a gun; the shove-then-catch is what says the
    thing is now flying on its own.
    """
    duration = 0.7
    out = silence(duration)

    push = white(0.18, rng) * env_exp(0.18, 0.0012, 0.028)
    push = bandpass(push, 60.0, 520.0, order=3)
    out = place(out, saturate(push * 2.0, 2.2), 0.0)
    gas = white(0.16, rng) * env_exp(0.16, 0.0008, 0.024)
    out = place(out, highpass(gas, 2400.0, order=2) * 0.42, 0.0)

    # The motor lighting and running out. It SWELLS - the attack is slow on
    # purpose, because a motor catching is the one thing here that is not an
    # impact.
    span = 0.55
    roar = white(span, rng) * env_ad(span, 0.05, span - 0.05, curve=1.8)
    roar = bandpass(roar, 90.0, 2000.0, order=2)
    out = place(out, saturate(roar * 1.6, 1.6) * 0.75, 0.09)
    core = sweep(span, 78.0, 150.0, curve=0.7) * env_ad(span, 0.06, span - 0.06, curve=1.6)
    out = place(out, core * 0.55, 0.09)

    return out


def torpedo_detonate(rng):
    """The warhead going off.

    Deliberately NOT the section-failure voice it has been borrowing. A
    section failing is structural - things tear and collapse. A warhead is a
    hard front and a spray of fragments, and the difference has to be audible
    or a torpedo hit reads as just another piece of the ship breaking.
    """
    duration = 1.1
    out = silence(duration)

    front = white(0.05, rng) * env_exp(0.05, 0.00012, 0.0045)
    out = place(out, highpass(front, 1800.0, order=2) * 1.5, 0.0)

    blast = white(0.5, rng) * env_exp(0.5, 0.0008, 0.085)
    blast = bandpass(blast, 40.0, 420.0, order=3)
    blast += sweep(0.5, 240.0, 34.0, curve=1.2) * env_exp(0.5, 0.001, 0.07) * 0.75
    out = place(out, saturate(blast * 2.4, 2.8) * 1.15, 0.001)

    # Fragments: dense, bright and over inside a third of a second. This is
    # what separates it from the debris field of a hull coming apart, which is
    # sparse and goes on for two seconds.
    for index in range(26):
        at = 0.004 + 0.30 * (index / 26.0) ** 1.4 + float(rng.uniform(0.0, 0.012))
        frag = white(0.03, rng) * env_exp(0.03, 0.0001, 0.0022)
        frag = bandpass(frag, float(rng.uniform(2200.0, 5200.0)), 9000.0, order=2)
        out = place(out, frag * float(rng.uniform(0.25, 0.7)) * 3.2, at)

    tail = white(0.7, rng) * env_exp(0.7, 0.02, 0.16)
    out = place(out, lowpass(tail, 300.0, order=2) * 0.5, 0.03)

    return out


# --- Impacts --------------------------------------------------------------


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


def impact_pierce(rng):
    """A pierce round raking through the plating.

    The one damage type whose sound has to say something the kinetic hit does
    not: it went THROUGH. So it is not one strike but seven, tightening as the
    slug crosses the stack, over a long metallic shear. The closing gaps are
    the whole read - a constant spacing would sound like a burst of fire.
    """
    duration = 0.35
    out = silence(duration)

    entry = white(0.03, rng) * env_exp(0.03, 0.00012, 0.0022)
    out = place(out, highpass(entry, 3600.0, order=3) * 0.7, 0.0)

    for index in range(7):
        at = 0.006 * index**1.25
        hit = white(0.05, rng) * env_exp(0.05, 0.0002, 0.004)
        hit = bandpass(hit, 300.0, 4200.0, order=2)
        out = place(out, hit * (0.9 - 0.09 * index) * 1.1, at)

    shear = white(0.22, rng) * env_exp(0.22, 0.0006, 0.030)
    shear = modes(
        shear,
        [(1180.0, 0.10, 1.0), (2360.0, 0.06, 0.55), (4100.0, 0.035, 0.30), (6800.0, 0.02, 0.18)],
    )
    out = place(out, shear * 1.3, 0.002)

    # It still has to LAND. The shear is the identity, but a hit made only of
    # shear has no weight at all and reads as a scrape rather than a round.
    body = white(0.10, rng) * env_exp(0.10, 0.0006, 0.014)
    out = place(out, bandpass(body, 110.0, 520.0, order=3) * 3.2, 0.001)

    return out


def impact_explosive(rng):
    """A shaped charge landing.

    Softer at the front than the kinetic round and much heavier underneath -
    it does not punch a hole, it pushes. Shorter than the torpedo's warhead
    because this is a hit, not a kill.
    """
    duration = 0.45
    out = silence(duration)

    front = white(0.03, rng) * env_exp(0.03, 0.0004, 0.003)
    out = place(out, bandpass(front, 900.0, 6000.0, order=2) * 1.45, 0.0)
    # Spall coming off the plate. The charge is a pressure event, but it still
    # lands on metal, and without this the hit reads as muffled next to the
    # kinetic round it shares a hull with.
    spall = white(0.06, rng) * env_exp(0.06, 0.0002, 0.006)
    out = place(out, highpass(spall, 2600.0, order=2) * 1.10, 0.0)

    blast = white(0.26, rng) * env_exp(0.26, 0.0012, 0.040)
    blast = bandpass(blast, 50.0, 380.0, order=3)
    blast += sweep(0.26, 180.0, 44.0, curve=1.3) * env_exp(0.26, 0.0015, 0.033) * 0.7
    out = place(out, saturate(blast * 2.2, 2.5) * 0.80, 0.0015)

    ring = modes(
        white(0.2, rng) * env_exp(0.2, 0.0004, 0.010),
        [(280.0, 0.13, 1.0), (640.0, 0.08, 0.5), (1350.0, 0.05, 0.28)],
    )
    out = place(out, ring * 1.7, 0.003)

    tail = white(0.3, rng) * env_exp(0.3, 0.012, 0.070)
    out = place(out, lowpass(tail, 420.0, order=2) * 0.45, 0.02)

    return out


def impact_rock(rng):
    """A round landing on an asteroid.

    Rock does not ring, so this is the one impact with no resonator bank at
    all - a broad dull body and a scatter of grit. That absence is the cue:
    hitting stone should tell a pilot instantly that they are not hitting a
    ship.
    """
    duration = 0.25
    out = silence(duration)

    strike = white(0.02, rng) * env_exp(0.02, 0.0003, 0.0016)
    out = place(out, bandpass(strike, 800.0, 5000.0, order=2) * 0.45, 0.0)

    # Order 2, not 3: a broad skirt with no corner, because a sharp band edge
    # is itself a resonance and rock has none.
    body = white(0.12, rng) * env_exp(0.12, 0.0008, 0.016)
    body = bandpass(body, 90.0, 700.0, order=2)
    out = place(out, saturate(body * 1.8, 1.8) * 1.3, 0.0)

    for index in range(18):
        at = 0.003 + 0.13 * (index / 18.0) ** 1.5 + float(rng.uniform(0.0, 0.008))
        grain = white(0.012, rng) * env_exp(0.012, 0.0001, 0.0010)
        grain = bandpass(grain, float(rng.uniform(900.0, 2600.0)), 7000.0, order=1)
        out = place(out, grain * float(rng.uniform(0.10, 0.30)), at)

    return out


# --- Destruction ----------------------------------------------------------


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


def destroy_rock(rng):
    """An asteroid breaking up.

    The rock counterpart to a section failing, and built from the same two
    parts - a crack and a long spill - with every metallic mode removed. What
    is left is a deep split and rubble, which is a slower and much duller
    event than a hull coming apart.
    """
    duration = 1.3
    out = silence(duration)

    crack = white(0.06, rng) * env_exp(0.06, 0.0006, 0.006)
    out = place(out, bandpass(crack, 400.0, 5200.0, order=2) * 2.4, 0.0)

    split = white(0.55, rng) * env_exp(0.55, 0.0015, 0.085)
    split = bandpass(split, 40.0, 400.0, order=3)
    split += sweep(0.55, 130.0, 30.0, curve=1.5) * env_exp(0.55, 0.002, 0.07) * 0.65
    out = place(out, saturate(split * 2.0, 2.4) * 1.1, 0.001)

    for index in range(44):
        at = 0.03 + 0.95 * (index / 44.0) ** 1.35 + float(rng.uniform(0.0, 0.025))
        gain = float(rng.uniform(0.08, 0.30)) * (1.0 - index / 50.0)
        lump = white(0.05, rng) * env_exp(0.05, 0.0004, 0.0045)
        lump = bandpass(
            lump, float(rng.uniform(150.0, 600.0)), float(rng.uniform(1200.0, 3000.0)), order=2
        )
        out = place(out, lump * gain * 6.0, at)

    return out


def destroy_ship(rng):
    """A whole hull going.

    The longest cue in the game, and it earns the length: it has to be
    obviously bigger than a section failing, and the only honest way to do
    that is to let it go on. The spine fails, the mass falls, the bays cook
    off one at a time, and the debris keeps arriving for two seconds.
    """
    duration = 2.4
    out = silence(duration)

    snap = white(0.10, rng) * env_exp(0.10, 0.0008, 0.012)
    out = place(out, saturate(bandpass(snap, 300.0, 5000.0, order=2) * 1.8, 1.6) * 1.90, 0.0)

    fall = white(1.0, rng) * env_exp(1.0, 0.004, 0.170)
    fall = bandpass(fall, 32.0, 300.0, order=3)
    fall += sweep(1.0, 130.0, 24.0, curve=1.8) * env_exp(1.0, 0.004, 0.150) * 0.7
    out = place(out, saturate(fall * 2.2, 2.8) * 1.25, 0.008)

    # Secondaries. Uneven spacing, because five evenly spaced bursts is a
    # rhythm and a ship coming apart does not have one.
    for at in (0.18, 0.36, 0.52, 0.79, 1.12):
        burst = white(0.30, rng) * env_exp(0.30, 0.0009, 0.038)
        burst = bandpass(burst, 55.0, 900.0, order=2)
        gain = float(rng.uniform(0.45, 0.85))
        out = place(out, saturate(burst * 1.8, 2.0) * gain, at + float(rng.uniform(0.0, 0.03)))

    for index in range(38):
        at = 0.10 + 2.05 * (index / 38.0) ** 1.5 + float(rng.uniform(0.0, 0.04))
        gain = float(rng.uniform(0.08, 0.30)) * (1.0 - index / 44.0)
        hit = white(0.06, rng) * env_exp(0.06, 0.0002, 0.005)
        hit = modes(
            hit,
            [
                (float(rng.uniform(400.0, 1800.0)), 0.06, 1.0),
                (float(rng.uniform(1800.0, 4200.0)), 0.035, 0.5),
            ],
        )
        out = place(out, hit * gain * 7.5, at)

    out = place(out, lowpass(white(1.6, rng) * env_exp(1.6, 0.03, 0.34), 220.0, order=2) * 0.5, 0.02)

    return out


# --- Drives ---------------------------------------------------------------
#
# Three drives, one recipe, three pitches: capital 34 Hz, basic 52, vector 78.
# The ladder is the identity - a pilot should hear the SIZE of what just lit
# its engines - so nothing else about them is decorated to tell them apart.


def _drive_loop(rng, spine_spec, spectrum, drive, bed_mix, breath_hz, breath_depth):
    """A seamless one-second drive bed: tonal spine, turbulence over it.

    Every partial is an integer number of Hz, so it is a whole number of
    cycles over the loop and the seam stays silent. The noise is synthesized
    in the frequency domain for the same reason.
    """
    duration = 1.0
    spine = saturate(partials(duration, spine_spec) * drive, 1.5)
    bed = loop_noise(duration, spectrum, rng)
    bed = bed / (np.std(bed) + 1e-9)
    t = np.arange(n_samples(duration)) / SAMPLE_RATE
    return (spine + bed_mix * bed) * (1.0 + breath_depth * np.sin(2.0 * math.pi * breath_hz * t))


def thruster_basic_loop(rng):
    """The main drive, running.

    The thing it has to be is HEAVY. Two rounds of notes landed on the same
    word: the noise-only version read as floaty, and the old placeholder - a
    bare two-oscillator hum - was closer to right despite being cruder. So it
    is built the other way up: a tonal spine carries it, low and steady like a
    reactor under load, and the turbulence is texture layered over the top
    rather than the whole substance.
    """

    def spectrum(freq):
        # Turbulence, sitting UNDER the spine. The broadband term above the
        # resonances stays small - letting it run is what made the first bed
        # hiss.
        rumble = 1.0 / (1.0 + (freq / 50.0) ** 2.8)
        throat = 0.40 / (1.0 + ((freq - 165.0) / 42.0) ** 2)
        plenum = 0.22 / (1.0 + ((freq - 322.0) / 78.0) ** 2)
        breath = 0.016 / (1.0 + (freq / 620.0) ** 2.4)
        return rumble + throat + plenum + breath

    # 52 Hz reads as machinery under load; 26 underneath it is felt more than
    # heard and is where the weight comes from. Each partial breathes on its
    # own slow LFO so the stack never freezes into a chord.
    return _drive_loop(
        rng,
        [
            (26.0, 0.42, 2.0, 0.10),
            (52.0, 1.00, 3.0, 0.07),
            (104.0, 0.46, 5.0, 0.09),
            (156.0, 0.20, 7.0, 0.12),
            (208.0, 0.09, 4.0, 0.14),
        ],
        spectrum,
        drive=0.75,
        bed_mix=0.34,
        breath_hz=3.0,
        breath_depth=0.05,
    )


def thruster_vector_loop(rng):
    """The 3x3x2 vectoring drive: the same machine, smaller and busier.

    Up a fifth from the basic drive at 78 Hz, with more of its energy in the
    throat resonance - a smaller chamber is brighter, and it has to cut
    through when a ship is running both.
    """

    def spectrum(freq):
        rumble = 0.70 / (1.0 + (freq / 80.0) ** 2.6)
        throat = 0.55 / (1.0 + ((freq - 320.0) / 90.0) ** 2)
        plenum = 0.28 / (1.0 + ((freq - 680.0) / 160.0) ** 2)
        breath = 0.030 / (1.0 + (freq / 1100.0) ** 2.2)
        return rumble + throat + plenum + breath

    return _drive_loop(
        rng,
        [
            (39.0, 0.28, 3.0, 0.12),
            (78.0, 1.00, 4.0, 0.09),
            (156.0, 0.52, 6.0, 0.11),
            (234.0, 0.26, 9.0, 0.14),
            (312.0, 0.12, 5.0, 0.16),
        ],
        spectrum,
        drive=0.70,
        bed_mix=0.46,
        breath_hz=5.0,
        breath_depth=0.06,
    )


def thruster_capital_loop(rng):
    """The 5x5x3 capital drive: 34 Hz, and most of it below hearing.

    The fundamental is deliberately near the bottom of what a speaker will
    reproduce, so on small speakers this reads mostly as its harmonics and on
    anything with a woofer it reads as pressure. Its LFOs are the slowest of
    the three: a big machine breathes slowly.
    """

    def spectrum(freq):
        rumble = 1.40 / (1.0 + (freq / 34.0) ** 3.0)
        throat = 0.30 / (1.0 + ((freq - 110.0) / 30.0) ** 2)
        plenum = 0.16 / (1.0 + ((freq - 230.0) / 60.0) ** 2)
        breath = 0.012 / (1.0 + (freq / 460.0) ** 2.6)
        return rumble + throat + plenum + breath

    return _drive_loop(
        rng,
        [
            (17.0, 0.55, 1.0, 0.10),
            (34.0, 1.00, 2.0, 0.07),
            (68.0, 0.50, 3.0, 0.08),
            (102.0, 0.24, 5.0, 0.10),
            (136.0, 0.11, 2.0, 0.13),
        ],
        spectrum,
        drive=0.85,
        bed_mix=0.28,
        breath_hz=2.0,
        breath_depth=0.04,
    )


def rcs_loop(rng):
    """Cold gas out of an attitude nozzle.

    The only loop with NO tonal spine, because there is no reactor behind it -
    a thruster bottle is pressure through a hole. The one tone in it is the
    nozzle's own whistle, kept faint; pushing it up turns the cue into a kettle
    immediately.
    """
    duration = 1.0

    def spectrum(freq):
        # A jet: rolling off above the throat and cut off hard below it, so
        # this never competes with the drives for the low end.
        jet = 1.0 / (1.0 + (freq / 900.0) ** 1.4) / (1.0 + (240.0 / np.maximum(freq, 1.0)) ** 3.0)
        throat = 0.50 / (1.0 + ((freq - 1750.0) / 700.0) ** 2)
        return jet + throat

    bed = loop_noise(duration, spectrum, rng)
    bed = bed / (np.std(bed) + 1e-9)
    whistle = partials(duration, [(1290.0, 1.0, 6.0, 0.30)])
    t = np.arange(n_samples(duration)) / SAMPLE_RATE
    return (bed + 0.05 * whistle) * (1.0 + 0.07 * np.sin(2.0 * math.pi * 7.0 * t))


# --- Handling -------------------------------------------------------------


def salvage_pickup(rng):
    """A crate coming aboard: the grapple takes it, the latch closes, the bay
    answers.

    The one world cue whose job is to feel GOOD, and the temptation is to
    write it as interface chrome. It stays machinery instead - the reward is
    in the bay's low ring, and its two modes are deliberately not a musical
    interval, which is what would tip it into the other voice.
    """
    duration = 0.30
    out = silence(duration)

    grab = white(0.05, rng) * env_exp(0.05, 0.0006, 0.005)
    out = place(out, bandpass(grab, 250.0, 2400.0, order=2) * 0.8, 0.0)

    latch = white(0.04, rng) * env_exp(0.04, 0.0002, 0.0032)
    out = place(out, modes(latch, [(1450.0, 0.030, 1.0), (2600.0, 0.018, 0.5)]) * 1.5, 0.045)

    ring = modes(
        white(0.22, rng) * env_exp(0.22, 0.0004, 0.012),
        [(196.0, 0.14, 1.0), (505.0, 0.08, 0.45)],
    )
    out = place(out, ring * 1.6, 0.050)

    return out


# name -> (renderer, output path relative to the repo root)
#
# Nine cues render onto LEGACY filenames, and they keep them. Those paths are
# public modding surface - `dep://base/sounds/impact.wav` is documented in
# `web/src/create/sections.md` and `objects.md` and referenced by mods we do
# not own - so renaming them to match the cue names would break other people's
# content to make our filenames prettier. The cue name is the design's name;
# the path is the content's name; they are allowed to differ.
CUES = {
    "pdc_gatling_fire": (pdc_gatling_fire, "assets/base/sounds/turret_fire.wav"),
    "pdc_twin_fire": (pdc_twin_fire, "assets/base/sounds/pdc_twin_fire.wav"),
    "pdc_dry_fire": (pdc_dry_fire, "assets/base/sounds/dry_fire.wav"),
    "pdc_stow_open": (pdc_stow_open, "assets/base/sounds/pdc_stow_open.wav"),
    "pdc_stow_close": (pdc_stow_close, "assets/base/sounds/pdc_stow_close.wav"),
    "bay_door": (bay_door, "assets/base/sounds/bay_door.wav"),
    "railgun_charge": (railgun_charge, "assets/base/sounds/railgun_charge.wav"),
    "railgun_fire": (railgun_fire, "assets/base/sounds/railgun_fire.wav"),
    "railgun_reload": (railgun_reload, "assets/base/sounds/railgun_reload.wav"),
    "torpedo_launch": (torpedo_launch, "assets/base/sounds/torpedo_launch.wav"),
    "torpedo_detonate": (torpedo_detonate, "assets/base/sounds/torpedo_detonate.wav"),
    "impact_kinetic": (impact_kinetic, "assets/base/sounds/impact.wav"),
    "impact_pierce": (impact_pierce, "assets/base/sounds/impact_pierce.wav"),
    "impact_explosive": (impact_explosive, "assets/base/sounds/impact_explosive.wav"),
    "impact_rock": (impact_rock, "assets/base/sounds/impact_rock.wav"),
    "destroy_section": (destroy_section, "assets/base/sounds/explosion.wav"),
    "destroy_rock": (destroy_rock, "assets/base/sounds/destroy_rock.wav"),
    "destroy_ship": (destroy_ship, "assets/base/sounds/destroy_ship.wav"),
    "thruster_basic_loop": (thruster_basic_loop, "assets/base/sounds/thruster_loop.wav"),
    "thruster_vector_loop": (thruster_vector_loop, "assets/base/sounds/thruster_vector_loop.wav"),
    "thruster_capital_loop": (
        thruster_capital_loop,
        "assets/base/sounds/thruster_capital_loop.wav",
    ),
    "rcs_loop": (rcs_loop, "assets/base/sounds/rcs_loop.wav"),
    "salvage_pickup": (salvage_pickup, "assets/base/sounds/salvage_pickup.wav"),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", action="append", help="render just this cue (repeatable)")
    args = parser.parse_args()
    render_all(CUES, args.only or sorted(CUES), REPO_ROOT)


if __name__ == "__main__":
    main()
