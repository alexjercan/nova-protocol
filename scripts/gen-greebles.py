#!/usr/bin/env python3
"""Generate the base mod's greeble meshes (.glb) from JSON recipes.

A greeble is a small decorative fixture that sits ON a hull plate - a vent, a
rib run, a blister, a mast. The base mod GENERATES its own, so a look is
authored by writing a RECIPE (a list of primitives with sizes and offsets)
instead of modelling: an agent cannot model an antenna, but it can write six
lines of JSON. Recipes and this script are the source; the `.glb` files under
`assets/base/gltf/greebles/` are committed build output, the same contract the
placeholder sounds and scenario thumbnails have.

Run from anywhere (paths are resolved from this file):

    python3 scripts/gen-greebles.py             # write every .glb
    python3 scripts/gen-greebles.py --check     # verify, write nothing
    python3 scripts/gen-greebles.py --self-test # internal checks, no I/O

`--check` rebuilds every piece in memory and compares it byte for byte with the
committed file, so a stale commit or a non-deterministic edit fails here rather
than churning the art in an unrelated diff.

Scale convention (see `crates/nova_ship/src/sections/shell_shape.rs`): a hull
plate is ONE cell, the unit cube, out along `+Y`. A greeble is authored in that
same frame - `+Y` is out of the plate, `y = 0` is the mounting face, and the
footprint is centred on the origin - so placing one is a translate to the plate
face plus the rotation that takes `+Y` to the plate normal. Nothing may sit
behind the mount plane, and by default a piece stays inside half a cell in
every direction (`FOOTPRINT` / `HEIGHT`), so a scattered greeble cannot spill
across a plate seam into its neighbour. A recipe that genuinely needs more (a
mast) raises its own budget explicitly.

Primitives: `box`, `cylinder`, `taper`, `ribs`, `disc`. That set covers the
vocabulary the kits need - panels and blocks, pipes and nozzles, cones and
antennae, ribbing runs, sensor faces - and stops there deliberately: four
candidate kits are built on top of these, and grafting the trim from one look
onto the vents of another has to stay a RECIPE merge rather than a mesh-code
merge.

Solids compose by OVERLAP, not by CSG: a rib sunk into its base plate keeps
both sets of faces and the buried ones are simply never seen. Flat-shaded,
low-poly and untextured, matching the shipped ship parts (52-251 triangles
each, one primitive per flat colour).
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys

from nova_glb import pbr_material, read_glb, write_glb

GENERATOR = "gen-greebles"

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RECIPE_DIR = os.path.join(REPO_ROOT, "scripts", "greeble-recipes")
OUT_DIR = os.path.join(REPO_ROOT, "assets", "base", "gltf", "greebles")

# Budgets, in cells (a plate is 1.0 x 1.0). Half a cell across leaves room to
# scatter a piece off-centre without crossing the seam; half a cell tall keeps
# it inside the volume the cell already owns. A recipe can raise either.
FOOTPRINT = 0.5
HEIGHT = 0.5

# Decoration is scattered many times over a hull, so a piece stays cheap. The
# heaviest shipped ship PART is 251 triangles; a prop that approaches that is a
# modelling job that wandered into the wrong file.
MAX_TRIANGLES = 200

# Vertices are rounded to this many decimals (micrometres at cell scale) before
# they reach the writer, so a value that only differs in float noise cannot
# change the committed bytes.
PRECISION = 6

EPSILON = 1e-9


class Triangle:
    __slots__ = ("a", "b", "c", "material")

    def __init__(self, a, b, c, material):
        self.a = a
        self.b = b
        self.c = c
        self.material = material

    def verts(self):
        return (self.a, self.b, self.c)


def triangle_area(tri):
    ax, ay, az = tri.a
    bx, by, bz = tri.b
    cx, cy, cz = tri.c
    ux, uy, uz = bx - ax, by - ay, bz - az
    vx, vy, vz = cx - ax, cy - ay, cz - az
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    return 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)


def bounds(triangles):
    lo = [math.inf] * 3
    hi = [-math.inf] * 3
    for tri in triangles:
        for v in tri.verts():
            for k in range(3):
                lo[k] = min(lo[k], v[k])
                hi[k] = max(hi[k], v[k])
    return tuple(lo), tuple(hi)


# ---------------------------------------------------------------------------
# Primitives
#
# Every builder returns triangles with OUTWARD winding, centred on the local
# origin. Placement (`at`, `rotate`) is applied afterwards by the recipe layer,
# so a primitive never has to know where it ends up.
# ---------------------------------------------------------------------------


def _quad(a, b, c, d, material):
    """Two triangles for a planar quad, winding preserved a->b->c->d."""
    return [Triangle(a, b, c, material), Triangle(a, c, d, material)]


def box(size, material):
    """An axis-aligned cuboid centred on the origin. 12 triangles."""
    sx, sy, sz = (float(s) / 2.0 for s in size)
    # Corners, low y first, counter-clockwise seen from +Y.
    p = [
        (-sx, -sy, -sz), (sx, -sy, -sz), (sx, -sy, sz), (-sx, -sy, sz),
        (-sx, sy, -sz), (sx, sy, -sz), (sx, sy, sz), (-sx, sy, sz),
    ]
    tris = []
    tris += _quad(p[4], p[7], p[6], p[5], material)  # +Y
    tris += _quad(p[0], p[1], p[2], p[3], material)  # -Y
    tris += _quad(p[3], p[2], p[6], p[7], material)  # +Z
    tris += _quad(p[1], p[0], p[4], p[5], material)  # -Z
    tris += _quad(p[2], p[1], p[5], p[6], material)  # +X
    tris += _quad(p[0], p[3], p[7], p[4], material)  # -X
    return tris


def _tube(radius_bottom, radius_top, height, sides, material):
    """A capped frustum around +Y, centred on the origin.

    The one code path behind `cylinder`, `taper` and `disc`: they differ only
    in which radii and defaults a recipe gets to name. A zero radius drops its
    cap, so a cone is a taper with `radius_top: 0` and costs no special case.
    """
    if sides < 3:
        raise ValueError("a tube needs at least 3 sides, got %d" % sides)
    half = height / 2.0
    step = 2.0 * math.pi / sides

    def ring(radius, y):
        return [
            (radius * math.cos(i * step), y, radius * math.sin(i * step))
            for i in range(sides)
        ]

    low = ring(radius_bottom, -half)
    high = ring(radius_top, half)
    tris = []
    for i in range(sides):
        j = (i + 1) % sides
        # Side face. Degenerate to a triangle where one ring is a point.
        if radius_bottom == 0.0:
            tris.append(Triangle(low[i], high[i], high[j], material))
        elif radius_top == 0.0:
            tris.append(Triangle(low[i], high[i], low[j], material))
        else:
            tris += _quad(low[i], high[i], high[j], low[j], material)
    if radius_bottom > 0.0:
        centre = (0.0, -half, 0.0)
        for i in range(sides):
            j = (i + 1) % sides
            tris.append(Triangle(centre, low[i], low[j], material))
    if radius_top > 0.0:
        centre = (0.0, half, 0.0)
        for i in range(sides):
            j = (i + 1) % sides
            tris.append(Triangle(centre, high[j], high[i], material))
    return tris


def cylinder(radius, height, sides, material):
    """A capped cylinder around +Y: pipes, drums, mast sections."""
    return _tube(radius, radius, height, sides, material)


def taper(radius_bottom, radius_top, height, sides, material):
    """A frustum (or a cone at `radius_top: 0`): nozzles, antenna tips."""
    return _tube(radius_bottom, radius_top, height, sides, material)


def disc(radius, thickness, sides, material):
    """A thin faceted puck: sensor faces, trim rings, bolt heads.

    A cylinder by construction, but a distinct primitive because its DEFAULTS
    are the point - thin, and round enough to read as a circle rather than as a
    drum - and because a recipe naming `disc` says what the piece is for.
    """
    return _tube(radius, radius, thickness, sides, material)


def ribs(size, count, rib_height, rib_width, across, material):
    """A flat base panel carrying `count` raised ribs.

    Ribs run the full length of the panel along one axis and repeat evenly
    across the other, which is the shape that reads as "engineered surface" for
    the fewest triangles: a base box plus one box per rib.
    """
    if count < 1:
        raise ValueError("a ribbed panel needs at least 1 rib, got %d" % count)
    sx, sy, sz = (float(s) for s in size)
    tris = box((sx, sy, sz), material)
    axis = {"x": 0, "z": 2}.get(across)
    if axis is None:
        raise ValueError("ribs 'across' must be 'x' or 'z', got %r" % across)
    span = sx if axis == 0 else sz
    # Ribs sit at the centres of `count` equal lanes, so the outermost pair
    # keeps half a lane of margin and the run stays inside the panel.
    lane = span / count
    for i in range(count):
        offset = -span / 2.0 + lane * (i + 0.5)
        rib_size = [sx, rib_height, sz]
        rib_size[axis] = rib_width
        place = [0.0, (sy + rib_height) / 2.0, 0.0]
        place[axis] = offset
        tris += translate(box(rib_size, material), place)
    return tris


# ---------------------------------------------------------------------------
# Placement
# ---------------------------------------------------------------------------


def _rotate_point(v, radians):
    """Rotate a point by X, then Y, then Z (the order a recipe reads)."""
    x, y, z = v
    rx, ry, rz = radians
    if rx:
        c, s = math.cos(rx), math.sin(rx)
        y, z = y * c - z * s, y * s + z * c
    if ry:
        c, s = math.cos(ry), math.sin(ry)
        x, z = x * c + z * s, -x * s + z * c
    if rz:
        c, s = math.cos(rz), math.sin(rz)
        x, y = x * c - y * s, x * s + y * c
    return (x, y, z)


def rotate(triangles, degrees):
    radians = tuple(math.radians(d) for d in degrees)
    if not any(radians):
        return triangles
    return [
        Triangle(*(_rotate_point(v, radians) for v in tri.verts()), tri.material)
        for tri in triangles
    ]


def translate(triangles, offset):
    ox, oy, oz = offset
    if (ox, oy, oz) == (0.0, 0.0, 0.0):
        return triangles
    return [
        Triangle(
            *((v[0] + ox, v[1] + oy, v[2] + oz) for v in tri.verts()), tri.material
        )
        for tri in triangles
    ]


def quantize(triangles):
    """Round every vertex to `PRECISION` decimals.

    Float noise from the trig in `_tube` is deterministic, but it is also
    invisible and it would survive into the committed bytes; rounding keeps a
    recipe's numbers and the file's numbers the same thing.
    """
    return [
        Triangle(
            *(tuple(round(c, PRECISION) for c in v) for v in tri.verts()),
            tri.material,
        )
        for tri in triangles
    ]


# ---------------------------------------------------------------------------
# Recipes
# ---------------------------------------------------------------------------


def _require(part, key):
    if key not in part:
        raise ValueError("%s part needs '%s'" % (part.get("primitive"), key))
    return part[key]


def build_part(part, material):
    """One recipe entry -> triangles, placed."""
    kind = part.get("primitive")
    if kind == "box":
        tris = box(_require(part, "size"), material)
    elif kind == "cylinder":
        tris = cylinder(
            _require(part, "radius"),
            _require(part, "height"),
            part.get("sides", 8),
            material,
        )
    elif kind == "taper":
        tris = taper(
            _require(part, "radius_bottom"),
            _require(part, "radius_top"),
            _require(part, "height"),
            part.get("sides", 8),
            material,
        )
    elif kind == "disc":
        tris = disc(
            _require(part, "radius"),
            part.get("thickness", 0.015),
            part.get("sides", 12),
            material,
        )
    elif kind == "ribs":
        tris = ribs(
            _require(part, "size"),
            _require(part, "count"),
            _require(part, "rib_height"),
            _require(part, "rib_width"),
            part.get("across", "x"),
            material,
        )
    else:
        raise ValueError("unknown primitive %r" % (kind,))
    turned = rotate(tris, part.get("rotate", (0.0, 0.0, 0.0)))
    return translate(turned, part.get("at", (0.0, 0.0, 0.0)))


def build_materials(recipe):
    """(glTF material list, {name: index}) from the recipe's own palette.

    Sorted by name: the index a primitive resolves to must not depend on JSON
    key order.
    """
    declared = recipe.get("materials")
    if not declared:
        raise ValueError("recipe declares no materials")
    materials = []
    index = {}
    for name in sorted(declared):
        spec = declared[name]
        index[name] = len(materials)
        materials.append(
            pbr_material(
                name,
                spec["color"],
                metallic=spec.get("metallic", 0.1),
                roughness=spec.get("roughness", 0.8),
            )
        )
    return materials, index


def build_recipe(recipe, name):
    """A whole recipe -> (blob, triangles, materials). Raises on any budget or
    structural violation, so a bad recipe fails here and never reaches art."""
    materials, index = build_materials(recipe)
    default_material = recipe.get("material")
    parts = recipe.get("parts") or []
    if not parts:
        raise ValueError("%s: recipe has no parts" % name)

    triangles = []
    for part in parts:
        material = part.get("material", default_material)
        if material not in index:
            raise ValueError("%s: part names undeclared material %r" % (name, material))
        triangles.extend(build_part(part, material))
    triangles = quantize(triangles)

    for tri in triangles:
        if triangle_area(tri) <= EPSILON:
            raise ValueError("%s: degenerate triangle %s" % (name, tri.verts()))

    budget = recipe.get("budget", {})
    footprint = budget.get("footprint", FOOTPRINT)
    height = budget.get("height", HEIGHT)
    lo, hi = bounds(triangles)
    if lo[1] < -EPSILON:
        raise ValueError(
            "%s: reaches y=%.4f, behind the mount plane at y=0" % (name, lo[1])
        )
    if hi[1] > height + EPSILON:
        raise ValueError(
            "%s: stands %.4f cells tall, budget %.4f" % (name, hi[1], height)
        )
    reach = max(abs(lo[0]), abs(hi[0]), abs(lo[2]), abs(hi[2])) * 2.0
    if reach > footprint + EPSILON:
        raise ValueError(
            "%s: footprint %.4f cells, budget %.4f" % (name, reach, footprint)
        )
    if len(triangles) > MAX_TRIANGLES:
        raise ValueError(
            "%s: %d triangles, budget %d" % (name, len(triangles), MAX_TRIANGLES)
        )

    return write_glb(triangles, materials, index, GENERATOR), triangles


def load_recipes(recipe_dir):
    """[(name, recipe)] in name order - the output must not depend on readdir."""
    out = []
    seen = {}
    for filename in sorted(os.listdir(recipe_dir)):
        if not filename.endswith(".json"):
            continue
        path = os.path.join(recipe_dir, filename)
        with open(path, "r", encoding="utf-8") as handle:
            recipe = json.load(handle)
        name = recipe.get("name", os.path.splitext(filename)[0])
        # Two recipes claiming one id would silently overwrite each other's art.
        if name in seen:
            raise ValueError("%s and %s both build '%s'" % (seen[name], filename, name))
        seen[name] = filename
        out.append((name, recipe))
    return out


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def verify(path, expected_blob, triangles):
    """Re-open a written .glb and check it decodes back to what we built.

    The same self-check the parts cutter runs: exit status proves the write
    call returned, decoding the file proves what landed on disk.
    """
    with open(path, "rb") as handle:
        if handle.read() != expected_blob:
            raise ValueError("%s: file differs from the blob just written" % path)
    _, positions = read_glb(path)
    if len(positions) != 3 * len(triangles):
        raise ValueError(
            "%s: %d vertices decoded, expected %d"
            % (path, len(positions), 3 * len(triangles))
        )
    lo, hi = bounds(triangles)
    for k in range(3):
        got = (min(p[k] for p in positions), max(p[k] for p in positions))
        if abs(got[0] - lo[k]) > 1e-5 or abs(got[1] - hi[k]) > 1e-5:
            raise ValueError(
                "%s: decoded bounds %s != built %s" % (path, got, (lo[k], hi[k]))
            )


def run(recipe_dir, out_dir, check):
    recipes = load_recipes(recipe_dir)
    if not recipes:
        print("no recipes in %s" % recipe_dir, file=sys.stderr)
        return 1

    if not check:
        os.makedirs(out_dir, exist_ok=True)
    stale = []
    for name, recipe in recipes:
        blob, triangles = build_recipe(recipe, name)
        path = os.path.join(out_dir, name + ".glb")
        rel = os.path.relpath(path, REPO_ROOT)
        lo, hi = bounds(triangles)
        size = tuple(hi[k] - lo[k] for k in range(3))
        summary = "%-24s %4d tris  %5d B  size %.3fx%.3fx%.3f  y %.3f..%.3f" % (
            name, len(triangles), len(blob), size[0], size[1], size[2], lo[1], hi[1],
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
        verify(path, blob, triangles)
        print("  %s" % summary)

    if check:
        for line in stale:
            print("  %s" % line)
        if stale:
            print(
                "\n%d of %d greeble(s) out of date - run scripts/gen-greebles.py"
                % (len(stale), len(recipes))
            )
            return 1
        print("\n%d greeble(s) match a fresh build (byte for byte)." % len(recipes))
        return 0

    print(
        "\n%d greeble(s) written to %s/"
        % (len(recipes), os.path.relpath(out_dir, REPO_ROOT))
    )
    return 0


def self_test():
    """Exercise the primitives, placement, budgets and byte reproducibility."""
    grey = "grey"

    # A box is 12 triangles and its bounds are the size it was asked for.
    unit = box((0.2, 0.1, 0.3), grey)
    assert len(unit) == 12, len(unit)
    lo, hi = bounds(unit)
    assert [round(hi[k] - lo[k], 6) for k in range(3)] == [0.2, 0.1, 0.3], (lo, hi)

    # Every face winds OUTWARD: the volume of a closed hull computed from its
    # own winding is positive and equals the solid (a flipped face would
    # subtract instead of add). Checked on a box and on a tube, which is the
    # other winding the primitives depend on - side quads and both fanned caps.
    def signed_volume(triangles):
        total = 0.0
        for tri in triangles:
            ax, ay, az = tri.a
            bx, by, bz = tri.b
            cx, cy, cz = tri.c
            total += (
                ax * (by * cz - bz * cy)
                - ay * (bx * cz - bz * cx)
                + az * (bx * cy - by * cx)
            ) / 6.0
        return total

    assert abs(signed_volume(unit) - 0.2 * 0.1 * 0.3) < 1e-9, signed_volume(unit)
    sides = 8
    # An n-gon of circumradius r has area 0.5 * n * r^2 * sin(2pi/n).
    base = 0.5 * sides * 0.05**2 * math.sin(2.0 * math.pi / sides)
    tube = cylinder(0.05, 0.1, sides, grey)
    assert abs(signed_volume(tube) - base * 0.1) < 1e-12, signed_volume(tube)
    for point in (taper(0.05, 0.0, 0.1, sides, grey), taper(0.0, 0.05, 0.1, sides, grey)):
        assert abs(signed_volume(point) - base * 0.1 / 3.0) < 1e-12, signed_volume(point)

    # A tube's caps drop out where a radius is zero: a cone is sides side
    # triangles plus one fanned cap.
    cone = taper(0.05, 0.0, 0.1, 8, grey)
    assert len(cone) == 16, len(cone)  # 8 sides + 8 cap
    drum = cylinder(0.05, 0.1, 8, grey)
    assert len(drum) == 32, len(drum)  # 8 quads + 2 fanned caps
    assert all(abs(v[1]) <= 0.05 + 1e-9 for t in drum for v in t.verts())

    # Ribs land inside their panel and each adds one box.
    panel = ribs((0.3, 0.02, 0.2), 3, 0.03, 0.04, "x", grey)
    assert len(panel) == 12 * 4, len(panel)
    lo, hi = bounds(panel)
    assert lo[0] >= -0.15 - 1e-9 and hi[0] <= 0.15 + 1e-9, (lo, hi)
    assert abs(hi[1] - (0.01 + 0.03)) < 1e-9, hi

    # A quarter turn about Z takes +Y to -X, and `at` moves the piece bodily.
    turned = quantize(rotate(box((0.1, 0.2, 0.1), grey), (0.0, 0.0, 90.0)))
    lo, hi = bounds(turned)
    assert abs((hi[0] - lo[0]) - 0.2) < 1e-6 and abs((hi[1] - lo[1]) - 0.1) < 1e-6
    moved = translate(unit, (0.0, 0.05, 0.0))
    assert abs(bounds(moved)[0][1] - 0.0) < 1e-9

    # A recipe builds, and building it twice yields the same bytes.
    recipe = {
        "materials": {"a": {"color": [1.0, 0.0, 1.0]}, "b": {"color": [0.1, 0.1, 0.1]}},
        "material": "a",
        "parts": [
            {"primitive": "box", "size": [0.2, 0.05, 0.2], "at": [0.0, 0.025, 0.0]},
            {
                "primitive": "cylinder",
                "radius": 0.03,
                "height": 0.1,
                "at": [0.0, 0.1, 0.0],
                "material": "b",
            },
        ],
    }
    blob, tris = build_recipe(recipe, "probe")
    again, _ = build_recipe(recipe, "probe")
    assert blob == again, "the same recipe produced different bytes"
    assert len(tris) == 12 + 32, len(tris)
    doc = json.loads(blob[20 : 20 + int.from_bytes(blob[12:16], "little")].decode("utf-8"))
    assert doc["asset"]["generator"] == GENERATOR
    assert [m["name"] for m in doc["materials"]] == ["a", "b"]
    assert len(doc["meshes"][0]["primitives"]) == 2  # one per material

    # Budgets are enforced, not advisory.
    for bad, why in (
        ({"primitive": "box", "size": [0.2, 0.2, 0.2]}, "behind the mount plane"),
        ({"primitive": "box", "size": [0.9, 0.1, 0.2], "at": [0.0, 0.05, 0.0]}, "footprint"),
        ({"primitive": "box", "size": [0.2, 0.9, 0.2], "at": [0.0, 0.45, 0.0]}, "tall"),
    ):
        try:
            build_recipe({**recipe, "parts": [bad]}, "probe")
        except ValueError as err:
            assert why in str(err), (why, str(err))
        else:
            raise AssertionError("budget not enforced: %s" % why)

    print("self-test OK")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Generate the base mod's greeble meshes (.glb) from JSON recipes."
    )
    parser.add_argument("--recipes", default=RECIPE_DIR, help="recipe folder (JSON)")
    parser.add_argument("--out", default=OUT_DIR, help="output folder for the .glb pieces")
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
