#!/usr/bin/env python3
"""Render the sound-audition bench for the audio direction pass.

Each cue is auditioned at the rate it is HEARD at, which is why the PDC leads
with its fire loop rather than with one round.

Reads the rendered WAVs, measures them, and writes a self-contained page with
the clips embedded so the owner can listen, look at the anatomy, and check the
numbers against the style spec in one place. Committed with the task because
the page IS the reasoning - `TASK.md` only summarises the verdict.

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

# One entry per audition strip: the file, what the cue is, what the design is
# doing, and whether the game plays it today.
# One entry per audition strip. `takes` is what you can actually press: a cue
# is auditioned at the rate it is HEARD at, so the PDC leads with its fire loop
# and the single round is the last take, not the first.
STRIPS = [
    {
        "name": "PDC gatling",
        "cue": "turret fire_sound",
        "wired": "partly",
        "note": (
            "Re-voiced with the top end it was missing - the muzzle report and "
            "the rotary action, 2-9 kHz, which is what a PDC is recognised by. "
            "The fire loop is new: the gun runs at 100 rounds a second, its low "
            "body now REPEATS from round to round instead of being re-rolled, "
            "and 48% of the loop's energy lands on exact multiples of 100 Hz. "
            "That is the buzz. The 20/s take below is what the game plays "
            "today, and the difference is the whole argument for the loop."
        ),
        "takes": [
            {"label": "fire loop, 100/s", "file": "assets/base/sounds/pdc_gatling_loop.wav", "loop": True},
            {"label": "20/s, as shipped", "file": "assets/base/sounds/turret_fire.wav", "rate": 0.05},
            {"label": "one round", "file": "assets/base/sounds/turret_fire.wav"},
        ],
    },
    {
        "name": "Railgun lance, discharge",
        "cue": "railgun fire_sound",
        "wired": "yes",
        "note": (
            "Accepted last round, unchanged - per-cue seeding means retuning "
            "the PDC could not touch its bytes. The capacitor bank dumping, the "
            "slug leaving on a downward-swept low body, and the hull taking the "
            "recoil, in the order the shot does them."
        ),
        "takes": [{"label": "play", "file": "assets/base/sounds/railgun_fire.wav"}],
    },
    {
        "name": "Main drive, running",
        "cue": "thruster loop_sound",
        "wired": "yes",
        "note": (
            "Less noisy. The broadband layer above the resonances was running "
            "loose and the bed read as hiss rather than as an engine; it is cut "
            "to a fifth, the rumble is steeper, and a second resonance at the "
            "plenum gives it machine character. One slow waver instead of two, "
            "because two beat against each other into something the ear tracks. "
            "Still built in the frequency domain, so the seam is where to listen."
        ),
        "takes": [{"label": "play looped", "file": "assets/base/sounds/thruster_loop.wav", "loop": True}],
    },
    {
        "name": "Kinetic round, on plate",
        "cue": "section impact_sound",
        "wired": "yes",
        "note": (
            "Brightened to match the new brief: the strike now carries spall "
            "above 3 kHz and the mode bank gained a high mode. Still "
            "deliberately small - it is the most-heard event in a fight and the "
            "easiest to make tiring."
        ),
        "takes": [
            {"label": "play", "file": "assets/base/sounds/impact.wav"},
            {"label": "burst, 8/s", "file": "assets/base/sounds/impact.wav", "rate": 0.125},
        ],
    },
    {
        "name": "Section failing",
        "cue": "section destroy_sound",
        "wired": "yes",
        "note": (
            "The tear reaches into the top end now and a shear layer sits above "
            "3.6 kHz, so the failure has an edge and not only weight. The shape "
            "is unchanged: a tear, the mass letting go, then debris rattling off "
            "the plating on the way out."
        ),
        "takes": [{"label": "play", "file": "assets/base/sounds/explosion.wav"}],
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


def envelope(samples, columns=440):
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
    strips = []
    for strip in STRIPS:
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
                "wired": strip["wired"],
                "note": strip["note"],
                "takes": takes,
            }
        )
    return strips


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
.page { max-width: 980px; margin: 0 auto; padding: 56px 24px 96px; }
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
.lede { max-width: 68ch; color: var(--phosphor-dim); margin: 0 0 52px; }
h2 {
  font-size: 12px; letter-spacing: .2em; text-transform: uppercase;
  color: var(--phosphor-muted); font-weight: 600;
  margin: 0 0 20px; padding-bottom: 10px;
  border-bottom: 1px solid rgba(54,255,121,.16);
}
.bench { display: flex; flex-direction: column; gap: 18px; margin-bottom: 44px; }
.strip {
  background: linear-gradient(180deg, var(--case-1) 0%, var(--case-0) 100%);
  border: 1px solid rgba(54,255,121,.14); border-radius: 10px;
  box-shadow: var(--drop); padding: 18px 20px 20px;
}
.strip__head { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; margin-bottom: 12px; }
.strip__name { font-size: 16px; font-weight: 600; color: var(--amber); margin: 0; }
.strip__cue { font-size: 11px; letter-spacing: .1em; color: var(--phosphor-muted); margin-left: auto; }
.tag {
  font-size: 10px; letter-spacing: .14em; text-transform: uppercase;
  padding: 2px 7px; border-radius: 2px; border: 1px solid;
}
.tag--yes { color: var(--phosphor); border-color: rgba(54,255,121,.45); background: rgba(54,255,121,.08); }
.tag--partly { color: var(--amber); border-color: rgba(255,184,74,.45); background: rgba(255,184,74,.07); }
.note { max-width: 70ch; color: var(--phosphor-dim); font-size: 13px; margin: 0 0 16px; }
.take { padding-top: 14px; border-top: 1px dashed rgba(54,255,121,.14); }
.take + .take { margin-top: 12px; }
.take__bar { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; margin-bottom: 10px; }
.scope {
  background: linear-gradient(180deg, var(--screen-1) 0%, var(--screen-0) 100%);
  border-radius: 3px; box-shadow: var(--well); padding: 5px;
}
.scope canvas { display: block; width: 100%; height: 62px; }
button {
  font-family: var(--mono); font-size: 12px; font-weight: 600;
  letter-spacing: .08em; text-transform: uppercase;
  color: var(--text); background: var(--face);
  border: 1px solid var(--case-0); border-radius: 2px;
  box-shadow: var(--rim), var(--drop); padding: 8px 16px; cursor: pointer;
  white-space: nowrap;
}
button:hover { background: var(--face-hot); color: var(--phosphor); }
button:active { box-shadow: var(--well); }
button:focus-visible { outline: 2px solid var(--phosphor); outline-offset: 2px; }
button[aria-pressed="true"] { background: var(--phosphor); color: var(--ink); border-color: var(--phosphor); }
.metrics {
  display: flex; gap: 20px; flex-wrap: wrap; font-size: 11px;
  color: var(--phosphor-muted); font-variant-numeric: tabular-nums;
}
.metrics b { color: var(--text); font-weight: 500; }
.callout {
  border: 1px solid rgba(255,184,74,.4); background: rgba(255,184,74,.06);
  border-radius: 8px; padding: 18px 20px; margin-bottom: 64px; max-width: 74ch;
}
.callout h3 {
  font-size: 12px; letter-spacing: .16em; text-transform: uppercase;
  color: var(--amber); margin: 0 0 10px; font-weight: 600;
}
.callout p { margin: 0 0 10px; font-size: 13px; color: var(--phosphor-dim); }
.callout p:last-child { margin: 0; }
.callout b { color: var(--text); font-weight: 500; }
.layers { display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); margin-bottom: 28px; }
.layer { border: 1px solid rgba(54,255,121,.14); border-radius: 6px; padding: 14px 16px; background: rgba(54,255,121,.03); }
.layer h3 { font-size: 12px; letter-spacing: .14em; text-transform: uppercase; color: var(--amber); margin: 0 0 4px; font-weight: 600; }
.layer .when { font-size: 11px; color: var(--phosphor-muted); margin: 0 0 8px; font-variant-numeric: tabular-nums; }
.layer p { margin: 0; font-size: 13px; color: var(--phosphor-dim); }
ul.rules { list-style: none; padding: 0; margin: 0 0 64px; max-width: 76ch; }
ul.rules li { padding: 7px 0 7px 22px; position: relative; font-size: 13px; color: var(--phosphor-dim); border-bottom: 1px solid rgba(54,255,121,.08); }
ul.rules li::before { content: ">"; position: absolute; left: 0; color: var(--phosphor-muted); }
ul.rules b { color: var(--text); font-weight: 500; }
.tally { display: flex; gap: 32px; flex-wrap: wrap; margin-bottom: 20px; }
.tally div { min-width: 120px; }
.tally .n { font-size: 32px; font-weight: 700; color: var(--phosphor); line-height: 1; font-variant-numeric: tabular-nums; margin: 0; }
.tally .l { font-size: 11px; letter-spacing: .14em; text-transform: uppercase; color: var(--phosphor-muted); margin: 6px 0 0; }
@media (prefers-reduced-motion: reduce) { * { transition: none !important; } }
</style>

<div class="page">
  <p class="eyebrow">Nova Protocol / Audio direction / 20260824-125955 / round two</p>
  <h1>Nova Sound Bench</h1>
  <p class="thesis">Combat in a vacuum would be silent, and a silent fight is a boring fight. Nova's guns sound the way a film's guns sound - <b>present, bright and physical</b> - and the game does not apologise for it.</p>
  <p class="lede">A realism mode, where every cue is instead conducted through your own hull or synthesized by the ship's computer, stays on the table as a future setting. It is deliberately not what these are, and it costs nothing to keep open: every world sound is mod content behind an asset reference, so that mode is a second set of files under the same names.</p>

  <h2>The bench</h2>
  <div class="bench" id="bench"></div>

  <div class="callout">
    <h3>One decision this raises</h3>
    <p>The gun runs at <b>100 rounds a second</b>. The cue that plays it is one sound per round, throttled to twenty a second, because a hundred audio entities a second is not something to spawn - so the shipped gun can only ever rattle. Press the two takes back to back and the gap is obvious.</p>
    <p>The fix is the standard one: a <b>held fire loop</b> while the trigger is down, with the single round kept for the ragged ends of a burst. That is a change to the turret cue in <b>ship_audio</b>, which the engine lane currently owns - so it is a call to make, not something to slip in behind it.</p>
  </div>

  <h2>Anatomy - every cue is these three layers</h2>
  <div class="layers">
    <div class="layer">
      <h3>Transient</h3>
      <p class="when">0 - 8 ms</p>
      <p>The crack. Broadband and bright: where a gun gets its edge and most of its identity.</p>
    </div>
    <div class="layer">
      <h3>Body</h3>
      <p class="when">10 - 200 ms</p>
      <p>Filtered noise carrying the mass, 80 - 800 Hz. The chest punch.</p>
    </div>
    <div class="layer">
      <h3>Ring</h3>
      <p class="when">up to 400 ms</p>
      <p>Three to six detuned modes, the hardware answering.</p>
    </div>
  </div>

  <h2>Rules that hold across the set</h2>
  <ul class="rules">
    <li><b>Designed for the rate it is heard at.</b> The PDC is never heard alone, so it is shaped to stack at a 10 ms period - 48% of the fire loop's energy lands on exact multiples of 100 Hz, and that is the buzz. This is the lesson of this round.</li>
    <li><b>Mono.</b> The engine pans it. A pre-panned file cannot be placed.</li>
    <li><b>Full spectrum.</b> Punch lives under 500 Hz, identity lives 2 - 8 kHz. A cue with only the first is dull; only the second is thin.</li>
    <li><b>Attack under 5 ms</b> on anything that is an event.</li>
    <li><b>No musical intervals, no arpeggios, no bare square waves.</b> Tonal content belongs to the NOVA OS interface voice, and the separation is what makes both legible.</li>
    <li><b>Peak at -3 dBFS</b> and let the per-cue volume constants do the mixing. The file is not where balance lives.</li>
    <li><b>Deterministic.</b> Every cue seeds from a hash of its own name, so a rerun is byte-identical and retuning one cue cannot touch another - which is why the railgun is unchanged after a round spent on the PDC.</li>
  </ul>

  <h2>What this pass covers</h2>
  <div class="tally">
    <div><p class="n">6</p><p class="l">files on the bench</p></div>
    <div><p class="n">40</p><p class="l">on the production list</p></div>
    <div><p class="n">11</p><p class="l">NOVA OS files kept as the interface standard</p></div>
    <div><p class="n">6</p><p class="l">shared voices to retire</p></div>
  </div>
  <p class="lede">The full inventory - what exists, what is silent, and which cue each file is authored on - is in <b>tasks/20260824-125955/INVENTORY.md</b>.</p>
</div>

<script>
const STRIPS = __DATA__;
const bench = document.getElementById("bench");
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

for (const strip of STRIPS) {
  const el = document.createElement("section");
  el.className = "strip";
  el.innerHTML = `
    <div class="strip__head">
      <h3 class="strip__name">${strip.name}</h3>
      <span class="tag tag--${strip.wired}">${strip.wired === "yes" ? "plays in game now" : "loop not wired yet"}</span>
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
          <span>peak <b>${m.peak.toFixed(1)} dBFS</b></span>
          <span>centroid <b>${Math.round(m.centroid)} Hz</b></span>
          <span>punch &lt;500 Hz <b>${m.punch.toFixed(0)}%</b></span>
          <span>character 2-8 kHz <b>${m.character.toFixed(0)}%</b></span>
        </div>
      </div>
      <div class="scope"><canvas></canvas></div>`;
    el.appendChild(row);
    wire(take, row.querySelector("canvas"), row.querySelector("button"));
  }
  bench.appendChild(el);
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
    print(f"{OUT} ({len(page) / 1024:.0f} KB)")


if __name__ == "__main__":
    main()
