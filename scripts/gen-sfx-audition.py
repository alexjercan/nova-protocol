#!/usr/bin/env python3
"""Render the sound-audition bench for the audio direction pass.

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
STRIPS = [
    {
        "name": "PDC gatling, one round",
        "file": "assets/base/sounds/turret_fire.wav",
        "cue": "turret fire_sound",
        "wired": True,
        "burst": 0.05,
        "note": (
            "The flagship. Fired up to twenty times a second, so what you hear "
            "in a fight is this clip beating against itself. 99.9% of its "
            "energy is inside 38 ms, under the cue's 50 ms throttle, so a burst "
            "peaks at exactly one round - the growl comes from the beat, never "
            "from stacking. Listen to the burst, not the single round."
        ),
    },
    {
        "name": "Railgun lance, discharge",
        "file": "assets/base/sounds/railgun_fire.wav",
        "cue": "railgun fire_sound",
        "wired": False,
        "note": (
            "Three events in the order the gameplay does them: the capacitor "
            "bank dumping, the slug leaving on a downward-swept low body, and "
            "the hull taking the recoil as long modes rolling down the spine. "
            "The loudest one-shot in the game, and the only one that can afford "
            "to be - it fires once every twelve seconds."
        ),
    },
    {
        "name": "Kinetic round, on plate",
        "file": "assets/base/sounds/impact.wav",
        "cue": "section impact_sound",
        "wired": True,
        "note": (
            "The most-heard event in a fight and the easiest to make tiring, so "
            "it is deliberately small: a strike and a short answer, with no low "
            "weight to accumulate when a burst walks across a hull."
        ),
    },
    {
        "name": "Section failing",
        "file": "assets/base/sounds/explosion.wav",
        "cue": "section destroy_sound",
        "wired": True,
        "note": (
            "Not an explosion - there is no air to carry a blast wave. What "
            "reaches the pilot through the hull is structural: a tear, the mass "
            "letting go, and debris rattling off the plating on the way out."
        ),
    },
    {
        "name": "Main drive, running",
        "file": "assets/base/sounds/thruster_loop.wav",
        "cue": "thruster loop_sound",
        "wired": True,
        "loop": True,
        "note": (
            "Built in the frequency domain, so the last sample joins the first "
            "with no seam at all - the button loops it, and the seam is where "
            "to listen. Broadband turbulence with the throat resonating in it, "
            "never a held tone: a tone reads as a synthesizer within a second."
        ),
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
    window = np.hanning(len(samples))
    spectrum = np.abs(np.fft.rfft(samples * window))
    freqs = np.fft.rfftfreq(len(samples), 1.0 / SAMPLE_RATE)
    power = spectrum**2
    return {
        "seconds": len(samples) / SAMPLE_RATE,
        "peak": 20.0 * math.log10(max(float(np.max(np.abs(samples))), 1e-9)),
        "rms": 20.0 * math.log10(max(float(np.sqrt((samples**2).mean())), 1e-9)),
        "centroid": float((freqs * spectrum).sum() / max(spectrum.sum(), 1e-9)),
        "low": float(power[freqs < 2000.0].sum() / max(power.sum(), 1e-9)) * 100.0,
    }


def burst(samples, interval, seconds=1.4):
    out = np.zeros(int(seconds * SAMPLE_RATE) + len(samples))
    step = int(interval * SAMPLE_RATE)
    for index in range(int(seconds / interval)):
        start = index * step
        out[start : start + len(samples)] += samples
    return out[: int(seconds * SAMPLE_RATE)]


def build():
    strips = []
    for strip in STRIPS:
        samples = load(strip["file"])
        entry = {
            "name": strip["name"],
            "cue": strip["cue"],
            "file": strip["file"].split("/")[-1],
            "wired": strip["wired"],
            "loop": strip.get("loop", False),
            "note": strip["note"],
            "clip": encode(samples),
            "wave": envelope(samples),
            "metrics": measure(samples),
        }
        if "burst" in strip:
            train = burst(samples, strip["burst"])
            entry["burst"] = encode(train)
            entry["burstWave"] = envelope(train)
            entry["burstLabel"] = f"at fire rate ({round(1.0 / strip['burst'])}/s)"
        strips.append(entry)
    return strips


HTML = """<title>Hull Voice Audition</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap">
<style>
:root {
  --space: #03060b;
  --case-0: #0a0d10;
  --case-1: #161b20;
  --case-2: #232a31;
  --case-3: #2f383f;
  --screen-0: #001304;
  --screen-1: #002b0f;
  --phosphor: #36ff79;
  --phosphor-dim: #19a64f;
  --phosphor-muted: #0d6e35;
  --amber: #ffb84a;
  --amber-dim: #8a6222;
  --red: #ff4e42;
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
  margin: 0;
  background: var(--space);
  color: var(--text);
  font-family: var(--mono);
  font-size: 14px;
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}
body::before {
  content: "";
  position: fixed; inset: 0; pointer-events: none; z-index: 100;
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
.lede { max-width: 66ch; color: var(--phosphor-dim); margin: 0 0 52px; }
h2 {
  font-size: 12px; letter-spacing: .2em; text-transform: uppercase;
  color: var(--phosphor-muted); font-weight: 600;
  margin: 0 0 20px; padding-bottom: 10px;
  border-bottom: 1px solid rgba(54,255,121,.16);
}
.bench { display: flex; flex-direction: column; gap: 18px; margin-bottom: 64px; }
.strip {
  background: linear-gradient(180deg, var(--case-1) 0%, var(--case-0) 100%);
  border: 1px solid rgba(54,255,121,.14); border-radius: 10px;
  box-shadow: var(--drop); padding: 18px 20px 20px;
}
.strip__head {
  display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap;
  margin-bottom: 14px;
}
.strip__name { font-size: 16px; font-weight: 600; color: var(--amber); margin: 0; }
.strip__cue {
  font-size: 11px; letter-spacing: .1em; color: var(--phosphor-muted);
  margin-left: auto;
}
.tag {
  font-size: 10px; letter-spacing: .14em; text-transform: uppercase;
  padding: 2px 7px; border-radius: 2px; border: 1px solid;
}
.tag--live { color: var(--phosphor); border-color: rgba(54,255,121,.45); background: rgba(54,255,121,.08); }
.tag--held { color: var(--amber); border-color: rgba(255,184,74,.45); background: rgba(255,184,74,.07); }
.scope {
  background: linear-gradient(180deg, var(--screen-1) 0%, var(--screen-0) 100%);
  border-radius: 3px; box-shadow: var(--well); padding: 6px;
  margin-bottom: 14px;
}
.scope + .scope { margin-top: -4px; }
.scope canvas { display: block; width: 100%; height: 74px; }
.transport { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; }
button {
  font-family: var(--mono); font-size: 12px; font-weight: 600;
  letter-spacing: .08em; text-transform: uppercase;
  color: var(--text); background: var(--face);
  border: 1px solid var(--case-0); border-radius: 2px;
  box-shadow: var(--rim), var(--drop); padding: 8px 16px; cursor: pointer;
}
button:hover { background: var(--face-hot); color: var(--phosphor); }
button:active { box-shadow: var(--well); }
button:focus-visible { outline: 2px solid var(--phosphor); outline-offset: 2px; }
button[aria-pressed="true"] { background: var(--phosphor); color: var(--ink); border-color: var(--phosphor); }
.metrics {
  display: flex; gap: 22px; flex-wrap: wrap;
  font-size: 11px; color: var(--phosphor-muted);
  font-variant-numeric: tabular-nums;
  padding-top: 12px; border-top: 1px dashed rgba(54,255,121,.14);
}
.metrics b { color: var(--text); font-weight: 500; }
.note { max-width: 68ch; color: var(--phosphor-dim); font-size: 13px; margin: 0 0 14px; }
.layers { display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); margin-bottom: 28px; }
.layer {
  border: 1px solid rgba(54,255,121,.14); border-radius: 6px;
  padding: 14px 16px; background: rgba(54,255,121,.03);
}
.layer h3 {
  font-size: 12px; letter-spacing: .14em; text-transform: uppercase;
  color: var(--amber); margin: 0 0 4px; font-weight: 600;
}
.layer .when { font-size: 11px; color: var(--phosphor-muted); margin: 0 0 8px; font-variant-numeric: tabular-nums; }
.layer p { margin: 0; font-size: 13px; color: var(--phosphor-dim); }
ul.rules { list-style: none; padding: 0; margin: 0 0 64px; max-width: 74ch; }
ul.rules li { padding: 7px 0 7px 22px; position: relative; font-size: 13px; color: var(--phosphor-dim); border-bottom: 1px solid rgba(54,255,121,.08); }
ul.rules li::before { content: ">"; position: absolute; left: 0; color: var(--phosphor-muted); }
ul.rules b { color: var(--text); font-weight: 500; }
.tally { display: flex; gap: 32px; flex-wrap: wrap; margin-bottom: 20px; }
.tally div { min-width: 120px; }
.tally .n { font-size: 32px; font-weight: 700; color: var(--phosphor); line-height: 1; font-variant-numeric: tabular-nums; }
.tally .l { font-size: 11px; letter-spacing: .14em; text-transform: uppercase; color: var(--phosphor-muted); margin-top: 6px; }
@media (prefers-reduced-motion: reduce) { * { transition: none !important; } }
</style>

<div class="page">
  <p class="eyebrow">Nova Protocol / Audio direction / 20260824-125955</p>
  <h1>Hull Voice Audition</h1>
  <p class="thesis">Sound does not travel in vacuum, so everything the pilot hears is either <b>conducted through their own hull</b> or <b>synthesized by the ship's computer</b> as feedback. That is why the game has sound at all, it is what the Vacuum setting turns off, and it is the whole sound-design brief: a gun heard through a deck plate, not through air.</p>
  <p class="lede">Five cues rendered against that brief - a fast repeat, a heavy one-off, a small hit, a big break and a bed. If the language holds here, the remaining thirty-five follow it.</p>

  <h2>The bench</h2>
  <div class="bench" id="bench"></div>

  <h2>Anatomy - every world cue is these three layers</h2>
  <div class="layers">
    <div class="layer">
      <h3>Transient</h3>
      <p class="when">0 - 8 ms</p>
      <p>The mechanical event: a click, a crack, a strike. Broadband and very fast. It is the edge, not the weight.</p>
    </div>
    <div class="layer">
      <h3>Body</h3>
      <p class="when">10 - 200 ms</p>
      <p>Filtered noise carrying the mass, 80 - 800 Hz. This is where a PDC gets its chest punch.</p>
    </div>
    <div class="layer">
      <h3>Ring</h3>
      <p class="when">up to 400 ms</p>
      <p>Three to six detuned modes, the structure answering. A ring is metal; a tail is a room, and there are no rooms.</p>
    </div>
  </div>

  <h2>Rules that hold across the set</h2>
  <ul class="rules">
    <li><b>Mono.</b> The engine pans it. A pre-panned file cannot be placed.</li>
    <li><b>No musical intervals, no arpeggios, no bare square waves.</b> Tonal content belongs to the interface voice, and the separation is the point. This is the no-chiptune rule, stated so it is checkable.</li>
    <li><b>Bulk energy under 2 kHz.</b> Air carries the top end and there is none. Every clip below is at 99.5% or better.</li>
    <li><b>Attack under 5 ms</b> on anything that is an event.</li>
    <li><b>Dry.</b> No tail past about 400 ms anywhere.</li>
    <li><b>Peak at -3 dBFS</b> and let the per-cue volume constants do the mixing. The file is not where balance lives.</li>
    <li><b>Deterministic.</b> Every cue seeds from a hash of its own name, so a rerun is byte-identical and adding a cue rewrites no other cue's bytes.</li>
  </ul>

  <h2>What this pass covers</h2>
  <div class="tally">
    <div><p class="n">5</p><p class="l">auditioned here</p></div>
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
  canvas.width = w * dpr; canvas.height = h * dpr;
  const g = canvas.getContext("2d");
  g.scale(dpr, dpr);
  g.clearRect(0, 0, w, h);
  g.strokeStyle = "rgba(54,255,121,.14)";
  g.beginPath(); g.moveTo(0, h / 2); g.lineTo(w, h / 2); g.stroke();
  const step = w / wave.length;
  for (let i = 0; i < wave.length; i++) {
    const played = i / wave.length <= progress;
    g.fillStyle = played ? "#36ff79" : "rgba(25,166,79,.55)";
    const lo = h / 2 - wave[i][1] * (h / 2 - 3);
    const hi = h / 2 - wave[i][0] * (h / 2 - 3);
    g.fillRect(i * step, lo, Math.max(step - 0.4, 0.6), Math.max(hi - lo, 1));
  }
  if (progress > 0 && progress < 1) {
    g.strokeStyle = "#ffb84a"; g.lineWidth = 1;
    g.beginPath(); g.moveTo(progress * w, 0); g.lineTo(progress * w, h); g.stroke();
  }
}

function makeVoice(src, wave, canvas, button, label, loop) {
  const audio = new Audio(src);
  audio.loop = !!loop;
  let frame = 0;
  const paint = () => drawScope(canvas, wave, audio.duration ? audio.currentTime / audio.duration : 0);
  const stop = () => {
    cancelAnimationFrame(frame);
    audio.pause(); audio.currentTime = 0;
    button.setAttribute("aria-pressed", "false");
    button.textContent = label;
    drawScope(canvas, wave, 0);
    if (current === stop) current = null;
  };
  const tick = () => { paint(); frame = requestAnimationFrame(tick); };
  button.addEventListener("click", () => {
    if (button.getAttribute("aria-pressed") === "true") { stop(); return; }
    if (current) current();
    current = stop;
    button.setAttribute("aria-pressed", "true");
    button.textContent = loop ? "Stop" : label;
    audio.play(); tick();
  });
  audio.addEventListener("ended", stop);
  drawScope(canvas, wave, 0);
  return stop;
}

for (const strip of STRIPS) {
  const el = document.createElement("section");
  el.className = "strip";
  const m = strip.metrics;
  el.innerHTML = `
    <div class="strip__head">
      <h3 class="strip__name">${strip.name}</h3>
      <span class="tag ${strip.wired ? "tag--live" : "tag--held"}">${strip.wired ? "plays in game now" : "awaiting authoring"}</span>
      <span class="strip__cue">${strip.cue} &middot; ${strip.file}</span>
    </div>
    <p class="note">${strip.note}</p>
    <div class="scope"><canvas></canvas></div>
    ${strip.burst ? '<div class="scope"><canvas></canvas></div>' : ""}
    <div class="transport"></div>
    <div class="metrics">
      <span>length <b>${m.seconds.toFixed(3)} s</b></span>
      <span>peak <b>${m.peak.toFixed(1)} dBFS</b></span>
      <span>rms <b>${m.rms.toFixed(1)} dBFS</b></span>
      <span>centroid <b>${Math.round(m.centroid)} Hz</b></span>
      <span>under 2 kHz <b>${m.low.toFixed(1)}%</b></span>
    </div>`;
  bench.appendChild(el);

  const scopes = el.querySelectorAll("canvas");
  const transport = el.querySelector(".transport");
  const one = document.createElement("button");
  one.textContent = strip.loop ? "Play looped" : "Play";
  one.setAttribute("aria-pressed", "false");
  transport.appendChild(one);
  makeVoice(strip.clip, strip.wave, scopes[0], one, strip.loop ? "Play looped" : "Play", strip.loop);

  if (strip.burst) {
    const many = document.createElement("button");
    many.textContent = strip.burstLabel;
    many.setAttribute("aria-pressed", "false");
    transport.appendChild(many);
    makeVoice(strip.burst, strip.burstWave, scopes[1], many, strip.burstLabel, false);
  }
}
window.addEventListener("resize", () => {
  document.querySelectorAll(".strip").forEach((el, i) => {
    const scopes = el.querySelectorAll("canvas");
    drawScope(scopes[0], STRIPS[i].wave, 0);
    if (scopes[1]) drawScope(scopes[1], STRIPS[i].burstWave, 0);
  });
});
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
