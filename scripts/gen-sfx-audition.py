#!/usr/bin/env python3
"""Render the sound-audition bench for the audio direction pass.

Reads every rendered WAV, measures it, and writes a self-contained page with
the clips embedded so the owner can listen, look at the anatomy, and check the
numbers against the style spec in one place. Committed with the task because
the page IS the reasoning - `TASK.md` only summarises the verdict.

Cues are grouped by FAMILY, and a family is auditioned together on purpose:
what matters about `pdc_twin_fire` is not how it sounds but how it sounds NEXT
TO `pdc_gatling_fire`. Each cue is auditioned at the rate it is HEARD at, so
the guns and the kinetic round lead with a burst rather than one round.

Run:  nix develop --command python3 scripts/gen-sfx-audition.py
"""

import base64
import io
import json
import math
import os
import wave

import numpy as np

SAMPLE_RATE = 44100
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO_ROOT, "tasks/20260824-125955/audition.html")

WORLD = "assets/base/sounds/"
CHROME = "assets/sounds/"

# ACCEPTED is the language the owner signed off in round three; everything else
# on this page is written to match it and is being auditioned for the first
# time. WIRED says whether the game can play the file today - "authoring" is
# this lane's remaining work, "hook" is the engine lane's.
FAMILIES = [
    {
        "id": "guns",
        "title": "Guns",
        "blurb": (
            "Two PDCs and the lance. The mounts are separated by PITCH and "
            "nothing else - the twin is the same round, lower and slower, "
            "which is what a bigger gun is."
        ),
        "strips": [
            {
                "name": "PDC gatling, one round",
                "cue": "turret fire_sound",
                "status": "accepted",
                "wired": "plays now",
                "note": (
                    "The gun authors 50 rounds a second per muzzle and the cue "
                    "throttles to twenty, so the round is shaped to stand alone "
                    "at 50 ms spacing. Identity lives in the top: the muzzle "
                    "report and the rotary action, 2-9 kHz."
                ),
                "takes": [
                    {"label": "burst, 20/s as shipped", "file": WORLD + "turret_fire.wav", "rate": 0.05},
                    {"label": "one round", "file": WORLD + "turret_fire.wav"},
                ],
            },
            {
                "name": "PDC twin, one round",
                "cue": "turret fire_sound",
                "status": "new",
                "wired": "needs authoring",
                "note": (
                    "The heavier mount, and the first cue to retire a shared "
                    "voice: both turrets fire the gatling's file today. Body at "
                    "70-380 Hz where the gatling sits at 95-450, mount ringing a "
                    "third lower, and deliberately darker - 19% character against "
                    "the gatling's 30%."
                ),
                "takes": [
                    {"label": "burst, 20/s", "file": WORLD + "pdc_twin_fire.wav", "rate": 0.05},
                    {"label": "one round", "file": WORLD + "pdc_twin_fire.wav"},
                    {"label": "A/B: gatling", "file": WORLD + "turret_fire.wav", "rate": 0.05},
                ],
            },
            {
                "name": "PDC dry fire",
                "cue": "turret dry_fire_sound",
                "status": "new",
                "wired": "plays now",
                "note": (
                    "Everything that makes a shot is missing on purpose - no "
                    "primer, no report, no body. What is left is the mechanism "
                    "cycling into nothing, which is the whole message: the gun "
                    "worked and there was no round in it."
                ),
                "takes": [{"label": "dry click", "file": WORLD + "dry_fire.wav"}],
            },
            {
                "name": "Railgun lance, discharge",
                "cue": "railgun fire_sound",
                "status": "accepted",
                "wired": "plays now",
                "note": (
                    "Accepted in round one and untouched since. The capacitor "
                    "bank dumping, the slug leaving on a downward-swept low body, "
                    "and the hull taking the recoil, in the order the shot does "
                    "them."
                ),
                "takes": [{"label": "one shot", "file": WORLD + "railgun_fire.wav"}],
            },
            {
                "name": "Railgun charge",
                "cue": "the three-second charge",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "The biggest silence in the game today: the lance charges for "
                    "three seconds with no sound at all. A half-second LOOP rather "
                    "than a long file, because it is meant to be played at a rising "
                    "rate as the charge fills. Every partial is a multiple of 2 Hz "
                    "so the seam stays silent at any speed."
                ),
                "takes": [
                    {"label": "held loop", "file": WORLD + "railgun_charge.wav", "loop": True},
                    {"label": "one cycle", "file": WORLD + "railgun_charge.wav"},
                ],
            },
            {
                "name": "Railgun reload",
                "cue": "the twelve-second reload",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "Four separable events - breech, rail, seat, lock - so a pilot "
                    "can hear how far through it is. Twelve seconds is a long "
                    "silence to leave someone in."
                ),
                "takes": [{"label": "full cycle", "file": WORLD + "railgun_reload.wav"}],
            },
            {
                "name": "PDC housing, opening",
                "cue": "StowLift / StowDoors animation",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "Latch, servo, and the lid arriving at its stop. Open and close "
                    "share one servo and differ only in which way its whine travels "
                    "- rendering them apart would have made two mechanisms out of one."
                ),
                "takes": [{"label": "open", "file": WORLD + "pdc_stow_open.wav"}],
            },
            {
                "name": "PDC housing, closing",
                "cue": "StowLift / StowDoors animation",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "The same machine going the other way, and the panel SEATS "
                    "rather than being caught - shorter, lower, more damped than "
                    "the open."
                ),
                "takes": [
                    {"label": "close", "file": WORLD + "pdc_stow_close.wav"},
                    {"label": "A/B: open", "file": WORLD + "pdc_stow_open.wav"},
                ],
            },
            {
                "name": "Muzzle iris",
                "cue": "MuzzleDoor animation",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "Six petals seating not quite together. The scatter is the "
                    "point: six ticks on a grid reads as a synthesizer, six that "
                    "arrive within a few milliseconds of each other reads as "
                    "hardware."
                ),
                "takes": [{"label": "iris", "file": WORLD + "bay_door.wav"}],
            },
        ],
    },
    {
        "id": "ordnance",
        "title": "Ordnance",
        "blurb": "The torpedo, which until now borrowed the hull-failure voice for its warhead.",
        "strips": [
            {
                "name": "Torpedo launch",
                "cue": "bay launch_sound",
                "status": "new",
                "wired": "plays now",
                "note": (
                    "Gas shoves it out, then the motor catches. The GAP between "
                    "those two is the cue - a launch that lights its motor "
                    "instantly reads as a gun."
                ),
                "takes": [{"label": "launch", "file": WORLD + "torpedo_launch.wav"}],
            },
            {
                "name": "Torpedo detonation",
                "cue": "bay detonation_sound",
                "status": "new",
                "wired": "needs authoring",
                "note": (
                    "Retires the worst of the shared voices: every warhead "
                    "currently plays the section-failure file. A section failing "
                    "is structural - things tear and collapse. A warhead is a hard "
                    "front and a spray of fragments, dense and over inside a third "
                    "of a second."
                ),
                "takes": [
                    {"label": "detonation", "file": WORLD + "torpedo_detonate.wav"},
                    {"label": "A/B: section failing", "file": WORLD + "explosion.wav"},
                ],
            },
        ],
    },
    {
        "id": "impacts",
        "title": "Impacts",
        "blurb": (
            "One per damage type, plus rock. These are the cues the material "
            "table would key on - the damage type is the 'what hit it' half and "
            "already exists."
        ),
        "strips": [
            {
                "name": "Kinetic round, on plate",
                "cue": "section impact_sound",
                "status": "accepted",
                "wired": "plays now",
                "note": (
                    "The most-heard event in a fight and the easiest to make "
                    "tiring, so it is deliberately small: a strike and a short "
                    "answer, with no low-end weight to accumulate when a burst "
                    "walks across a hull."
                ),
                "takes": [
                    {"label": "burst on target", "file": WORLD + "impact.wav", "rate": 0.05},
                    {"label": "one hit", "file": WORLD + "impact.wav"},
                ],
            },
            {
                "name": "Pierce round, raking",
                "cue": "DamageType::Pierce",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "The one damage type whose sound has to say something the "
                    "kinetic hit does not: it went THROUGH. Seven strikes "
                    "tightening as the slug crosses the stack, over a long "
                    "metallic shear. The closing gaps are the read - constant "
                    "spacing would just sound like gunfire."
                ),
                "takes": [
                    {"label": "one rake", "file": WORLD + "impact_pierce.wav"},
                    {"label": "A/B: kinetic", "file": WORLD + "impact.wav"},
                ],
            },
            {
                "name": "Explosive round, landing",
                "cue": "DamageType::Explosive",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "Softer at the front than the kinetic round and much heavier "
                    "underneath - it does not punch a hole, it pushes. Shorter "
                    "than the torpedo's warhead because this is a hit, not a kill."
                ),
                "takes": [{"label": "one hit", "file": WORLD + "impact_explosive.wav"}],
            },
            {
                "name": "Round on rock",
                "cue": "asteroid impact",
                "status": "new",
                "wired": "needs authoring",
                "note": (
                    "Rock does not ring, so this is the only impact with no "
                    "resonator bank at all - a broad dull body and a scatter of "
                    "grit. That absence IS the cue. Asteroids are silent today "
                    "and this is the file the material table would reach for."
                ),
                "takes": [
                    {"label": "one hit", "file": WORLD + "impact_rock.wav"},
                    {"label": "A/B: on plate", "file": WORLD + "impact.wav"},
                ],
            },
        ],
    },
    {
        "id": "destruction",
        "title": "Destruction",
        "blurb": "Three scales of thing coming apart, separated by how long they go on.",
        "strips": [
            {
                "name": "Section failing",
                "cue": "section destroy_sound",
                "status": "accepted",
                "wired": "plays now",
                "note": (
                    "A tear, a collapse, and debris rattling off the plating on "
                    "the way out. 85% punch and 9% character is the profile the "
                    "three new destruction cues were retuned onto."
                ),
                "takes": [{"label": "section", "file": WORLD + "explosion.wav"}],
            },
            {
                "name": "Asteroid breaking",
                "cue": "asteroid destroy_sound",
                "status": "new",
                "wired": "needs authoring",
                "note": (
                    "The rock counterpart, built from the same crack-and-spill "
                    "with every metallic mode removed. Slower and much duller "
                    "than a hull coming apart."
                ),
                "takes": [
                    {"label": "asteroid", "file": WORLD + "destroy_rock.wav"},
                    {"label": "A/B: section", "file": WORLD + "explosion.wav"},
                ],
            },
            {
                "name": "Ship destroyed",
                "cue": "ship death",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "The longest cue in the game at 2.4 seconds, and it earns the "
                    "length: it has to be obviously bigger than a section failing "
                    "and the only honest way to do that is to let it go on. The "
                    "spine fails, the mass falls, the bays cook off one at a time, "
                    "and debris keeps arriving. A ship dying is silent today."
                ),
                "takes": [
                    {"label": "ship", "file": WORLD + "destroy_ship.wav"},
                    {"label": "A/B: section", "file": WORLD + "explosion.wav"},
                ],
            },
        ],
    },
    {
        "id": "drives",
        "title": "Drives",
        "blurb": (
            "34 / 52 / 78 Hz, capital to basic to vector. The ladder is the "
            "identity - a pilot should hear the SIZE of what just lit its "
            "engines - so nothing else about them is decorated to tell them apart. "
            "All three currently play the same file."
        ),
        "strips": [
            {
                "name": "Main drive",
                "cue": "thruster loop_sound",
                "status": "accepted",
                "wired": "plays now",
                "note": (
                    "A tonal spine carries it, low and steady like a reactor under "
                    "load, with turbulence as texture over the top. 52 Hz reads as "
                    "machinery under load; 26 underneath is felt rather than heard. "
                    "Bytes changed since round three by DC removal only - the "
                    "spectrum is otherwise identical."
                ),
                "takes": [{"label": "held loop", "file": WORLD + "thruster_loop.wav", "loop": True}],
            },
            {
                "name": "Vector drive (3x3x2)",
                "cue": "thruster loop_sound",
                "status": "new",
                "wired": "needs authoring",
                "note": (
                    "Up a fifth at 78 Hz with more of its energy in the throat "
                    "resonance - a smaller chamber is brighter, and it has to cut "
                    "through when a ship is running both."
                ),
                "takes": [
                    {"label": "held loop", "file": WORLD + "thruster_vector_loop.wav", "loop": True},
                    {"label": "A/B: main drive", "file": WORLD + "thruster_loop.wav", "loop": True},
                ],
            },
            {
                "name": "Capital drive (5x5x3)",
                "cue": "thruster loop_sound",
                "status": "new",
                "wired": "needs authoring",
                "note": (
                    "34 Hz, deliberately near the bottom of what a speaker will "
                    "reproduce: on small speakers this reads as its harmonics, on "
                    "anything with a woofer it reads as pressure. Its LFOs are the "
                    "slowest of the three - a big machine breathes slowly."
                ),
                "takes": [
                    {"label": "held loop", "file": WORLD + "thruster_capital_loop.wav", "loop": True},
                    {"label": "A/B: main drive", "file": WORLD + "thruster_loop.wav", "loop": True},
                ],
            },
            {
                "name": "RCS",
                "cue": "controller rcs_loop_sound",
                "status": "new",
                "wired": "plays now",
                "note": (
                    "The only loop with no tonal spine, because there is no reactor "
                    "behind it - a thruster bottle is pressure through a hole. The "
                    "one tone is the nozzle's own whistle, kept faint; pushing it up "
                    "turns the cue into a kettle immediately."
                ),
                "takes": [{"label": "held loop", "file": WORLD + "rcs_loop.wav", "loop": True}],
            },
        ],
    },
    {
        "id": "avionics",
        "title": "Avionics",
        "blurb": (
            "The cockpit. World CONTENT, not chrome, because locking is a "
            "CAPABILITY of a controller section - so a cheap civilian controller "
            "and a military one are allowed to sound different. The interface "
            "recipe darkened, with a little of the world voice's metal underneath."
        ),
        "strips": [
            {
                "name": "Lock on",
                "cue": "controller_lock_on_sound",
                "status": "new",
                "wired": "plays now",
                "note": "Two steps up, and the panel rings - the instrument acted.",
                "takes": [{"label": "lock on", "file": WORLD + "lock_on.wav"}],
            },
            {
                "name": "Lock off",
                "cue": "controller_lock_off_sound",
                "status": "new",
                "wired": "plays now",
                "note": "The mirror, and no ring: the instrument stopped, it did not act.",
                "takes": [
                    {"label": "lock off", "file": WORLD + "lock_off.wav"},
                    {"label": "A/B: lock on", "file": WORLD + "lock_on.wav"},
                ],
            },
            {
                "name": "Radar deny",
                "cue": "controller_radar_deny_sound",
                "status": "new",
                "wired": "plays now",
                "note": (
                    "Nothing in the cone, or nothing it will take. Deliberately "
                    "rhymes with the editor's deny - a beating pair of low squares "
                    "- because 'no' should be recognisable as 'no' wherever a "
                    "player meets it. Moved up into the cockpit's register and "
                    "given the panel ring so it belongs to the instruments."
                ),
                "takes": [
                    {"label": "deny", "file": WORLD + "radar_deny.wav"},
                    {"label": "A/B: editor deny", "file": CHROME + "editor_deny.wav"},
                ],
            },
            {
                "name": "Radar retarget",
                "cue": "controller_radar_retarget_sound",
                "status": "new",
                "wired": "plays now",
                "note": (
                    "Fires on every tap of the target key, so it is the avionics "
                    "answer to the menu cursor: almost nothing."
                ),
                "takes": [{"label": "retarget", "file": WORLD + "radar_retarget.wav"}],
            },
            {
                "name": "Safety on",
                "cue": "controller_safety_on_sound",
                "status": "new",
                "wired": "plays now",
                "note": "The ship going cold. A descent that settles, with the panel answering once as it seats.",
                "takes": [{"label": "safety", "file": WORLD + "safety_on.wav"}],
            },
            {
                "name": "Magazine dry",
                "cue": "the ammo readout",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "The ring here is the MECHANISM rather than the panel - the "
                    "same 1.8-3 kHz metal the gun's dry-fire uses. The two fire at "
                    "nearly the same moment and are meant to be heard as one event "
                    "from two places: the gun outside, the gauge inside."
                ),
                "takes": [
                    {"label": "dry", "file": WORLD + "ammo_dry.wav"},
                    {"label": "A/B: gun dry-fire", "file": WORLD + "dry_fire.wav"},
                ],
            },
            {
                "name": "Threat: locked",
                "cue": "ThreatContacts",
                "status": "moved",
                "wired": "needs a hook",
                "note": (
                    "Someone has locked YOU. Three fast pips on a pair 24 Hz apart "
                    "- the beat is what makes it read as an alarm rather than a "
                    "notification. The HUD already tracks the data "
                    "(edge_indicators.rs); nothing plays a sound for it."
                ),
                "takes": [{"label": "threat lock", "file": WORLD + "warn_lock.wav"}],
            },
            {
                "name": "Threat: hull critical",
                "cue": "player hull fraction",
                "status": "moved",
                "wired": "needs a hook",
                "note": (
                    "Everything the lock alarm is, an octave down and half the "
                    "speed. Slower is more serious, which is the opposite of how "
                    "alarms usually escalate and is why this one lands. No hull "
                    "threshold alert exists today."
                ),
                "takes": [
                    {"label": "hull critical", "file": WORLD + "warn_hull.wav"},
                    {"label": "A/B: threat lock", "file": WORLD + "warn_lock.wav"},
                ],
            },
        ],
    },
    {
        "id": "handling",
        "title": "Handling",
        "blurb": "The one world cue whose job is to feel good.",
        "strips": [
            {
                "name": "Salvage aboard",
                "cue": "crate pickup_sound",
                "status": "new",
                "wired": "plays now",
                "note": (
                    "The grapple takes it, the latch closes, the bay answers. The "
                    "temptation is to write this as interface chrome; it stays "
                    "machinery instead, and its two ring modes are deliberately "
                    "not a musical interval - that is what would tip it into the "
                    "other voice."
                ),
                "takes": [{"label": "pickup", "file": WORLD + "salvage_pickup.wav"}],
            },
        ],
    },
    {
        "id": "chrome",
        "title": "Interface",
        "blurb": (
            "Engine chrome: what the game plays uniformly and what still has an "
            "event to fire on with zero mods loaded. Built on the NOVA OS "
            "primitives - a square or triangle with an exponential slide, a noise "
            "blip through one resonant band, and that family's envelope - so these "
            "sit beside the eleven existing terminal cues rather than beside the guns."
        ),
        "strips": [
            {
                "name": "Menu select",
                "cue": "UiSfx::MenuSelect",
                "status": "revoiced",
                "wired": "plays now",
                "note": "Commit. The one cue in the set that rises without arriving anywhere.",
                "takes": [
                    {"label": "select", "file": CHROME + "menu_select.wav"},
                    {"label": "A/B: nova_enter", "file": CHROME + "nova_enter.wav"},
                ],
            },
            {
                "name": "Menu back",
                "cue": "menu dismiss",
                "status": "new",
                "wired": "needs a hook",
                "note": "Select played backwards in every way that matters.",
                "takes": [
                    {"label": "back", "file": CHROME + "menu_back.wav"},
                    {"label": "A/B: select", "file": CHROME + "menu_select.wav"},
                ],
            },
            {
                "name": "Menu focus",
                "cue": "cursor movement",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "Fires more than anything else in the game, so it is the "
                    "shortest cue with the least in it - one blip and a hint of pitch."
                ),
                "takes": [{"label": "focus", "file": CHROME + "menu_focus.wav"}],
            },
            {
                "name": "Toggle",
                "cue": "UiSfx::UiToggle",
                "status": "revoiced",
                "wired": "plays now",
                "note": (
                    "Two discrete pips rather than select's slide, because a "
                    "toggle went from one place to another and a slide is a journey."
                ),
                "takes": [{"label": "toggle", "file": CHROME + "ui_toggle.wav"}],
            },
            {
                "name": "Slider detent",
                "cue": "slider / stepper",
                "status": "new",
                "wired": "needs a hook",
                "note": "The smallest sound the game makes, at 18 ms.",
                "takes": [
                    {"label": "tick", "file": CHROME + "ui_tick.wav"},
                    {"label": "train, dragging a slider", "file": CHROME + "ui_tick.wav", "rate": 0.09},
                ],
            },
            {
                "name": "Objective, new",
                "cue": "UiSfx::ObjectiveNew",
                "status": "revoiced",
                "wired": "plays now",
                "note": "Attention, not alarm: it rises and stops, withholding the arrival.",
                "takes": [{"label": "new", "file": CHROME + "objective_new.wav"}],
            },
            {
                "name": "Objective, complete",
                "cue": "UiSfx::ObjectiveComplete",
                "status": "revoiced",
                "wired": "plays now",
                "note": "Three steps up and the last one held - the arrival the new-objective cue deliberately withholds.",
                "takes": [
                    {"label": "complete", "file": CHROME + "objective_complete.wav"},
                    {"label": "A/B: new", "file": CHROME + "objective_new.wav"},
                ],
            },
            {
                "name": "Objective, failed",
                "cue": "objective failure",
                "status": "new",
                "wired": "needs a hook",
                "note": "The mirror: three steps down, and the last one keeps falling instead of landing.",
                "takes": [
                    {"label": "failed", "file": CHROME + "objective_fail.wav"},
                    {"label": "A/B: complete", "file": CHROME + "objective_complete.wav"},
                ],
            },
            {
                "name": "Comms line",
                "cue": "dialogue line opening",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "Squelch open, carrier, squelch closed - then the whole thing "
                    "band-limited to 320-3200 Hz, which is the entire identity. It "
                    "is not a menu sound that happens to precede dialogue, it is a "
                    "radio."
                ),
                "takes": [{"label": "squelch", "file": CHROME + "comms_line.wav"}],
            },
            {
                "name": "Editor: place",
                "cue": "part placed",
                "status": "new",
                "wired": "needs a hook",
                "note": (
                    "The heaviest cue in the interface voice - the one moment the "
                    "editor has real mass. All of nova_editor is silent today."
                ),
                "takes": [{"label": "place", "file": CHROME + "editor_place.wav"}],
            },
            {
                "name": "Editor: remove",
                "cue": "part removed",
                "status": "new",
                "wired": "needs a hook",
                "note": "Lighter than placing and falling instead of rising, so the pair reads as a direction.",
                "takes": [
                    {"label": "remove", "file": CHROME + "editor_remove.wav"},
                    {"label": "A/B: place", "file": CHROME + "editor_place.wav"},
                ],
            },
            {
                "name": "Editor: rotate",
                "cue": "rotation detent",
                "status": "new",
                "wired": "needs a hook",
                "note": "The slider tick with mass - the same gesture on a part rather than on a slider.",
                "takes": [{"label": "rotate", "file": CHROME + "editor_rotate.wav"}],
            },
            {
                "name": "Editor: deny",
                "cue": "illegal placement",
                "status": "new",
                "wired": "needs a hook",
                "note": "A beating pair of low squares, 8 Hz apart - a rasp rather than a chord.",
                "takes": [
                    {"label": "deny", "file": CHROME + "editor_deny.wav"},
                    {"label": "A/B: radar deny", "file": WORLD + "radar_deny.wav"},
                ],
            },
        ],
    },
]


def load(relative):
    with wave.open(os.path.join(REPO_ROOT, relative), "rb") as w:
        raw = w.readframes(w.getnframes())
    return np.frombuffer(raw, dtype="<i2").astype(float) / 32767.0


def encode(samples):
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(SAMPLE_RATE)
        out.writeframes((np.clip(samples, -1.0, 1.0) * 32767.0).astype("<i2").tobytes())
    return "data:audio/wav;base64," + base64.b64encode(buffer.getvalue()).decode("ascii")


def envelope(samples, columns=360):
    edges = np.linspace(0, len(samples), columns + 1).astype(int)
    return [
        [
            round(float(np.min(samples[a:b])), 4) if b > a else 0.0,
            round(float(np.max(samples[a:b])), 4) if b > a else 0.0,
        ]
        for a, b in zip(edges[:-1], edges[1:])
    ]


def measure(samples):
    # No window: these are impulsive clips whose energy sits in the first few
    # milliseconds, and a Hanning window would attenuate exactly the attack the
    # numbers are meant to describe.
    spectrum = np.abs(np.fft.rfft(samples))
    freqs = np.fft.rfftfreq(len(samples), 1.0 / SAMPLE_RATE)
    power = spectrum**2

    def share(low, high):
        return float(power[(freqs >= low) & (freqs < high)].sum() / max(power.sum(), 1e-9)) * 100.0

    return {
        "seconds": len(samples) / SAMPLE_RATE,
        "peak": 20.0 * math.log10(max(float(np.max(np.abs(samples))), 1e-9)),
        "centroid": float((freqs * spectrum).sum() / max(spectrum.sum(), 1e-9)),
        "punch": share(0.0, 500.0),
        "character": share(2000.0, 8000.0),
    }


def train(samples, interval, seconds=1.4):
    out = np.zeros(int(seconds * SAMPLE_RATE) + len(samples))
    step = int(interval * SAMPLE_RATE)
    for index in range(int(seconds / interval)):
        start = index * step
        out[start : start + len(samples)] += samples
    return out[: int(seconds * SAMPLE_RATE)]


def build():
    families = []
    for family in FAMILIES:
        strips = []
        for strip in family["strips"]:
            takes = []
            for take in strip["takes"]:
                samples = load(take["file"])
                if "rate" in take:
                    samples = train(samples, take["rate"])
                takes.append(
                    {
                        "label": take["label"],
                        "loop": take.get("loop", False),
                        "clip": encode(samples),
                        "wave": envelope(samples),
                        "metrics": measure(samples),
                    }
                )
            strips.append(
                {
                    "name": strip["name"],
                    "cue": strip["cue"],
                    "file": strip["takes"][0]["file"].split("/")[-1],
                    "status": strip["status"],
                    "wired": strip["wired"],
                    "note": strip["note"],
                    "takes": takes,
                }
            )
        families.append(
            {
                "id": family["id"],
                "title": family["title"],
                "blurb": family["blurb"],
                "strips": strips,
            }
        )
    return families


HTML = """<title>Nova Sound Bench</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap">
<style>
:root {
  --space: #03060b;
  --case-0: #0a0d10;
  --case-1: #161b20;
  --case-3: #2f383f;
  --screen-0: #001304;
  --screen-1: #002b0f;
  --phosphor: #36ff79;
  --phosphor-dim: #19a64f;
  --phosphor-muted: #0d6e35;
  --amber: #ffb84a;
  --text: #b9ffc9;
  --ink: #04140a;
  --face: linear-gradient(180deg, var(--case-3) 0%, var(--case-1) 55%, var(--case-0) 100%);
  --face-hot: linear-gradient(180deg, #3a444c 0%, #222a31 55%, #12171b 100%);
  --rim: inset 0 1px 0 rgba(255,255,255,.14);
  --drop: 0 2px 4px rgba(0,0,0,.55), 0 6px 16px -6px rgba(0,0,0,.7);
  --well: inset 0 2px 5px rgba(0,0,0,.85), inset 0 -1px 0 rgba(255,255,255,.05);
  --mono: "JetBrains Mono", "Cascadia Mono", "SFMono-Regular", Consolas, monospace;
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--space); color: var(--text);
  font-family: var(--mono); font-size: 14px; line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}
body::before {
  content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 100;
  background: repeating-linear-gradient(180deg, rgba(0,0,0,.22) 0 1px, transparent 1px 3px);
  opacity: .5;
}
.page { max-width: 1000px; margin: 0 auto; padding: 56px 24px 96px; }
.eyebrow {
  font-size: 11px; letter-spacing: .22em; text-transform: uppercase;
  color: var(--phosphor-muted); margin: 0 0 20px;
}
h1 {
  font-size: clamp(28px, 4.4vw, 44px); font-weight: 700; line-height: 1.15;
  letter-spacing: -.01em; margin: 0 0 28px; color: var(--phosphor);
  text-shadow: 0 0 12px rgba(54,255,121,.28); text-wrap: balance;
}
.thesis {
  border-left: 2px solid var(--amber); padding: 4px 0 4px 20px;
  max-width: 62ch; font-size: clamp(15px, 1.7vw, 18px); line-height: 1.65;
  color: #d9ffe6; margin: 0 0 16px;
}
.thesis b { color: var(--amber); font-weight: 600; }
.lede { max-width: 70ch; color: var(--phosphor-dim); margin: 0 0 24px; }
.lede b { color: var(--text); font-weight: 500; }
h2 {
  font-size: 12px; letter-spacing: .2em; text-transform: uppercase;
  color: var(--phosphor-muted); font-weight: 600;
  margin: 0 0 8px; padding-bottom: 10px;
  border-bottom: 1px solid rgba(54,255,121,.16);
}
.index {
  display: flex; flex-wrap: wrap; gap: 8px; margin: 0 0 52px;
  padding: 16px 18px; border: 1px solid rgba(54,255,121,.14);
  border-radius: 8px; background: rgba(54,255,121,.03);
}
.index a {
  font-size: 11px; letter-spacing: .12em; text-transform: uppercase;
  color: var(--phosphor-dim); text-decoration: none;
  border: 1px solid rgba(54,255,121,.2); border-radius: 2px; padding: 5px 10px;
}
.index a:hover { color: var(--ink); background: var(--phosphor); border-color: var(--phosphor); }
.index a:focus-visible { outline: 2px solid var(--phosphor); outline-offset: 2px; }
.index a span { color: var(--phosphor-muted); }
.index a:hover span { color: var(--ink); }
.family { margin-bottom: 56px; scroll-margin-top: 20px; }
.family__blurb { max-width: 72ch; color: var(--phosphor-dim); font-size: 13px; margin: 0 0 20px; }
.bench { display: flex; flex-direction: column; gap: 14px; }
.strip {
  background: linear-gradient(180deg, var(--case-1) 0%, var(--case-0) 100%);
  border: 1px solid rgba(54,255,121,.14); border-radius: 10px;
  box-shadow: var(--drop); padding: 16px 18px 18px;
}
.strip__head { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
.strip__name { font-size: 15px; font-weight: 600; color: var(--amber); margin: 0; }
.strip__cue { font-size: 11px; letter-spacing: .06em; color: var(--phosphor-muted); margin-left: auto; }
.tag {
  font-size: 10px; letter-spacing: .14em; text-transform: uppercase;
  padding: 2px 7px; border-radius: 2px; border: 1px solid;
}
.tag--accepted { color: var(--phosphor); border-color: rgba(54,255,121,.5); background: rgba(54,255,121,.1); }
.tag--new { color: var(--amber); border-color: rgba(255,184,74,.45); background: rgba(255,184,74,.07); }
.tag--revoiced { color: var(--amber); border-color: rgba(255,184,74,.45); background: rgba(255,184,74,.07); }
.tag--moved { color: #8fd0ff; border-color: rgba(143,208,255,.4); background: rgba(143,208,255,.07); }
.wire { font-size: 10px; letter-spacing: .12em; text-transform: uppercase; color: var(--phosphor-muted); }
.note { max-width: 72ch; color: var(--phosphor-dim); font-size: 13px; margin: 0 0 12px; }
.take { padding-top: 12px; border-top: 1px dashed rgba(54,255,121,.14); }
.take + .take { margin-top: 10px; }
.take__bar { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; margin-bottom: 8px; }
.scope {
  background: linear-gradient(180deg, var(--screen-1) 0%, var(--screen-0) 100%);
  border-radius: 3px; box-shadow: var(--well); padding: 4px;
}
.scope canvas { display: block; width: 100%; height: 46px; }
button {
  font-family: var(--mono); font-size: 12px; font-weight: 600;
  letter-spacing: .08em; text-transform: uppercase;
  color: var(--text); background: var(--face);
  border: 1px solid var(--case-0); border-radius: 2px;
  box-shadow: var(--rim), var(--drop); padding: 7px 14px; cursor: pointer;
  white-space: nowrap; min-width: 128px;
}
button:hover { background: var(--face-hot); color: var(--phosphor); }
button:active { box-shadow: var(--well); }
button:focus-visible { outline: 2px solid var(--phosphor); outline-offset: 2px; }
button[aria-pressed="true"] { background: var(--phosphor); color: var(--ink); border-color: var(--phosphor); }
.metrics {
  display: flex; gap: 18px; flex-wrap: wrap; font-size: 11px;
  color: var(--phosphor-muted); font-variant-numeric: tabular-nums;
}
.metrics b { color: var(--text); font-weight: 500; }
.callout {
  border: 1px solid rgba(255,184,74,.4); background: rgba(255,184,74,.06);
  border-radius: 8px; padding: 18px 20px; margin: 0 0 56px; max-width: 76ch;
}
.callout h3 {
  font-size: 12px; letter-spacing: .16em; text-transform: uppercase;
  color: var(--amber); margin: 0 0 10px; font-weight: 600;
}
.callout p { margin: 0 0 10px; font-size: 13px; color: var(--phosphor-dim); }
.callout p:last-child { margin: 0; }
.callout b { color: var(--text); font-weight: 500; }
ul.rules { list-style: none; padding: 0; margin: 0 0 56px; max-width: 78ch; }
ul.rules li { padding: 7px 0 7px 22px; position: relative; font-size: 13px; color: var(--phosphor-dim); border-bottom: 1px solid rgba(54,255,121,.08); }
ul.rules li::before { content: ">"; position: absolute; left: 0; color: var(--phosphor-muted); }
ul.rules b { color: var(--text); font-weight: 500; }
.tally { display: flex; gap: 32px; flex-wrap: wrap; margin-bottom: 20px; }
.tally div { min-width: 130px; }
.tally .n { font-size: 32px; font-weight: 700; color: var(--phosphor); line-height: 1; font-variant-numeric: tabular-nums; margin: 0; }
.tally .l { font-size: 11px; letter-spacing: .14em; text-transform: uppercase; color: var(--phosphor-muted); margin: 6px 0 0; }
@media (prefers-reduced-motion: reduce) { * { transition: none !important; } }
</style>

<div class="page">
  <p class="eyebrow">Nova Protocol / Audio direction / 20260824-125955 / the full set</p>
  <h1>Nova Sound Bench</h1>
  <p class="thesis">Combat in a vacuum would be silent, and a silent fight is a boring fight. Nova's guns sound the way a film's guns sound - <b>present, bright and physical</b> - and the game does not apologise for it.</p>
  <p class="lede">Round three settled the language on five cues. This is the rest of it: <b>44 files</b> in that language, every one rendered from first principles by two Python scripts. Five are marked <b>accepted</b> and are here for comparison, not for judgement - per-cue seeding means a round spent retuning the guns cannot reach their bytes.</p>
  <p class="lede">Families are auditioned together on purpose. What matters about the twin PDC is not how it sounds but how it sounds <b>next to</b> the gatling, so most strips carry an A/B take against the cue they have to be distinguishable from.</p>

  <div class="index" id="index"></div>

  <div id="bench"></div>

  <div class="callout">
    <h3>Where a sound lives, and why</h3>
    <p>The split is not UI against SFX. It is <b>where the variation lives</b>: what the engine plays uniformly for everyone, and what still has an event to fire on with zero mods loaded, is chrome and sits in <b>assets/</b>. What content authors per thing - because two of that thing could reasonably differ - sits in <b>assets/base/</b> behind an asset reference.</p>
    <p>That is why the cockpit is content and not chrome. Locking is a <b>capability of a controller section</b>, so a cheap civilian controller and a military one should be allowed to sound different - and the field is already an asset reference on the controller config, so this costs nothing to allow. The two threat alarms moved to the base mod by the same argument. What is left in <b>assets/</b> is menus, the editor, objectives, comms, and the eleven NOVA OS files.</p>
  </div>

  <h2>Rules that hold across the set</h2>
  <ul class="rules">
    <li><b>Designed for the rate it is heard at.</b> Not the rate the gun runs at: the PDC's cue throttles to twenty a second, so its round is shaped to stand alone at 50 ms spacing rather than to fuse at 10 ms. Auditioning a cue solo when it is heard in bursts is how the first two rounds went wrong.</li>
    <li><b>Same event, different hardware, separated by PITCH.</b> The drives run 34 / 52 / 78 Hz from capital to basic to vector and the guns run heavy to light the same way. Pitch is what survives a firefight; decoration is not.</li>
    <li><b>Mono.</b> The engine pans it. A pre-panned file cannot be placed.</li>
    <li><b>Full spectrum.</b> Punch lives under 500 Hz, identity lives 2 - 8 kHz. A cue with only the first is dull; only the second is thin. Four of the new destruction cues came out at 1 - 2% character and were retuned onto the accepted section-failure profile.</li>
    <li><b>Attack under 5 ms</b> on anything that is an event.</li>
    <li><b>Two voices, kept disjoint.</b> Tonal content, musical intervals and bare squares belong to the interface; layered noise, resonator banks and saturation belong to the world. The separation is what makes both legible. Avionics is the one deliberate crossing: the interface recipe with a little of the world's metal underneath.</li>
    <li><b>Peak at -3 dBFS</b> and let the per-cue volume constants do the mixing. The file is not where balance lives, so everything on this page is equally loud and nothing here tells you how loud it will be in a fight.</li>
    <li><b>Loops are built in the frequency domain</b>, so the last sample joins the first exactly. Verified by concatenating three copies: every loop's joint step sits inside its own internal step range.</li>
    <li><b>Deterministic.</b> Every cue seeds from a hash of its own name, so a rerun is byte-identical and retuning one cue cannot touch another.</li>
  </ul>

  <h2>What this pass covers</h2>
  <div class="tally">
    <div><p class="n">44</p><p class="l">files on the bench</p></div>
    <div><p class="n">5</p><p class="l">accepted in round three</p></div>
    <div><p class="n">11</p><p class="l">NOVA OS files kept as the interface standard</p></div>
    <div><p class="n">6</p><p class="l">shared voices retired</p></div>
  </div>
  <p class="lede">The full inventory - what exists, what is silent, and which cue each file is authored on - is in <b>tasks/20260824-125955/INVENTORY.md</b>. The renderers are <b>scripts/gen-world-sfx.py</b> and <b>scripts/gen-ui-sfx.py</b> over the shared toolkit in <b>scripts/nova_sfx.py</b>.</p>
</div>

<script>
const FAMILIES = __DATA__;
const bench = document.getElementById("bench");
const index = document.getElementById("index");
let current = null;

function drawScope(canvas, wave, progress) {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (!w) return;
  canvas.width = w * dpr; canvas.height = h * dpr;
  const g = canvas.getContext("2d");
  g.scale(dpr, dpr);
  g.clearRect(0, 0, w, h);
  g.strokeStyle = "rgba(54,255,121,.14)";
  g.beginPath(); g.moveTo(0, h / 2); g.lineTo(w, h / 2); g.stroke();
  const step = w / wave.length;
  for (let i = 0; i < wave.length; i++) {
    g.fillStyle = i / wave.length <= progress ? "#36ff79" : "rgba(25,166,79,.55)";
    const lo = h / 2 - wave[i][1] * (h / 2 - 3);
    const hi = h / 2 - wave[i][0] * (h / 2 - 3);
    g.fillRect(i * step, lo, Math.max(step - 0.4, 0.6), Math.max(hi - lo, 1));
  }
  if (progress > 0 && progress < 1) {
    g.strokeStyle = "#ffb84a"; g.lineWidth = 1;
    g.beginPath(); g.moveTo(progress * w, 0); g.lineTo(progress * w, h); g.stroke();
  }
}

const scopes = [];

function wire(take, canvas, button) {
  const audio = new Audio(take.clip);
  audio.loop = take.loop;
  let frame = 0;
  const stop = () => {
    cancelAnimationFrame(frame);
    audio.pause(); audio.currentTime = 0;
    button.setAttribute("aria-pressed", "false");
    button.textContent = take.label;
    drawScope(canvas, take.wave, 0);
    if (current === stop) current = null;
  };
  const tick = () => {
    drawScope(canvas, take.wave, audio.duration ? audio.currentTime / audio.duration : 0);
    frame = requestAnimationFrame(tick);
  };
  button.addEventListener("click", () => {
    if (button.getAttribute("aria-pressed") === "true") { stop(); return; }
    if (current) current();
    current = stop;
    button.setAttribute("aria-pressed", "true");
    button.textContent = "Stop";
    audio.play(); tick();
  });
  audio.addEventListener("ended", stop);
  scopes.push([canvas, take.wave]);
  drawScope(canvas, take.wave, 0);
}

for (const family of FAMILIES) {
  const link = document.createElement("a");
  link.href = "#" + family.id;
  link.innerHTML = `${family.title} <span>${family.strips.length}</span>`;
  index.appendChild(link);

  const section = document.createElement("section");
  section.className = "family";
  section.id = family.id;
  section.innerHTML = `<h2>${family.title}</h2><p class="family__blurb">${family.blurb}</p>`;
  const list = document.createElement("div");
  list.className = "bench";

  for (const strip of family.strips) {
    const el = document.createElement("article");
    el.className = "strip";
    el.innerHTML = `
      <div class="strip__head">
        <h3 class="strip__name">${strip.name}</h3>
        <span class="tag tag--${strip.status}">${strip.status}</span>
        <span class="wire">${strip.wired}</span>
        <span class="strip__cue">${strip.cue} &middot; ${strip.file}</span>
      </div>
      <p class="note">${strip.note}</p>`;
    for (const take of strip.takes) {
      const m = take.metrics;
      const row = document.createElement("div");
      row.className = "take";
      row.innerHTML = `
        <div class="take__bar">
          <button aria-pressed="false">${take.label}</button>
          <div class="metrics">
            <span>length <b>${m.seconds.toFixed(3)} s</b></span>
            <span>centroid <b>${Math.round(m.centroid)} Hz</b></span>
            <span>punch &lt;500 Hz <b>${m.punch.toFixed(0)}%</b></span>
            <span>character 2-8 kHz <b>${m.character.toFixed(0)}%</b></span>
          </div>
        </div>
        <div class="scope"><canvas></canvas></div>`;
      el.appendChild(row);
      wire(take, row.querySelector("canvas"), row.querySelector("button"));
    }
    list.appendChild(el);
  }
  section.appendChild(list);
  bench.appendChild(section);
}

requestAnimationFrame(() => scopes.forEach(([c, w]) => drawScope(c, w, 0)));
window.addEventListener("resize", () => scopes.forEach(([c, w]) => drawScope(c, w, 0)));
</script>
"""


def main():
    page = HTML.replace("__DATA__", json.dumps(build(), separators=(",", ":")))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as out:
        out.write(page)
    cues = sum(len(f["strips"]) for f in FAMILIES)
    print(f"{OUT} ({len(page) / 1024:.0f} KB) - {cues} strips in {len(FAMILIES)} families")


if __name__ == "__main__":
    main()
