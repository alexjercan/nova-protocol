#!/usr/bin/env python3
"""Sketches for the lore book, drawn on the comic's CRT vocabulary.

Run from the repository root:

    python3 lore/sketches.py

It writes lore/src/sketches/*.svg at 1200x800 for the pages' infoboxes. The
vocabulary lives in art/comics/crt.py: add shapes there, compose them here.
"""

from __future__ import annotations

import math
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "art" / "comics"))

from crt import (EMBER, EWI, FLEET, GOLD, INK, KESTREL, LIME, PALE, PHOS, PHOS_BRIGHT, PHOS_DIM, PHOS_TEXT, SIL,  # noqa: E402
                 Panel, console, derelict, g, harrier, ice, junk, mark, meridian, mono, orbit_ring, plot_text, rings,
                 rock, skiff, station, tag, warship)

OUT = ROOT / "lore" / "src" / "sketches"
W, H = 1200, 800
DEAD = dict(SIL, hull="#101e16", dark="#0b1610", light="#1c3024", edge="#2f5240")
ROOST = dict(KESTREL, win=EMBER, glow=GOLD)
BLUE = "#36a3ff"
GREY = "#7b8f8a"


def box(x: float, y: float, w: float, h: float, title: str, sub1: str, sub2: str = "", color: str = PHOS, dashed: bool = False) -> str:
    dash = ' stroke-dasharray="10 8"' if dashed else ""
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{INK}" stroke="{color}" stroke-width="3"{dash}/>'
            + mono(x + w / 2, y + 32, title, 19, color, weight=700, anchor="middle", spacing=3)
            + mono(x + w / 2, y + 56, sub1, 13, PHOS_TEXT, anchor="middle")
            + (mono(x + w / 2, y + 76, sub2, 13, PHOS_TEXT, anchor="middle") if sub2 else ""))


def arrow(x1: float, y1: float, x2: float, y2: float, color: str = PHOS, label: str = "", dashed: bool = False, lx: float = 0, ly: float = -8) -> str:
    a = math.atan2(y2 - y1, x2 - x1)
    p1 = (x2 - 16 * math.cos(a) + 7 * math.sin(a), y2 - 16 * math.sin(a) - 7 * math.cos(a))
    p2 = (x2 - 16 * math.cos(a) - 7 * math.sin(a), y2 - 16 * math.sin(a) + 7 * math.cos(a))
    dash = ' stroke-dasharray="10 8"' if dashed else ""
    out = (f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="3"{dash}/>'
           f'<path d="M{x2},{y2} L{p1[0]:.1f},{p1[1]:.1f} L{p2[0]:.1f},{p2[1]:.1f} Z" fill="{color}"/>')
    if label:
        out += mono((x1 + x2) / 2 + lx, (y1 + y2) / 2 + ly, label, 13, color, anchor="middle")
    return out


def saturn() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(console(60, 60, 1080, 680, "SATURN // THE PLATE // WORKING CHART Y0"))
    p.add(f'<circle cx="300" cy="430" r="130" fill="url(#moon)" stroke="{PHOS_DIM}" stroke-width="3"/>')
    for rx, ry in ((230, 52), (300, 68), (380, 86), (470, 106)):
        p.add(orbit_ring(300, 430, rx, ry, -14, PHOS_DIM))
    p.add(mono(300, 620, "SATURN", 18, PHOS_TEXT, anchor="middle", spacing=5))
    p.add(f'<line x1="1100" y1="110" x2="680" y2="300" stroke="{PHOS_DIM}" stroke-width="2" stroke-dasharray="8 10"/>')
    p.add(mark(640, 300, "station", "DATUM", PHOS, "SECTOR K. THE SHELTER'S ROCK. THE DESK"))
    p.add(mark(760, 480, "charge", "THE JUNK SITE", LIME, "SECTOR M. A MOONLET AND THE SCRAP"))
    p.add(mark(880, 190, "ship", "AZIMUTH", PHOS, "THE LANE. INBOUND, TEN DAYS"))
    p.add(mark(520, 620, "lost", "KESTREL STATION", PALE, "DEAD. UNREGISTERED"))
    p.add(mark(900, 620, "lost", "KESTREL STATION", PALE, "DEAD. UNREGISTERED"))
    p.add(mark(560, 160, "lost", "KESTREL STATION", PALE, "DEAD. UNREGISTERED"))
    p.add(plot_text(100, 705, [("SMALL-SHIP RADIO REACHES WHAT IT CAN SEE. THE RELAY IS DATUM'S.", PHOS_TEXT)], 16))
    return p


def ice_share() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(console(60, 60, 1080, 680, "SATURN ICE // SHARE OF OUTPUT BY OWNER"))
    p.add(mono(100, 136, "EARTHWORKS INDUSTRIAL", 20, PHOS_BRIGHT, spacing=3))
    p.add(mono(100, 164, "THE REST: FOURTEEN SMALL COMPANIES, THEN KESTREL, THEN NOBODY", 15, PHOS_TEXT))
    base, x = 640, 150
    for label, pct in (("Y-20", 0), ("Y-15", 9), ("Y-12", 31), ("Y-8", 58), ("Y-5", 74), ("Y-4", 96), ("Y0", 100)):
        h = max(pct * 4.2, 3)
        p.add(f'<rect x="{x}" y="{base - h}" width="100" height="{h}" fill="{PHOS}" opacity=".7"/>')
        p.add(f'<rect x="{x}" y="{base - 420}" width="100" height="{420 - h}" fill="none" stroke="{PHOS_DIM}" stroke-dasharray="4 6" opacity=".5"/>')
        p.add(mono(x + 50, base + 32, label, 17, PHOS_TEXT, anchor="middle"))
        p.add(mono(x + 50, base - h - 12, f"{pct}%", 18, PHOS_BRIGHT, anchor="middle"))
        x += 140
    p.add(f'<line x1="120" y1="{base}" x2="1120" y2="{base}" stroke="{PHOS_DIM}" stroke-width="2"/>')
    return p


def earth_board() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(f'<circle cx="990" cy="230" r="120" fill="url(#moon)" stroke="{GREY}" stroke-width="3" opacity=".9"/>')
    p.add(f'<ellipse cx="990" cy="230" rx="122" ry="40" fill="none" stroke="{GREY}" stroke-width="1" opacity=".5"/>')
    p.add(mono(990, 390, "EARTH", 18, GREY, anchor="middle", spacing=5))
    p.add(mono(990, 416, "70 TO 90 LIGHT-MINUTES", 13, PHOS_TEXT, anchor="middle"))
    p.add(console(80, 120, 720, 580, "EWI BOARD // EARTH DESK // TO SATURN DESK"))
    rows = [("RE: SITE CONSOLIDATION, RING SECTOR K", PHOS_BRIGHT), ("", PHOS), ("APPROVED.", PHOS_BRIGHT),
            ("MEANS AT REGIONAL DISCRETION.", PHOS), ("COSTS AGAINST REGIONAL TERM.", PHOS),
            ("NO WITNESSES AT WORKING LEVEL.", PHOS), ("", PHOS), ("ACKNOWLEDGE BY VOICE.", GOLD), ("", PHOS),
            ("THIS DESK DOES NOT SPEAK.", PHOS_TEXT)]
    p.add(plot_text(120, 210, rows, 21, 42))
    return p


def ewi() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(rings(W, 210, -7, seed=3), ice(2, 40, 40, 1160, 760, 90))
    p.add(g(600, 430, 1.15, inner=meridian(EWI, bay_open=True)))
    return p


def fleet() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(rings(W, 260, -6, seed=5), ice(4, 40, 40, 1160, 760, 70))
    p.add(g(600, 420, 1.1, -4, inner=warship(FLEET, lit=True, name="RESOLUTE", old_name=False)))
    return p


def kestrel() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(rings(W, 200, -7, seed=2), ice(1, 40, 40, 1160, 760, 100))
    p.add(g(760, 560, .62, inner=rock(0, 250, 620, 250) + station(KESTREL)))
    p.add(g(260, 380, .9, 8, inner=harrier()))
    return p


def dead_companies() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(rings(W, 240, -8, seed=7), ice(9, 40, 40, 1160, 760, 120))
    p.add(junk(3, 60, 120, 1140, 700, 40, "dark"))
    p.add(g(700, 250, .45, -3, inner=derelict(DEAD, 8, broken=False)))
    p.add(g(380, 430, .8, -10, inner=derelict(DEAD, 2)))
    p.add(g(880, 570, .6, 14, True, derelict(DEAD, 5)))
    p.add(tag(380, 330, "TAG 04 // ...ARD ICE"))
    p.add(tag(700, 190, "TAG 12 // BRIGHTWATER"))
    p.add(tag(880, 490, "TAG 11 // NO NAME"))
    return p


def kites() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(rings(W, 210, -7, seed=4), ice(6, 40, 40, 1160, 760, 90))
    p.add(g(700, 600, .5, inner=rock(0, 250, 620, 250, seed=8) + station(ROOST, "ROOST")))
    for seed, (x, y, s, flip, label) in enumerate(((240, 300, 1.5, False, "ROTATION"), (420, 520, 1.3, False, "LEDGER"), (1060, 380, 1.2, True, "")), start=1):
        p.add(g(x, y, s, 0, flip, skiff(seed, label)))
    return p


def politics() -> Panel:
    p = Panel(W, H, grid=True)
    p.add(console(60, 60, 1080, 680, "SATURN // WHO ANSWERS TO WHOM"))
    p.add(box(80, 130, 260, 90, "EARTH", "RIGHTS ON PAPER", "ONE RULE: THE RECORDER", GREY, dashed=True))
    p.add(box(450, 130, 300, 90, "THE BOARD", "EARTH. ORDERS OUTCOMES", "NEVER ON THE RADIO"))
    p.add(box(860, 130, 260, 90, "EARTH FLEET", "SELLS ITS SURPLUS", "HUNTS BY RADIO", BLUE))
    p.add(box(450, 290, 300, 90, "SATURN DESK", "DATUM. OWNS ROSTER AND RELAY", "OWNS THE GUNS"))
    p.add(box(230, 450, 260, 90, "CARRIERS", "MERIDIAN, AZIMUTH", "CAPTAINS ALLOCATED"))
    p.add(box(700, 450, 260, 90, "PARALLAX", "SECURITY SHIP, PICKETS", "THE DESK'S GUNS"))
    p.add(box(270, 610, 260, 90, "CREWS", "RENEWED", "ONE ANTENNA HEARS THEM"))
    p.add(box(80, 610, 170, 90, "KESTREL", "DEAD", "ALIVE AS WORDS", PALE, dashed=True))
    p.add(box(900, 610, 220, 90, "THE KITES", "OFF EVERY ROSTER", "SKIFFS, THEN A WARSHIP", EMBER))
    p.add(arrow(600, 220, 600, 290, PHOS, "ORDERS", lx=52))
    p.add(arrow(560, 380, 400, 450, PHOS, "ALLOCATION", lx=-70))
    p.add(arrow(660, 380, 800, 450, PHOS))
    p.add(arrow(400, 540, 400, 610, PHOS, "SHIFTS", lx=48))
    p.add(arrow(530, 620, 750, 380, PHOS_TEXT, "DISTRESS. ONE ANTENNA", dashed=True, lx=-40, ly=-60))
    p.add(arrow(860, 175, 750, 175, BLUE, "SURPLUS", dashed=True))
    p.add(arrow(210, 220, 320, 450, GREY, "RECORDER, ROSTER", dashed=True, lx=-90))
    p.add(f'<path d="M165,700 V724 H1010 V706" fill="none" stroke="{PALE}" stroke-width="3" stroke-dasharray="10 8"/>')
    p.add(f'<path d="M1010,700 L1003,716 L1017,716 Z" fill="{PALE}"/>')
    p.add(mono(590, 718, "WORDS AND PEOPLE", 13, PALE, anchor="middle"))
    p.add(arrow(900, 630, 530, 480, EMBER, "RAIDS. CARGO, NEVER CREWS", dashed=True, lx=45, ly=85))
    return p


SKETCHES = {
    "saturn.svg": saturn,
    "ice.svg": ice_share,
    "earth-board.svg": earth_board,
    "ewi.svg": ewi,
    "fleet.svg": fleet,
    "kestrel.svg": kestrel,
    "dead-companies.svg": dead_companies,
    "kites.svg": kites,
    "politics.svg": politics,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, make in SKETCHES.items():
        (OUT / name).write_text(make().svg())
        print(name)


if __name__ == "__main__":
    main()
