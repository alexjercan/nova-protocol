#!/usr/bin/env python3
"""HD comic panel candidates in the campaign CRT style.

1920 x 1080 vector panels. The look comes from the campaign portraits in
art/portrait-candidates: dark green ground, phosphor bezel, faint scanlines,
muted khaki EWI hulls, amber for the Kites, phosphor for light. Run from the
repository root:

    python3 tasks/20260905-094102/art/hd_panels.py

It writes the-strike-*.svg next to itself.
"""

from __future__ import annotations

import math
import pathlib
import random

W, H = 1920, 1080
INSET = 48
MONO = "IBM Plex Mono, DejaVu Sans Mono, Menlo, monospace"

EWI = dict(hull="#8f8656", dark="#5d603e", light="#b0aa75", edge="#3e4130", win="#4ed184", glow="#4ed184", band="#c2a23b")
SIL = dict(hull="#0a1711", dark="#08130d", light="#0d1d14", edge="#1f3a2b", win="#0a1711", glow="none", band="#0f1a12")
KITE = dict(hull="#1a2620", dark="#111a15", light="#243428", edge="#344832", win="#9d8128", glow="#c2a23b", band="#9d8128")

_uid = 0


def uid(prefix: str) -> str:
    global _uid
    _uid += 1
    return f"{prefix}{_uid}"


def defs(oy: int = 0) -> str:
    return f"""
  <defs>
    <radialGradient id="screen{oy}" cx="50%" cy="45%" r="75%">
      <stop offset="0" stop-color="#07190f"/><stop offset="1" stop-color="#030d08"/>
    </radialGradient>
    <radialGradient id="vignette{oy}" cx="50%" cy="50%" r="70%">
      <stop offset=".55" stop-color="#030d08" stop-opacity="0"/><stop offset="1" stop-color="#030d08" stop-opacity=".7"/>
    </radialGradient>
    <pattern id="scan{oy}" width="4" height="4" patternUnits="userSpaceOnUse">
      <rect width="4" height="1" fill="#001008" opacity=".5"/>
    </pattern>
    <pattern id="grid{oy}" width="120" height="120" patternUnits="userSpaceOnUse">
      <path d="M120 0H0V120" fill="none" stroke="#164c31" stroke-opacity=".28"/>
    </pattern>
    <radialGradient id="moon{oy}" cx="62%" cy="40%" r="70%">
      <stop offset="0" stop-color="#132c1e"/><stop offset=".6" stop-color="#08190f"/><stop offset="1" stop-color="#030c07"/>
    </radialGradient>
    <radialGradient id="flash{oy}">
      <stop offset="0" stop-color="#e8fff0"/><stop offset=".18" stop-color="#c0ffd4"/><stop offset=".45" stop-color="#4ed184" stop-opacity=".85"/><stop offset="1" stop-color="#4ed184" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="ember{oy}">
      <stop offset="0" stop-color="#ffd67a"/><stop offset=".4" stop-color="#c2a23b" stop-opacity=".8"/><stop offset="1" stop-color="#c2a23b" stop-opacity="0"/>
    </radialGradient>
    <filter id="glow{oy}" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="6" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
    <filter id="soft{oy}" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="14"/>
    </filter>
    <clipPath id="clip{oy}"><rect x="{INSET}" y="{oy + INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" rx="8"/></clipPath>
  </defs>"""


def frame_open(oy: int, grid: bool) -> str:
    out = [
        f'  <rect y="{oy}" width="{W}" height="{H}" fill="#030d08"/>',
        f'  <rect x="24" y="{oy + 24}" width="{W - 48}" height="{H - 48}" rx="18" fill="#082117" stroke="#55d68b" stroke-width="6"/>',
        f'  <rect x="{INSET}" y="{oy + INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" rx="8" fill="url(#screen{oy})"/>',
    ]
    if grid:
        out.append(f'  <rect x="{INSET}" y="{oy + INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" rx="8" fill="url(#grid{oy})"/>')
    out.append(f'  <g clip-path="url(#clip{oy})">')
    return "\n".join(out)


def frame_close(oy: int) -> str:
    return "\n".join([
        "  </g>",
        f'  <rect x="{INSET}" y="{oy + INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" rx="8" fill="url(#scan{oy})"/>',
        f'  <rect x="{INSET}" y="{oy + INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" rx="8" fill="url(#vignette{oy})"/>',
        f'  <rect x="{INSET}" y="{oy + INSET}" width="160" height="10" fill="#b8da5d"/>',
        f'  <rect x="{W - INSET - 160}" y="{oy + H - INSET - 10}" width="160" height="10" fill="#55d68b"/>',
    ])


def svg(h: int, body: str) -> str:
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{h}" viewBox="0 0 {W} {h}">\n'
            f"{body}\n</svg>\n")


def g(x: float, y: float, s: float = 1, rot: float = 0, flip: bool = False, inner: str = "") -> str:
    sx = -s if flip else s
    return f'<g transform="translate({x} {y}) rotate({rot}) scale({sx} {s})">{inner}</g>'


# ------------------------------------------------------------------ ships

def meridian(pal: dict, oy: int, lit: bool = True, part: str | None = None, text: bool = True) -> str:
    """EWI carrier, 700 long, facing +x, origin at centre. part: None, 'bow' or 'stern'."""
    win = pal["win"] if lit else "#0e1f16"
    glow = f' filter="url(#glow{oy})"' if lit and pal["glow"] != "none" else ""
    cid = uid("mc")
    clip = ""
    if part == "stern":
        clip = f'<clipPath id="{cid}"><path d="M-400,-200 H-40 L-60,-120 L-30,-60 L-70,0 L-40,60 L-70,120 L-50,200 H-400 Z"/></clipPath>'
    elif part == "bow":
        clip = f'<clipPath id="{cid}"><path d="M400,-200 H-40 L-60,-120 L-30,-60 L-70,0 L-40,60 L-70,120 L-50,200 H400 Z"/></clipPath>'
    windows = "".join(f'<rect x="{x}" y="-22" width="16" height="8" fill="{win}"/>' for x in range(-280, 230, 36))
    windows += "".join(f'<rect x="{x}" y="12" width="16" height="8" fill="{win}"/>' for x in range(-262, 230, 36))
    bridge_win = "".join(f'<rect x="{x}" y="-104" width="10" height="6" fill="{win}"/>' for x in range(130, 210, 16))
    engines = "".join(
        f'<ellipse cx="-368" cy="{cy}" rx="10" ry="14" fill="#04120b" stroke="{pal["dark"]}" stroke-width="3"/>'
        f'<ellipse cx="-392" cy="{cy}" rx="22" ry="9" fill="{pal["glow"] if lit else "none"}" opacity=".45"{glow}/>'
        for cy in (-30, 0, 30))
    label = ""
    if text and pal is not SIL:
        label = (f'<text x="-230" y="4" font-family="{MONO}" font-size="30" font-weight="600" letter-spacing="6" fill="{pal["edge"]}">MERIDIAN</text>'
                 f'<text x="-40" y="86" font-family="{MONO}" font-size="12" letter-spacing="3" fill="{pal["band"]}">WE ARE EXPANDING</text>'
                 f'<rect x="300" y="-14" width="32" height="28" fill="{pal["band"]}"/>'
                 f'<text x="304" y="6" font-family="{MONO}" font-size="16" font-weight="700" fill="#06170f">EWI</text>')
    body = f"""{clip}<g{f' clip-path="url(#{cid})"' if part else ''}>
      <path d="M-330,-96 L-280,-104 L120,-104 L120,-70 L-300,-70 Z" fill="{pal["dark"]}"/>
      <path d="M-320,70 L-320,100 L240,100 L240,70 Z" fill="{pal["dark"]}"/>
      <path d="M-100,-200 L-60,-104 L-20,-104 L-40,-200 Z" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-100,200 L-60,100 L-20,100 L-40,200 Z" fill="{pal["dark"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-360,-40 L-300,-70 L250,-70 L350,-30 L350,30 L250,70 L-300,70 L-360,40 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-300,-70 L250,-70 L250,-56 L-300,-56 Z" fill="{pal["light"]}"/>
      <path d="M-300,52 L250,52 L250,70 L-300,70 Z" fill="{pal["dark"]}"/>
      <path d="M-240,-70 V70 M-120,-70 V70 M0,-70 V70 M120,-70 V70 M200,-70 V70" stroke="{pal["edge"]}" stroke-width="2" opacity=".6"/>
      <rect x="-60" y="40" width="200" height="30" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>
      <rect x="-60" y="70" width="200" height="6" fill="{pal["band"]}"/>
      <path d="M110,-130 L130,-150 L210,-150 L230,-130 L230,-104 L110,-104 Z" fill="{pal["light"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      {bridge_win}
      <g{glow}>{windows}</g>
      {engines}
      {label}
    </g>"""
    return body


def severance(oy: int, firing: bool, pal: dict = KITE, flip: bool = False) -> str:
    """Stolen Fleet warship, 520 long, facing +x. The gun muzzle is at (300, 0)."""
    glow = f' filter="url(#glow{oy})"'
    lights = "".join(f'<rect x="{x}" y="{y}" width="10" height="6" fill="{pal["win"]}"/>' for x, y in ((-236, -12), (-236, 8), (90, -38), (-40, 32)))
    muzzle = (f'<circle cx="300" cy="0" r="16" fill="#e8fff0"{glow}/><circle cx="300" cy="0" r="34" fill="#c0ffd4" opacity=".5"{glow}/>' if firing else "")
    if flip:
        registry = (f'<g transform="scale(-1 1)"><text x="28" y="26" font-family="{MONO}" font-size="16" font-weight="700" letter-spacing="2" fill="#06170f">SEVERANCE</text>'
                    f'<text x="-84" y="26" font-family="{MONO}" font-size="12" letter-spacing="2" fill="{pal["edge"]}" text-decoration="line-through">RESOLUTE</text></g>')
    else:
        registry = (f'<text x="-146" y="26" font-family="{MONO}" font-size="16" font-weight="700" letter-spacing="2" fill="#06170f">SEVERANCE</text>'
                    f'<text x="-10" y="26" font-family="{MONO}" font-size="12" letter-spacing="2" fill="{pal["edge"]}" text-decoration="line-through">RESOLUTE</text>')
    return f"""
      <path d="M-260,-20 L-200,-46 L180,-46 L262,-8 L262,8 L180,46 L-200,46 L-260,20 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="3"/>
      <path d="M-200,-46 L180,-46 L180,-34 L-200,-34 Z" fill="{pal["light"]}"/>
      <path d="M-140,-46 L-120,-74 L20,-74 L40,-46 Z" fill="{pal["light"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <rect x="40" y="-66" width="150" height="8" fill="{pal["edge"]}"/><rect x="40" y="-54" width="150" height="8" fill="{pal["edge"]}"/>
      <rect x="170" y="-7" width="132" height="14" fill="{pal["edge"]}"/>
      <path d="M-170,-46 V46 M-60,-46 V46 M60,-46 V46 M140,-46 V46" stroke="{pal["edge"]}" stroke-width="2" opacity=".7"/>
      <path d="M-120,46 L-100,80 L20,80 L40,46 Z" fill="{pal["light"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <ellipse cx="-266" cy="-10" rx="8" ry="10" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>
      <ellipse cx="-266" cy="10" rx="8" ry="10" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>
      <ellipse cx="-286" cy="0" rx="18" ry="10" fill="{pal["glow"]}" opacity=".5"{glow}/>
      <rect x="-150" y="8" width="130" height="24" fill="{pal["win"]}"/>
      {registry}
      <g{glow}>{lights}</g>
      {muzzle}"""


def cutter(oy: int, lit: bool = True, pal: dict = EWI, text: bool = True) -> str:
    """Cutter One, 150 long, facing +x."""
    glow = f' filter="url(#glow{oy})"' if lit and pal["glow"] != "none" else ""
    canopy = f'<rect x="20" y="-20" width="26" height="12" fill="{pal["win"]}" opacity=".9"{glow}/>' if lit else ""
    engine = f'<ellipse cx="-84" cy="0" rx="12" ry="6" fill="{pal["glow"]}" opacity=".6"{glow}/>' if lit else ""
    label = f'<text x="-50" y="14" font-family="{MONO}" font-size="12" letter-spacing="2" fill="{pal["edge"]}">CUTTER ONE</text>' if text and pal is not SIL else ""
    return f"""
      <path d="M-75,-18 L-55,-32 L40,-32 L78,-8 L78,8 L40,32 L-55,32 L-75,18 Z" fill="{pal["hull"]}" stroke="{pal["edge"]}" stroke-width="2"/>
      <path d="M-55,-32 L40,-32 L40,-24 L-55,-24 Z" fill="{pal["light"]}"/>
      <path d="M10,-32 L20,-40 L50,-40 L62,-20 L60,-8 L10,-8 Z" fill="#04120b" stroke="{pal["light"]}" stroke-width="2"/>
      {canopy}
      <path d="M50,22 L92,40 M92,40 L104,32 M92,40 L100,50" stroke="{pal["dark"]}" stroke-width="5" fill="none" stroke-linecap="round"/>
      <ellipse cx="-78" cy="0" rx="6" ry="9" fill="#04120b" stroke="{pal["edge"]}" stroke-width="2"/>
      {engine}
      {label}"""


def skiff(oy: int) -> str:
    """A Kites skiff, 90 long, facing +x."""
    glow = f' filter="url(#glow{oy})"'
    return f"""
      <path d="M-45,-10 L-30,-22 L30,-12 L45,0 L30,12 L-30,22 L-45,10 Z" fill="#232d24" stroke="#796424" stroke-width="2"/>
      <path d="M-20,-16 L20,-10 L20,10 L-20,16 Z" fill="#344832"/>
      <rect x="28" y="-3" width="9" height="6" fill="#c2a23b"{glow}/>
      <ellipse cx="-50" cy="0" rx="10" ry="5" fill="#c2a23b" opacity=".7"{glow}/>"""


def moonlet(cx: float, cy: float, r: float, oy: int) -> str:
    craters = "".join(
        f'<ellipse cx="{cx + dx * r}" cy="{cy + dy * r}" rx="{rr * r}" ry="{rr * r * .7}" fill="none" stroke="#164c31" stroke-opacity=".45" stroke-width="2"/>'
        for dx, dy, rr in ((-.2, -.3, .12), (.15, .25, .18), (-.45, .3, .08), (.4, -.4, .1), (-.1, .55, .06)))
    a0, a1 = math.radians(-70), math.radians(70)
    limb = (f'<path d="M{cx + r * math.cos(a0):.1f},{cy + r * math.sin(a0):.1f} A{r},{r} 0 0 1 '
            f'{cx + r * math.cos(a1):.1f},{cy + r * math.sin(a1):.1f}" fill="none" stroke="#267b4d" stroke-width="7" opacity=".8" filter="url(#glow{oy})"/>')
    return (f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#moon{oy})" stroke="#0b2b1c" stroke-width="3"/>{craters}{limb}')


def ice(seed: int, x0: float, y0: float, x1: float, y1: float, n: int) -> str:
    rng = random.Random(seed)
    return "".join(
        f'<circle cx="{rng.uniform(x0, x1):.0f}" cy="{rng.uniform(y0, y1):.0f}" r="{rng.uniform(1, 3):.1f}" fill="#8fb69b" opacity="{rng.uniform(.25, .7):.2f}"/>'
        for _ in range(n))


def junk(seed: int, x0: float, y0: float, x1: float, y1: float, n: int, dark: bool = False) -> str:
    rng = random.Random(seed)
    fills = ["#0f1a12", "#142219", "#1a2620"] if dark else ["#3a3f2c", "#4a4d33", "#5d603e", "#2b3527", "#6d6845"]
    edge = "#1f3a2b" if dark else "#8f8656"
    out = []
    for _ in range(n):
        x, y, rot = rng.uniform(x0, x1), rng.uniform(y0, y1), rng.uniform(0, 360)
        kind = rng.random()
        fill = rng.choice(fills)
        if kind < .4:
            L, t = rng.uniform(60, 260), rng.uniform(6, 16)
            shape = f'<rect x="{-L / 2:.0f}" y="{-t / 2:.0f}" width="{L:.0f}" height="{t:.0f}" fill="{fill}" stroke="{edge}" stroke-opacity=".5" stroke-width="1.5"/>'
        elif kind < .8:
            a, b = rng.uniform(40, 140), rng.uniform(30, 90)
            shape = f'<path d="M0,0 L{a:.0f},{rng.uniform(-10, 10):.0f} L{a * .9:.0f},{b:.0f} L{rng.uniform(-10, 10):.0f},{b * .8:.0f} Z" fill="{fill}" stroke="{edge}" stroke-opacity=".5" stroke-width="1.5"/>'
        else:
            shape = (f'<path d="M-90,-20 L-60,-34 L60,-34 L90,-10 L90,10 L60,34 L-60,34 L-90,20 Z" fill="{fill}" stroke="{edge}" stroke-opacity=".6" stroke-width="2"/>'
                     f'<path d="M-60,-34 H60 M-30,-34 V34 M20,-34 V34" stroke="{edge}" stroke-opacity=".3" stroke-width="1.5"/>')
        out.append(g(x, y, rng.uniform(.6, 1.3), rot, inner=shape))
    return "".join(out)


def bolt(x1: float, y1: float, x2: float, y2: float, oy: int) -> str:
    return (f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#4ed184" stroke-width="26" opacity=".25" filter="url(#soft{oy})"/>'
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#4ed184" stroke-width="7" opacity=".8" filter="url(#glow{oy})"/>'
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#e8fff0" stroke-width="2.5"/>')


def burst(cx: float, cy: float, r: float, oy: int, seed: int = 1) -> str:
    rng = random.Random(seed)
    spikes = "".join(
        f'<line x1="{cx}" y1="{cy}" x2="{cx + math.cos(a) * r * k:.0f}" y2="{cy + math.sin(a) * r * k:.0f}" stroke="#c0ffd4" stroke-width="{3 if i % 2 else 5}" opacity=".85" filter="url(#glow{oy})"/>'
        for i, (a, k) in enumerate((math.radians(i * 30 + rng.uniform(-8, 8)), rng.uniform(1.1, 2.2)) for i in range(12)))
    shards = "".join(
        g(cx + math.cos(a) * d, cy + math.sin(a) * d, 1, math.degrees(a),
          inner=f'<path d="M0,0 L{rng.uniform(14, 40):.0f},{rng.uniform(-6, 6):.0f} L{rng.uniform(6, 20):.0f},{rng.uniform(6, 14):.0f} Z" fill="#8f8656" stroke="#3e4130"/>')
        for a, d in ((rng.uniform(0, 6.28), rng.uniform(r * .8, r * 1.9)) for _ in range(14)))
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#flash{oy})"/>{spikes}<circle cx="{cx}" cy="{cy}" r="{r * .28}" fill="#e8fff0" filter="url(#glow{oy})"/>{shards}'


def helmet(oy: int) -> str:
    """A crew member from behind, helmet and shoulders, 300 wide."""
    return f"""
      <path d="M-170,170 L-130,60 L130,60 L170,170 Z" fill="#3b4a3a" stroke="#1d2b1f" stroke-width="3"/>
      <rect x="-50" y="40" width="100" height="40" fill="#5d603e"/>
      <circle cx="0" cy="-30" r="112" fill="#8f8656" stroke="#5d603e" stroke-width="4"/>
      <path d="M-112,-30 A112,112 0 0 1 0,-142" fill="none" stroke="#b0aa75" stroke-width="10" opacity=".8"/>
      <path d="M-116,-40 A118,118 0 0 1 -40,-140" fill="none" stroke="#4ed184" stroke-width="14" opacity=".9" filter="url(#glow{oy})"/>
      <rect x="-118" y="-60" width="20" height="60" rx="6" fill="#4ed184" opacity=".9"/>
      <rect x="98" y="-60" width="20" height="60" rx="6" fill="#4ed184" opacity=".9"/>
      <path d="M-70,20 L70,20 L60,70 L-60,70 Z" fill="#6d6845" stroke="#3e4130" stroke-width="2"/>"""


# --------------------------------------------------------------- variants

def strike_a_wide() -> str:
    oy = 0
    b = [defs(oy), frame_open(oy, grid=True)]
    b.append(ice(3, 60, 60, 1860, 1020, 140))
    b.append(moonlet(1600, 420, 430, oy))
    b.append(g(1330, 330, 1.0, 0, True, severance(oy, firing=True, flip=True)))
    b.append(bolt(1030, 330, 780, 590, oy))
    b.append(g(480, 640, 0.95, 0, False, meridian(EWI, oy)))
    b.append(burst(780, 590, 110, oy))
    b.append(junk(7, 60, 820, 900, 1030, 26))
    b.append(g(360, 900, 1.0, -6, False, cutter(oy)))
    b.append(frame_close(oy))
    return svg(H, "\n".join(b))


def strike_b_cockpit() -> str:
    oy = 0
    b = [defs(oy), frame_open(oy, grid=False)]
    # space through the canopy
    b.append(f'<rect x="{INSET}" y="{INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" fill="#020a06"/>')
    b.append(ice(5, 60, 60, 1860, 760, 90))
    b.append(junk(8, 60, 120, 700, 700, 14, dark=True))
    b.append(junk(9, 1500, 500, 1860, 720, 6, dark=True))
    b.append(bolt(1872, 300, 1250, 470, oy))
    b.append(g(1080, 500, 0.62, -4, False, meridian(EWI, oy)))
    b.append(burst(1250, 470, 80, oy, seed=4))
    # canopy struts
    strut = 'fill="#5d603e" stroke="#3e4130" stroke-width="3"'
    b.append(f'<rect x="{INSET}" y="{INSET}" width="{W - 2 * INSET}" height="70" {strut}/>')
    b.append(f'<rect x="{INSET}" y="{INSET}" width="70" height="700" {strut}/>')
    b.append(f'<rect x="{W - INSET - 70}" y="{INSET}" width="70" height="700" {strut}/>')
    b.append(f'<rect x="920" y="{INSET}" width="60" height="700" {strut}/>')
    b.append(''.join(f'<circle cx="{x}" cy="83" r="4" fill="#8f8656"/>' for x in range(140, 1800, 120)))
    # console
    b.append(f'<rect x="{INSET}" y="720" width="{W - 2 * INSET}" height="{H - INSET - 720}" fill="#3a3f2c" stroke="#8f8656" stroke-width="4"/>')
    b.append(f'<rect x="{INSET}" y="720" width="{W - 2 * INSET}" height="18" fill="#8f8656"/>')
    for x, w in ((430, 400), (860, 200), (1090, 410)):
        b.append(f'<rect x="{x}" y="790" width="{w}" height="210" fill="#04120b" stroke="#267b4d" stroke-width="3"/>')
    rng = random.Random(11)
    pts = " ".join(f"{450 + i * 8},{905 + (rng.uniform(-60, 60) if (i // 7) % 3 else rng.uniform(-6, 6)):.0f}" for i in range(46))
    b.append(f'<polyline points="{pts}" fill="none" stroke="#4ed184" stroke-width="3" filter="url(#glow{oy})"/>')
    b.append(f'<text x="450" y="826" font-family="{MONO}" font-size="19" letter-spacing="2" fill="#6ea985">GUARD 243.0  SIG 18%  NO CODE</text>')
    b.append(f'<circle cx="960" cy="912" r="66" fill="none" stroke="#267b4d" stroke-width="2"/><circle cx="960" cy="912" r="33" fill="none" stroke="#267b4d" stroke-width="2"/>'
             f'<line x1="960" y1="912" x2="1016" y2="880" stroke="#4ed184" stroke-width="3" filter="url(#glow{oy})"/>'
             f'<circle cx="1006" cy="886" r="6" fill="#c2a23b" filter="url(#glow{oy})"/><circle cx="920" cy="926" r="5" fill="#4ed184"/>'
             f'<text x="876" y="826" font-family="{MONO}" font-size="19" letter-spacing="2" fill="#6ea985">PLOT  312</text>')
    lines_ = ["> ...ERIDIAN CONTROL, THIS IS...", "> ...FOUR YEARS...", "> ...YOUR ROSTER...", "> [ CARRIER LINK LOST ]"]
    for i, t in enumerate(lines_):
        b.append(f'<text x="1110" y="{840 + i * 40}" font-family="{MONO}" font-size="21" fill="{"#c0ffd4" if i == 3 else "#4ed184"}">{t}</text>')
    b.append(''.join(f'<rect x="{1110 + i * 26}" y="760" width="14" height="8" fill="{c}"/>' for i, c in enumerate("#4ed184 #4ed184 #b8da5d #4ed184 #c2a23b #4ed184 #4ed184".split())))
    # crew
    b.append(g(240, 740, 1.0, 0, False, helmet(oy)))
    b.append(g(1680, 740, 1.0, 0, False, helmet(oy)))
    b.append(frame_close(oy))
    return svg(H, "\n".join(b))


def strike_c_flash() -> str:
    oy = 0
    b = [defs(oy), frame_open(oy, grid=False)]
    b.append(f'<rect x="{INSET}" y="{INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" fill="#020a06"/>')
    b.append(f'<circle cx="1120" cy="520" r="620" fill="url(#flash{oy})"/>')
    b.append(f'<line x1="{INSET}" y1="520" x2="{W - INSET}" y2="520" stroke="#c0ffd4" stroke-width="3" opacity=".7" filter="url(#glow{oy})"/>')
    b.append(f'<line x1="300" y1="520" x2="1900" y2="520" stroke="#4ed184" stroke-width="40" opacity=".18" filter="url(#soft{oy})"/>')
    b.append(g(860, 560, 1.05, 3, False, meridian(SIL, oy, lit=False, part="stern")))
    b.append(g(1280, 470, 1.05, -8, False, meridian(SIL, oy, lit=False, part="bow")))
    b.append(junk(21, 900, 380, 1300, 700, 16, dark=True))
    b.append(g(1560, 200, 0.7, 6, True, severance(oy, firing=False, flip=True)))
    b.append(junk(22, 60, 700, 700, 1030, 14, dark=True))
    b.append(g(330, 880, 1.0, -8, False, cutter(oy, lit=False, pal=SIL)))
    b.append(f'<path d="M370,850 L410,872 L410,890" fill="none" stroke="#c0ffd4" stroke-width="3" opacity=".9" filter="url(#glow{oy})"/>')
    b.append(frame_close(oy))
    return svg(H, "\n".join(b))


def strike_d_aftermath() -> str:
    oy = 0
    b = [defs(oy), frame_open(oy, grid=True)]
    b.append(ice(31, 60, 60, 1860, 1020, 160))
    b.append(moonlet(1660, 240, 300, oy))
    b.append(g(560, 440, 0.95, 8, False, meridian(EWI, oy, lit=False, part="stern")))
    b.append(g(1080, 640, 0.95, -14, False, meridian(EWI, oy, lit=False, part="bow")))
    b.append(f'<rect x="512" y="418" width="16" height="8" fill="#4ed184" filter="url(#glow{oy})"/>')
    b.append(''.join(f'<circle cx="{x}" cy="{y}" r="{r}" fill="url(#ember{oy})"/>' for x, y, r in ((790, 470, 70), (900, 560, 50), (840, 620, 40))))
    b.append(junk(41, 760, 380, 1020, 700, 22))
    for x, y, rot in ((1500, 300, 190), (1620, 460, 200), (1560, 650, 175), (1400, 800, 165), (1720, 760, 185)):
        b.append(g(x, y, 1.1, rot, False, skiff(oy)))
    b.append(junk(42, 60, 780, 760, 1030, 24))
    b.append(g(320, 900, 1.0, -6, False, cutter(oy, lit=False)))
    b.append(frame_close(oy))
    return svg(H, "\n".join(b))


def strike_e_page() -> str:
    b = []
    # panel 1: the shadow
    oy = 0
    b += [defs(oy), frame_open(oy, grid=True)]
    b.append(ice(51, 60, oy + 60, 1860, oy + 1020, 140))
    b.append(moonlet(1600, oy + 420, 430, oy))
    b.append(f'<rect x="1176" y="{oy + 402}" width="10" height="6" fill="#c2a23b" filter="url(#glow{oy})"/>')
    b.append(g(560, oy + 520, 0.95, 0, False, meridian(EWI, oy)))
    b.append(junk(52, 60, oy + 820, 900, oy + 1030, 26))
    b.append(g(360, oy + 900, 1.0, -6, False, cutter(oy)))
    b.append(f'<line x1="410" y1="{oy + 888}" x2="620" y2="{oy + 960}" stroke="#c0ffd4" stroke-width="2" opacity=".5" filter="url(#glow{oy})"/>')
    b.append(frame_close(oy))
    # panel 2: the shot
    oy = H
    b += [defs(oy), frame_open(oy, grid=True)]
    b.append(ice(53, 60, oy + 60, 1860, oy + 1020, 140))
    b.append(moonlet(1600, oy + 420, 430, oy))
    b.append(g(1330, oy + 400, 1.0, 0, True, severance(oy, firing=True, flip=True)))
    b.append(bolt(1030, oy + 400, 820, oy + 500, oy))
    b.append(g(560, oy + 520, 0.95, 0, False, meridian(EWI, oy)))
    b.append(burst(820, oy + 500, 110, oy, seed=7))
    b.append(junk(52, 60, oy + 820, 900, oy + 1030, 26))
    b.append(g(360, oy + 900, 1.0, -6, False, cutter(oy)))
    b.append(frame_close(oy))
    # panel 3: the junk
    oy = 2 * H
    b += [defs(oy), frame_open(oy, grid=False)]
    b.append(f'<rect x="{INSET}" y="{oy + INSET}" width="{W - 2 * INSET}" height="{H - 2 * INSET}" fill="#020a06"/>')
    b.append(ice(54, 60, oy + 60, 1860, oy + 1020, 60))
    b.append(g(1500, oy + 300, 0.36, 10, False, meridian(EWI, oy, lit=False, part="stern", text=False)))
    b.append(g(1720, oy + 380, 0.36, -12, False, meridian(EWI, oy, lit=False, part="bow", text=False)))
    b.append(f'<circle cx="1600" cy="{oy + 330}" r="40" fill="url(#ember{oy})"/>')
    for x, y, rot in ((1380, oy + 200, 200), (1650, oy + 520, 170), (1800, oy + 220, 190)):
        b.append(g(x, y, 0.5, rot, False, skiff(oy)))
    b.append(junk(55, 60, oy + 100, 1300, oy + 1030, 60))
    b.append(g(660, oy + 600, 2.4, -10, False, cutter(oy, lit=False)))
    b.append(f'<rect x="700" y="{oy + 548}" width="60" height="26" fill="#0e1f16" stroke="#267b4d" stroke-width="2"/>')
    b.append(f'<path d="M560,{oy + 560} L620,{oy + 545} L700,{oy + 548}" fill="none" stroke="#c0ffd4" stroke-width="3" opacity=".8" filter="url(#glow{oy})"/>')
    b.append(frame_close(oy))
    return svg(3 * H, "\n".join(b))


VARIANTS = {
    "the-strike-a-wide.svg": strike_a_wide,
    "the-strike-b-cockpit.svg": strike_b_cockpit,
    "the-strike-c-flash.svg": strike_c_flash,
    "the-strike-d-aftermath.svg": strike_d_aftermath,
    "the-strike-e-page.svg": strike_e_page,
}


def main() -> None:
    here = pathlib.Path(__file__).resolve().parent
    for name, make in VARIANTS.items():
        (here / name).write_text(make())
        print(name)


if __name__ == "__main__":
    main()
