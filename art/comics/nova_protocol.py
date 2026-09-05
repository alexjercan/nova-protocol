#!/usr/bin/env python3
"""Panel art for the Nova Protocol comic, prologue "The Shelter".

Run from the repository root:

    python3 art/comics/nova_protocol.py

It writes the SVG assets the comic pages name into
web/src/assets/story/nova-protocol/. Wide panels are 1920x900, half panels
1440x1000, the cover and the bleed page 1920x1080. The comic engine crops
each image to its panel, so every subject sits inside the central band.
"""

from __future__ import annotations

import pathlib

from crt import (EWI, GOLD, GRID, INK, KESTREL, LIME, PALE, PHOS, PHOS_BRIGHT, PHOS_DIM, PHOS_TEXT, SIL, SUIT_EDGE, Panel,
                 banner, bay, bolt, burst, charge, claim_line, console, deck_hand, ember, eva_crew, face, flash_disc, g,
                 harrier, helmet, ice, ice_slab, junk, lamp_pool, mark, meridian, mono, plaque, plot_text, rings, rock,
                 station, station_wreck, tender)

OUT = pathlib.Path(__file__).resolve().parents[2] / "web" / "src" / "assets" / "story" / "nova-protocol"

WIDE = (1920, 900)
HALF = (1440, 1000)
FULL = (1920, 1080)

CALLOWAY = dict(skin="#a08a62", shadow="#5a4c34", light="#c7b489", hair="#4a4a3c", style="neat", headset="slim",
                uniform="#3e4a3c", uniform_dark="#22301f", collar="officer", jaw="narrow", age=.5, mouth="flat")
PELL = dict(skin="#8b7350", shadow="#4a3c28", light="#b09a72", hair="#2e2c22", style="cropped", headset="deck",
            uniform="#6d6845", uniform_dark="#3e4130", collar="deck", jaw="wide", age=.8, stubble=True, heavy_brow=True,
            mouth="set", helmet_ring=True, glow_below=False)


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
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, make in PANELS.items():
        (OUT / name).write_text(make().svg())
        print(name)


if __name__ == "__main__":
    main()
