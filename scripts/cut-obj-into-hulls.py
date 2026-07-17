#!/usr/bin/env python3
"""Cut a monolithic Kenney .obj spaceship into grid-aligned cube pieces (.glb).

A Kenney ship model such as `art/kenney/craft_cargoB.obj` is one solid mesh.
This script scales that mesh so its natural half-unit grid lands on a 1.0 unit
grid, CLIPS every triangle at the cube-boundary planes so each fragment is
strictly inside one cube ("cut the floor tile to fit"), then writes one
glTF-binary (`.glb`) mesh per non-empty cube into the output folder.

Scope: this script only cuts. It parses the obj, slices it on the grid, and
emits the cube meshes - nothing else (no section classification, no mod
scaffold). Turning the cubes into game sections is done elsewhere.

Design (see tasks/20260717-220919/SPIKE.md, option A2 + B1):

- Grid clipping is a true partition: total fragment area equals the original,
  so placing every cube back at its grid position reproduces the ship exactly.
  The split mirrors bevy-common-systems `src/mesh/builder.rs` `triangle_slice`.
- A hand-rolled glTF 2.0 binary writer, standard library only (no trimesh, no
  Blender), matching the repo's stdlib-only asset-script convention.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import struct
import sys
from collections import defaultdict

# ---------------------------------------------------------------------------
# Parsing
# ---------------------------------------------------------------------------

# A Triangle is three vertex positions plus the material name in effect for the
# face it came from. Polygons are fan-triangulated on read, so everything
# downstream deals only in triangles.
class Triangle:
    __slots__ = ("a", "b", "c", "material")

    def __init__(self, a, b, c, material):
        self.a = a
        self.b = b
        self.c = c
        self.material = material

    def verts(self):
        return (self.a, self.b, self.c)

    def centroid(self):
        return (
            (self.a[0] + self.b[0] + self.c[0]) / 3.0,
            (self.a[1] + self.b[1] + self.c[1]) / 3.0,
            (self.a[2] + self.b[2] + self.c[2]) / 3.0,
        )


def parse_mtl(path):
    """Return {material_name: (r, g, b)} from a .mtl file's `Kd` lines.

    Missing file or missing `Kd` is tolerated; callers fall back to a default
    colour for any material not present here.
    """
    colours = {}
    if not path or not os.path.exists(path):
        return colours
    current = None
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            parts = line.split()
            if not parts:
                continue
            if parts[0] == "newmtl":
                current = parts[1]
            elif parts[0] == "Kd" and current is not None:
                colours[current] = (float(parts[1]), float(parts[2]), float(parts[3]))
    return colours


def _vertex_index(token, count):
    """Resolve one OBJ face vertex reference ("v", "v/vt", "v//vn", "v/vt/vn").

    Returns the 0-based position index. Negative OBJ indices are relative to the
    current vertex count.
    """
    raw = int(token.split("/")[0])
    if raw > 0:
        return raw - 1
    return count + raw  # raw is negative: -1 -> last vertex


def parse_obj(path):
    """Parse an OBJ into (triangles, mtl_path).

    Faces are fan-triangulated. The active `usemtl` is recorded on each triangle
    so cells can be classified and coloured later. The referenced `mtllib` path
    (resolved next to the OBJ) is returned so the caller can load colours.
    """
    positions = []
    triangles = []
    material = None
    mtl_path = None
    obj_dir = os.path.dirname(os.path.abspath(path))
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            parts = line.split()
            if not parts:
                continue
            tag = parts[0]
            if tag == "v":
                positions.append((float(parts[1]), float(parts[2]), float(parts[3])))
            elif tag == "usemtl":
                material = parts[1]
            elif tag == "mtllib":
                mtl_path = os.path.join(obj_dir, parts[1])
            elif tag == "f":
                idx = [_vertex_index(tok, len(positions)) for tok in parts[1:]]
                # Fan-triangulate: (0, i, i+1).
                for i in range(1, len(idx) - 1):
                    triangles.append(
                        Triangle(
                            positions[idx[0]],
                            positions[idx[i]],
                            positions[idx[i + 1]],
                            material,
                        )
                    )
    return triangles, mtl_path


# ---------------------------------------------------------------------------
# Transform + bucketing
# ---------------------------------------------------------------------------


def bounds(triangles):
    """Axis-aligned bounding box of all triangle vertices: (min_xyz, max_xyz)."""
    lo = [math.inf, math.inf, math.inf]
    hi = [-math.inf, -math.inf, -math.inf]
    for tri in triangles:
        for v in tri.verts():
            for k in range(3):
                lo[k] = min(lo[k], v[k])
                hi[k] = max(hi[k], v[k])
    return tuple(lo), tuple(hi)


def triangle_area(tri):
    """Area of a triangle (used for the loss-free area-conservation check)."""
    ax, ay, az = tri.a
    bx, by, bz = tri.b
    cx, cy, cz = tri.c
    ux, uy, uz = bx - ax, by - ay, bz - az
    vx, vy, vz = cx - ax, cy - ay, cz - az
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    return 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)


def scale_triangles(triangles, scale):
    """Uniformly scale every vertex about the world origin by `scale`.

    Scaling about the origin (not the mesh centre) keeps the ship centred on the
    grid the same way the source model is centred on x=z=0, so cell indices come
    out symmetric.
    """
    out = []
    for tri in triangles:
        a = tuple(c * scale for c in tri.a)
        b = tuple(c * scale for c in tri.b)
        c = tuple(cc * scale for cc in tri.c)
        out.append(Triangle(a, b, c, tri.material))
    return out


def translate_triangles(triangles, offset):
    """Shift every vertex by `-offset` so the grid can be re-anchored (--center)."""
    ox, oy, oz = offset
    out = []
    for tri in triangles:
        a = (tri.a[0] - ox, tri.a[1] - oy, tri.a[2] - oz)
        b = (tri.b[0] - ox, tri.b[1] - oy, tri.b[2] - oz)
        c = (tri.c[0] - ox, tri.c[1] - oy, tri.c[2] - oz)
        out.append(Triangle(a, b, c, tri.material))
    return out


def _round_cell(r):
    """Round to nearest integer, breaking exact .5 ties toward zero.

    A flat hull wall can lie exactly on a cube boundary (e.g. the port/starboard
    faces at x = +-1.5). Half-up rounding would push the +x wall into a phantom
    outer cell while leaving the -x wall in the real one - asymmetric, and it
    invents empty-shell cells. Ties-toward-zero keeps every boundary face in the
    outermost *occupied* cell on both sides.
    """
    fl = math.floor(r)
    diff = r - fl
    if diff < 0.5:
        return fl
    if diff > 0.5:
        return fl + 1
    return fl if r > 0 else fl + 1  # exactly .5: toward zero (interior)


def cell_of(point, cell):
    """Grid cell index (i, j, k) whose cube is centred on `point`.

    A section at grid position p occupies [p-0.5, p+0.5]*cell, so the owning cell
    of a point is round(point/cell) with boundary ties broken toward zero.
    """
    return tuple(_round_cell(point[k] / cell) for k in range(3))


# --- Grid slicing (clip triangles at cube boundaries so each fragment is
# strictly inside one cube - the "cut the floor tile to fit" model). The split
# mirrors bevy-common-systems' `triangle_slice` (src/mesh/builder.rs): classify
# each vertex by the signed distance to the plane, then, when the triangle
# straddles it, cut off the lonely vertex into one fragment and the opposite
# edge into two, preserving winding so normals stay outward. We do not cap the
# cut faces (fill_boundary) - the hollow backing is the game's default-hull
# scaffold cube, not a generated cap.


def _edge_plane_intersection(a, b, axis, p):
    """Point where edge a->b crosses the axis-aligned plane `coord[axis] == p`.

    Clamps the parameter to the segment and falls back to the midpoint for an
    edge parallel to the plane, so the result is always finite and on the edge.
    """
    ab = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
    denom = ab[axis]
    if abs(denom) < 1e-9:
        return (a[0] + ab[0] * 0.5, a[1] + ab[1] * 0.5, a[2] + ab[2] * 0.5)
    t = (p - a[axis]) / denom
    t = min(1.0, max(0.0, t))
    return (a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t)


def split_triangle(tri, axis, p, eps=1e-12):
    """Split `tri` by the plane `coord[axis] == p` into fragments none of which
    cross the plane. Returns 1 fragment (no crossing) or up to 3. Zero-area
    slivers are dropped."""
    v = (tri.a, tri.b, tri.c)
    sides = [v[i][axis] - p >= 0.0 for i in range(3)]
    if all(sides) or not any(sides):
        return [tri]

    if sides[0] == sides[1]:
        lonely = 2
    elif sides[0] == sides[2]:
        lonely = 1
    else:
        lonely = 0
    order = {0: (v[0], v[2], v[1]), 1: (v[1], v[0], v[2]), 2: (v[2], v[1], v[0])}
    apex, first, second = order[lonely]

    fi = _edge_plane_intersection(apex, first, axis, p)
    si = _edge_plane_intersection(apex, second, axis, p)
    frags = [
        Triangle(apex, si, fi, tri.material),      # apex side
        Triangle(first, fi, second, tri.material),  # far side
        Triangle(second, fi, si, tri.material),     # far side
    ]
    return [f for f in frags if triangle_area(f) > eps]


def grid_planes(triangles, cell):
    """Interior cube-boundary planes crossed by the mesh, per axis.

    Cell k on an axis spans [(k-0.5), (k+0.5)]*cell, so boundaries sit at
    (k+0.5)*cell. Returns {axis: [plane_coord, ...]} for planes strictly inside
    the mesh bounds (planes at the extreme faces cut nothing).
    """
    lo, hi = bounds(triangles)
    planes = {}
    for axis in range(3):
        axis_planes = []
        k = int(math.floor(lo[axis] / cell - 0.5))
        stop = int(math.ceil(hi[axis] / cell + 0.5))
        while k <= stop:
            p = (k + 0.5) * cell
            if lo[axis] + 1e-9 < p < hi[axis] - 1e-9:
                axis_planes.append(p)
            k += 1
        planes[axis] = axis_planes
    return planes


def slice_grid(triangles, cell):
    """Clip every triangle against all interior grid planes so no fragment
    crosses a cube boundary. Area is conserved (fragments partition the mesh)."""
    planes = grid_planes(triangles, cell)
    frags = list(triangles)
    for axis in range(3):
        for p in planes[axis]:
            nxt = []
            for tri in frags:
                nxt.extend(split_triangle(tri, axis, p))
            frags = nxt
    return frags


def bucket_cells(triangles, cell):
    """Group (already-sliced) fragments by the grid cell of their centroid.

    Returns {(i, j, k): [Triangle, ...]}. Since fragments never cross a cube
    boundary, each falls unambiguously into one cell.
    """
    cells = defaultdict(list)
    for tri in triangles:
        cells[cell_of(tri.centroid(), cell)].append(tri)
    return dict(cells)


def recentre(tri, origin):
    """Translate a triangle so `origin` becomes (0, 0, 0) in its local space."""
    def shift(v):
        return (v[0] - origin[0], v[1] - origin[1], v[2] - origin[2])

    return Triangle(shift(tri.a), shift(tri.b), shift(tri.c), tri.material)


# ---------------------------------------------------------------------------
# glTF 2.0 binary (.glb) writer
# ---------------------------------------------------------------------------

_GLB_MAGIC = 0x46546C67  # "glTF"
_CHUNK_JSON = 0x4E4F534A  # "JSON"
_CHUNK_BIN = 0x004E4942  # "BIN\0"
_FLOAT = 5126
_UINT = 5125
_ARRAY_BUFFER = 34962
_ELEMENT_ARRAY_BUFFER = 34963


def _flat_normal(tri):
    """Unit face normal from the triangle winding (flat shading)."""
    ax, ay, az = tri.a
    bx, by, bz = tri.b
    cx, cy, cz = tri.c
    ux, uy, uz = bx - ax, by - ay, bz - az
    vx, vy, vz = cx - ax, cy - ay, cz - az
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    length = math.sqrt(nx * nx + ny * ny + nz * nz)
    if length == 0.0:
        return (0.0, 0.0, 0.0)
    return (nx / length, ny / length, nz / length)


def write_glb(triangles, materials, material_index):
    """Serialize a triangle soup to a glTF-binary blob (bytes).

    - `materials`: ordered list of glTF material dicts, embedded whole in every
      glb (there are only a handful, so global indices stay stable and simple).
    - `material_index`: {material_name: index into `materials`}.

    Triangles are grouped into one primitive per material. Each primitive gets a
    flat-shaded NORMAL and an explicit index buffer.
    """
    by_material = defaultdict(list)
    for tri in triangles:
        by_material[tri.material].append(tri)

    bin_blob = bytearray()
    buffer_views = []
    accessors = []

    def add_view(data, target):
        while len(bin_blob) % 4 != 0:  # 4-byte align each view
            bin_blob.append(0)
        offset = len(bin_blob)
        bin_blob.extend(data)
        buffer_views.append(
            {"buffer": 0, "byteOffset": offset, "byteLength": len(data), "target": target}
        )
        return len(buffer_views) - 1

    primitives = []
    for material_name in sorted(by_material, key=lambda m: (m is None, m)):
        tris = by_material[material_name]
        positions = []
        normals = []
        for tri in tris:
            n = _flat_normal(tri)
            for v in tri.verts():
                positions.append(v)
                normals.append(n)
        indices = list(range(len(positions)))

        pos_bytes = b"".join(struct.pack("<3f", *v) for v in positions)
        pos_view = add_view(pos_bytes, _ARRAY_BUFFER)
        lo = [min(v[k] for v in positions) for k in range(3)]
        hi = [max(v[k] for v in positions) for k in range(3)]
        pos_acc = len(accessors)
        accessors.append(
            {
                "bufferView": pos_view,
                "componentType": _FLOAT,
                "count": len(positions),
                "type": "VEC3",
                "min": lo,
                "max": hi,
            }
        )

        nrm_bytes = b"".join(struct.pack("<3f", *v) for v in normals)
        nrm_view = add_view(nrm_bytes, _ARRAY_BUFFER)
        nrm_acc = len(accessors)
        accessors.append(
            {
                "bufferView": nrm_view,
                "componentType": _FLOAT,
                "count": len(normals),
                "type": "VEC3",
            }
        )

        idx_bytes = b"".join(struct.pack("<I", i) for i in indices)
        idx_view = add_view(idx_bytes, _ELEMENT_ARRAY_BUFFER)
        idx_acc = len(accessors)
        accessors.append(
            {
                "bufferView": idx_view,
                "componentType": _UINT,
                "count": len(indices),
                "type": "SCALAR",
            }
        )

        primitives.append(
            {
                "attributes": {"POSITION": pos_acc, "NORMAL": nrm_acc},
                "indices": idx_acc,
                "material": material_index.get(material_name, 0),
            }
        )

    # Pad the BIN chunk to a 4-byte boundary BEFORE reporting its length, so
    # buffers[0].byteLength always equals the emitted chunk length regardless of
    # which attribute happens to be the last view.
    while len(bin_blob) % 4 != 0:  # pad BIN chunk with zeros
        bin_blob.append(0)

    gltf = {
        "asset": {"version": "2.0", "generator": "cut-obj-into-hulls"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": primitives}],
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(bin_blob)}],
    }

    json_bytes = json.dumps(gltf, separators=(",", ":"), sort_keys=True).encode("utf-8")
    while len(json_bytes) % 4 != 0:  # pad JSON chunk with spaces
        json_bytes += b" "

    total = 12 + 8 + len(json_bytes) + 8 + len(bin_blob)
    out = bytearray()
    out += struct.pack("<III", _GLB_MAGIC, 2, total)
    out += struct.pack("<II", len(json_bytes), _CHUNK_JSON)
    out += json_bytes
    out += struct.pack("<II", len(bin_blob), _CHUNK_BIN)
    out += bin_blob
    return bytes(out)


def build_materials(colours):
    """Turn {name: (r, g, b)} into (glTF material list, {name: index}).

    A stable `_default` material at index 0 catches faces whose material has no
    `Kd` (or no material at all). Order is sorted for deterministic output.
    """
    materials = [
        {
            "name": "_default",
            "pbrMetallicRoughness": {
                "baseColorFactor": [0.6, 0.6, 0.6, 1.0],
                "metallicFactor": 0.1,
                "roughnessFactor": 0.8,
            },
            "doubleSided": True,
        }
    ]
    index = {None: 0}
    for name in sorted(colours):
        r, g, b = colours[name]
        index[name] = len(materials)
        materials.append(
            {
                "name": name,
                "pbrMetallicRoughness": {
                    "baseColorFactor": [r, g, b, 1.0],
                    "metallicFactor": 0.6 if "metal" in name.lower() else 0.1,
                    "roughnessFactor": 0.4 if "metal" in name.lower() else 0.8,
                },
                "doubleSided": True,
            }
        )
    return materials, index


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def cut(obj_path, scale, cell, center=(0.0, 0.0, 0.0)):
    """Full cut pipeline. Returns (cells, materials, material_index, colours).

    Scale the mesh, re-anchor the grid by `center`, clip every triangle at the
    cube-boundary planes (so each fragment is strictly inside one cube), then
    bucket the fragments by cell. `cells` maps (i, j, k) -> list of fragments
    already recentred to the cell origin, ready to hand to `write_glb`.
    """
    triangles, mtl_path = parse_obj(obj_path)
    colours = parse_mtl(mtl_path)
    materials, material_index = build_materials(colours)

    scaled = scale_triangles(triangles, scale)
    if center != (0.0, 0.0, 0.0):
        scaled = translate_triangles(scaled, center)
    fragments = slice_grid(scaled, cell)
    raw_cells = bucket_cells(fragments, cell)

    cells = {}
    for (i, j, k), tris in raw_cells.items():
        origin = (i * cell, j * cell, k * cell)
        cells[(i, j, k)] = [recentre(tri, origin) for tri in tris]
    return cells, materials, material_index, colours


def piece_name(cell):
    """Deterministic per-cell mesh id, e.g. 'cube_i1_jm1_k0' (m = minus)."""
    def tag(value):
        return ("m%d" % -value) if value < 0 else str(value)

    i, j, k = cell
    return "cube_i%s_j%s_k%s" % (tag(i), tag(j), tag(k))


def write_cells(cells, materials, material_index, out_dir):
    """Write one .glb per cell directly into `out_dir`. Returns [(cell, filename)]."""
    os.makedirs(out_dir, exist_ok=True)
    written = []
    for cell in sorted(cells):
        name = piece_name(cell)
        blob = write_glb(cells[cell], materials, material_index)
        path = os.path.join(out_dir, name + ".glb")
        with open(path, "wb") as handle:
            handle.write(blob)
        written.append((cell, name + ".glb"))
    return written


def dominant_material(triangles):
    """Material covering the most *area* in a cell (for the manifest/classify)."""
    area = defaultdict(float)
    for tri in triangles:
        area[tri.material] += triangle_area(tri)
    return max(area, key=lambda m: area[m]) if area else None


def run(args):
    triangles, _ = parse_obj(args.obj)
    scaled = scale_triangles(triangles, args.scale)
    if args.center != (0.0, 0.0, 0.0):
        scaled = translate_triangles(scaled, args.center)
    original_area = sum(triangle_area(t) for t in scaled)

    cells, materials, material_index, _ = cut(
        args.obj, args.scale, args.cell, args.center
    )
    written = write_cells(cells, materials, material_index, args.out)

    # Loss-free invariant: clipping partitions the surface, so total area of the
    # fragments equals the original (within float tolerance).
    cut_area = sum(triangle_area(t) for tris in cells.values() for t in tris)
    cut_frags = sum(len(tris) for tris in cells.values())

    lo, hi = bounds(scaled)
    print("input:            %s (%d triangles)" % (args.obj, len(triangles)))
    print("scale/cell/center: x%.3g / %.3g / %s" % (args.scale, args.cell, args.center))
    print(
        "grid bounds:      x[%.2f,%.2f] y[%.2f,%.2f] z[%.2f,%.2f]"
        % (lo[0], hi[0], lo[1], hi[1], lo[2], hi[2])
    )
    print("cubes written:    %d (%d fragments) -> %s/" % (len(written), cut_frags, args.out))
    for cell, filename in written:
        tris = cells[cell]
        print(
            "  %-18s %3d frags  dominant=%s"
            % (filename, len(tris), dominant_material(tris))
        )
    ok = abs(cut_area - original_area) <= 1e-6 * max(1.0, original_area)
    print("area-conserved:   %.6f cut vs %.6f original -> %s" % (
        cut_area, original_area, "OK" if ok else "MISMATCH"))
    return 0 if ok else 1


def self_test():
    """Exercise slicing (partition + area conservation) and the glb writer."""
    # A triangle straddling the x=0.5 cube boundary must split into fragments
    # that live in cells 0 and 1, with area preserved.
    straddler = Triangle((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), "a")
    frags = slice_grid([straddler], 1.0)
    assert all(triangle_area(f) > 0 for f in frags)
    assert abs(sum(triangle_area(f) for f in frags) - triangle_area(straddler)) < 1e-9
    cells = bucket_cells(frags, 1.0)
    assert set(cells) >= {(0, 0, 0), (1, 0, 0)}, cells
    # No fragment may cross the x=0.5 plane after slicing.
    for f in frags:
        xs = [f.a[0], f.b[0], f.c[0]]
        assert not (min(xs) < 0.5 - 1e-9 and max(xs) > 0.5 + 1e-9), f.verts()

    assert cell_of((1.4, 0.0, 0.0), 1.0) == (1, 0, 0)
    assert cell_of((-1.4, 0.0, 0.0), 1.0) == (-1, 0, 0)
    # Exact boundary ties break toward zero (symmetric, no phantom outer cell).
    assert cell_of((1.5, 0.0, 0.0), 1.0) == (1, 0, 0)
    assert cell_of((-1.5, 0.0, 0.0), 1.0) == (-1, 0, 0)

    mats, index = build_materials({"a": (1.0, 0.0, 0.0), "b": (0.0, 1.0, 0.0)})
    blob = write_glb([straddler], mats, index)
    assert blob[:4] == struct.pack("<I", _GLB_MAGIC)
    assert struct.unpack("<I", blob[8:12])[0] == len(blob)
    json_len = struct.unpack("<I", blob[12:16])[0]
    doc = json.loads(blob[20 : 20 + json_len].decode("utf-8"))
    assert doc["scene"] == 0 and len(doc["scenes"]) == 1
    assert doc["buffers"][0]["byteLength"] == struct.unpack(
        "<I", blob[20 + json_len : 24 + json_len]
    )[0]  # byteLength matches the (padded) BIN chunk length
    assert piece_name((1, -1, 0)) == "cube_i1_jm1_k0"
    print("self-test OK")
    return 0


def _parse_center(text):
    parts = text.split(",")
    if len(parts) != 3:
        raise argparse.ArgumentTypeError("center must be 'x,y,z'")
    return tuple(float(p) for p in parts)


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Cut a Kenney .obj spaceship into grid-aligned modular hull cubes (.glb)."
    )
    parser.add_argument("obj", nargs="?", help="input .obj path")
    parser.add_argument("--out", required=False, help="output folder for the .glb cube meshes (required unless --self-test)")
    parser.add_argument("--scale", type=float, default=2.0, help="uniform scale about origin (default 2.0)")
    parser.add_argument("--cell", type=float, default=1.0, help="cube size in world units (default 1.0)")
    parser.add_argument(
        "--center",
        type=_parse_center,
        default=(0.0, 0.0, 0.0),
        help="re-anchor the grid: 'x,y,z' (post-scale) that becomes a cell centre (default 0,0,0)",
    )
    parser.add_argument("--self-test", action="store_true", help="run internal checks and exit")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()
    if not args.obj:
        parser.error("the obj argument is required (or pass --self-test)")
    if not args.out:
        parser.error("--out is required (the output folder for the .glb cubes)")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
