#!/usr/bin/env python3
"""Generate thruster SHELL candidates (.glb) from JSON recipes.

Art candidates for the thruster-shell spike (tasks/20260816-200255/THRUSTERS.md
section 2.5; candidates task 20260817-013639): recipe-generated drive shells
(1x1 and larger display formats) in the cross-faction mechanical voice,
judged in the `thruster_gallery` example. CANDIDATES ONLY - the output lands under
`art/part-candidates/shells/` (never shipped, like every other candidate);
promotion into `assets/` with a `render_mesh` on the prototype is the
follow-up, not this script.

A sibling of `gen-greebles.py` rather than more greeble recipes, because the
FRAME is different while the doctrine is the same. A greeble is a half-cell
fixture standing on a mount plane (+Y out, y = 0 face, footprint/height
budgets); a shell is a whole-cell PART: authored centred on the unit cell,
exhaust face at +Z (the bell opens the way `exit_normal` says the shipped
drive fires), flanks meant to run flat to the cell boundary so the skin can
clad them. Every greeble budget check would be wrong for that shape, so this
script owns the cell-frame checks and imports the greeble script's primitive
and recipe layer instead of copying it - one primitive vocabulary, two frames.

Run from anywhere (paths are resolved from this file):

    python3 scripts/gen-thruster-shells.py             # write every .glb
    python3 scripts/gen-thruster-shells.py --check     # verify, write nothing
    python3 scripts/gen-thruster-shells.py --self-test # internal checks, no I/O

`--check` rebuilds every shell in memory and compares it byte for byte with
the committed file - the same CI-checkable determinism contract the greebles
carry, and the reason the recipes rather than the binaries are the source.
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
RECIPE_DIR = os.path.join(SCRIPT_DIR, "thruster-shell-recipes")
OUT_DIR = os.path.join(REPO_ROOT, "art", "part-candidates", "shells")

GENERATOR = "gen-thruster-shells"

# A shell fills a WxHxL box of cells (default one), centred on the origin,
# exhaust at +Z. Nothing may leave the box - a proud lip would collide with
# the neighbour cell the skin expects to own. A recipe names a bigger box
# with `"cells": [w, h, l]`; the big formats are ART CANDIDATES only
# (multi-cell sections are parked in THRUSTERS.md section 4.3).
CELL = 1.0

# A drive shell renders once per drive, not scattered like a greeble, so it
# gets a part-scale budget: the heaviest shipped ship part runs 251
# triangles, and a whole-cell shell with a faceted bell warrants the same
# order of magnitude with headroom, not the greeble kit's 200 scatter cap.
# Scaled by cell volume - a 3x3 exhaust face has nine bells to draw.
MAX_TRIANGLES_PER_CELL = 450


def _load_greebles():
    """Import gen-greebles.py (hyphenated name, so via importlib) for its
    primitive/recipe/verify layer."""
    path = os.path.join(SCRIPT_DIR, "gen-greebles.py")
    spec = importlib.util.spec_from_file_location("gen_greebles", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# nova_glb (and gen-greebles' own import of it) resolves from the scripts dir.
sys.path.insert(0, SCRIPT_DIR)
gg = _load_greebles()

from nova_glb import write_glb  # noqa: E402  (path set up just above)


def build_shell(recipe, name):
    """A whole recipe -> (blob, triangles). Raises on any budget or structural
    violation, so a bad recipe fails here and never reaches the gallery."""
    materials, index = gg.build_materials(recipe)
    default_material = recipe.get("material")
    parts = recipe.get("parts") or []
    if not parts:
        raise ValueError("%s: recipe has no parts" % name)

    triangles = []
    for part in parts:
        material = part.get("material", default_material)
        if material not in index:
            raise ValueError("%s: part names undeclared material %r" % (name, material))
        triangles.extend(gg.build_part(part, material))
    triangles = gg.quantize(triangles)

    for tri in triangles:
        if gg.triangle_area(tri) <= gg.EPSILON:
            raise ValueError("%s: degenerate triangle %s" % (name, tri.verts()))

    cells = recipe.get("cells", [1, 1, 1])
    if len(cells) != 3 or any(int(c) != c or c < 1 for c in cells):
        raise ValueError("%s: 'cells' must be three integers >= 1, got %r" % (name, cells))

    lo, hi = gg.bounds(triangles)
    for axis, axis_name in enumerate("xyz"):
        half = cells[axis] * CELL / 2.0
        if lo[axis] < -half - gg.EPSILON or hi[axis] > half + gg.EPSILON:
            raise ValueError(
                "%s: %s spans %.4f..%.4f, outside the %dx%dx%d cell box"
                % (name, axis_name, lo[axis], hi[axis], cells[0], cells[1], cells[2])
            )
    # A shell with nothing near +Z is a mount plate, not a drive: the bell
    # (or petal ring) must reach into the aft half of its box.
    if hi[2] < 0.25 * cells[2] * CELL:
        raise ValueError(
            "%s: aft-most geometry at z=%.4f, no exhaust presence" % (name, hi[2])
        )
    budget = MAX_TRIANGLES_PER_CELL * cells[0] * cells[1] * cells[2]
    if len(triangles) > budget:
        raise ValueError(
            "%s: %d triangles, budget %d" % (name, len(triangles), budget)
        )

    return write_glb(triangles, materials, index, GENERATOR), triangles


def run(recipe_dir, out_dir, check):
    recipes = gg.load_recipes(recipe_dir)
    if not recipes:
        print("no recipes in %s" % recipe_dir, file=sys.stderr)
        return 1

    if not check:
        os.makedirs(out_dir, exist_ok=True)
    stale = []
    for name, recipe in recipes:
        blob, triangles = build_shell(recipe, name)
        path = os.path.join(out_dir, name + ".glb")
        rel = os.path.relpath(path, REPO_ROOT)
        lo, hi = gg.bounds(triangles)
        size = tuple(hi[k] - lo[k] for k in range(3))
        summary = "%-24s %4d tris  %5d B  size %.3fx%.3fx%.3f  z %.3f..%.3f" % (
            name, len(triangles), len(blob), size[0], size[1], size[2], lo[2], hi[2],
        )
        if check:
            if not os.path.exists(path):
                stale.append("MISSING  %s" % rel)
            elif open(path, "rb").read() != blob:
                stale.append("STALE    %s (differs from a fresh build)" % rel)
            print("  %s" % summary)
            continue
        with open(path, "wb") as handle:
            handle.write(blob)
        gg.verify(path, blob, triangles)
        print("  %s" % summary)

    if check:
        for line in stale:
            print("  %s" % line)
        if stale:
            print(
                "\n%d of %d shell(s) out of date - run scripts/gen-thruster-shells.py"
                % (len(stale), len(recipes))
            )
            return 1
        print("\n%d shell(s) match a fresh build (byte for byte)." % len(recipes))
        return 0

    print(
        "\n%d shell(s) written to %s/"
        % (len(recipes), os.path.relpath(out_dir, REPO_ROOT))
    )
    return 0


def self_test():
    """Exercise the cell-frame budgets and byte reproducibility. The primitive
    layer's own checks live in gen-greebles.py --self-test."""
    recipe = {
        "materials": {
            "steel": {"color": [0.4, 0.4, 0.4]},
            "throat": {"color": [0.05, 0.05, 0.05]},
        },
        "material": "steel",
        "parts": [
            {"primitive": "box", "size": [1.0, 1.0, 0.6], "at": [0.0, 0.0, -0.2]},
            {
                "primitive": "taper",
                "radius_bottom": 0.1,
                "radius_top": 0.3,
                "height": 0.3,
                "rotate": [90.0, 0.0, 0.0],
                "at": [0.0, 0.0, 0.3],
                "material": "throat",
            },
        ],
    }

    # The same recipe yields the same bytes, twice.
    blob, tris = build_shell(recipe, "probe")
    again, _ = build_shell(recipe, "probe")
    assert blob == again, "the same recipe produced different bytes"
    assert len(tris) == 12 + 32, len(tris)  # box + 8-side taper

    # A declared cell box widens the budget: the same 1.8-wide housing that a
    # 1x1x1 shell must refuse is legal inside a 2x1x1 box.
    wide = {
        **recipe,
        "cells": [2, 1, 1],
        "parts": [
            {"primitive": "box", "size": [1.8, 1.0, 0.6], "at": [0.0, 0.0, -0.2]},
            recipe["parts"][1],
        ],
    }
    build_shell(wide, "probe_wide")

    # Budgets are enforced, not advisory.
    for bad_parts, why in (
        # A flank past the cell boundary would collide with the neighbour.
        ([{"primitive": "box", "size": [1.2, 1.0, 0.6], "at": [0.0, 0.0, -0.2]}],
         "outside the 1x1x1 cell box"),
        # A shell hugging the -Z face has no exhaust to read.
        ([{"primitive": "box", "size": [1.0, 1.0, 0.2], "at": [0.0, 0.0, -0.4]}],
         "no exhaust presence"),
        # The triangle cap: 12 discs at 16 sides is 768 triangles.
        ([{"primitive": "disc", "radius": 0.15, "sides": 16,
           "at": [0.0, 0.0, 0.02 * i]} for i in range(12)],
         "triangles"),
    ):
        try:
            build_shell({**recipe, "parts": bad_parts}, "probe")
        except ValueError as err:
            assert why in str(err), (why, str(err))
        else:
            raise AssertionError("budget not enforced: %s" % why)

    print("self-test OK")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Generate thruster shell candidate meshes (.glb) from JSON recipes."
    )
    parser.add_argument("--recipes", default=RECIPE_DIR, help="recipe folder (JSON)")
    parser.add_argument("--out", default=OUT_DIR, help="output folder for the .glb shells")
    parser.add_argument(
        "--check",
        action="store_true",
        help="rebuild in memory and compare with the committed files; write nothing",
    )
    parser.add_argument("--self-test", action="store_true", help="run internal checks and exit")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()
    return run(args.recipes, args.out, args.check)


if __name__ == "__main__":
    sys.exit(main())
