#!/usr/bin/env python3
"""Generate the Nova Protocol campaign's green CRT speaker portraits."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE_DIR = ROOT / "art" / "portraits"
ASSET_DIR = ROOT / "assets" / "base" / "portraits"

BG = "#030d08"
PANEL = "#06170f"
GRID = "#164c31"
GREEN = "#55d68b"
BRIGHT = "#c0ffd4"
DARK = "#183426"
AMBER = "#c2a23b"
AMBER_DARK = "#796424"


def rect(x: int, y: int, w: int, h: int, fill: str) -> str:
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}"/>'


def polygon(points: str, fill: str) -> str:
    return f'<polygon points="{points}" fill="{fill}"/>'


def frame() -> list[str]:
    shapes = [
        rect(0, 0, 32, 32, BG),
        rect(1, 1, 30, 30, GREEN),
        rect(2, 2, 28, 28, PANEL),
        rect(3, 3, 26, 26, BG),
    ]
    for y in range(4, 29, 4):
        shapes.append(rect(3, y, 26, 1, GRID))
    shapes.extend([rect(3, 3, 6, 1, AMBER), rect(23, 28, 6, 1, GREEN)])
    return shapes


def person(
    skin: str,
    shadow: str,
    hair: str,
    uniform: str,
    uniform_shadow: str,
    eyes: str = BRIGHT,
) -> list[str]:
    return [
        rect(6, 25, 20, 4, uniform_shadow),
        rect(8, 22, 16, 6, uniform),
        polygon("8,22 12,20 20,20 24,22 21,25 11,25", uniform),
        rect(13, 19, 6, 4, shadow),
        rect(11, 8, 10, 12, skin),
        rect(10, 10, 2, 7, shadow),
        rect(20, 10, 2, 7, shadow),
        rect(12, 11, 3, 2, DARK),
        rect(17, 11, 3, 2, DARK),
        rect(13, 11, 1, 1, eyes),
        rect(18, 11, 1, 1, eyes),
        rect(15, 13, 2, 4, shadow),
        rect(14, 17, 4, 1, DARK),
        rect(15, 18, 3, 1, skin),
        rect(11, 7, 10, 2, hair),
    ]


def control() -> list[str]:
    s = frame() + person("#7f7650", "#555b3c", "#253f30", "#347951", "#245c3d")
    s += [
        rect(10, 7, 3, 4, "#253f30"),
        rect(19, 7, 3, 5, "#253f30"),
        rect(8, 9, 2, 8, GREEN),
        rect(22, 9, 2, 7, GREEN),
        rect(23, 15, 4, 1, GREEN),
        rect(26, 15, 1, 3, BRIGHT),
        rect(12, 24, 8, 1, BRIGHT),
    ]
    return s


def deck_chief() -> list[str]:
    s = frame() + person("#8f8656", "#5d603e", "#b7dfc3", AMBER, AMBER_DARK)
    s += [
        rect(12, 6, 8, 2, "#b7dfc3"),
        rect(10, 7, 3, 5, "#73977f"),
        rect(20, 7, 3, 5, "#73977f"),
        rect(7, 9, 2, 9, GREEN),
        rect(23, 9, 2, 7, GREEN),
        rect(24, 15, 3, 1, GREEN),
        rect(26, 15, 1, 3, BRIGHT),
        rect(7, 26, 3, 1, "#b8da5d"),
        rect(22, 26, 3, 1, "#b8da5d"),
    ]
    return s


def copilot() -> list[str]:
    s = frame() + person("#9b8e5c", "#686344", "#24382b", "#398763", "#20543b")
    s += [
        polygon("10,8 12,5 21,6 22,9", "#24382b"),
        rect(9, 8, 3, 6, "#24382b"),
        rect(21, 9, 2, 5, "#24382b"),
        rect(8, 10, 2, 7, GREEN),
        rect(22, 10, 2, 6, GREEN),
        rect(23, 15, 4, 1, GREEN),
        rect(26, 15, 1, 2, BRIGHT),
        rect(14, 24, 4, 3, DARK),
    ]
    return s


def engineer() -> list[str]:
    s = frame() + person("#766c48", "#4e5035", "#1a3024", "#a78a2e", "#6d5c24")
    s += [
        rect(10, 6, 12, 3, "#1a3024"),
        rect(9, 8, 3, 5, "#1a3024"),
        rect(20, 8, 3, 5, "#1a3024"),
        rect(11, 8, 4, 2, "#315e44"),
        rect(17, 8, 4, 2, "#315e44"),
        rect(12, 8, 2, 1, BRIGHT),
        rect(18, 8, 2, 1, BRIGHT),
        rect(15, 8, 2, 1, GREEN),
        rect(9, 23, 3, 5, "#347951"),
        rect(20, 23, 3, 5, "#347951"),
        rect(12, 24, 8, 1, AMBER),
    ]
    return s


def player() -> list[str]:
    s = frame()
    s += [
        rect(7, 25, 18, 4, "#245c3d"),
        polygon("8,24 11,20 21,20 24,24 22,28 10,28", "#347951"),
        polygon("10,8 13,5 19,5 22,8 23,18 20,22 12,22 9,18", "#8fb69b"),
        polygon("11,9 13,7 19,7 21,9 21,16 19,19 13,19 11,16", "#0b2b1c"),
        rect(12, 10, 8, 5, "#123f2d"),
        polygon("12,10 20,10 18,12 13,12", GREEN),
        rect(13, 11, 4, 1, BRIGHT),
        rect(9, 11, 2, 6, GREEN),
        rect(21, 11, 2, 6, GREEN),
        rect(14, 22, 4, 5, DARK),
    ]
    return s


def beacon() -> list[str]:
    s = frame()
    s += [
        polygon("16,5 26,16 16,27 6,16", "#245c3d"),
        polygon("16,7 24,16 16,25 8,16", GREEN),
        polygon("16,10 21,16 16,22 11,16", BG),
        rect(15, 11, 2, 7, BRIGHT),
        rect(15, 20, 2, 2, BRIGHT),
        rect(4, 15, 4, 1, AMBER),
        rect(24, 15, 4, 1, AMBER),
        rect(15, 3, 2, 4, AMBER),
        rect(15, 25, 2, 4, AMBER),
    ]
    return s


def unknown() -> list[str]:
    s = frame()
    s += [
        polygon("8,24 11,9 16,5 21,9 24,24", "#123122"),
        polygon("11,11 16,7 21,11 20,20 12,20", "#06100b"),
        rect(12, 12, 8, 2, "#1b4b32"),
        rect(13, 13, 2, 1, GREEN),
        rect(18, 13, 1, 1, GREEN),
        rect(9, 23, 14, 5, "#1d4932"),
        rect(4, 8, 9, 1, BRIGHT),
        rect(18, 10, 10, 1, GREEN),
        rect(5, 17, 7, 1, GREEN),
        rect(19, 19, 8, 1, BRIGHT),
        rect(7, 26, 5, 1, AMBER),
        rect(21, 6, 6, 1, AMBER),
    ]
    return s


PORTRAITS = {
    "meridian-control": control,
    "deck-chief": deck_chief,
    "copilot": copilot,
    "engineer": engineer,
    "player": player,
    "automated-beacon": beacon,
    "unknown-channel": unknown,
}


def svg(shapes: list[str]) -> str:
    body = "\n  ".join(shapes)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" '
        'viewBox="0 0 32 32" shape-rendering="crispEdges">\n  '
        + body
        + "\n</svg>\n"
    )


def main() -> None:
    SOURCE_DIR.mkdir(parents=True, exist_ok=True)
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    for name, build in PORTRAITS.items():
        source = SOURCE_DIR / f"{name}.svg"
        output = ASSET_DIR / f"{name}.png"
        source.write_text(svg(build()), encoding="utf-8")
        subprocess.run(
            ["magick", "-background", "none", str(source), str(output)],
            check=True,
        )
        print(output.relative_to(ROOT))


if __name__ == "__main__":
    main()
