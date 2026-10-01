#!/usr/bin/env python3
"""Render STOWED vs DEPLOYED comparisons of the mining-beam candidates.

`gen-section-parts.py` writes the candidates' REST pose only, and the whole
catalog's convention is that rest pose IS deployed (see `pdc_housing.json`,
`turret.rs`'s `insert_turret_stow`). This script poses the STOWED alternative
- doors shut, tip retracted - by moving the recipes' own named nodes
(`stow_lid_right`, `stow_lid_left`, `beam_tip`) with Blender, using the exact
world-space deltas a runtime `SectionAnimationMotion::Translate` track would
drive them with, and renders both poses side by side for owner review.

A DUAL-MODE script, not a `--python`-only Blender snippet: run under plain
`python3` it has no `bpy` and re-execs itself under
`blender --background --python <this file> -- <args>`; run BY that spawned
Blender it imports `bpy` and does the real work. `main(argv)` is the one
entry point either way, so `python3 scripts/render-mining-beam-candidates.py
--output-dir DIR` is the whole interface.

Before rendering either pose, every candidate is graded the way
`gen-section-parts.py --check` grades the rest pose: cell-box bounds (this
time in BOTH poses), no AABB overlap between the two doors and the tip, the
tip clear of the collar bore in both poses, and - stowed only - the tip's
AABB entirely behind the shut doors (no visual beam). The closed casing
encloses the doors and the tip, so the housing's AABB contains theirs by
design and is not paired with them. A missing named node or a violated
bound is a RuntimeError, not a skipped candidate: a bad pose here means the
candidate does not do what its recipe comment claims.

    python3 scripts/render-mining-beam-candidates.py --output-dir /tmp/mb-out

Writes `<candidate>_deployed.png` and `<candidate>_stowed.png` per candidate
requested (both by default) to `--output-dir`, which must not be inside the
repo - these are review renders, not committed art.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
GLB_DIR = os.path.join(REPO_ROOT, "art", "part-candidates", "sections")

# Recipe-space (the JSON's own x, y, z - see gen-section-parts.py's node
# docstring) world deltas that carry each named node from its authored REST
# (deployed) transform to the STOWED one: the doors slide inward out of their
# frame-hidden pockets by the leaf width plus the pocket gap, and the tip
# slides +Z far enough that its lens clears the shut doors and nests in the
# collar's bore. Hand-derived from each recipe's own geometry and re-verified
# below against the imported mesh bounds every run, so a recipe edit that
# outruns this table fails the render instead of quietly drifting from it.
CANDIDATES = {
    "mining_beam_compact": {
        "cells": (1, 1, 1),
        "stow": {
            "stow_lid_right": (-0.22, 0.0, 0.0),
            "stow_lid_left": (0.22, 0.0, 0.0),
            "beam_tip": (0.0, 0.0, 0.16),
        },
        # The collar's bore radius (constant along its span). The tip rides
        # INSIDE it in both poses, so that pair gets a radial check instead of
        # the pairwise AABB one below.
        "bore_radius": 0.14,
    },
    "mining_beam_forward": {
        "cells": (1, 1, 2),
        "stow": {
            "stow_lid_right": (-0.22, 0.0, 0.0),
            "stow_lid_left": (0.22, 0.0, 0.0),
            "beam_tip": (0.0, 0.0, 0.30),
        },
        "bore_radius": 0.14,
    },
}

CELL = 1.0
BOUNDS_MARGIN = 1e-4

# The name every candidate's un-named root mesh node receives from the glTF
# importer for a single-mesh scene (glb has one un-named node with "mesh": 0
# - see nova_glb.py's write_glb_nodes - and Blender's importer falls back to
# the source mesh's name for it, which nova_glb.py leaves as this default).
STATIC_OBJECT_NAME = "Mesh_0"


def _in_blender():
    try:
        import bpy  # noqa: F401

        return True
    except ImportError:
        return False


def _own_argv():
    """This script's own args, whether run by plain `python3` (`sys.argv[1:]`
    already) or re-exec'd by Blender, which leaves the WHOLE command line -
    its own flags included - in `sys.argv` and expects a script to split on
    the `--` separator itself."""
    argv = sys.argv
    if "--" in argv:
        return argv[argv.index("--") + 1 :]
    return argv[1:]


def _build_arg_parser():
    parser = argparse.ArgumentParser(
        description="Render stowed/deployed comparisons of the mining-beam art candidates."
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="directory (outside the repo) to write the comparison PNGs into",
    )
    parser.add_argument(
        "--candidates",
        nargs="+",
        choices=sorted(CANDIDATES),
        default=sorted(CANDIDATES),
        help="which candidates to render (default: both)",
    )
    parser.add_argument(
        "--samples",
        type=int,
        default=64,
        help="Cycles render samples (default: 64)",
    )
    return parser


def _spawn_blender(argv):
    """Re-exec this file under `blender --background --python`."""
    args = _build_arg_parser().parse_args(argv)
    output_dir = os.path.abspath(args.output_dir)
    if output_dir == REPO_ROOT or output_dir.startswith(REPO_ROOT + os.sep):
        raise SystemExit(
            "--output-dir %r is inside the repo; these are review renders, "
            "point it somewhere else" % output_dir
        )
    os.makedirs(output_dir, exist_ok=True)

    blender = shutil.which(os.environ.get("BLENDER", "blender"))
    if not blender:
        raise SystemExit(
            "blender executable not found on PATH (set BLENDER=/path/to/blender)"
        )
    cmd = [
        blender,
        "--background",
        "--factory-startup",
        "--python",
        os.path.abspath(__file__),
        "--",
    ] + list(argv)
    return subprocess.run(cmd, check=False).returncode


def _recipe_to_blender(vec):
    """Recipe-space (x, y, z) -> the Blender-space vector the glTF importer's
    fixed Y-up -> Z-up conversion places it at: `(x, -z, y)`. Verified against
    the importer directly (a synthetic node at recipe (0.1, 0.05, -0.4) lands
    at Blender (0.1, 0.4, 0.05)) rather than assumed from the glTF spec, since
    this whole script's poses depend on getting it right."""
    x, y, z = vec
    return (x, -z, y)


def _blender_cell_bounds(cells):
    """Recipe `cells` -> Blender-space half-extents, axis-swapped the same
    way `_recipe_to_blender` swaps a point."""
    cx, cy, cz = cells
    return (cx * CELL / 2.0, cz * CELL / 2.0, cy * CELL / 2.0)


def _world_bounds(obj):
    import mathutils

    corners = [obj.matrix_world @ mathutils.Vector(v) for v in obj.bound_box]
    los = [min(c[axis] for c in corners) for axis in range(3)]
    his = [max(c[axis] for c in corners) for axis in range(3)]
    return tuple(los), tuple(his)


def _aabb_overlap(a, b):
    (a_lo, a_hi), (b_lo, b_hi) = a, b
    return all(a_lo[k] < b_hi[k] - BOUNDS_MARGIN and b_lo[k] < a_hi[k] - BOUNDS_MARGIN for k in range(3))


def _radial_extent(obj):
    """Max distance from the housing's tube axis (Blender Y, the recipe's Z)
    any VERTEX of `obj`'s mesh reaches, in world space - not its bounding
    box, whose square corners overreach a round part's true radius by up to
    sqrt(2) and would fail a part that actually clears the bore. Also the
    check a plain AABB overlap can't do at all, since a hollow sleeve's own
    AABB always overlaps whatever nests inside its bore."""
    world = obj.matrix_world
    return max(
        ((world @ v.co).x ** 2 + (world @ v.co).z ** 2) ** 0.5 for v in obj.data.vertices
    )


def _check_pose(name, pose, objects, half_extent, bore_radius):
    """The stowed/deployed contract this whole script exists to grade. Raises
    loudly on the first violation rather than rendering a candidate that
    fails its own recipe comment."""
    bounds = {n: _world_bounds(o) for n, o in objects.items()}

    for n, (lo, hi) in bounds.items():
        for axis, axis_name in enumerate("xyz"):
            if lo[axis] < -half_extent[axis] - BOUNDS_MARGIN or hi[axis] > half_extent[axis] + BOUNDS_MARGIN:
                raise RuntimeError(
                    "%s %s: %r spans %s %.4f..%.4f, outside the cell box +-%.4f"
                    % (name, pose, n, axis_name, lo[axis], hi[axis], half_extent[axis])
                )

    # The closed casing encloses the doors and the tip, so the housing's AABB
    # contains theirs by design; only the moving parts are paired. The tip
    # rides inside the collar bore, graded radially below.
    pairs = [
        ("stow_lid_right", "stow_lid_left"),
        ("stow_lid_right", "beam_tip"),
        ("stow_lid_left", "beam_tip"),
    ]
    for a, b in pairs:
        if _aabb_overlap(bounds[a], bounds[b]):
            raise RuntimeError(
                "%s %s: %r and %r bounding boxes overlap (%s vs %s)"
                % (name, pose, a, b, bounds[a], bounds[b])
            )

    tip_radius = _radial_extent(objects["beam_tip"])
    if tip_radius > bore_radius - BOUNDS_MARGIN:
        raise RuntimeError(
            "%s %s: beam_tip radial reach %.4f does not clear the "
            "collar bore %.4f - it would touch the housing wall"
            % (name, pose, tip_radius, bore_radius)
        )

    if pose == "stowed":

        # No visual beam stowed: recipe -Z (the emitter's outward/forward
        # direction, see `_recipe_to_blender`) is +Blender-Y, so the tip's
        # whole depth must stay at OR BELOW the doors' outward face - the
        # largest Y either door reaches - to sit fully behind them.
        _, tip_hi = bounds["beam_tip"]
        _, right_hi = bounds["stow_lid_right"]
        _, left_hi = bounds["stow_lid_left"]
        door_front = max(right_hi[1], left_hi[1])
        if tip_hi[1] > door_front + BOUNDS_MARGIN:
            raise RuntimeError(
                "%s stowed: beam_tip reaches Y=%.4f, past the shut doors' "
                "Y=%.4f outward face - the tip would still show" % (name, tip_hi[1], door_front)
            )


def _look_at(obj, target):
    import mathutils

    direction = mathutils.Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def _render_one(name, spec, pose, output_dir, samples):
    import bpy
    import mathutils

    bpy.ops.wm.read_factory_settings(use_empty=True)
    glb_path = os.path.join(GLB_DIR, name + ".glb")
    if not os.path.exists(glb_path):
        raise RuntimeError(
            "missing candidate glb: %s (run gen-section-parts.py first)" % glb_path
        )
    bpy.ops.import_scene.gltf(filepath=glb_path)

    expected = set(spec["stow"]) | {STATIC_OBJECT_NAME}
    imported = {o.name for o in bpy.data.objects}
    missing = expected - imported
    if missing:
        raise RuntimeError(
            "%s: expected named node(s) %s missing from %s (got %s)"
            % (name, sorted(missing), glb_path, sorted(imported))
        )
    extra = imported - expected
    if extra:
        raise RuntimeError(
            "%s: unexpected object(s) %s in %s - the render/pose table is stale"
            % (name, sorted(extra), glb_path)
        )

    objects = {n: bpy.data.objects[n] for n in expected}
    if pose == "stowed":
        for node_name, delta in spec["stow"].items():
            objects[node_name].location = objects[node_name].location + mathutils.Vector(
                _recipe_to_blender(delta)
            )
        bpy.context.view_layer.update()

    half_extent = _blender_cell_bounds(spec["cells"])
    _check_pose(name, pose, objects, half_extent, spec["bore_radius"])

    # Lighting: a bright key from front-above and a dim fill from behind, so
    # the -Z emitter face (Blender +Y) reads clearly in both poses.
    key = bpy.data.lights.new("key", type="SUN")
    key.energy = 3.0
    key_obj = bpy.data.objects.new("key", key)
    bpy.context.collection.objects.link(key_obj)
    key_obj.location = (2.0, 3.0, 2.5)
    _look_at(key_obj, (0.0, 0.0, 0.0))

    fill = bpy.data.lights.new("fill", type="SUN")
    fill.energy = 0.8
    fill_obj = bpy.data.objects.new("fill", fill)
    bpy.context.collection.objects.link(fill_obj)
    fill_obj.location = (-1.5, -2.0, -1.0)
    _look_at(fill_obj, (0.0, 0.0, 0.0))

    # Frame the union of every object's world bounds, from a fixed 3/4 angle
    # that keeps the -Z/+Y emitter face in view - the whole point of the
    # comparison is whether that face reads as concealed or exposed.
    los = [min(_world_bounds(o)[0][k] for o in objects.values()) for k in range(3)]
    his = [max(_world_bounds(o)[1][k] for o in objects.values()) for k in range(3)]
    center = mathutils.Vector(((los[k] + his[k]) / 2.0 for k in range(3)))
    radius = max(0.5, (mathutils.Vector(his) - mathutils.Vector(los)).length / 2.0)

    # +Blender-Y is recipe -Z (see `_recipe_to_blender`): the emitter's
    # outward/forward direction, and where the doors and tip sit. A negative
    # Y camera direction would frame the rear block instead of the mechanism
    # under review.
    cam_dir = mathutils.Vector((1.1, 1.9, 0.9)).normalized()
    cam_data = bpy.data.cameras.new("cam")
    cam_obj = bpy.data.objects.new("cam", cam_data)
    bpy.context.collection.objects.link(cam_obj)
    cam_obj.location = center + cam_dir * radius * 3.0
    _look_at(cam_obj, center)
    bpy.context.scene.camera = cam_obj

    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    # Standard, not the default AgX/Filmic: this is a geometry/pose review
    # render, and a contrast-rolloff view transform muddies the flat, fully-
    # saturated glTF baseColorFactor materials `nova_glb.py` writes.
    scene.view_settings.view_transform = "Standard"
    scene.render.resolution_x = 960
    scene.render.resolution_y = 720
    scene.render.image_settings.file_format = "PNG"
    out_path = os.path.join(output_dir, "%s_%s.png" % (name, pose))
    scene.render.filepath = out_path
    bpy.ops.render.render(write_still=True)
    return out_path


def _render(argv):
    args = _build_arg_parser().parse_args(argv)
    output_dir = os.path.abspath(args.output_dir)
    os.makedirs(output_dir, exist_ok=True)

    written = []
    for name in args.candidates:
        spec = CANDIDATES[name]
        for pose in ("deployed", "stowed"):
            written.append(_render_one(name, spec, pose, output_dir, args.samples))

    for path in written:
        print("wrote %s" % path)
    return 0


def main(argv=None):
    argv = list(_own_argv() if argv is None else argv)
    if _in_blender():
        return _render(argv)
    return _spawn_blender(argv)


if __name__ == "__main__":
    sys.exit(main())
