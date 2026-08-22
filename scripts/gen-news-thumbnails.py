#!/usr/bin/env python3
"""Generate branded archive art for News cards that have no surviving capture.

The old posts are historical and will not be re-shot with the current game.
These deterministic cards give the index intentional art without pretending a
new screenshot came from an old release.

Run from the repository root:

    python3 scripts/gen-news-thumbnails.py
    python3 scripts/gen-news-thumbnails.py --check
"""

import argparse
import math
import os
import random
import struct
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "web", "src", "assets")
WIDTH, HEIGHT = 1200, 675
VERSIONS = ("0.3.0", "0.4.0", "0.5.0", "0.6.0", "0.7.0", "0.8.0", "0.9.0")


def chunk(tag, data):
    body = tag + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def encode_png(pixels):
    stride = WIDTH * 4
    raw = bytearray()
    for y in range(HEIGHT):
        raw.append(0)
        raw.extend(pixels[y * stride : (y + 1) * stride])
    ihdr = struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")


class Canvas:
    def __init__(self, version):
        minor = int(version.split(".")[1])
        self.rng = random.Random(0x4E4F5641 + minor)
        self.pixels = bytearray(WIDTH * HEIGHT * 4)
        for y in range(HEIGHT):
            horizon = math.exp(-((y - HEIGHT * 0.82) / (HEIGHT * 0.16)) ** 2)
            for x in range(WIDTH):
                edge = abs(x - WIDTH / 2) / (WIDTH / 2)
                glow = horizon * max(0.0, 1.0 - edge * edge)
                i = (y * WIDTH + x) * 4
                self.pixels[i : i + 4] = bytes(
                    (
                        int(3 + 18 * glow),
                        int(8 + 8 * glow),
                        int(15 + 5 * glow),
                        255,
                    )
                )

    def point(self, x, y, color, radius=1):
        for py in range(y - radius, y + radius + 1):
            if not 0 <= py < HEIGHT:
                continue
            for px in range(x - radius, x + radius + 1):
                if not 0 <= px < WIDTH:
                    continue
                i = (py * WIDTH + px) * 4
                self.pixels[i : i + 4] = bytes((*color, 255))

    def line(self, x0, y0, x1, y1, color, width=1):
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
        error = dx + dy
        while True:
            self.point(x0, y0, color, width - 1)
            if x0 == x1 and y0 == y1:
                break
            twice = 2 * error
            if twice >= dy:
                error += dy
                x0 += sx
            if twice <= dx:
                error += dx
                y0 += sy

    def ellipse(self, cx, cy, rx, ry, color, start=0.0, end=math.tau):
        steps = max(80, int((rx + ry) * (end - start) / 8))
        previous = None
        for step in range(steps + 1):
            angle = start + (end - start) * step / steps
            point = (round(cx + math.cos(angle) * rx), round(cy + math.sin(angle) * ry))
            if previous is not None:
                self.line(*previous, *point, color)
            previous = point

    def ship(self, cx, cy, scale, flip, color):
        direction = -1 if flip else 1
        nose = (cx + direction * int(95 * scale), cy)
        tail_top = (cx - direction * int(65 * scale), cy - int(38 * scale))
        tail_bottom = (cx - direction * int(65 * scale), cy + int(38 * scale))
        self.line(*nose, *tail_top, color, 2)
        self.line(*tail_top, *tail_bottom, color, 2)
        self.line(*tail_bottom, *nose, color, 2)
        self.line(cx - direction * int(20 * scale), cy, *nose, color)
        self.line(cx - direction * int(35 * scale), cy - int(25 * scale), cx + direction * int(18 * scale), cy + int(25 * scale), color)
        plume = (38, 255, 190) if not flip else (255, 139, 54)
        self.line(
            cx - direction * int(65 * scale),
            cy,
            cx - direction * int(108 * scale),
            cy,
            plume,
            2,
        )

    def render(self, version):
        minor = int(version.split(".")[1])
        for _ in range(95):
            x = self.rng.randrange(24, WIDTH - 24)
            y = self.rng.randrange(18, int(HEIGHT * 0.78))
            value = self.rng.randrange(80, 190)
            self.point(x, y, (value // 2, value, value * 3 // 4), 1 if self.rng.random() < 0.08 else 0)

        green = (54, 255, 121)
        dim = (22, 104, 62)
        amber = (255, 165, 62)
        self.ellipse(WIDTH // 2, int(HEIGHT * 0.53), 420, 125, dim)
        self.ellipse(WIDTH // 2, int(HEIGHT * 0.53), 315, 92, (13, 73, 45), 0.2, 5.8)
        for ring in range(1 + minor % 3):
            radius = 72 + ring * 38
            self.ellipse(WIDTH // 2, int(HEIGHT * 0.53), radius * 2, radius // 2, (12, 62 + ring * 12, 42))

        offset = 120 + (minor % 3) * 24
        self.ship(WIDTH // 2 - offset, int(HEIGHT * 0.49), 1.0, False, green)
        self.ship(WIDTH // 2 + offset, int(HEIGHT * 0.57), 0.82, True, amber)
        self.line(WIDTH // 2 - 28, int(HEIGHT * 0.49), WIDTH // 2 + 48, int(HEIGHT * 0.56), (150, 231, 190))

        # A release-coded telemetry comb. The visible title below the card owns
        # the text; these bars make each generated card distinct without a font
        # dependency in the build script.
        baseline = HEIGHT - 72
        for index in range(minor):
            x = 58 + index * 24
            height = 18 + ((minor * 17 + index * 11) % 44)
            self.line(x, baseline, x, baseline - height, green, 2)
        self.line(52, baseline, 52 + max(1, minor) * 24, baseline, dim)
        return self.pixels


def generate(version):
    return encode_png(Canvas(version).render(version))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if committed cards differ")
    args = parser.parse_args()
    changed = []
    for version in VERSIONS:
        name = f"thumb-news-{version}.png"
        path = os.path.join(OUT, name)
        expected = generate(version)
        current = open(path, "rb").read() if os.path.exists(path) else None
        if current != expected:
            changed.append(name)
            if not args.check:
                with open(path, "wb") as handle:
                    handle.write(expected)
                print(f"wrote {name}")
    if args.check and changed:
        print("generated news thumbnails differ: " + ", ".join(changed), file=sys.stderr)
        return 1
    if args.check:
        print(f"news thumbnails OK: {len(VERSIONS)} deterministic cards")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
