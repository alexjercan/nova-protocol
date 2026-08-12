#!/usr/bin/env python3
"""Bake a palette-atlas textured .obj down to true flat `Kd` materials.

Palette-atlas packs (Quaternius, KayKit, newer Kenney kits) export ONE grey
`Kd` and keep every colour in a UV texture - `cut-obj-into-parts.py` reads
colours from `Kd` only, so cutting such a pack yields colourless parts (the
trap recorded in tasks/20260812-100256/SPIKE.md). This script closes that gap:
it samples the atlas under each face, quantizes the samples into a small
palette, and writes a new OBJ+MTL whose materials carry those colours as flat
`Kd` values - the same shape as a born-flat pack like the Fertile Soil blocks,
ready for the part cutter.

Method, per face: sample the texture on a fixed barycentric lattice inside
each triangle and take the per-channel MEDIAN - painted panel lines and decals
are thin, so the median lands on the fill colour the face mostly wears. Face
colours are then clustered (area-weighted k-means, deterministic seeding from
a coarse histogram; clusters closer than a merge threshold collapse), so a
whole ship resolves to a handful of materials instead of one per face.

Colour space: the atlas PNG is sRGB, but the downstream cutter copies `Kd`
floats straight into glTF `baseColorFactor`, which bevy reads as LINEAR. The
baked `Kd` values are therefore written linear (sRGB decoded), so the baked
ship renders in the colours the textured original shows. Material names carry
the sRGB hex (`kd_8a3c2f`) so a human can still read the palette.

Stdlib-only and standalone like the other asset scripts: the PNG decoder
handles the non-interlaced 8-bit RGB/RGBA subset these packs use. The run
verifies its own output: every input face must come out again (count checked
by re-parsing the written OBJ), every emitted material must be referenced, and
the palette is printed with per-colour face counts and area shares.

Usage:
  bake-atlas-to-kd.py ship.obj --texture Textures/Ship_Red.png --out baked/ship.obj
  bake-atlas-to-kd.py --self-test
"""

from __future__ import annotations

import argparse
import math
import os
import struct
import sys
import zlib
from collections import defaultdict

# ---------------------------------------------------------------------------
# PNG decoding (non-interlaced 8-bit RGB / RGBA - the palette-atlas subset)
# ---------------------------------------------------------------------------


class Png:
    __slots__ = ("width", "height", "channels", "rows")

    def __init__(self, width, height, channels, rows):
        self.width = width
        self.height = height
        self.channels = channels
        self.rows = rows

    def sample(self, u, v):
        """sRGB (r, g, b) bytes at texture coordinate (u, v), OBJ convention
        (v up, origin bottom-left), repeat-wrapped."""
        u = u - math.floor(u)
        w = 1.0 - v
        w = w - math.floor(w)
        x = min(self.width - 1, int(u * self.width))
        y = min(self.height - 1, int(w * self.height))
        row = self.rows[y]
        i = x * self.channels
        return (row[i], row[i + 1], row[i + 2])


def _paeth(a, b, c):
    p = a + b - c
    pa = abs(p - a)
    pb = abs(p - b)
    pc = abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def read_png(path):
    with open(path, "rb") as handle:
        data = handle.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{path}: not a PNG")
    pos = 8
    width = height = None
    channels = None
    idat = bytearray()
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, depth, colour, _comp, _filt, interlace = struct.unpack(
                ">IIBBBBB", body
            )
            if depth != 8 or colour not in (2, 6) or interlace != 0:
                raise SystemExit(
                    f"{path}: unsupported PNG (need non-interlaced 8-bit RGB/RGBA, "
                    f"got depth {depth} colour-type {colour} interlace {interlace})"
                )
            channels = 3 if colour == 2 else 4
        elif kind == b"IDAT":
            idat.extend(body)
        elif kind == b"IEND":
            break
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    if len(raw) != (stride + 1) * height:
        raise SystemExit(f"{path}: PNG payload size mismatch")
    rows = []
    prev = bytearray(stride)
    bpp = channels
    for y in range(height):
        offset = y * (stride + 1)
        ftype = raw[offset]
        row = bytearray(raw[offset + 1 : offset + 1 + stride])
        if ftype == 1:  # Sub
            for i in range(bpp, stride):
                row[i] = (row[i] + row[i - bpp]) & 0xFF
        elif ftype == 2:  # Up
            for i in range(stride):
                row[i] = (row[i] + prev[i]) & 0xFF
        elif ftype == 3:  # Average
            for i in range(bpp):
                row[i] = (row[i] + prev[i] // 2) & 0xFF
            for i in range(bpp, stride):
                row[i] = (row[i] + (row[i - bpp] + prev[i]) // 2) & 0xFF
        elif ftype == 4:  # Paeth
            for i in range(bpp):
                row[i] = (row[i] + prev[i]) & 0xFF
            for i in range(bpp, stride):
                row[i] = (row[i] + _paeth(row[i - bpp], prev[i], prev[i - bpp])) & 0xFF
        elif ftype != 0:
            raise SystemExit(f"{path}: unknown PNG filter {ftype} on row {y}")
        rows.append(bytes(row))
        prev = row
    return Png(width, height, channels, rows)


def write_png(path, width, height, pixels):
    """Minimal RGB writer for the self-test (filter 0 rows)."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(pixels[y][x])

    def chunk(kind, body):
        payload = kind + body
        return (
            struct.pack(">I", len(body)) + payload + struct.pack(">I", zlib.crc32(payload))
        )

    with open(path, "wb") as handle:
        handle.write(b"\x89PNG\r\n\x1a\n")
        handle.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)))
        handle.write(chunk(b"IDAT", zlib.compress(bytes(raw))))
        handle.write(chunk(b"IEND", b""))


# ---------------------------------------------------------------------------
# OBJ parsing (positions + texcoords per face corner; lines kept for rewrite)
# ---------------------------------------------------------------------------


class Face:
    __slots__ = ("line_index", "corners")

    def __init__(self, line_index, corners):
        self.line_index = line_index
        # [(position, uv)] per corner, in file order.
        self.corners = corners


def parse_obj(path):
    """(all file lines, faces). A face without texture coordinates is an
    error - there is nothing to bake from."""
    positions = []
    texcoords = []
    faces = []
    with open(path, "r", encoding="utf-8") as handle:
        lines = handle.read().splitlines()
    for index, line in enumerate(lines):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "v":
            positions.append(tuple(float(x) for x in parts[1:4]))
        elif parts[0] == "vt":
            texcoords.append((float(parts[1]), float(parts[2])))
        elif parts[0] == "f":
            corners = []
            for token in parts[1:]:
                pieces = token.split("/")
                v = int(pieces[0])
                v = v - 1 if v > 0 else len(positions) + v
                if len(pieces) < 2 or not pieces[1]:
                    raise SystemExit(
                        f"{path}: face on line {index + 1} has no texture "
                        "coordinates - nothing to bake from"
                    )
                t = int(pieces[1])
                t = t - 1 if t > 0 else len(texcoords) + t
                corners.append((positions[v], texcoords[t]))
            faces.append(Face(index, corners))
    return lines, faces


# ---------------------------------------------------------------------------
# Sampling + quantization
# ---------------------------------------------------------------------------

# Barycentric lattice: 6 interior points + the centroid, fixed and symmetric.
_LATTICE = [
    (1.0 / 3.0, 1.0 / 3.0),
    (1.0 / 6.0, 1.0 / 6.0),
    (4.0 / 6.0, 1.0 / 6.0),
    (1.0 / 6.0, 4.0 / 6.0),
    (3.0 / 6.0, 1.0 / 6.0),
    (1.0 / 6.0, 3.0 / 6.0),
    (2.0 / 6.0, 2.0 / 6.0),
]


def _srgb_to_linear(c):
    c = c / 255.0
    if c <= 0.04045:
        return c / 12.92
    return ((c + 0.055) / 1.055) ** 2.4


def _linear_to_srgb_byte(c):
    c = max(0.0, min(1.0, c))
    if c <= 0.0031308:
        c = c * 12.92
    else:
        c = 1.055 * (c ** (1.0 / 2.4)) - 0.055
    return round(c * 255.0)


def _tri_area(a, b, c):
    ux, uy, uz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    vx, vy, vz = c[0] - a[0], c[1] - a[1], c[2] - a[2]
    nx = uy * vz - uz * vy
    ny = uz * vx - ux * vz
    nz = ux * vy - uy * vx
    return 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)


def face_colour(face, png):
    """(median sRGB colour, 3D area) for one face, fan-triangulated."""
    samples = []
    area = 0.0
    corners = face.corners
    for i in range(1, len(corners) - 1):
        (pa, ta), (pb, tb), (pc, tc) = corners[0], corners[i], corners[i + 1]
        area += _tri_area(pa, pb, pc)
        for s, t in _LATTICE:
            r = 1.0 - s - t
            u = r * ta[0] + s * tb[0] + t * tc[0]
            v = r * ta[1] + s * tb[1] + t * tc[1]
            samples.append(png.sample(u, v))
    mid = len(samples) // 2
    median = tuple(sorted(sample[ch] for sample in samples)[mid] for ch in range(3))
    return median, area


def quantize(face_colours, max_colours, merge_distance):
    """Cluster (sRGB colour, weight) pairs; returns (assignments, palette)
    where palette is [(sRGB colour, weight)] and assignments maps each input
    index to a palette index. Deterministic: seeds come from a coarse
    histogram by greedy farthest-point, k-means refines, near-identical
    clusters merge."""
    bins = defaultdict(float)
    for colour, weight in face_colours:
        bins[(colour[0] >> 4, colour[1] >> 4, colour[2] >> 4)] += weight
    candidates = sorted(bins.items(), key=lambda kv: (-kv[1], kv[0]))[:64]
    centres = []
    for _ in range(min(max_colours, len(candidates))):
        best = None
        best_score = -1.0
        for (bin_key, weight) in candidates:
            colour = tuple(c * 16 + 8 for c in bin_key)
            if not centres:
                score = weight
            else:
                score = min(
                    sum((colour[ch] - centre[ch]) ** 2 for ch in range(3))
                    for centre in centres
                )
            if score > best_score:
                best_score = score
                best = colour
        if best is None or (centres and best_score == 0.0):
            break
        centres.append(best)

    assignments = [0] * len(face_colours)
    for _ in range(24):
        sums = [[0.0, 0.0, 0.0, 0.0] for _ in centres]
        changed = False
        for index, (colour, weight) in enumerate(face_colours):
            nearest = min(
                range(len(centres)),
                key=lambda k: sum((colour[ch] - centres[k][ch]) ** 2 for ch in range(3)),
            )
            if assignments[index] != nearest:
                assignments[index] = nearest
                changed = True
            bucket = sums[nearest]
            bucket[0] += colour[0] * weight
            bucket[1] += colour[1] * weight
            bucket[2] += colour[2] * weight
            bucket[3] += weight
        centres = [
            (bucket[0] / bucket[3], bucket[1] / bucket[3], bucket[2] / bucket[3])
            if bucket[3] > 0.0
            else centres[k]
            for k, bucket in enumerate(sums)
        ]
        if not changed:
            break

    # Merge clusters closer than the threshold (weight-preserving), then drop
    # empties. Mapping stays index-stable via `remap`.
    weights = [0.0] * len(centres)
    for index, (_, weight) in enumerate(face_colours):
        weights[assignments[index]] += weight
    order = sorted(range(len(centres)), key=lambda k: -weights[k])
    remap = {}
    merged = []  # [(colour sums, weight)]
    for k in order:
        if weights[k] == 0.0:
            continue
        target = None
        for m, (mc, mw) in enumerate(merged):
            dist = math.sqrt(
                sum((centres[k][ch] - mc[ch] / mw) ** 2 for ch in range(3))
            )
            if dist < merge_distance:
                target = m
                break
        if target is None:
            merged.append(
                (
                    [centres[k][ch] * weights[k] for ch in range(3)],
                    weights[k],
                )
            )
            remap[k] = len(merged) - 1
        else:
            mc, mw = merged[target]
            for ch in range(3):
                mc[ch] += centres[k][ch] * weights[k]
            merged[target] = (mc, mw + weights[k])
            remap[k] = target
    palette = [
        (tuple(mc[ch] / mw for ch in range(3)), mw) for mc, mw in merged
    ]
    assignments = [remap[a] for a in assignments]
    return assignments, palette


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------


def material_name(colour):
    return "kd_{:02x}{:02x}{:02x}".format(*(round(c) for c in colour))


def write_baked(out_obj, lines, faces, assignments, palette, source_note):
    out_mtl = os.path.splitext(out_obj)[0] + ".mtl"
    os.makedirs(os.path.dirname(os.path.abspath(out_obj)), exist_ok=True)

    names = [material_name(colour) for colour, _ in palette]
    with open(out_mtl, "w", encoding="utf-8") as handle:
        handle.write(f"# Baked flat-Kd palette - {source_note}\n")
        handle.write("# Kd is LINEAR (sRGB-decoded); names carry the sRGB hex.\n")
        for (colour, _weight), name in zip(palette, names):
            r, g, b = (_srgb_to_linear(c) for c in colour)
            handle.write(f"\nnewmtl {name}\n")
            handle.write(f"Kd {r:.6f} {g:.6f} {b:.6f}\n")

    material_of = {face.line_index: names[assignments[i]] for i, face in enumerate(faces)}
    with open(out_obj, "w", encoding="utf-8") as handle:
        handle.write(f"# Baked from {source_note} by bake-atlas-to-kd.py\n")
        handle.write(f"mtllib {os.path.basename(out_mtl)}\n")
        current = None
        for index, line in enumerate(lines):
            stripped = line.split()
            if stripped and stripped[0] in ("mtllib", "usemtl"):
                continue
            if index in material_of:
                if material_of[index] != current:
                    current = material_of[index]
                    handle.write(f"usemtl {current}\n")
            handle.write(line + "\n")
    return out_mtl


# ---------------------------------------------------------------------------
# Run + verification
# ---------------------------------------------------------------------------


def run(args):
    lines, faces = parse_obj(args.obj)
    if not faces:
        raise SystemExit(f"{args.obj}: no faces")
    png = read_png(args.texture)
    print(
        f"bake: {args.obj} ({len(faces)} faces) x {args.texture} "
        f"({png.width}x{png.height})"
    )

    face_colours = [face_colour(face, png) for face in faces]
    assignments, palette = quantize(face_colours, args.colors, args.merge_distance)

    source_note = f"{os.path.basename(args.obj)} + {os.path.basename(args.texture)}"
    out_mtl = write_baked(args.out, lines, faces, assignments, palette, source_note)

    # Verify: the written OBJ must parse back with the same face count, and
    # every palette entry must be worn by at least one face.
    _lines, refaces = parse_obj(args.out)
    if len(refaces) != len(faces):
        raise SystemExit(
            f"VERIFY FAILED: {args.out} has {len(refaces)} faces, input had {len(faces)}"
        )
    worn = set(assignments)
    missing = [k for k in range(len(palette)) if k not in worn]
    if missing:
        raise SystemExit(f"VERIFY FAILED: unworn palette entries {missing}")

    total_area = sum(weight for _, weight in palette) or 1.0
    counts = defaultdict(int)
    for a in assignments:
        counts[a] += 1
    print(f"palette ({len(palette)} colours) -> {args.out} + {os.path.basename(out_mtl)}")
    for k, (colour, weight) in enumerate(palette):
        print(
            f"  {material_name(colour)}  faces {counts[k]:5d}  "
            f"area {100.0 * weight / total_area:5.1f}%"
        )
    return 0


def self_test():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        # A 4x4 atlas: left half red, right half blue, one green texel that
        # sampling must NOT pick up as a palette colour (it covers no face).
        red, blue, green = (200, 40, 30), (20, 60, 220), (0, 255, 0)
        pixels = [
            [red, red, blue, blue],
            [red, red, blue, blue],
            [red, red, blue, blue],
            [red, green, blue, blue],
        ]
        atlas = os.path.join(tmp, "atlas.png")
        write_png(atlas, 4, 4, pixels)
        decoded = read_png(atlas)
        assert decoded.sample(0.1, 0.9) == red, decoded.sample(0.1, 0.9)
        assert decoded.sample(0.9, 0.9) == blue

        # Two quads: one UV-mapped onto the red half, one onto the blue half.
        obj = os.path.join(tmp, "ship.obj")
        with open(obj, "w", encoding="utf-8") as handle:
            handle.write(
                "o test\n"
                "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n"
                "v 2 0 0\nv 3 0 0\nv 3 1 0\nv 2 1 0\n"
                "vt 0.05 0.55\nvt 0.45 0.55\nvt 0.45 0.95\nvt 0.05 0.95\n"
                "vt 0.55 0.55\nvt 0.95 0.55\nvt 0.95 0.95\nvt 0.55 0.95\n"
                "f 1/1 2/2 3/3 4/4\n"
                "f 5/5 6/6 7/7 8/8\n"
            )
        out = os.path.join(tmp, "baked", "ship.obj")
        args = argparse.Namespace(
            obj=obj, texture=atlas, out=out, colors=8, merge_distance=12.0
        )
        run(args)

        with open(os.path.splitext(out)[0] + ".mtl", "r", encoding="utf-8") as handle:
            mtl = handle.read()
        assert mtl.count("newmtl") == 2, mtl
        assert material_name(red) in mtl and material_name(blue) in mtl, mtl
        # Linear Kd: red 200 -> ~0.578, not 0.784.
        expected = _srgb_to_linear(200)
        assert f"{expected:.6f}" in mtl, mtl
        with open(out, "r", encoding="utf-8") as handle:
            baked = handle.read()
        assert baked.count("usemtl") == 2, baked
        # Round-trip: the linear Kd values map back to the sampled sRGB bytes.
        assert _linear_to_srgb_byte(_srgb_to_linear(200)) == 200
    print("self-test OK")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Bake a palette-atlas textured .obj to flat-Kd materials."
    )
    parser.add_argument("obj", nargs="?", help="input .obj path (needs vt per face)")
    parser.add_argument("--texture", help="atlas PNG the OBJ's UVs index into")
    parser.add_argument("--out", help="output .obj path (writes a sibling .mtl)")
    parser.add_argument(
        "--colors",
        type=int,
        default=14,
        help="maximum palette size before merging (default 14)",
    )
    parser.add_argument(
        "--merge-distance",
        type=float,
        default=12.0,
        help="sRGB distance under which clusters merge (default 12)",
    )
    parser.add_argument("--self-test", action="store_true", help="run internal checks and exit")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()
    if not args.obj or not args.texture or not args.out:
        parser.error("obj, --texture and --out are required (or pass --self-test)")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
