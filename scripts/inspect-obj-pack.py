#!/usr/bin/env python3
"""Score candidate .obj asset packs for cut-obj-into-hulls.py compatibility.

Given .obj files, directories, or .zip archives, report per model:

- mesh inventory: vertex/face/object counts, bounding box
- materials: .mtl resolution, flat Kd colours vs texture maps (map_Kd)
- cell counts at candidate --scale values for cut-obj-into-hulls.py (cell=1.0)
- a one-line verdict: GOOD / PARTIAL / POOR pipeline fit

What the cutter actually requires (verified against the repo's known-good
Kenney inputs): a single mesh with flat .mtl Kd colours and no texture maps.
Vertices do NOT need to sit on a grid - the cutter clips triangles at the
cube planes, so any geometry partitions cleanly. "Half-unit grid" in the
Kenney models is proportional (bbox spans, greeble placement), not vertex
quantisation; --scale is an aesthetic choice of how many cube sections the
ship should span. Stdlib only, like the other asset scripts in this repo.
"""

from __future__ import annotations

import argparse
import math
import os
import sys
import tempfile
import zipfile

# Kenney convention is --scale 2.0 (half-unit proportions -> 1.0 cells).
# The sweep brackets it so oversized or miniature packs still get a hint.
SCALES = (1.0, 2.0, 4.0)
# A ship section grid feels right when the longest axis spans a few cells;
# far outside this and the model needs a non-standard scale.
CELL_RANGE = (2, 12)


def parse_obj(path):
    """Return (vertices, face_count, objects, materials, mtl_paths)."""
    vertices = []
    face_count = 0
    objects = set()
    materials = set()
    mtl_paths = []
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            parts = line.split()
            if not parts:
                continue
            tag = parts[0]
            if tag == "v":
                vertices.append(tuple(float(c) for c in parts[1:4]))
            elif tag == "f":
                face_count += 1
            elif tag in ("o", "g"):
                objects.add(" ".join(parts[1:]) or "<anon>")
            elif tag == "usemtl":
                materials.add(parts[1] if len(parts) > 1 else "<anon>")
            elif tag == "mtllib" and len(parts) > 1:
                mtl_paths.append(os.path.join(os.path.dirname(path), parts[1]))
    return vertices, face_count, objects, materials, mtl_paths


def parse_mtl(paths):
    """Return (kd_colours, textured_materials) across the given .mtl files."""
    kd = {}
    textured = set()
    for path in paths:
        if not os.path.exists(path):
            continue
        current = None
        with open(path, "r", encoding="utf-8", errors="replace") as handle:
            for line in handle:
                parts = line.split()
                if not parts:
                    continue
                if parts[0] == "newmtl" and len(parts) > 1:
                    current = parts[1]
                elif parts[0] == "Kd" and current is not None:
                    kd[current] = tuple(float(c) for c in parts[1:4])
                elif parts[0].startswith("map_") and current is not None:
                    textured.add(current)
    return kd, textured


def bbox(vertices):
    lo = [min(v[i] for v in vertices) for i in range(3)]
    hi = [max(v[i] for v in vertices) for i in range(3)]
    return lo, hi


def inspect_obj(path, label=None):
    label = label or path
    vertices, faces, objects, materials, mtl_paths = parse_obj(path)
    if not vertices:
        print(f"== {label}: no vertices, skipped")
        return
    kd, textured = parse_mtl(mtl_paths)
    lo, hi = bbox(vertices)
    dims = tuple(hi[i] - lo[i] for i in range(3))

    print(f"== {label}")
    print(f"   verts {len(vertices)}  faces {faces}  objects {len(objects)}")
    print(
        "   bbox %.3f x %.3f x %.3f  (y %.3f..%.3f)"
        % (dims[0], dims[1], dims[2], lo[1], hi[1])
    )
    mtl_found = any(os.path.exists(p) for p in mtl_paths)
    print(
        f"   materials {len(materials)}  mtl {'found' if mtl_found else 'MISSING'}"
        f"  kd-colours {len(kd)}  textured {len(textured)}"
    )

    scale_hint = None
    for scale in SCALES:
        cells = tuple(max(1, math.ceil(d * scale)) for d in dims)
        in_range = CELL_RANGE[0] <= max(cells) <= CELL_RANGE[1]
        marker = ""
        if scale_hint is None and in_range:
            scale_hint = scale
            marker = "  <- usable"
        print(
            f"   --scale {scale:<4g} -> {cells[0]}x{cells[1]}x{cells[2]} cells"
            f"{marker}"
        )

    issues = []
    if textured:
        issues.append(f"{len(textured)} textured material(s)")
    if not mtl_found:
        issues.append("no resolvable .mtl (flat colours unavailable)")
    elif len(kd) <= 1:
        # Quaternius-style packs export one grey Kd and keep the real colours
        # in a UV-mapped atlas PNG the .mtl never references. Cuttable, but the
        # cut output loses all colour unless the atlas is baked to materials.
        issues.append("<=1 Kd colour: palette-atlas pack likely, colours in UV texture")
    if len(objects) > 1:
        issues.append(f"{len(objects)} objects (cutter treats as one mesh)")
    if scale_hint is None:
        issues.append(
            "no swept --scale yields %d-%d cells; pick one from bbox" % CELL_RANGE
        )

    if textured or not mtl_found:
        verdict = "POOR"
    elif issues:
        verdict = "PARTIAL"
    else:
        verdict = "GOOD"
    detail = "; ".join(issues) if issues else f"cuttable at --scale {scale_hint:g}"
    print(f"   {verdict}: {detail}")
    print()


def iter_objs(target):
    """Yield (path, label) for every .obj reachable from target."""
    if os.path.isfile(target) and target.lower().endswith(".obj"):
        yield target, target
    elif os.path.isfile(target) and target.lower().endswith(".zip"):
        with tempfile.TemporaryDirectory(prefix="inspect-obj-") as tmp:
            with zipfile.ZipFile(target) as archive:
                archive.extractall(tmp)
            base = os.path.basename(target)
            for root, _dirs, files in os.walk(tmp):
                for name in sorted(files):
                    if name.lower().endswith(".obj"):
                        path = os.path.join(root, name)
                        yield path, f"{base}:{os.path.relpath(path, tmp)}"
    elif os.path.isdir(target):
        for root, _dirs, files in os.walk(target):
            for name in sorted(files):
                if name.lower().endswith(".obj"):
                    path = os.path.join(root, name)
                    yield path, os.path.relpath(path, target)
    else:
        print(f"skip {target}: not an .obj, .zip, or directory", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("targets", nargs="+", help=".obj files, dirs, or zips")
    args = parser.parse_args()
    for target in args.targets:
        for path, label in iter_objs(target):
            inspect_obj(path, label)


if __name__ == "__main__":
    main()
