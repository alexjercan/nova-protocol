"""Drawing vocabulary for the story comics' CRT panels.

Every panel is a phosphor screen: dark green ground, faint grid, scanlines and
a vignette, drawn in the palette of the campaign portraits. Hulls are muted
khaki for EWI, bone for Kestrel, gold for the Kites, and light is phosphor.
The comic engine adds the bezel around each panel, so the art stops at the
screen. Compositions live in the per-comic scripts beside this module.
"""

from __future__ import annotations

import math
import random

MONO = "IBM Plex Mono, DejaVu Sans Mono, Menlo, monospace"

GROUND = "#030d08"
SPACE = "#020a06"
GRID = "#164c31"
PHOS = "#4ed184"
PHOS_BRIGHT = "#c0ffd4"
PHOS_DIM = "#267b4d"
PHOS_TEXT = "#6ea985"
PHOS_WHITE = "#e8fff0"
LIME = "#b8da5d"
DEEP = "#183426"
GOLD = "#c2a23b"
GOLD_DARK = "#9d8128"
GOLD_DEEP = "#796424"
EMBER = "#ffd67a"
INK = "#06170f"
PALE = "#8fb69b"
PALE_BRIGHT = "#b7dfc3"
PALE_DIM = "#73977f"
SUIT = "#6d6845"
SUIT_DARK = "#5d603e"
SUIT_EDGE = "#3e4130"
GEAR = "#3b4a3a"
GEAR_EDGE = "#1d2b1f"

EWI = dict(hull="#8f8656", dark="#5d603e", light="#b0aa75", edge="#3e4130", win=PHOS, glow=PHOS, band=GOLD)
SIL = dict(hull="#0a1711", dark="#08130d", light="#0d1d14", edge="#1f3a2b", win="#0a1711", glow="none", band="#0f1a12")
KESTREL = dict(hull="#c4c2ae", dark="#8c8b78", light="#e6e4d2", edge="#55564a", win="#e0b64a", glow=GOLD, band=GOLD_DARK)
KESTREL_DEAD = dict(KESTREL, win="#3a3a2c", glow="none")

DEFS = f"""
  <defs>
    <radialGradient id="screen" cx="50%" cy="45%" r="75%">
      <stop offset="0" stop-color="#07190f"/><stop offset="1" stop-color="{GROUND}"/>
    </radialGradient>
    <radialGradient id="vignette" cx="50%" cy="50%" r="70%">
      <stop offset=".55" stop-color="{GROUND}" stop-opacity="0"/><stop offset="1" stop-color="{GROUND}" stop-opacity=".7"/>
    </radialGradient>
    <pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse">
      <rect width="4" height="1" fill="#001008" opacity=".5"/>
    </pattern>
    <pattern id="grid" width="120" height="120" patternUnits="userSpaceOnUse">
      <path d="M120 0H0V120" fill="none" stroke="{GRID}" stroke-opacity=".28"/>
    </pattern>
    <radialGradient id="moon" cx="40%" cy="30%" r="80%">
      <stop offset="0" stop-color="#1e4431"/><stop offset=".55" stop-color="#0d2418"/><stop offset="1" stop-color="#040f09"/>
    </radialGradient>
    <radialGradient id="flash">
      <stop offset="0" stop-color="{PHOS_WHITE}"/><stop offset=".18" stop-color="{PHOS_BRIGHT}"/><stop offset=".45" stop-color="{PHOS}" stop-opacity=".85"/><stop offset="1" stop-color="{PHOS}" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="ember">
      <stop offset="0" stop-color="{EMBER}"/><stop offset=".4" stop-color="{GOLD}" stop-opacity=".8"/><stop offset="1" stop-color="{GOLD}" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="lamp">
      <stop offset="0" stop-color="{GOLD}" stop-opacity=".55"/><stop offset="1" stop-color="{GOLD}" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="screenlight">
      <stop offset="0" stop-color="{PHOS}" stop-opacity=".45"/><stop offset="1" stop-color="{PHOS}" stop-opacity="0"/>
    </radialGradient>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="6" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
    <filter id="soft" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="14"/>
    </filter>
  </defs>"""


class Panel:
    """One screen. Collects SVG fragments and writes them between ground and scanlines."""

    def __init__(self, width: int, height: int, *, space: bool = False, grid: bool = False):
        self.w = width
        self.h = height
        self.space = space
        self.grid = grid
        self.parts: list[str] = []

    def add(self, *parts: str) -> "Panel":
        self.parts.extend(parts)
        return self

    def svg(self) -> str:
        ground = SPACE if self.space else "url(#screen)"
        body = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{self.w}" height="{self.h}" viewBox="0 0 {self.w} {self.h}">',
            DEFS,
            f'  <rect width="{self.w}" height="{self.h}" fill="{GROUND}"/>',
            f'  <rect width="{self.w}" height="{self.h}" fill="{ground}"/>',
        ]
        if self.grid:
            body.append(f'  <rect width="{self.w}" height="{self.h}" fill="url(#grid)"/>')
        body.extend(f"  {part}" for part in self.parts)
        body.append(f'  <rect width="{self.w}" height="{self.h}" fill="url(#scan)"/>')
        body.append(f'  <rect width="{self.w}" height="{self.h}" fill="url(#vignette)"/>')
        body.append("</svg>\n")
        return "\n".join(body)


def g(x: float, y: float, s: float = 1, rot: float = 0, flip: bool = False, inner: str = "", opacity: float = 1) -> str:
    sx = -s if flip else s
    op = f' opacity="{opacity}"' if opacity != 1 else ""
    return f'<g transform="translate({x} {y}) rotate({rot}) scale({sx} {s})"{op}>{inner}</g>'


def mono(x: float, y: float, text: str, size: float, fill: str, *, weight: int = 400, spacing: float = 2, anchor: str = "start", italic: bool = False, extra: str = "") -> str:
    style = ' font-style="italic"' if italic else ""
    return (f'<text x="{x}" y="{y}" font-family="{MONO}" font-size="{size}" font-weight="{weight}" letter-spacing="{spacing}" '
            f'text-anchor="{anchor}" fill="{fill}"{style}{extra}>{text}</text>')


def glow(on: bool = True) -> str:
    return ' filter="url(#glow)"' if on else ""


# ---------------------------------------------------------------- backdrop

def rings(w: int, cy: float, tilt: float = -8, seed: int = 2) -> str:
    """Saturn's rings crossing the whole screen as faint tilted bands."""
    rng = random.Random(seed)
    bands = []
    for i in range(7):
        dy = -110 + i * 36 + rng.uniform(-4, 4)
        width = rng.uniform(2, 12)
        op = rng.uniform(.1, .3)
        color = PALE if i % 3 else PHOS_DIM
        bands.append(f'<line x1="-400" y1="{dy:.0f}" x2="{w + 400}" y2="{dy:.0f}" stroke="{color}" stroke-width="{width:.1f}" opacity="{op:.2f}"/>')
    return f'<g transform="translate(0 {cy}) rotate({tilt} {w / 2} 0)">{"".join(bands)}</g>'


def ice(seed: int, x0: float, y0: float, x1: float, y1: float, n: int) -> str:
    rng = random.Random(seed)
    return "".join(
        f'<circle cx="{rng.uniform(x0, x1):.0f}" cy="{rng.uniform(y0, y1):.0f}" r="{rng.uniform(1, 3):.1f}" fill="{PALE}" opacity="{rng.uniform(.25, .7):.2f}"/>'
        for _ in range(n))


def rock(cx: float, cy: float, rx: float, ry: float, seed: int = 4) -> str:
    """A ring rock: a jagged body with a lit upper-left face and craters."""
    rng = random.Random(seed)
    pts = []
    for i in range(18):
        a = math.tau * i / 18
        k = rng.uniform(.82, 1.08)
        pts.append((cx + math.cos(a) * rx * k, cy + math.sin(a) * ry * k))
    outline = " ".join(f"{x:.0f},{y:.0f}" for x, y in pts)
    lit = " ".join(f"{x:.0f},{y:.0f}" for x, y in pts[8:15])
    craters = "".join(
        f'<ellipse cx="{cx + dx * rx:.0f}" cy="{cy + dy * ry:.0f}" rx="{rr * rx:.0f}" ry="{rr * ry * .8:.0f}" fill="none" stroke="{GRID}" stroke-opacity=".5" stroke-width="2"/>'
        for dx, dy, rr in ((-.3, .1, .1), (.2, .35, .14), (-.55, .45, .07), (.5, -.1, .08), (0, .6, .06)))
    return (f'<polygon points="{outline}" fill="url(#moon)" stroke="#0b2b1c" stroke-width="3"/>'
            f'<polyline points="{lit}" fill="none" stroke="{PHOS_DIM}" stroke-width="6" opacity=".7"{glow()}/>{craters}')


def ice_slab(x0: float, y0: float, x1: float, y1: float, seed: int = 6) -> str:
    """The ice at the claim: a facetted slab with a bright top edge."""
    rng = random.Random(seed)
    top = [(x0, y0 + rng.uniform(-30, 30))]
    x = x0
    while x < x1:
        x += rng.uniform(90, 200)
        top.append((min(x, x1), y0 + rng.uniform(-70, 40)))
    pts = top + [(x1, y1), (x0, y1)]
    outline = " ".join(f"{px:.0f},{py:.0f}" for px, py in pts)
    edge = " ".join(f"{px:.0f},{py:.0f}" for px, py in top)
    facets = "".join(
        f'<line x1="{px:.0f}" y1="{py:.0f}" x2="{px + rng.uniform(-80, 80):.0f}" y2="{y1:.0f}" stroke="{PALE_DIM}" stroke-opacity=".35" stroke-width="2"/>'
        for px, py in top[1:-1])
    return (f'<polygon points="{outline}" fill="#0e2a1c"/><polygon points="{outline}" fill="{PALE_DIM}" opacity=".18"/>'
            f'{facets}<polyline points="{edge}" fill="none" stroke="{PALE_BRIGHT}" stroke-width="4" opacity=".75"{glow()}/>')


def claim_line(x: float, y0: float, y1: float, left: str, right: str, y_text: float) -> str:
    """The survey line between two claims, dashed lime, labelled on both sides."""
    return (f'<line x1="{x}" y1="{y0}" x2="{x}" y2="{y1}" stroke="{LIME}" stroke-width="3" stroke-dasharray="18 14" opacity=".7"/>'
            + mono(x - 16, y_text, left, 18, LIME, anchor="end", spacing=4)
            + mono(x + 16, y_text, right, 18, KESTREL["hull"], spacing=4))


def junk(seed: int, x0: float, y0: float, x1: float, y1: float, n: int, pal: str = "ewi") -> str:
    rng = random.Random(seed)
    fills = {"ewi": ["#3a3f2c", "#4a4d33", "#5d603e", "#2b3527", "#6d6845"],
             "dark": ["#0f1a12", "#142219", "#1a2620"],
             "bone": ["#8c8b78", "#a5a391", "#c4c2ae", "#6d6c5c"]}[pal]
    edge = {"ewi": "#8f8656", "dark": "#1f3a2b", "bone": "#e6e4d2"}[pal]
    out = []
    for _ in range(n):
        x, y, rot = rng.uniform(x0, x1), rng.uniform(y0, y1), rng.uniform(0, 360)
        kind = rng.random()
        fill = rng.choice(fills)
        if kind < .4:
            length, t = rng.uniform(60, 260), rng.uniform(6, 16)
            shape = f'<rect x="{-length / 2:.0f}" y="{-t / 2:.0f}" width="{length:.0f}" height="{t:.0f}" fill="{fill}" stroke="{edge}" stroke-opacity=".5" stroke-width="1.5"/>'
        elif kind < .8:
            a, b = rng.uniform(40, 140), rng.uniform(30, 90)
            shape = f'<path d="M0,0 L{a:.0f},{rng.uniform(-10, 10):.0f} L{a * .9:.0f},{b:.0f} L{rng.uniform(-10, 10):.0f},{b * .8:.0f} Z" fill="{fill}" stroke="{edge}" stroke-opacity=".5" stroke-width="1.5"/>'
        else:
            shape = (f'<path d="M-90,-20 L-60,-34 L60,-34 L90,-10 L90,10 L60,34 L-60,34 L-90,20 Z" fill="{fill}" stroke="{edge}" stroke-opacity=".6" stroke-width="2"/>'
                     f'<path d="M-60,-34 H60 M-30,-34 V34 M20,-34 V34" stroke="{edge}" stroke-opacity=".3" stroke-width="1.5"/>')
        out.append(g(x, y, rng.uniform(.6, 1.3), rot, inner=shape))
    return "".join(out)


# ------------------------------------------------------------------- light

def bolt(x1: float, y1: float, x2: float, y2: float) -> str:
    return (f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{PHOS}" stroke-width="26" opacity=".25" filter="url(#soft)"/>'
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{PHOS}" stroke-width="7" opacity=".8"{glow()}/>'
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{PHOS_WHITE}" stroke-width="2.5"/>')


def burst(cx: float, cy: float, r: float, seed: int = 1, shard_fill: str = "#8f8656", shard_edge: str = "#3e4130") -> str:
    rng = random.Random(seed)
    spikes = "".join(
        f'<line x1="{cx}" y1="{cy}" x2="{cx + math.cos(a) * r * k:.0f}" y2="{cy + math.sin(a) * r * k:.0f}" stroke="{PHOS_BRIGHT}" stroke-width="{3 if i % 2 else 5}" opacity=".85"{glow()}/>'
        for i, (a, k) in enumerate((math.radians(i * 30 + rng.uniform(-8, 8)), rng.uniform(1.1, 2.2)) for i in range(12)))
    shards = "".join(
        g(cx + math.cos(a) * d, cy + math.sin(a) * d, 1, math.degrees(a),
          inner=f'<path d="M0,0 L{rng.uniform(14, 40):.0f},{rng.uniform(-6, 6):.0f} L{rng.uniform(6, 20):.0f},{rng.uniform(6, 14):.0f} Z" fill="{shard_fill}" stroke="{shard_edge}"/>')
        for a, d in ((rng.uniform(0, 6.28), rng.uniform(r * .8, r * 1.9)) for _ in range(14)))
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#flash)"/>{spikes}<circle cx="{cx}" cy="{cy}" r="{r * .28}" fill="{PHOS_WHITE}"{glow()}/>{shards}'


def flash_disc(cx: float, cy: float, r: float) -> str:
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#flash)"/>'


def ember(cx: float, cy: float, r: float) -> str:
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#ember)"/>'


def lamp_pool(cx: float, cy: float, r: float) -> str:
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#lamp)"/>'


# ------------------------------------------------------------------- ships

def meridian(pal: dict = EWI, lit: bool = True, part: str | None = None, text: bool = True, bay_open: bool = False, tag: str = "mc") -> str:
    """EWI carrier, 700 long, facing +x, origin at centre. part: None, 'bow' or 'stern'."""
    win = pal["win"] if lit else "#0e1f16"
    gl = glow(lit and pal["glow"] != "none")
    clip = ""
    if part == "stern":
        clip = f'<clipPath id="{tag}"><path d="M-400,-200 H-40 L-60,-120 L-30,-60 L-70,0 L-40,60 L-70,120 L-50,200 H-400 Z"/></clipPath>'
    elif part == "bow":
        clip = f'<clipPath id="{tag}"><path d="M400,-200 H-40 L-60,-120 L-30,-60 L-70,0 L-40,60 L-70,120 L-50,200 H400 Z"/></clipPath>'
    windows = "".join(f'<rect x="{x}" y="-22" width="16" height="8" fill="{win}"/>' for x in range(-280, 230, 36))
    windows += "".join(f'<rect x="{x}" y="12" width="16" height="8" fill="{win}"/>' for x in range(-262, 230, 36))
    bridge_win = "".join(f'<rect x="{x}" y="-104" width="10" height="6" fill="{win}"/>' for x in range(130, 210, 16))
    engines = "".join(
        f'<ellipse cx="-368" cy="{cy}" rx="10" ry="14" fill="#04120b" stroke="{pal["dark"]}" stroke-width="3"/>'
        f'<ellipse cx="-392" cy="{cy}" rx="22" ry="9" fill="{pal["glow"] if lit else "none"}" opacity=".45"{gl}/>'
        for cy in (-30, 0, 30))
    if bay_open:
        bay = (f'<rect x="-60" y="40" width="200" height="30" fill="#1a2014"/>'
               f'<rect x="-60" y="40" width="200" height="30" fill="url(#lamp)"/>'
               f'<rect x="-40" y="44" width="150" height="14" fill="{GOLD}"/>'
               + mono(35, 55, "WE ARE EXPANDING", 9, INK, weight=700, spacing=1.5, anchor="middle")
               + f'<rect x="60" y="58" width="60" height="10" fill="{pal["dark"]}"/>')
    else:
        bay = f'<rect x="-60" y="40" width="200" height="30" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>'
    label = ""
    if text and pal is not SIL:
        label = (mono(-230, 4, "MERIDIAN", 30, pal["edge"], weight=600, spacing=6)
                 + ("" if bay_open else mono(-40, 86, "WE ARE EXPANDING", 12, pal["band"], spacing=3))
                 + f'<rect x="300" y="-14" width="32" height="28" fill="{pal["band"]}"/>'
                 + mono(304, 6, "EWI", 16, INK, weight=700, spacing=0))
    return f"""{clip}<g{f' clip-path="url(#{tag})"' if part else ''}>
      <path d="M-330,-96 L-280,-104 L120,-104 L120,-70 L-300,-70 Z" fill="{pal["dark"]}"/>
      <path d="M-320,70 L-320,100 L240,100 L240,70 Z" fill="{pal["dark"]}"/>
      <path d="M-100,-200 L-60,-104 L-20,-104 L-40,-200 Z" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-100,200 L-60,100 L-20,100 L-40,200 Z" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-360,-40 L-300,-70 L250,-70 L350,-30 L350,30 L250,70 L-300,70 L-360,40 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-300,-70 L250,-70 L250,-56 L-300,-56 Z" fill="{pal["light"]}"/>
      <path d="M-300,52 L250,52 L250,70 L-300,70 Z" fill="{pal["dark"]}"/>
      <path d="M-240,-70 V70 M-120,-70 V70 M0,-70 V70 M120,-70 V70 M200,-70 V70" stroke="{pal["edge"]}" stroke-width="2" opacity=".6"/>
      {bay}
      <rect x="-60" y="70" width="200" height="6" fill="{pal["band"]}"/>
      <path d="M110,-130 L130,-150 L210,-150 L230,-130 L230,-104 L110,-104 Z" fill="{pal["light"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      {bridge_win}
      <g{gl}>{windows}</g>
      {engines}
      {label}
    </g>"""


def tender(pal: dict = EWI, label: str = "", lit: bool = True, flip: bool = False) -> str:
    """An EWI work tender, 150 long, facing +x: a boxy boat with a cargo cradle and a work arm.

    Pass flip=True when the caller mirrors the boat, so the name still reads left to right."""
    gl = glow(lit and pal["glow"] != "none")
    canopy = f'<rect x="28" y="-20" width="26" height="12" fill="{pal["win"]}" opacity=".9"{gl}/>' if lit else ""
    engine = f'<ellipse cx="-84" cy="0" rx="12" ry="6" fill="{pal["glow"]}" opacity=".6"{gl}/>' if lit else ""
    name = mono(-56, 16, label, 12, pal["edge"], spacing=2) if label and pal is not SIL else ""
    if flip and name:
        name = f'<g transform="scale(-1 1)">{mono(-20, 16, label, 12, pal["edge"], spacing=2)}</g>'
    return f"""
      <path d="M-75,-18 L-55,-32 L40,-32 L78,-8 L78,8 L40,32 L-55,32 L-75,18 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-55,-32 L40,-32 L40,-24 L-55,-24 Z" fill="{pal["light"]}"/>
      <rect x="-50" y="-44" width="70" height="14" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M18,-32 L28,-40 L58,-40 L70,-20 L68,-8 L18,-8 Z" fill="#04120b" stroke="{pal["light"]}" stroke-width="2"/>
      {canopy}
      <path d="M50,22 L92,40 M92,40 L104,32 M92,40 L100,50" stroke="{pal["dark"]}" stroke-width="5" fill="none" stroke-linecap="round"/>
      <ellipse cx="-78" cy="0" rx="6" ry="9" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>
      {engine}
      {name}"""


def harrier(lit: bool = True, flip: bool = False) -> str:
    """Kestrel's supply boat, 170 long, bone white, facing +x."""
    pal = KESTREL
    gl = glow(lit)
    windows = "".join(f'<rect x="{x}" y="-14" width="10" height="7" fill="{pal["win"]}"{gl}/>' for x in range(-50, 40, 22)) if lit else ""
    engines = "".join(
        f'<ellipse cx="-86" cy="{cy}" rx="6" ry="8" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>'
        + (f'<ellipse cx="-100" cy="{cy}" rx="16" ry="6" fill="{GOLD}" opacity=".7"{gl}/>' if lit else "")
        for cy in (-14, 14))
    return f"""
      <path d="M-84,-26 L-60,-38 L50,-38 L86,-10 L86,10 L50,38 L-60,38 L-84,26 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-60,-38 L50,-38 L50,-28 L-60,-28 Z" fill="{pal["light"]}"/>
      <path d="M-60,26 L50,26 L50,38 L-60,38 Z" fill="{pal["dark"]}"/>
      <path d="M44,-30 L58,-30 L76,-8 L76,4 L44,4 Z" fill="#04120b" stroke="{pal["light"]}" stroke-width="2"/>
      <path d="M-30,-38 V38 M20,-38 V38" stroke="{pal["edge"]}" stroke-width="2" opacity=".6"/>
      {windows}
      {engines}
      {f'<g transform="scale(-1 1)">{mono(-20, 24, "HARRIER", 11, pal["edge"], weight=700, spacing=2)}</g>' if flip else mono(-56, 24, "HARRIER", 11, pal["edge"], weight=700, spacing=2)}"""


# ----------------------------------------------------------------- station

def station(pal: dict = KESTREL) -> str:
    """The Shelter: Kestrel's largest station, standing on its rock. Origin at the rock's top centre, 1100 wide, 720 tall."""
    gl = glow(pal["glow"] != "none")
    win = pal["win"]
    tower_win = "".join(f'<rect x="{x}" y="{y}" width="14" height="8" fill="{win}"/>' for y in range(-470, -130, 42) for x in (-64, -34, -4, 26, 50))
    ring_win = "".join(
        f'<rect x="{math.cos(a) * 380 - 6:.0f}" y="{-300 + math.sin(a) * 70 - 3:.0f}" width="12" height="6" fill="{win}"/>'
        for a in (math.radians(d) for d in range(15, 166, 10)))
    module_win = "".join(f'<rect x="{x}" y="-205" width="12" height="8" fill="{win}"/>' for x in list(range(-440, -280, 30)) + list(range(280, 440, 30)))
    return f"""
      <rect x="-300" y="-60" width="600" height="70" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-330,-360 L-250,-240 M330,-360 L250,-240 M-250,-140 L-110,-200 M250,-140 L110,-200" stroke="{pal["dark"]}" stroke-width="12"/>
      <rect x="-470" y="-250" width="220" height="110" rx="44" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="250" y="-250" width="220" height="110" rx="44" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="-470" y="-250" width="220" height="24" rx="12" fill="{pal["light"]}" opacity=".8"/>
      <rect x="250" y="-250" width="220" height="24" rx="12" fill="{pal["light"]}" opacity=".8"/>
      <circle cx="-330" cy="-70" r="58" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <circle cx="330" cy="-70" r="58" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="250" y="-420" width="330" height="18" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <rect x="560" y="-436" width="30" height="50" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <circle cx="575" cy="-446" r="8" fill="{win}"{gl}/>
      <ellipse cx="0" cy="-300" rx="380" ry="70" fill="none" stroke="{pal["hull"]}" stroke-width="46"/>
      <ellipse cx="0" cy="-300" rx="380" ry="70" fill="none" stroke="{pal["edge"]}" stroke-width="2"/>
      <ellipse cx="0" cy="-300" rx="336" ry="50" fill="none" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-380,-300 A380,70 0 0 0 380,-300" fill="none" stroke="{pal["light"]}" stroke-width="10" opacity=".6"/>
      <rect x="-110" y="-520" width="220" height="470" rx="26" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="-110" y="-520" width="34" height="470" rx="18" fill="{pal["light"]}" opacity=".85"/>
      <rect x="76" y="-520" width="34" height="470" rx="18" fill="{pal["dark"]}" opacity=".8"/>
      <path d="M-110,-520 A110,110 0 0 1 110,-520 Z" fill="{pal["light"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="-6" y="-700" width="12" height="80" fill="{pal["dark"]}"/>
      <path d="M0,-700 L-60,-640 M0,-700 L60,-640" stroke="{pal["dark"]}" stroke-width="5"/>
      <circle cx="0" cy="-708" r="9" fill="{win}"{gl}/>
      <g{gl}>{tower_win}{ring_win}{module_win}</g>
      {mono(-380, -170, "SHELTER", 26, pal["edge"], weight=700, spacing=6)}
      {mono(300, -170, "KESTREL", 18, pal["edge"], spacing=5)}"""


def station_wreck() -> str:
    """The Shelter opened: the tower broken, the ring in two arcs, the modules torn. Same origin as station()."""
    pal = KESTREL_DEAD
    upper = f"""
      <rect x="-110" y="-520" width="220" height="230" rx="26" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="-110" y="-520" width="34" height="230" rx="18" fill="{pal["light"]}" opacity=".85"/>
      <path d="M-110,-290 L-70,-330 L-30,-280 L20,-340 L60,-290 L110,-320 L110,-290 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-110,-520 A110,110 0 0 1 110,-520 Z" fill="{pal["light"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="-6" y="-700" width="12" height="80" fill="{pal["dark"]}"/>"""
    return f"""
      <rect x="-300" y="-60" width="600" height="70" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-330,-360 L-250,-240 M-250,-140 L-110,-200" stroke="{pal["dark"]}" stroke-width="12"/>
      <rect x="-470" y="-250" width="220" height="110" rx="44" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M250,-250 L400,-250 L440,-180 L380,-140 L250,-140 Z" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <circle cx="-330" cy="-70" r="58" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-380,-300 A380,70 0 0 0 -60,-236" fill="none" stroke="{pal["hull"]}" stroke-width="46"/>
      <path d="M120,-232 A380,70 0 0 0 380,-300" fill="none" stroke="{pal["hull"]}" stroke-width="46"/>
      <path d="M-110,-50 L-110,-300 L-70,-260 L-20,-330 L40,-270 L110,-310 L110,-50 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <rect x="76" y="-300" width="34" height="250" rx="18" fill="{pal["dark"]}" opacity=".8"/>
      {ember(0, -300, 170)}{ember(-140, -230, 90)}{ember(330, -230, 110)}
      {g(60, -180, 1, 26, inner=upper)}
      {junk(61, -520, -600, 560, -80, 14, "bone")}"""


# ---------------------------------------------------------------- hardware

def charge(marker: bool = True) -> str:
    """A Fleet-surplus demolition charge on its clamps, 200 long, origin at centre."""
    stencil = mono(-56, -20, "EF DEMO-12", 13, INK, weight=700, spacing=2) + mono(-56, 32, "SURPLUS // FLEET", 9, INK, spacing=2)
    note = mono(-38, 5, "FOR THE ICE", 12, LIME, weight=700, spacing=1, italic=True) if marker else ""
    return f"""
      <path d="M-70,52 L-90,80 M70,52 L90,80 M-90,80 L-110,80 M90,80 L110,80" stroke="{SUIT_EDGE}" stroke-width="8" stroke-linecap="round"/>
      <rect x="-100" y="-36" width="200" height="72" rx="30" fill="#7f7a50" stroke="{SUIT_EDGE}" stroke-width="3"/>
      <rect x="-100" y="-36" width="200" height="14" rx="7" fill="#a39c6a" opacity=".8"/>
      <rect x="-88" y="-36" width="16" height="72" fill="{GOLD}"/><rect x="-72" y="-36" width="8" height="72" fill="{INK}"/>
      <rect x="72" y="-36" width="16" height="72" fill="{GOLD}"/><rect x="64" y="-36" width="8" height="72" fill="{INK}"/>
      <rect x="-40" y="-14" width="80" height="24" fill="{INK}" opacity=".85"/>
      <circle cx="84" cy="-48" r="7" fill="{GOLD}"{glow()}/>
      {stencil}
      {note}"""


def eva_crew(tether_to: tuple[float, float] | None = None) -> str:
    """A suited crew member working, side view, 200 tall, facing +x. Origin at the hips."""
    tether = f'<path d="M-40,-30 Q{tether_to[0] / 2:.0f},{tether_to[1] / 2 - 60:.0f} {tether_to[0]},{tether_to[1]}" fill="none" stroke="{PALE_DIM}" stroke-width="2" opacity=".7"/>' if tether_to else ""
    return f"""
      {tether}
      <rect x="-52" y="-92" width="24" height="76" rx="6" fill="{SUIT_DARK}" stroke="{SUIT_EDGE}" stroke-width="2"/>
      <path d="M-14,40 L-24,118 M14,40 L44,110" stroke="{SUIT_DARK}" stroke-width="20" stroke-linecap="round"/>
      <path d="M-30,118 L-8,124 M36,112 L60,108" stroke="{SUIT_EDGE}" stroke-width="12" stroke-linecap="round"/>
      <rect x="-30" y="-100" width="62" height="140" rx="16" fill="{SUIT}" stroke="{SUIT_EDGE}" stroke-width="3"/>
      <rect x="-12" y="-70" width="10" height="14" fill="{PHOS}"{glow()}/>
      <path d="M22,-72 L64,-30 L84,-40 M20,-56 L70,-14 L92,-20" stroke="{SUIT}" stroke-width="16" stroke-linecap="round" fill="none"/>
      <circle cx="0" cy="-138" r="40" fill="#8f8656" stroke="{SUIT_DARK}" stroke-width="4"/>
      <path d="M8,-176 A40,40 0 0 1 26,-108 L8,-108 Z" fill="{PHOS}" opacity=".85"{glow()}/>
      <path d="M-40,-138 A40,40 0 0 1 0,-178" fill="none" stroke="#b0aa75" stroke-width="6" opacity=".8"/>"""


def helmet() -> str:
    """A crew member from behind, helmet and shoulders, 340 wide. Origin between the shoulders."""
    return f"""
      <path d="M-170,170 L-130,60 L130,60 L170,170 Z" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="3"/>
      <rect x="-50" y="40" width="100" height="40" fill="{SUIT_DARK}"/>
      <circle cx="0" cy="-30" r="112" fill="#8f8656" stroke="{SUIT_DARK}" stroke-width="4"/>
      <path d="M-112,-30 A112,112 0 0 1 0,-142" fill="none" stroke="#b0aa75" stroke-width="10" opacity=".8"/>
      <path d="M-116,-40 A118,118 0 0 1 -40,-140" fill="none" stroke="{PHOS}" stroke-width="14" opacity=".9"{glow()}/>
      <rect x="-118" y="-60" width="20" height="60" rx="6" fill="{PHOS}" opacity=".9"/>
      <rect x="98" y="-60" width="20" height="60" rx="6" fill="{PHOS}" opacity=".9"/>
      <path d="M-70,20 L70,20 L60,70 L-60,70 Z" fill="{SUIT}" stroke="{SUIT_EDGE}" stroke-width="2"/>"""


def deck_hand(helmet_on: bool = False, tool: bool = False) -> str:
    """A crew member standing, seen from behind, 260 tall. Origin at the feet."""
    head = (f'<circle cx="0" cy="-232" r="30" fill="#8f8656" stroke="{SUIT_DARK}" stroke-width="3"/>' if helmet_on
            else f'<circle cx="0" cy="-230" r="24" fill="#5a4c34"/><path d="M-24,-236 A24,24 0 0 1 24,-236 L24,-226 L-24,-226 Z" fill="#2e2c22"/>')
    held = f'<path d="M40,-150 L96,-120" stroke="{GEAR}" stroke-width="10" stroke-linecap="round"/><rect x="90" y="-134" width="34" height="22" rx="4" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="2"/>' if tool else ""
    return f"""
      <path d="M-22,-110 L-26,-8 M22,-110 L28,-8" stroke="{SUIT_DARK}" stroke-width="26" stroke-linecap="round"/>
      <rect x="-40" y="-14" width="30" height="14" rx="4" fill="{SUIT_EDGE}"/><rect x="12" y="-14" width="32" height="14" rx="4" fill="{SUIT_EDGE}"/>
      <path d="M-56,-200 L-56,-100 L56,-100 L56,-200 Z" fill="{SUIT}" stroke="{SUIT_EDGE}" stroke-width="3"/>
      <path d="M-56,-196 L-84,-120 M56,-196 L84,-120" stroke="{SUIT}" stroke-width="22" stroke-linecap="round"/>
      <rect x="-40" y="-190" width="80" height="12" fill="{SUIT_DARK}"/>
      {held}
      {head}"""


# -------------------------------------------------------------------- faces

def face(*, skin: str, shadow: str, light: str, hair: str, style: str, headset: str, uniform: str, uniform_dark: str,
         collar: str, jaw: str = "narrow", age: float = .4, stubble: bool = False, heavy_brow: bool = False,
         mouth: str = "flat", helmet_ring: bool = False, glow_below: bool = True) -> str:
    """A face lit by a screen, 720 tall from hair to shoulders. Origin at the centre of the head."""
    chin = 110 if jaw == "wide" else 80
    head = (f"M-160,-40 C-160,-230 160,-230 160,-40 C160,60 {135 if jaw == 'wide' else 130},150 {chin},205 "
            f"C{chin * .5:.0f},240 {-chin * .5:.0f},240 {-chin},205 C{-135 if jaw == 'wide' else -130},150 -160,60 -160,-40 Z")
    lit = f'<circle cx="0" cy="360" r="380" fill="url(#screenlight)"/>' if glow_below else ""
    shell = ""
    if helmet_ring:
        shell = (f'<path d="M-250,-60 A250,250 0 0 1 250,-60 L250,40 L-250,40 Z" fill="#8f8656" stroke="{SUIT_DARK}" stroke-width="5"/>'
                 f'<path d="M-236,-70 A236,236 0 0 1 236,-70" fill="none" stroke="{PHOS}" stroke-width="16" opacity=".85"{glow()}/>'
                 f'<rect x="-200" y="272" width="400" height="54" rx="24" fill="#8f8656" stroke="{SUIT_DARK}" stroke-width="4"/>')
    collar_marks = ""
    if collar == "officer":
        collar_marks = (f'<path d="M-120,290 L-70,300 L-40,420 M120,290 L70,300 L40,420" stroke="{GOLD}" stroke-width="6" fill="none"/>'
                        f'<rect x="-330" y="330" width="70" height="16" fill="{GOLD}"/>')
    elif collar == "deck":
        collar_marks = (f'<path d="M-70,300 L-40,420 M70,300 L40,420" stroke="{uniform_dark}" stroke-width="8" fill="none"/>'
                        f'<rect x="-300" y="360" width="130" height="34" rx="6" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="2"/>'
                        f'<rect x="-280" y="372" width="60" height="8" fill="{PHOS}"/>')
    hairpath = ""
    if style == "neat":
        hairpath = (f'<path d="M-165,-60 C-170,-250 170,-250 165,-60 C140,-100 100,-126 40,-130 C-20,-134 -110,-116 -165,-60 Z" fill="{hair}"/>'
                    f'<path d="M-165,-60 C-150,-110 -120,-140 -80,-150 C-120,-120 -150,-90 -165,-60 Z" fill="#9a9c86" opacity=".8"/>'
                    f'<path d="M165,-60 C150,-110 120,-140 80,-150 C120,-120 150,-90 165,-60 Z" fill="#9a9c86" opacity=".8"/>')
    elif style == "cropped":
        hairpath = f'<path d="M-160,-90 C-165,-235 165,-235 160,-90 C120,-150 60,-176 0,-178 C-60,-176 -120,-150 -160,-90 Z" fill="{hair}"/>'
    beard = f'<path d="M-140,40 C-130,150 -70,225 0,232 C70,225 130,150 140,40 C120,120 60,170 0,175 C-60,170 -120,120 -140,40 Z" fill="{hair}" opacity=".3"/>' if stubble else ""
    brow_w = 14 if heavy_brow else 9
    brow_y = -62 if heavy_brow else -74
    lines = ""
    if age > .3:
        lines += f'<path d="M-72,84 Q-64,120 -52,146 M72,84 Q64,120 52,146" stroke="{shadow}" stroke-width="4" opacity="{age:.2f}" fill="none"/>'
    if age > .6:
        lines += (f'<path d="M-90,-150 L90,-150 M-70,-128 L70,-128" stroke="{shadow}" stroke-width="3" opacity=".45" fill="none"/>'
                  f'<path d="M-140,-20 Q-125,-5 -115,20 M140,-20 Q125,-5 115,20" stroke="{shadow}" stroke-width="3" opacity=".5" fill="none"/>')
    mouthpath = ('<path d="M-52,138 L52,138" ' if mouth == "set" else '<path d="M-52,134 Q0,142 52,136" ') + f'stroke="{shadow}" stroke-width="6" fill="none" stroke-linecap="round"/>'
    if headset == "slim":
        gear = (f'<rect x="150" y="-46" width="24" height="76" rx="8" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="3"/>'
                f'<rect x="156" y="-36" width="12" height="8" fill="{PHOS}"{glow()}/>'
                f'<path d="M162,26 Q140,120 60,146" fill="none" stroke="{GEAR}" stroke-width="7"/>'
                f'<circle cx="58" cy="148" r="10" fill="{DEEP}" stroke="{PHOS}" stroke-width="3"/>')
    else:
        gear = (f'<path d="M-168,-30 C-168,-262 168,-262 168,-30" fill="none" stroke="{GEAR}" stroke-width="24"/>'
                f'<circle cx="-170" cy="-30" r="46" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="6"/>'
                f'<circle cx="170" cy="-30" r="46" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="6"/>'
                f'<rect x="-186" y="-8" width="32" height="10" fill="{GOLD}"{glow()}/>'
                f'<path d="M-170,16 Q-130,130 -40,152" fill="none" stroke="{GEAR}" stroke-width="8"/>'
                f'<circle cx="-38" cy="154" r="11" fill="{DEEP}" stroke="{PHOS}" stroke-width="3"/>')
    return f"""
      {lit}
      {shell}
      <path d="M-370,480 L-300,330 L-120,290 L-70,300 L70,300 L120,290 L300,330 L370,480 Z" fill="{uniform}" stroke="{uniform_dark}" stroke-width="4"/>
      <path d="M-300,330 L-120,290 L-70,300 L-40,480 L-370,480 Z" fill="{uniform_dark}" opacity=".35"/>
      {collar_marks}
      <rect x="-72" y="170" width="144" height="150" fill="{shadow}"/>
      <path d="{head}" fill="{skin}" stroke="{shadow}" stroke-width="3"/>
      <path d="M60,-215 C170,-180 165,80 90,200 C120,80 130,-100 60,-215 Z" fill="{shadow}" opacity=".55"/>
      <path d="M-150,40 C-140,140 -80,210 -30,225 C-100,190 -130,110 -150,40 Z" fill="{light}" opacity=".55"/>
      {beard}
      <path d="M0,-35 C-20,10 -30,50 -32,70 Q0,88 32,70 C30,50 20,10 8,-35 Z" fill="{shadow}" opacity=".35"/>
      <path d="M-10,-30 C-24,10 -30,50 -34,70" stroke="{light}" stroke-width="4" fill="none" opacity=".7"/>
      <path d="M-105,-45 Q-70,-70 -30,-45 Q-70,-24 -105,-45 Z" fill="#0a1a10"/>
      <path d="M105,-45 Q70,-70 30,-45 Q70,-24 105,-45 Z" fill="#0a1a10"/>
      <circle cx="-66" cy="-46" r="13" fill="{DEEP}"/><circle cx="66" cy="-46" r="13" fill="{DEEP}"/>
      <rect x="-74" y="-52" width="7" height="5" fill="{PHOS_BRIGHT}"/><rect x="58" y="-52" width="7" height="5" fill="{PHOS_BRIGHT}"/>
      <path d="M-122,{brow_y} Q-66,{brow_y - 24} -14,{brow_y - 2} M122,{brow_y} Q66,{brow_y - 24} 14,{brow_y - 2}" stroke="{hair}" stroke-width="{brow_w}" fill="none" stroke-linecap="round"/>
      {lines}
      {mouthpath}
      {hairpath}
      {gear}"""


# -------------------------------------------------------------- interiors

def bay(w: int, h: int, seed: int = 9) -> str:
    """Meridian's boat bay: back wall, ceiling lamps, deck plates converging to the vanishing point."""
    horizon = h * .58
    vp = (w * .5, horizon)
    floor_lines = "".join(
        f'<line x1="{vp[0]:.0f}" y1="{vp[1]:.0f}" x2="{x}" y2="{h}" stroke="{GRID}" stroke-opacity=".5" stroke-width="2"/>'
        for x in range(-600, w + 601, 240))
    depth = "".join(
        f'<line x1="0" y1="{y:.0f}" x2="{w}" y2="{y:.0f}" stroke="{GRID}" stroke-opacity=".35" stroke-width="2"/>'
        for y in (horizon + (h - horizon) * k for k in (.08, .2, .38, .62, .9)))
    panels = "".join(f'<line x1="{x}" y1="0" x2="{x}" y2="{horizon:.0f}" stroke="{GRID}" stroke-width="3" stroke-opacity=".8"/>' for x in range(0, w + 1, 190))
    lamps = "".join(
        f'<rect x="{x}" y="{h * .04:.0f}" width="70" height="8" fill="{GOLD}"{glow()}/>' + lamp_pool(x + 35, h * .06, 220)
        for x in range(120, w - 100, 380))
    return (f'<rect width="{w}" height="{horizon:.0f}" fill="#0a1710"/>{panels}'
            f'<rect y="{horizon:.0f}" width="{w}" height="{h - horizon:.0f}" fill="#0d1a12"/>{floor_lines}{depth}'
            f'<rect y="{horizon - 10:.0f}" width="{w}" height="14" fill="{SUIT_EDGE}"/>'
            f'<rect y="{h * .03:.0f}" width="{w}" height="26" fill="{GEAR}" stroke="{GEAR_EDGE}" stroke-width="2"/>{lamps}')


def banner(x: float, y: float, w: float, h: float) -> str:
    """The WE ARE EXPANDING banner, gold on the bay wall, with the EWI tag."""
    size = min(h * .42, (w - h * .9) / 11.2)
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{GOLD}" stroke="{GOLD_DEEP}" stroke-width="6"/>'
            f'<rect x="{x + 14}" y="{y + 14}" width="{w - 28}" height="{h - 28}" fill="none" stroke="{INK}" stroke-width="3" opacity=".6"/>'
            f'<rect x="{x + h * .22}" y="{y + h * .3}" width="{h * .4}" height="{h * .4}" fill="{INK}"/>'
            + mono(x + h * .42, y + h * .585, "EWI", h * .2, GOLD, weight=700, spacing=0, anchor="middle")
            + mono(x + h * .8, y + h * .5 + size * .36, "WE ARE EXPANDING", size, INK, weight=700, spacing=size * .1))


def plaque(x: float, y: float, w: float, h: float, lines: list[tuple[str, float, int]]) -> str:
    """A metal plate bolted to the wall. lines: (text, size, weight) from the top."""
    body = (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="#8f8656" stroke="{SUIT_EDGE}" stroke-width="5"/>'
            f'<rect x="{x + 10}" y="{y + 10}" width="{w - 20}" height="{h - 20}" rx="3" fill="none" stroke="{SUIT_EDGE}" stroke-width="2" opacity=".7"/>'
            + "".join(f'<circle cx="{cx}" cy="{cy}" r="7" fill="{SUIT_EDGE}"/>' for cx in (x + 22, x + w - 22) for cy in (y + 22, y + h - 22)))
    ty = y + 30
    for text, size, weight in lines:
        ty += size * 1.15
        body += mono(x + w / 2, ty, text, size, INK, weight=weight, spacing=size * .18, anchor="middle")
        ty += size * .45
    return body


# ---------------------------------------------------------------- consoles

def console(x: float, y: float, w: float, h: float, title: str) -> str:
    """A dark plot screen with its grid and title strip. Draw plot content on top."""
    lines = "".join(f'<line x1="{x}" y1="{yy}" x2="{x + w}" y2="{yy}" stroke="{GRID}" stroke-opacity=".5"/>' for yy in range(int(y) + 60, int(y + h), 60))
    lines += "".join(f'<line x1="{xx}" y1="{y}" x2="{xx}" y2="{y + h}" stroke="{GRID}" stroke-opacity=".5"/>' for xx in range(int(x) + 60, int(x + w), 60))
    return (f'<rect x="{x - 26}" y="{y - 26}" width="{w + 52}" height="{h + 52}" rx="14" fill="#3a3f2c" stroke="#8f8656" stroke-width="5"/>'
            f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#04120b" stroke="{PHOS_DIM}" stroke-width="3"/>{lines}'
            f'<rect x="{x}" y="{y}" width="{w}" height="34" fill="{DEEP}" opacity=".9"/>'
            + mono(x + 16, y + 24, title, 19, PHOS_BRIGHT, spacing=3)
            + "".join(f'<rect x="{x + w - 30 - i * 24}" y="{y + 12}" width="14" height="10" fill="{c}"/>' for i, c in enumerate((PHOS, LIME, PHOS, GOLD))))


def mark(x: float, y: float, kind: str, label: str, color: str = PHOS, sub: str = "") -> str:
    """A plot symbol with its label: 'ship', 'station', 'charge' or 'lost'."""
    if kind == "ship":
        sym = f'<path d="M-16,-8 L16,0 L-16,8 L-8,0 Z" fill="{color}"{glow()}/>'
    elif kind == "station":
        sym = f'<circle cx="0" cy="0" r="16" fill="none" stroke="{color}" stroke-width="3"{glow()}/><path d="M-22,0 H22 M0,-22 V22" stroke="{color}" stroke-width="3"/>'
    elif kind == "charge":
        sym = f'<path d="M0,-18 L18,0 L0,18 L-18,0 Z" fill="{color}"{glow()}/>'
    else:
        sym = f'<path d="M-14,-14 L14,14 M-14,14 L14,-14" stroke="{color}" stroke-width="5"{glow()}/>'
    text = mono(28, 6, label, 19, color, spacing=2) + (mono(28, 30, sub, 14, PHOS_TEXT, spacing=2) if sub else "")
    return g(x, y, inner=sym + text)


def plot_text(x: float, y: float, rows: list[tuple[str, str]], size: float = 19, gap: float = 34) -> str:
    """Console log rows, top down: (text, colour)."""
    return "".join(mono(x, y + i * gap, text, size, color, spacing=2) for i, (text, color) in enumerate(rows))
