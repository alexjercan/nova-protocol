#!/usr/bin/env python3
"""Generate section-model CANDIDATES (.glb) from JSON recipes.

Recipe-generated candidates for the section remodel (task 20260831-083625):
torpedo bays, PDC turret assemblies, and the hull/controller cores, all in
the one cross-faction mechanical voice the thruster shells set. Judged in
the `screenshot_section_gallery` example; selected candidates get promoted
to `assets/base/gltf/` paths, and the recipe remains the source either way.

A sibling of `gen-thruster-shells.py`, not more shell recipes, because the
FRAMES differ while the doctrine holds. A recipe here names one of three:

- `"frame": "bay"` - a whole-cell torpedo bay, `"cells": [w, h, l]` centred
  on the origin. The MUZZLE is the -Z face: the game fires a bay out of
  `spawn_offset` NEG_Z, and `link_points` leaves that face unlinkable so it
  can be a mouth. Geometry must reach the forward (-Z) quarter of the box -
  a bay with nothing at its muzzle is the unit-cube placeholder again.
- `"frame": "turret"` - a three-part mount assembly. One recipe carries
  `"yaw"`, `"pitch"` and `"barrel"` part lists and emits `<name>_yaw.glb`,
  `<name>_pitch.glb`, `<name>_barrel.glb`. Each part is authored around its
  OWN joint origin at unit-turret scale (the tree scales the assembly to
  the shipped 0.5 mount); the barrel extends toward -Z, the firing axis.
- `"frame": "core"` - a full-cell hull or controller block, `"cells"`
  centred on the origin, no directional requirement. Cores live UNDER the
  derived skin, so they read as machinery when exposed, and their flanks
  must stay inside the cell box the cladding expects to own.

Run from anywhere (paths are resolved from this file):

    python3 scripts/gen-section-parts.py             # write every .glb
    python3 scripts/gen-section-parts.py --check     # verify, write nothing
    python3 scripts/gen-section-parts.py --self-test # internal checks, no I/O

`--check` rebuilds every part in memory and compares it byte for byte with
the committed file - the same determinism contract the greebles and shells
carry, and the reason the recipes rather than the binaries are the source.
"""

from __future__ import annotations

import argparse
import importlib.util
import math
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
RECIPE_DIR = os.path.join(SCRIPT_DIR, "section-part-recipes")
OUT_DIR = os.path.join(REPO_ROOT, "art", "part-candidates", "sections")

# The owner-picked parts ship: their builds land on the asset paths the
# section catalog references (same contract as gen-thruster-shells.py).
# Everything else stays a candidate under `art/`.
PROMOTED_STEMS = (
    "bay_tube",
    "pdc_gatling_yaw",
    "pdc_gatling_pitch",
    "pdc_gatling_barrel",
    "pdc_twin_yaw",
    "pdc_twin_pitch",
    "pdc_twin_barrel",
    "hull_personnel",
    "hull_cargo",
    "hull_tank",
    "core_wires",
)
PROMOTED_OUTPUTS = {
    stem: os.path.join(REPO_ROOT, "assets", "base", "gltf", stem + ".glb")
    for stem in PROMOTED_STEMS
}

GENERATOR = "gen-section-parts"

CELL = 1.0

# The shells' part-scale budget, scaled by cell volume for the boxed frames.
# A turret part is graded per emitted file against one cell's budget: the
# whole assembly is three files, and the shipped mount runs at half scale.
MAX_TRIANGLES_PER_CELL = 450

# A turret part may stand proud of the unit cell (the assembled gun does),
# but not run away: the shipped barrel's muzzle sits 1.2 forward, so 1.5 is
# headroom, not licence.
TURRET_PART_BOUND = 1.5

FRAMES = ("bay", "turret", "core")


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

from nova_glb import write_glb, write_glb_nodes  # noqa: E402  (path set up just above)


def _cells(recipe, name):
    cells = recipe.get("cells", [1, 1, 1])
    if len(cells) != 3 or any(int(c) != c or c < 1 for c in cells):
        raise ValueError("%s: 'cells' must be three integers >= 1, got %r" % (name, cells))
    return cells


def _build_triangles(recipe, parts, name):
    """One part list -> quantized triangles with the recipe's materials."""
    materials, index = gg.build_materials(recipe)
    default_material = recipe.get("material")
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
    return materials, index, triangles


def _check_cell_box(triangles, cells, name):
    lo, hi = gg.bounds(triangles)
    for axis, axis_name in enumerate("xyz"):
        half = cells[axis] * CELL / 2.0
        if lo[axis] < -half - gg.EPSILON or hi[axis] > half + gg.EPSILON:
            raise ValueError(
                "%s: %s spans %.4f..%.4f, outside the %dx%dx%d cell box"
                % (name, axis_name, lo[axis], hi[axis], cells[0], cells[1], cells[2])
            )
    budget = MAX_TRIANGLES_PER_CELL * cells[0] * cells[1] * cells[2]
    if len(triangles) > budget:
        raise ValueError("%s: %d triangles, budget %d" % (name, len(triangles), budget))
    return lo, hi


def _quat_mul(q1, q2):
    x1, y1, z1, w1 = q1
    x2, y2, z2, w2 = q2
    return (
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
    )


def _euler_quat(degrees):
    """Euler degrees -> (x, y, z, w) quaternion, matching the X-then-Y-then-Z
    order the recipe layer's `rotate` applies to points. Rounded so the glb
    JSON stays byte-stable."""
    hx, hy, hz = (math.radians(float(d)) / 2.0 for d in degrees)
    qx = (math.sin(hx), 0.0, 0.0, math.cos(hx))
    qy = (0.0, math.sin(hy), 0.0, math.cos(hy))
    qz = (0.0, 0.0, math.sin(hz), math.cos(hz))
    q = _quat_mul(qz, _quat_mul(qy, qx))
    return tuple(round(c, gg.PRECISION) for c in q)


def _build_nodes(recipe, name):
    """The recipe's optional `nodes` list -> named animatable glTF nodes.

    Each entry is {"name", "at", "rotate", "parts"}: `parts` are authored in
    the NODE'S LOCAL frame (origin on the hinge, X along the hinge axis, by
    convention - the runtime's motion archetypes rotate about local axes),
    and `at`/`rotate` place the node in the section box. Returns
    (writer_nodes, world_triangles, local_triangles)."""
    writer_nodes = []
    world = []
    local = []
    seen = set()
    for node in recipe.get("nodes", ()):
        node_name = node.get("name")
        if not node_name or not isinstance(node_name, str):
            raise ValueError("%s: every node needs a non-empty 'name'" % name)
        if node_name in seen:
            raise ValueError("%s: duplicate node name %r" % (name, node_name))
        seen.add(node_name)
        _, _, triangles = _build_triangles(
            recipe, node.get("parts"), "%s/%s" % (name, node_name)
        )
        rotate = node.get("rotate", (0.0, 0.0, 0.0))
        at = tuple(round(float(c), gg.PRECISION) for c in node.get("at", (0.0, 0.0, 0.0)))
        placed = gg.quantize(gg.translate(gg.rotate(triangles, rotate), at))
        writer_nodes.append((node_name, at, _euler_quat(rotate), triangles))
        world.extend(placed)
        local.extend(triangles)
    return writer_nodes, world, local


def build_boxed(recipe, name, frame):
    """A bay or core recipe -> (blob, world triangles, mesh-local triangles).
    Raises on any budget or structural violation, so a bad recipe fails here
    and never reaches the gallery. The box, budget and muzzle checks grade
    the WORLD-placed rest pose; the glb stores node meshes in local frames."""
    materials, index, triangles = _build_triangles(recipe, recipe.get("parts"), name)
    writer_nodes, node_world, node_local = _build_nodes(recipe, name)
    cells = _cells(recipe, name)
    world = triangles + node_world
    lo, hi = _check_cell_box(world, cells, name)
    if frame == "bay" and lo[2] > -0.25 * cells[2] * CELL:
        # A bay with nothing near -Z has no mouth to launch out of.
        raise ValueError(
            "%s: forward-most geometry at z=%.4f, no muzzle presence" % (name, lo[2])
        )
    if writer_nodes:
        blob = write_glb_nodes(triangles, writer_nodes, materials, index, GENERATOR)
    else:
        blob = write_glb(triangles, materials, index, GENERATOR)
    return blob, world, triangles + node_local


TURRET_PARTS = ("yaw", "pitch", "barrel")


def build_turret(recipe, name):
    """A turret recipe -> {part: (blob, triangles)}, one glb per joint mesh."""
    outputs = {}
    for part_name in TURRET_PARTS:
        part_list = recipe.get(part_name)
        if not part_list:
            raise ValueError("%s: turret recipe has no '%s' parts" % (name, part_name))
        full = "%s_%s" % (name, part_name)
        materials, index, triangles = _build_triangles(recipe, part_list, full)
        lo, hi = gg.bounds(triangles)
        for axis, axis_name in enumerate("xyz"):
            if lo[axis] < -TURRET_PART_BOUND or hi[axis] > TURRET_PART_BOUND:
                raise ValueError(
                    "%s: %s spans %.4f..%.4f, outside the +-%.1f turret bound"
                    % (full, axis_name, lo[axis], hi[axis], TURRET_PART_BOUND)
                )
        if len(triangles) > MAX_TRIANGLES_PER_CELL:
            raise ValueError(
                "%s: %d triangles, budget %d"
                % (full, len(triangles), MAX_TRIANGLES_PER_CELL)
            )
        outputs[part_name] = (
            write_glb(triangles, materials, index, GENERATOR),
            triangles,
            triangles,
        )
    return outputs


def build_recipe(recipe, name):
    """A whole recipe -> {output name: (blob, world triangles, mesh-local
    triangles)}, keyed by the glb stem each build writes. World and local
    triangles differ only for recipes with animatable `nodes`."""
    frame = recipe.get("frame")
    if frame not in FRAMES:
        raise ValueError("%s: 'frame' must be one of %s, got %r" % (name, FRAMES, frame))
    if frame == "turret":
        return {
            "%s_%s" % (name, part): built
            for part, built in build_turret(recipe, name).items()
        }
    return {name: build_boxed(recipe, name, frame)}


def run(recipe_dir, out_dir, check):
    recipes = gg.load_recipes(recipe_dir)
    if not recipes:
        print("no recipes in %s" % recipe_dir, file=sys.stderr)
        return 1

    # Promoted paths apply only to the default layout; a custom --out-dir
    # gets every part side by side for comparison.
    use_promoted_layout = os.path.abspath(out_dir) == os.path.abspath(OUT_DIR)
    if not check:
        os.makedirs(out_dir, exist_ok=True)
    stale = []
    count = 0
    for name, recipe in recipes:
        for stem, (blob, world, local) in sorted(build_recipe(recipe, name).items()):
            count += 1
            path = os.path.join(out_dir, stem + ".glb")
            if use_promoted_layout:
                path = PROMOTED_OUTPUTS.get(stem, path)
            rel = os.path.relpath(path, REPO_ROOT)
            lo, hi = gg.bounds(world)
            size = tuple(hi[k] - lo[k] for k in range(3))
            summary = "%-24s %4d tris  %5d B  size %.3fx%.3fx%.3f  z %.3f..%.3f" % (
                stem, len(world), len(blob), size[0], size[1], size[2], lo[2], hi[2],
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
            # Verified against the MESH-LOCAL triangles: that is what the
            # file stores; node placement is graded in build_boxed.
            gg.verify(path, blob, local)
            print("  %s" % summary)

    if check:
        for line in stale:
            print("  %s" % line)
        if stale:
            print(
                "\n%d of %d part(s) out of date - run scripts/gen-section-parts.py"
                % (len(stale), count)
            )
            return 1
        print("\n%d part(s) match a fresh build (byte for byte)." % count)
        return 0

    print("\n%d part(s) written to %s." % (count, os.path.relpath(out_dir, REPO_ROOT)))
    return 0


def self_test():
    """Exercise the per-frame checks and byte reproducibility. The primitive
    layer's own checks live in gen-greebles.py --self-test."""
    palette = {
        "materials": {
            "steel": {"color": [0.4, 0.4, 0.4]},
            "throat": {"color": [0.05, 0.05, 0.05]},
        },
        "material": "steel",
    }
    bay = {
        **palette,
        "frame": "bay",
        "cells": [1, 1, 2],
        "parts": [
            {"primitive": "box", "size": [1.0, 1.0, 1.6], "at": [0.0, 0.0, 0.2]},
            {
                "primitive": "disc",
                "radius": 0.3,
                "thickness": 0.02,
                "rotate": [90.0, 0.0, 0.0],
                "at": [0.0, 0.0, -0.99],
                "material": "throat",
            },
        ],
    }

    # The same recipe yields the same bytes, twice.
    built = build_recipe(bay, "probe")
    again = build_recipe(bay, "probe")
    assert built.keys() == again.keys() == {"probe"}
    assert built["probe"][0] == again["probe"][0], "the same recipe produced different bytes"

    # A recipe with animatable nodes: named glTF nodes land in the file, the
    # bytes stay reproducible, and world/local triangles diverge only by the
    # node placement.
    doors = {
        **bay,
        "nodes": [
            {
                "name": "door_petal_%d" % k,
                "rotate": [0.0, 0.0, 60.0 * k],
                "at": [0.0, 0.3, -0.99],
                "parts": [
                    {
                        "primitive": "disc",
                        "radius": 0.2,
                        "thickness": 0.02,
                        "sides": 3,
                        "rotate": [90.0, 0.0, -90.0],
                    }
                ],
            }
            for k in range(2)
        ],
    }
    with_doors = build_recipe(doors, "probe")["probe"]
    assert with_doors[0] == build_recipe(doors, "probe")["probe"][0], (
        "a node recipe produced different bytes"
    )
    assert b'"name":"door_petal_1"' in with_doors[0], "node names missing from the glb"
    assert len(with_doors[1]) == len(with_doors[2]) > len(built["probe"][1]), (
        "node triangles missing from the check sets"
    )
    assert with_doors[1][-1].verts() != with_doors[2][-1].verts(), (
        "world triangles ignore the node placement"
    )

    # Node placement is graded where the node ENDS UP, and names are unique.
    for bad, why in (
        ({**doors, "nodes": [{**doors["nodes"][0], "at": [0.0, 0.6, -0.99]}]}, "cell box"),
        ({**doors, "nodes": [doors["nodes"][0], doors["nodes"][0]]}, "duplicate node name"),
        ({**doors, "nodes": [{**doors["nodes"][0], "name": ""}]}, "non-empty 'name'"),
    ):
        try:
            build_recipe(bad, "probe")
        except ValueError as err:
            assert why in str(err), (why, str(err))
        else:
            raise AssertionError("node check not enforced: %s" % why)

    # A turret recipe emits one glb per joint part.
    turret = {
        **palette,
        "frame": "turret",
        "yaw": [{"primitive": "cylinder", "radius": 0.4, "height": 0.2}],
        "pitch": [{"primitive": "box", "size": [0.6, 0.3, 0.3]}],
        "barrel": [
            {
                "primitive": "cylinder",
                "radius": 0.06,
                "height": 1.2,
                "rotate": [90.0, 0.0, 0.0],
                "at": [0.0, 0.0, -0.6],
            }
        ],
    }
    parts = build_recipe(turret, "probe_gun")
    assert set(parts) == {"probe_gun_yaw", "probe_gun_pitch", "probe_gun_barrel"}, parts.keys()

    # Budgets are enforced, not advisory.
    for recipe, why in (
        # A bay hugging +Z has no mouth to launch out of.
        (
            {
                **palette,
                "frame": "bay",
                "cells": [1, 1, 2],
                "parts": [{"primitive": "box", "size": [1.0, 1.0, 0.4], "at": [0.0, 0.0, 0.7]}],
            },
            "no muzzle presence",
        ),
        # A core past the cell boundary collides with the cladding's cell.
        (
            {
                **palette,
                "frame": "core",
                "parts": [{"primitive": "box", "size": [1.2, 1.0, 1.0]}],
            },
            "outside the 1x1x1 cell box",
        ),
        # A turret barrel out past the assembly bound.
        (
            {
                **turret,
                "barrel": [
                    {
                        "primitive": "cylinder",
                        "radius": 0.06,
                        "height": 2.0,
                        "rotate": [90.0, 0.0, 0.0],
                        "at": [0.0, 0.0, -1.0],
                    }
                ],
            },
            "turret bound",
        ),
        # Frames are closed: a recipe must name one.
        ({**palette, "parts": [{"primitive": "box", "size": [1.0, 1.0, 1.0]}]}, "'frame'"),
    ):
        try:
            build_recipe(recipe, "probe")
        except ValueError as err:
            assert why in str(err), (why, str(err))
        else:
            raise AssertionError("budget not enforced: %s" % why)

    print("self-test OK")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Generate section-model candidate meshes (.glb) from JSON recipes."
    )
    parser.add_argument("--recipes", default=RECIPE_DIR, help="recipe folder (JSON)")
    parser.add_argument("--out", default=OUT_DIR, help="output folder for the .glb parts")
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
