#!/usr/bin/env python3
"""Panel art for the Nova Protocol comic: the prologue "The Shelter" and act one "The job".

Run from the repository root:

    python3 art/comics/nova_protocol.py

It writes the SVG assets the comic pages name into
web/src/assets/story/nova-protocol/. Wide panels are 1920x900, half panels
1440x1000, the cover and the bleed page 1920x1080. The comic engine crops
each image to its panel, so every subject sits inside the central band.
"""

from __future__ import annotations

import pathlib

from crt import (EMBER, EWI, FLEET, GOLD, GRID, INK, KESTREL, KESTREL_DEAD, LIME, PALE, PHOS, PHOS_BRIGHT, PHOS_DIM,
                 PHOS_TEXT, SIL, SPACE, SUIT_EDGE, Panel, banner, bay, beacon_rings, bolt, burst, cabin, charge,
                 claim_line, console, crate, cutter, deck_hand, derelict, ember, eva_crew, face, flash_disc, g,
                 harrier, helmet, ice, ice_slab, junk, lamp_pool, mark, meridian, mono, orbit_ring, plaque, plot_text,
                 rings, rock, shadow_wedge, skiff, station, station_wreck, tag, tender, warship, waveform)

OUT = pathlib.Path(__file__).resolve().parents[2] / "web" / "src" / "assets" / "story" / "nova-protocol"

WIDE = (1920, 900)
HALF = (1440, 1000)
FULL = (1920, 1080)

CALLOWAY = dict(skin="#a08a62", shadow="#5a4c34", light="#c7b489", hair="#4a4a3c", style="neat", headset="slim",
                uniform="#3e4a3c", uniform_dark="#22301f", collar="officer", jaw="narrow", age=.5, mouth="flat")
PELL = dict(skin="#8b7350", shadow="#4a3c28", light="#b09a72", hair="#2e2c22", style="cropped", headset="deck",
            uniform="#6d6845", uniform_dark="#3e4130", collar="deck", jaw="wide", age=.8, stubble=True, heavy_brow=True,
            mouth="set", helmet_ring=True, glow_below=False)
HALLORAN = dict(skin="#8f7048", shadow="#4d3a24", light="#b8966c", hair="#1c1a16", style="swept", headset="slim",
                uniform="#2f5a4a", uniform_dark="#1a3a2e", collar="deck", jaw="narrow", age=.45, mouth="flat")
OKORO = dict(skin="#5c4530", shadow="#2e2117", light="#86684a", hair="#141210", style="cropped", headset="goggles",
             uniform="#a8842e", uniform_dark="#6b5320", collar="deck", jaw="wide", age=.5, stubble=True, mouth="flat")
DEMIR = dict(skin="#9a7a56", shadow="#55402a", light="#c2a47e", hair="#2a2620", style="cropped", headset="slim",
             uniform="#3e4a3c", uniform_dark="#22301f", collar="deck", jaw="narrow", age=.4, mouth="flat")


def shelter_on_rock(x: float, y: float, s: float, wreck: bool = False) -> str:
    """The station on its rock, the rock's top at (x, y)."""
    body = station_wreck() if wreck else station()
    return g(x, y, s, inner=rock(0, 250, 620, 250) + body)


def cover() -> Panel:
    p = Panel(*FULL, grid=True)
    p.add(rings(1920, 300, -7), ice(1, 40, 40, 1880, 1040, 170))
    p.add(shelter_on_rock(1330, 640, .95))
    p.add(g(560, 250, .5, inner=meridian(EWI)))
    p.add(claim_line(880, 40, 1040, "EWI CLAIM K-7", "KESTREL", 120))
    p.add(g(700, 540, .7, 12, inner=tender(EWI, "SURVEY 3")))
    return p


def shelter_wide() -> Panel:
    p = Panel(*WIDE, grid=True)
    p.add(rings(1920, 250, -7, seed=3), ice(2, 40, 40, 1880, 860, 150))
    p.add(g(400, 210, .4, inner=meridian(EWI)))
    p.add(claim_line(760, 40, 860, "EWI CLAIM K-7", "KESTREL", 110))
    p.add(shelter_on_rock(1240, 560, .78))
    p.add(g(640, 610, 1.0, 6, True, harrier(flip=True)))
    p.add(f'<line x1="720" y1="600" x2="1020" y2="440" stroke="{GOLD}" stroke-width="2" stroke-dasharray="6 10" opacity=".5"/>')
    return p


def meridian_on_station() -> Panel:
    p = Panel(*HALF, grid=True)
    p.add(rings(1440, 180, -6, seed=5), ice(3, 40, 40, 1400, 960, 110))
    p.add(shelter_on_rock(1250, 300, .28))
    p.add(g(660, 480, 1.3, inner=meridian(EWI, bay_open=True)))
    p.add(g(1130, 700, .8, -8, inner=tender(EWI, "SURVEY 3")))
    return p


def survey_plot() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(console(130, 110, 1180, 780, "SURVEY PLOT // CLAIM K-7 // MERIDIAN CONTROL"))
    p.add(f'<path d="M130,600 L560,560 L700,320 L880,300 L1310,280" fill="none" stroke="{LIME}" stroke-width="3" stroke-dasharray="18 14" opacity=".8"/>')
    p.add(mono(220, 640, "EWI CLAIM K-7", 20, LIME, spacing=4), mono(960, 260, "KESTREL CLAIM", 20, KESTREL["hull"], spacing=4))
    p.add(f'<circle cx="1010" cy="470" r="330" fill="none" stroke="{GOLD}" stroke-width="2" stroke-dasharray="8 10" opacity=".7"/>')
    p.add(mark(1010, 470, "station", "KESTREL // SHELTER", KESTREL["hull"], "RANGE FROM CHARGE 240 M"))
    p.add(mark(760, 500, "charge", "DEMOLITION", GOLD, "FOR THE ICE // SURVEY 3"))
    p.add(mark(420, 760, "ship", "MERIDIAN", PHOS, "ON STATION // 6 KM"))
    p.add(f'<line x1="780" y1="500" x2="990" y2="472" stroke="{GOLD}" stroke-width="2" opacity=".8"/>')
    p.add(mono(1010, 160, "BLAST RADIUS 600 M", 16, GOLD, spacing=3, anchor="middle"))
    p.add(plot_text(170, 840, [("CHARGE: EF DEMO-12, FLEET SURPLUS", PHOS_TEXT)], 17))
    p.add(plot_text(760, 840, [("AUTH: CMDR CALLOWAY // BOARD REF 9-K7", PHOS_TEXT)], 17))
    return p


def tender_at_the_charge() -> Panel:
    p = Panel(*WIDE, space=True)
    p.add(ice(4, 40, 40, 1880, 700, 120), rings(1920, 160, -6, seed=7))
    p.add(shelter_on_rock(1500, 520, .62))
    p.add(claim_line(1120, 40, 860, "EWI", "KESTREL", 100))
    p.add(ice_slab(-40, 560, 1160, 940, seed=8))
    p.add(g(760, 250, 1.15, 4, inner=tender(EWI, "SURVEY 3")))
    p.add(g(640, 530, 1, inner=charge()))
    p.add(g(470, 560, .95, inner=eva_crew(tether_to=(290, -300))))
    p.add(g(830, 570, .95, flip=True, inner=eva_crew(tether_to=(70, -320))))
    return p


def charge_armed() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(ice(5, 40, 40, 1400, 600, 60))
    p.add(shelter_on_rock(1180, 330, .3))
    p.add(ice_slab(-40, 640, 1480, 1040, seed=10))
    p.add(g(700, 560, 3.1, inner=charge()))
    p.add(f'<path d="M-40,770 L220,700 L330,690" stroke="#6d6845" stroke-width="56" stroke-linecap="round" fill="none"/>'
          f'<circle cx="360" cy="686" r="40" fill="#8f8656" stroke="{SUIT_EDGE}" stroke-width="3"/>'
          f'<path d="M372,650 L410,632" stroke="#8f8656" stroke-width="22" stroke-linecap="round"/>'
          f'<rect x="230" y="690" width="14" height="26" rx="4" fill="{PHOS}" filter="url(#glow)"/>')
    return p


def hold_at_the_site() -> Panel:
    p = Panel(*HALF, grid=True)
    p.add(rings(1440, 170, -6, seed=11), ice(6, 40, 40, 1400, 960, 120))
    p.add(shelter_on_rock(940, 520, .72))
    p.add(claim_line(560, 40, 960, "EWI", "KESTREL", 90))
    p.add(ice_slab(-40, 760, 620, 1040, seed=12))
    p.add(g(300, 780, .55, inner=charge()))
    p.add(g(270, 560, 1.0, -4, inner=tender(EWI, "SURVEY 3")))
    p.add(g(190, 800, .5, inner=eva_crew()), g(400, 810, .5, flip=True, inner=eva_crew()))
    return p


def calloway_bridge() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(ice(7, 40, 40, 1400, 500, 60))
    p.add(f'<rect x="0" y="0" width="1440" height="90" fill="#3a3f2c"/><rect x="0" y="90" width="1440" height="12" fill="#8f8656"/>')
    p.add(''.join(f'<rect x="{x}" y="0" width="70" height="560" fill="#3a3f2c" stroke="#8f8656" stroke-width="3"/>' for x in (-10, 1380)))
    p.add(g(1130, 330, .32, inner=shelter_on_rock(0, 0, 1)))
    p.add(g(600, 470, 1.0, inner=face(**CALLOWAY)))
    p.add(f'<rect x="0" y="880" width="1440" height="120" fill="#3a3f2c" stroke="#8f8656" stroke-width="4"/>')
    p.add(f'<rect x="220" y="900" width="1000" height="22" fill="#04120b"/><rect x="230" y="906" width="600" height="10" fill="{PHOS}" filter="url(#glow)"/>')
    return p


def the_blast() -> Panel:
    p = Panel(*FULL, space=True)
    p.add(ice(8, 40, 40, 1880, 1040, 80))
    p.add(flash_disc(1120, 520, 760))
    p.add(f'<line x1="0" y1="540" x2="1920" y2="540" stroke="{PHOS_BRIGHT}" stroke-width="3" opacity=".7" filter="url(#glow)"/>')
    p.add(f'<line x1="500" y1="540" x2="1920" y2="540" stroke="{PHOS}" stroke-width="44" opacity=".18" filter="url(#soft)"/>')
    p.add(g(1300, 700, .8, inner=rock(0, 250, 620, 250) + station_wreck()))
    p.add(burst(1120, 520, 150, seed=3, shard_fill=KESTREL["hull"], shard_edge=KESTREL["edge"]))
    p.add(junk(13, 700, 200, 1500, 900, 20, "bone"))
    p.add(g(720, 700, 1.0, 34, inner=tender(SIL, lit=False)))
    p.add(g(560, 640, .7, 60, inner=eva_crew()))
    p.add(g(300, 300, .55, inner=meridian(SIL, lit=False, text=False)))
    return p


def boat_deck() -> Panel:
    p = Panel(*WIDE, space=True)
    p.add(bay(1920, 900))
    p.add(banner(240, 150, 720, 150))
    p.add(f'<rect x="1040" y="150" width="440" height="150" fill="#04120b" stroke="{PHOS_DIM}" stroke-width="4"/>')
    p.add(plot_text(1064, 196, [("MERIDIAN CONTROL", PHOS_BRIGHT), ("SECTION NINE", PHOS), ("ROSTER CLOSED // NO RECOVERY", GOLD)], 22, 40))
    p.add(g(1360, 560, 2.0, flip=True, inner=tender(EWI, "TENDER 4", flip=True)))
    p.add(f'<path d="M1200,640 L1200,720 L1160,720 M1520,640 L1520,720 L1560,720" stroke="{SUIT_EDGE}" stroke-width="14" fill="none"/>')
    p.add(g(700, 700, .95, inner=deck_hand()), g(980, 730, 1.0, flip=True, inner=deck_hand(tool=True)), g(1120, 690, .8, inner=deck_hand(helmet_on=True)))
    p.add(lamp_pool(1360, 560, 420))
    p.add(g(330, 700, 1.15, inner=helmet()))
    return p


def pell_refuses() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(f'<rect width="1440" height="1000" fill="#0a1710"/>')
    p.add(''.join(f'<line x1="{x}" y1="0" x2="{x}" y2="1000" stroke="{GRID}" stroke-width="3" stroke-opacity=".8"/>' for x in range(0, 1441, 190)))
    p.add(lamp_pool(300, 60, 520), lamp_pool(1200, 60, 400))
    p.add(g(1120, 150, .5, inner=banner(0, 0, 720, 150)))
    p.add(g(600, 470, 1.0, inner=face(**PELL)))
    return p


def brandt_fits_the_boat() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(bay(1440, 1000, seed=14))
    p.add(g(900, 560, 2.6, flip=True, inner=tender(EWI, "TENDER 4", flip=True)))
    p.add(f'<path d="M700,690 L700,800 L640,800 M1100,690 L1100,800 L1160,800" stroke="{SUIT_EDGE}" stroke-width="16" fill="none"/>')
    p.add(lamp_pool(900, 560, 520))
    p.add(g(420, 860, 1.5, inner=deck_hand(tool=True)))
    p.add(g(1220, 820, 1.1, inner=deck_hand(helmet_on=True)))
    p.add(f'<rect x="60" y="120" width="360" height="150" fill="#04120b" stroke="{PHOS_DIM}" stroke-width="4"/>')
    p.add(plot_text(84, 166, [("BOAT DECK", PHOS_BRIGHT), ("TENDER 4 // CLAMPS", PHOS), ("RELEASED", GOLD)], 22, 40))
    return p


def tender_toward_the_shelter() -> Panel:
    p = Panel(*WIDE, grid=True)
    p.add(rings(1920, 240, -7, seed=15), ice(9, 40, 40, 1880, 860, 150))
    p.add(g(300, 280, .5, inner=meridian(EWI)))
    p.add(shelter_on_rock(1500, 560, .7, wreck=True))
    p.add(ember(1500, 300, 120), ember(1380, 380, 70))
    p.add(junk(16, 900, 200, 1400, 800, 18, "bone"), junk(17, 1000, 300, 1300, 700, 8, "ewi"))
    p.add(g(900, 520, 1.0, -6, inner=tender(EWI, "TENDER 4")))
    p.add(f'<line x1="420" y1="330" x2="820" y2="500" stroke="{PHOS}" stroke-width="2" stroke-dasharray="4 12" opacity=".45"/>')
    return p


def signal_lost() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(console(130, 110, 1180, 780, "BOAT DECK PLOT // MERIDIAN CONTROL // T+00:41"))
    p.add(mark(420, 700, "ship", "MERIDIAN", PHOS, "HOLDING // NO SEARCH ORDERED"))
    p.add(mark(940, 380, "station", "KESTREL // SHELTER", "#6b6a58", "NO SIGNAL"))
    p.add(f'<polyline points="440,690 560,640 700,560 820,500 900,450" fill="none" stroke="{GOLD}" stroke-width="4" stroke-dasharray="10 12"/>')
    p.add(mark(900, 450, "lost", "TENDER 4 // SIGNAL LOST", GOLD, "PELL, D. AND VOLUNTEERS"))
    p.add(f'<rect x="150" y="740" width="1140" height="130" fill="{INK}" opacity=".9"/>')
    p.add(plot_text(176, 776, [("SECTION NINE: APPLIED. ROSTER CLOSED.", PHOS), ("SEARCH: NONE ORDERED.", GOLD), ("REPORT: KESTREL REACTOR FAILURE. NO EWI CASUALTIES.", PHOS_TEXT)], 19, 38))
    return p


def the_plaque() -> Panel:
    p = Panel(*WIDE, space=True)
    p.add(f'<rect width="1920" height="900" fill="#0a1710"/>')
    p.add(''.join(f'<line x1="{x}" y1="0" x2="{x}" y2="900" stroke="{GRID}" stroke-width="3" stroke-opacity=".8"/>' for x in range(0, 1921, 190)))
    p.add(f'<rect y="760" width="1920" height="140" fill="#0d1a12"/><rect y="750" width="1920" height="14" fill="{SUIT_EDGE}"/>')
    p.add(lamp_pool(600, 40, 700), lamp_pool(1400, 40, 700))
    p.add(banner(180, 250, 860, 180))
    p.add(plaque(1180, 250, 560, 300, [("DORIAN PELL", 40, 700), ("BOAT DECK, EWI MERIDIAN", 20, 400), ("LOST IN RESCUE", 22, 700),
                                          ("\"ALL HANDS, TENDERS AWAY\"", 20, 400), ("WE REMEMBER OUR OWN", 16, 400)]))
    p.add(g(1800, 780, 1.3, inner=deck_hand(tool=True)))
    return p


# ------------------------------------------------------------------ act one

def junk_field(seed: int, x0: float, y0: float, x1: float, y1: float) -> str:
    """The junk site's scrap in three palettes: company khaki, Kestrel bone and the unnamed dark."""
    return junk(seed, x0, y0, x1, y1, 26, "dark") + junk(seed + 1, x0, y0, x1, y1, 12, "bone") + junk(seed + 2, x0, y0, x1, y1, 10, "ewi")


def junk_site() -> Panel:
    p = Panel(*HALF, grid=True)
    p.add(rings(1440, 240, -7, seed=21), ice(21, 40, 40, 1400, 960, 120))
    p.add(rock(1200, 520, 300, 290, seed=22), shadow_wedge(1200, 520, 290))
    p.add(junk_field(23, 560, 120, 1100, 900))
    p.add(g(880, 780, .9, 14, inner=derelict(SIL, seed=3)))
    p.add(g(980, 240, .8, -20, True, harrier(lit=False, flip=True)))
    p.add(g(500, 440, .75, inner=meridian(EWI, bay_open=True)))
    p.add(g(800, 580, .9, -10, inner=cutter()))
    p.add(f'<line x1="640" y1="500" x2="760" y2="565" stroke="{PHOS}" stroke-width="2" stroke-dasharray="4 12" opacity=".45"/>')
    return p


def boat_bay_shift() -> Panel:
    p = Panel(*WIDE, space=True)
    p.add(bay(1920, 900, seed=24))
    p.add(banner(120, 140, 580, 122))
    p.add(plaque(760, 130, 380, 205, [("DORIAN PELL", 27, 700), ("BOAT DECK, EWI MERIDIAN", 14, 400), ("LOST IN RESCUE", 15, 700),
                                       ("\"ALL HANDS, TENDERS AWAY\"", 14, 400), ("WE REMEMBER OUR OWN", 11, 400)]))
    p.add(f'<rect x="1440" y="150" width="380" height="150" fill="#04120b" stroke="{PHOS_DIM}" stroke-width="4"/>')
    p.add(plot_text(1464, 196, [("BOAT DECK // SHIFT 3", PHOS_BRIGHT), ("CUTTER ONE // CARD 7-R", PHOS), ("UNDER WAY IN 00:56", GOLD)], 22, 40))
    p.add(g(1420, 590, 2.6, flip=True, inner=cutter(flip=True)))
    p.add(f'<path d="M1300,660 L1300,740 L1260,740 M1560,660 L1560,740 L1600,740" stroke="{SUIT_EDGE}" stroke-width="14" fill="none"/>')
    p.add(lamp_pool(1420, 560, 460))
    p.add(g(560, 800, 1.25, inner=deck_hand(helmet_on=True)), g(720, 780, 1.15, inner=deck_hand()), g(880, 810, 1.2, inner=deck_hand(tool=True)))
    return p


def cutter_released() -> Panel:
    p = Panel(*HALF, grid=True)
    p.add(rings(1440, 180, -6, seed=25), ice(25, 40, 40, 1400, 960, 110))
    p.add(junk_field(26, 900, 500, 1440, 1000))
    p.add(g(520, 400, 1.6, inner=meridian(EWI, bay_open=True)))
    p.add(g(960, 700, 1.7, 10, inner=cutter()))
    p.add(f'<path d="M660,500 Q760,560 860,660" fill="none" stroke="{PHOS}" stroke-width="2" stroke-dasharray="4 12" opacity=".5"/>')
    return p


def handling_card() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(console(130, 110, 1180, 780, "MAINTENANCE RELEASE // CUTTER ONE // CARD 7-R"))
    p.add(f'<path d="M180,560 L420,470 L560,300 L700,290 L760,520 L560,640 L300,690 Z" fill="none" stroke="{LIME}" stroke-width="2" stroke-dasharray="14 12" opacity=".7"/>')
    p.add(mono(200, 540, "PLATE SEVEN", 20, LIME, spacing=4))
    p.add(mark(330, 610, "charge", "CRATE 1", GOLD), mark(560, 380, "charge", "CRATE 2", GOLD), mark(660, 590, "charge", "CRATE 3", GOLD, "NO MANIFEST // MASS UNKNOWN"))
    p.add(f'<rect x="840" y="220" width="220" height="180" fill="none" stroke="{PHOS}" stroke-width="2" stroke-dasharray="8 8" opacity=".7"/>')
    for x, y, t in ((840, 400, "TRIM A"), (840, 220, "TRIM B"), (1060, 220, "TRIM C"), (1060, 400, "TRIM D")):
        p.add(mark(x, y, "station", t, PHOS))
    p.add(f'<circle cx="1000" cy="640" r="90" fill="#0a1711" stroke="{PALE}" stroke-width="3"/>' + mono(1000, 646, "SURVEY BODY", 15, PALE, spacing=2, anchor="middle"))
    p.add(f'<path d="M760,520 Q880,540 900,600 Q1000,780 1160,700" fill="none" stroke="{GOLD}" stroke-width="3" stroke-dasharray="16 10" opacity=".8"/>')
    p.add(mark(900, 600, "ship", "TRANSIT 1", GOLD), mark(1160, 700, "ship", "TRANSIT 2", GOLD))
    p.add(mark(240, 760, "ship", "MERIDIAN", PHOS, "UNDER WAY IN 00:56"))
    p.add(plot_text(170, 840, [("PORT RCS MANIFOLD: REPLACED // YARD", PHOS_TEXT)], 17))
    p.add(plot_text(760, 840, [("GUIDANCE + AUTO BRAKE: RELEASE PENDING", PHOS_TEXT)], 17))
    return p


def crates_on_plate_seven() -> Panel:
    p = Panel(*HALF, grid=True)
    p.add(ice(27, 40, 40, 1400, 960, 90))
    p.add(junk_field(28, 60, 80, 1380, 940))
    p.add(g(1100, 300, 1.1, -12, inner=derelict(SIL, seed=7)))
    p.add(g(280, 760, .9, 24, True, harrier(lit=False, flip=True)))
    p.add(tag(1010, 210, "07 // 02 // SECURE"), tag(360, 560, "07 // 01 // SECURE"), tag(1120, 720, "07 // 03 // NO MANIFEST", GOLD))
    p.add(g(1080, 780, 1.6, 30, inner=crate()))
    p.add(g(640, 520, 2.8, -4, inner=cutter(crates=2)))
    p.add(g(770, 690, 1.6, inner=crate()))
    return p


def the_donut() -> Panel:
    p = Panel(*WIDE, grid=True)
    p.add(rings(1920, 230, -7, seed=29), ice(29, 40, 40, 1880, 860, 150))
    p.add(g(260, 300, .3, inner=meridian(EWI)))
    p.add(junk_field(30, 300, 150, 900, 800))
    p.add(rock(1180, 520, 330, 310, seed=31))
    p.add(shadow_wedge(1180, 520, 310))
    p.add(g(1640, 690, .42, -8, inner=warship(dict(FLEET, hull="#0b1812", dark="#08120d", light="#122219", edge="#1a3024"), name="", old_name=False)))
    p.add(orbit_ring(1180, 520, 540, 210, -14))
    p.add(g(760, 340, 1.0, -22, inner=cutter()))
    return p


def demir_at_control() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(f'<rect width="1440" height="1000" fill="#0a1710"/>')
    p.add(''.join(f'<line x1="{x}" y1="0" x2="{x}" y2="1000" stroke="{GRID}" stroke-width="3" stroke-opacity=".8"/>' for x in range(0, 1441, 190)))
    p.add(lamp_pool(700, 40, 600))
    for x, rows in ((60, [("CUTTER ONE", PHOS_BRIGHT), ("ORBIT HOLD // SURVEY BODY", GOLD), ("SIGHTLINE: RESTORED", PHOS)]),
                    (1040, [("MERIDIAN", PHOS_BRIGHT), ("UNDER WAY IN 00:31", PHOS), ("RELEASE 7-R: FILED", GOLD)])):
        p.add(f'<rect x="{x}" y="120" width="340" height="150" fill="#04120b" stroke="{PHOS_DIM}" stroke-width="4"/>')
        p.add(plot_text(x + 24, 166, rows, 20, 40))
    p.add(g(620, 430, 1.0, inner=face(**DEMIR)))
    return p


def third_crate_home() -> Panel:
    p = Panel(*HALF, grid=True)
    p.add(rings(1440, 200, -6, seed=32), ice(32, 40, 40, 1400, 960, 110))
    p.add(junk_field(33, 40, 80, 800, 940))
    p.add(rock(1320, 260, 260, 240, seed=34))
    p.add(g(1040, 560, .75, inner=meridian(EWI, bay_open=True)))
    p.add(g(480, 620, 1.7, 6, inner=cutter(crates=3)))
    p.add(f'<path d="M640,600 Q780,560 900,570" fill="none" stroke="{PHOS}" stroke-width="2" stroke-dasharray="4 12" opacity=".5"/>')
    p.add(mark(900, 570, "station", "OUTER HOLD", PHOS))
    return p


def the_reading() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(console(130, 110, 1180, 780, "CUTTER ONE // COMMS // GUARD CHANNEL // SIGNAL 12%"))
    p.add(f'<rect x="150" y="170" width="1140" height="270" fill="{INK}" opacity=".8"/>')
    p.add(waveform(160, 300, 1120, 70, seed=35))
    p.add(mono(720, 400, "...ROSTER...", 64, PHOS_BRIGHT, weight=700, spacing=10, anchor="middle", extra=' filter="url(#glow)"'))
    p.add(mono(300, 230, "...MERIDIAN CONTR...", 22, PHOS_DIM, spacing=4), mono(860, 230, "...SECTION N...", 22, PHOS_DIM, spacing=4))
    p.add(mono(260, 430, "...NO REC...", 22, PHOS_DIM, spacing=4, extra=' opacity=".7"'))
    p.add(mark(440, 620, "ship", "CUTTER ONE", PHOS, "INBOUND // OUTER HOLD"))
    p.add(mark(860, 560, "ship", "MERIDIAN", PHOS, "HOLDING // BAY OPEN"))
    p.add(f'<circle cx="1120" cy="720" r="80" fill="#0a1711" stroke="{PALE}" stroke-width="3"/>')
    p.add(mark(1040, 640, "lost", "CONTACT // NO CODE", GOLD, "DRIVE PLUME // CLEARING THE BODY"))
    p.add(f'<path d="M1040,650 L900,580" stroke="{GOLD}" stroke-width="3" stroke-dasharray="8 8" opacity=".8"/>')
    p.add(plot_text(170, 840, [("GUARD CHANNEL: FRAGMENTS. VOICE: NOT CONTROL.", GOLD)], 17))
    return p


def out_of_the_shadow() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(ice(36, 40, 40, 1400, 960, 90))
    p.add(rock(160, 500, 480, 520, seed=37))
    p.add(f'<path d="M560,140 Q700,500 560,860" fill="none" stroke="{PALE}" stroke-width="6" opacity=".35" filter="url(#soft)"/>')
    p.add(shadow_wedge(160, 500, 520))
    p.add(f'<ellipse cx="420" cy="560" rx="260" ry="60" fill="{PHOS}" opacity=".18" filter="url(#soft)"/>')
    p.add(g(900, 520, 1.1, -6, inner=warship(FLEET, lit=False)))
    p.add(f'<path d="M420,480 L1320,410" stroke="{PALE}" stroke-width="2" opacity=".25"/>')
    return p


def the_strike() -> Panel:
    p = Panel(*FULL, space=True)
    p.add(ice(38, 40, 40, 1880, 1040, 80))
    p.add(flash_disc(760, 500, 720))
    p.add(rock(1700, 380, 360, 340, seed=39), shadow_wedge(1700, 380, 340, toward=-1))
    p.add(g(1380, 560, .72, -4, inner=warship(FLEET, fire=True)))
    p.add(bolt(1690, 540, 900, 500), bolt(1690, 580, 940, 560))
    p.add(g(560, 480, .9, -10, inner=meridian(EWI, part="stern", text=False, tag="ms")))
    p.add(g(1000, 420, .9, 24, inner=meridian(EWI, part="bow", text=False, tag="mb")))
    p.add(burst(790, 480, 170, seed=40))
    p.add(junk(41, 300, 200, 1300, 900, 22, "ewi"))
    p.add(junk_field(42, 60, 600, 700, 1040))
    p.add(g(330, 860, 1.1, 8, inner=derelict(SIL, seed=9)))
    p.add(g(300, 940, .9, 8, inner=cutter(SIL, label="", lit=False)))
    return p


def hiding_in_the_junk() -> Panel:
    p = Panel(*WIDE, space=True)
    p.add(ice(43, 40, 40, 1880, 860, 80))
    p.add(g(1500, 300, .55, -20, inner=meridian(SIL, part="stern", lit=False, text=False, tag="hs")))
    p.add(ember(1560, 260, 90), ember(1420, 330, 60), ember(1680, 380, 40))
    p.add(beacon_rings(1520, 300, 4, 70))
    p.add(junk(44, 900, 100, 1900, 700, 24, "ewi"))
    p.add(junk_field(45, 60, 60, 1200, 860))
    p.add(g(1200, 180, 1.0, 20, inner=skiff(1)), g(1500, 620, 1.0, 160, inner=skiff(2)), g(980, 760, 1.0, -30, inner=skiff(3)))
    p.add(g(420, 560, 1.9, -8, inner=derelict(SIL, seed=11)))
    p.add(g(430, 700, 1.3, 6, inner=cutter(dict(SIL, hull="#0f1d15", light="#1a2c20", edge="#2a4a36"), label="", lit=False)))
    return p


def halloran_face() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(cabin(1440, 1000, seed=46))
    p.add(g(720, 430, 1.0, inner=face(**HALLORAN)))
    return p


def okoro_face() -> Panel:
    p = Panel(*HALF, space=True)
    p.add(cabin(1440, 1000, seed=47))
    p.add(g(720, 430, 1.0, inner=face(**OKORO)))
    return p


PANELS = {
    "cover.svg": cover,
    "shelter-wide.svg": shelter_wide,
    "meridian-on-station.svg": meridian_on_station,
    "survey-plot.svg": survey_plot,
    "tender-at-the-charge.svg": tender_at_the_charge,
    "charge-armed.svg": charge_armed,
    "hold-at-the-site.svg": hold_at_the_site,
    "calloway-bridge.svg": calloway_bridge,
    "the-blast.svg": the_blast,
    "boat-deck.svg": boat_deck,
    "pell-refuses.svg": pell_refuses,
    "brandt-fits-the-boat.svg": brandt_fits_the_boat,
    "tender-toward-the-shelter.svg": tender_toward_the_shelter,
    "signal-lost.svg": signal_lost,
    "the-plaque.svg": the_plaque,
    "junk-site.svg": junk_site,
    "boat-bay-shift.svg": boat_bay_shift,
    "cutter-released.svg": cutter_released,
    "handling-card.svg": handling_card,
    "crates-on-plate-seven.svg": crates_on_plate_seven,
    "the-donut.svg": the_donut,
    "demir-at-control.svg": demir_at_control,
    "third-crate-home.svg": third_crate_home,
    "the-reading.svg": the_reading,
    "out-of-the-shadow.svg": out_of_the_shadow,
    "the-strike.svg": the_strike,
    "hiding-in-the-junk.svg": hiding_in_the_junk,
    "halloran.svg": halloran_face,
    "okoro.svg": okoro_face,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, make in PANELS.items():
        (OUT / name).write_text(make().svg())
        print(name)


if __name__ == "__main__":
    main()
