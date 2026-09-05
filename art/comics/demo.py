#!/usr/bin/env python3
"""Poster art for the reader demo comic: one 1920x1080 cover.

Run from the repository root:

    python3 art/comics/demo.py

It writes web/src/assets/story/demo/cover.svg. The demo holds the Story
archive's place while the story is written, so this is the only panel.
"""

from __future__ import annotations

import pathlib

from crt import EWI, PHOS, Panel, cutter, g, ice, rings, rock, station, tag

OUT = pathlib.Path(__file__).resolve().parents[2] / "web" / "src" / "assets" / "story" / "demo"

FULL = (1920, 1080)


def poster() -> Panel:
    p = Panel(*FULL, grid=True)
    p.add(rings(1920, 300, -7), ice(1, 40, 40, 1880, 1040, 170))
    p.add(g(1330, 640, .95, inner=rock(0, 250, 620, 250) + station(name="STATION")))
    p.add(g(600, 520, .8, 8, inner=cutter(EWI, "DEMO ONE")))
    p.add(tag(120, 120, "READER DEMO", PHOS))
    return p


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "cover.svg").write_text(poster().svg())
    print("cover.svg")


if __name__ == "__main__":
    main()
