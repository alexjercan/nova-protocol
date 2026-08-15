#!/usr/bin/env python3
"""Cut a monolithic .obj spaceship into SEMANTIC parts (.glb) via a recipe.

Where `cut-obj-into-hulls.py` partitions a ship into uniform grid cubes, this
script partitions it into named parts (nose, wings, engines, fuselage) driven
by a small per-ship JSON recipe - the "one dedicated script per ship" idea as
recipe DATA plus one shared engine. Output matches a parts pack like the
Fertile Soil Spaceship Blocks Collection: one .glb per part with true flat Kd
colours, plus a manifest.json that records where each part sits in ship space
so a builder can reassemble the ship exactly.

Modes:
- `--recipe recipe.json`: ordered part rules; each rule claims triangles by
  clipping at axis-aligned box planes and/or filtering by material or obj
  object name. Whatever no rule claims becomes the `rest` part.
- `--per-object`: one part per `o`/`g` group (multi-object packs ship parts
  this way already - the blocks collection import path).
- neither: the whole mesh becomes one part named after the file (plain
  obj -> glb part conversion).

The clipping mirrors `cut-obj-into-hulls.py`; the glb container is the shared
stdlib-only writer in `nova_glb.py`. Cut cross
sections are capped SOLID with a flat `_cap` bulkhead material: the cutter
knows every plane it clipped at, so each part's cut outline is chained per
plane and ear-clipped in that plane with outward winding (see
`cap_cut_planes`) - non-convex sections and multi-plane parts stay closed
instead of getting a centroid-fan shard cloud.

The run VERIFIES its own output: total pre-cap fragment area must equal the
input area (caps are additional by design), every part reports its remaining
open boundary edges (watertightness), and every emitted .glb is re-opened
from disk, its POSITION accessors decoded, and the recomputed bounds compared
against the manifest.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import struct
import sys
from collections import defaultdict

from nova_glb import flat_normal, pbr_material, read_glb, write_glb

GENERATOR = "cut-obj-into-parts"

# ---------------------------------------------------------------------------
# Parsing (mirrors cut-obj-into-hulls.py, plus per-triangle object names)
# ---------------------------------------------------------------------------


class Triangle:
    __slots__ = ("a", "b", "c", "material", "obj")

    def __init__(self, a, b, c, material, obj=None):
        self.a = a
        self.b = b
        self.c = c
        self.material = material
        self.obj = obj

    def verts(self):
        return (self.a, self.b, self.c)

    def centroid(self):
        return (
            (self.a[0] + self.b[0] + self.c[0]) / 3.0,
            (self.a[1] + self.b[1] + self.c[1]) / 3.0,
            (self.a[2] + self.b[2] + self.c[2]) / 3.0,
        )


def parse_mtl(path):
    """{material_name: (r, g, b)} from `Kd` lines; missing file tolerated."""
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
    raw = int(token.split("/")[0])
    if raw > 0:
        return raw - 1
    return count + raw


def parse_obj(path):
    """Parse an OBJ into (triangles, mtl_path). Faces are fan-triangulated;
    the active `usemtl` and `o`/`g` group are recorded on each triangle."""
    positions = []
    triangles = []
    material = None
    obj_name = None
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
            elif tag in ("o", "g") and len(parts) > 1:
                obj_name = parts[1]
            elif tag == "mtllib":
                mtl_path = os.path.join(obj_dir, parts[1])
            elif tag == "f":
                idx = [_vertex_index(tok, len(positions)) for tok in parts[1:]]
                for i in range(1, len(idx) - 1):
                    triangles.append(
                        Triangle(
                            positions[idx[0]],
                            positions[idx[i]],
                            positions[idx[i + 1]],
                            material,
                            obj_name,
                        )
                    )
    return triangles, mtl_path


# ---------------------------------------------------------------------------
# Transform helpers
# ---------------------------------------------------------------------------


def triangle_area(tri):
    ax, ay, az = tri.a
    bx, by, bz = tri.b
    cx, cy, cz = tri.c
    ux, uy, uz = bx - ax, by - ay, bz - az
    vx, vy, vz = cx - ax, cy - ay, cz - az
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    return 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)


def scale_triangles(triangles, scale):
    out = []
    for tri in triangles:
        out.append(
            Triangle(
                tuple(c * scale for c in tri.a),
                tuple(c * scale for c in tri.b),
                tuple(c * scale for c in tri.c),
                tri.material,
                tri.obj,
            )
        )
    return out


def rotate_y_triangles(triangles, degrees):
    """Rotate about Y so the nose lands on the game's forward -Z axis - the
    same geometry-level fix the hulls cutter applies."""
    if degrees % 360.0 == 0.0:
        return triangles
    rad = math.radians(degrees)
    c, s = math.cos(rad), math.sin(rad)

    def rot(v):
        return (v[0] * c + v[2] * s, v[1], -v[0] * s + v[2] * c)

    return [
        Triangle(rot(t.a), rot(t.b), rot(t.c), t.material, t.obj) for t in triangles
    ]


def bounds(triangles):
    lo = [math.inf] * 3
    hi = [-math.inf] * 3
    for tri in triangles:
        for v in tri.verts():
            for k in range(3):
                lo[k] = min(lo[k], v[k])
                hi[k] = max(hi[k], v[k])
    return tuple(lo), tuple(hi)


def recentre(tri, origin):
    def shift(v):
        return (v[0] - origin[0], v[1] - origin[1], v[2] - origin[2])

    return Triangle(shift(tri.a), shift(tri.b), shift(tri.c), tri.material, tri.obj)


# ---------------------------------------------------------------------------
# Plane clipping (mirrors cut-obj-into-hulls.py split_triangle)
# ---------------------------------------------------------------------------


def _edge_plane_intersection(a, b, axis, p):
    ab = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
    denom = ab[axis]
    if abs(denom) < 1e-9:
        return (a[0] + ab[0] * 0.5, a[1] + ab[1] * 0.5, a[2] + ab[2] * 0.5)
    t = (p - a[axis]) / denom
    t = min(1.0, max(0.0, t))
    return (a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t)


def split_triangle(tri, axis, p, eps=1e-12):
    """Split `tri` at `coord[axis] == p`; no returned fragment crosses it."""
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
        Triangle(apex, si, fi, tri.material, tri.obj),
        Triangle(first, fi, second, tri.material, tri.obj),
        Triangle(second, fi, si, tri.material, tri.obj),
    ]
    return [f for f in frags if triangle_area(f) > eps]


# ---------------------------------------------------------------------------
# Recipe selection engine
# ---------------------------------------------------------------------------

_INF = math.inf


def _box_bounds(box):
    """Recipe box [[min...],[max...]] with null = unbounded -> (lo, hi)."""
    lo_raw, hi_raw = box
    lo = tuple(-_INF if v is None else float(v) for v in lo_raw)
    hi = tuple(_INF if v is None else float(v) for v in hi_raw)
    return lo, hi


def _inside_box(point, lo, hi, eps=1e-9):
    """STRICTLY inside against finite bounds (inf - eps stays inf, so
    unbounded axes always pass). A face lying exactly IN a box bound plane is
    boundary SKIN shared with the neighbour part, not interior: recipes cut at
    natural mesh seams, where side walls are coplanar with the plane and float
    noise (the yaw-180 rotation leaves ~1e-16 residue) must not decide which
    part gets them. Real cut fragments always sit strictly off their plane."""
    return all(lo[k] + eps <= point[k] <= hi[k] - eps for k in range(3))


def claim_part(pool, rule):
    """Apply one recipe rule to the remaining triangle `pool`.

    Returns (claimed, remaining). A `box` clause CLIPS straddling triangles at
    the box planes first, so the part boundary is a clean cut, then claims the
    fragments whose centroid is strictly inside; faces coplanar with a box
    bound stay in the pool for a later rule or the rest part. `materials` /
    `objects` clauses filter the candidates; non-matching fragments stay in
    the pool. Area is conserved across the split (fragments partition the
    pool).
    """
    frags = list(pool)
    lo = hi = None
    if "box" in rule:
        lo, hi = _box_bounds(rule["box"])
        for axis in range(3):
            for p in (lo[axis], hi[axis]):
                if math.isinf(p):
                    continue
                nxt = []
                for tri in frags:
                    nxt.extend(split_triangle(tri, axis, p))
                frags = nxt

    materials = set(rule.get("materials", []))
    objects = set(rule.get("objects", []))

    claimed, remaining = [], []
    for tri in frags:
        take = True
        if lo is not None and not _inside_box(tri.centroid(), lo, hi):
            take = False
        if take and materials and tri.material not in materials:
            take = False
        if take and objects and tri.obj not in objects:
            take = False
        (claimed if take else remaining).append(tri)
    return claimed, remaining


def snap(value, step=0.5):
    """Snap an anchor coordinate to the half-unit grid the game builds on."""
    return round(value / step) * step


def part_anchor(triangles, rule=None):
    """The part's origin in ship space: the recipe's `anchor` when given, else
    the part bbox centre snapped to the half-unit grid (grid-friendly mount
    point without forcing whole-unit placement)."""
    if rule and "anchor" in rule:
        return tuple(float(v) for v in rule["anchor"])
    lo, hi = bounds(triangles)
    return tuple(snap((lo[k] + hi[k]) / 2.0) for k in range(3))


def segment(triangles, recipe):
    """Run every recipe rule over the triangle soup. Returns an ordered list of
    (name, rule_or_None, triangles); the unclaimed remainder lands in the
    `rest` part (default name 'rest'). Empty parts are dropped with a note."""
    pool = list(triangles)
    parts = []
    for rule in recipe.get("parts", []):
        claimed, pool = claim_part(pool, rule)
        if not claimed:
            print("note: rule '%s' claimed nothing" % rule["name"])
            continue
        parts.append((rule["name"], rule, claimed))
    if pool:
        parts.append((recipe.get("rest", "rest"), None, pool))
    return parts


def segment_per_object(triangles):
    """One part per obj `o`/`g` group, in first-seen order."""
    groups = {}
    for tri in triangles:
        groups.setdefault(tri.obj or "mesh", []).append(tri)
    return [(name, None, tris) for name, tris in groups.items()]


# ---------------------------------------------------------------------------
# Link-point candidates
#
# Structural adjacency in the game comes ONLY from authored link points, and a
# recipe already knows where the seams are: two parts cut at the same plane
# meet across it. So the cutter proposes the obvious sockets - one per shared
# face, at its centre, normals opposed - and leaves the judgement to whoever
# promotes the part. These are CANDIDATES: shipped gameplay sockets stay
# hand-authored in Rust, and a recipe part can replace its own list outright.
# ---------------------------------------------------------------------------

# How close two part bounds must sit to count as meeting at a seam. The recipes
# deliberately cut ~0.01 outside their natural seams (so coplanar skin stays
# with one part), so the tolerance has to cover that offset.
LINK_TOUCH_TOLERANCE = 0.05
# The smallest shared face worth a socket: below this the parts touch along an
# edge, and an edge is not a structural interface.
LINK_MIN_FACE = 1e-3


def link_point_candidates(boxes, tolerance=LINK_TOUCH_TOLERANCE):
    """Propose one socket per shared face, keyed by part name.

    `boxes` is `[(name, origin, lo, hi)]` with `lo`/`hi` LOCAL to each part's
    origin, exactly as the manifest records them. Positions come back local
    too, and each normal points out of its part toward the neighbour - the
    frame a `LinkPoint` is authored in.

    Two parts share a face when their bounds meet within `tolerance` on one
    axis and overlap on the other two; the socket sits at the centre of that
    overlap. Parts that only touch along an edge or at a corner get nothing.
    """
    sockets = {name: [] for name, _, _, _ in boxes}
    world = [
        (
            name,
            origin,
            tuple(origin[k] + lo[k] for k in range(3)),
            tuple(origin[k] + hi[k] for k in range(3)),
        )
        for name, origin, lo, hi in boxes
    ]

    for i in range(len(world)):
        for j in range(i + 1, len(world)):
            a_name, a_origin, a_lo, a_hi = world[i]
            b_name, b_origin, b_lo, b_hi = world[j]
            for axis in range(3):
                if abs(a_hi[axis] - b_lo[axis]) <= tolerance:
                    seam, direction = (a_hi[axis] + b_lo[axis]) * 0.5, 1.0
                elif abs(b_hi[axis] - a_lo[axis]) <= tolerance:
                    seam, direction = (a_lo[axis] + b_hi[axis]) * 0.5, -1.0
                else:
                    continue

                centre = [0.0, 0.0, 0.0]
                shared = True
                for other in range(3):
                    if other == axis:
                        centre[other] = seam
                        continue
                    low = max(a_lo[other], b_lo[other])
                    high = min(a_hi[other], b_hi[other])
                    if high - low <= LINK_MIN_FACE:
                        shared = False
                        break
                    centre[other] = (low + high) * 0.5
                if not shared:
                    continue

                normal = [0.0, 0.0, 0.0]
                normal[axis] = direction
                sockets[a_name].append(
                    {
                        "id": "to_" + b_name,
                        "position": [centre[k] - a_origin[k] for k in range(3)],
                        "normal": list(normal),
                    }
                )
                sockets[b_name].append(
                    {
                        "id": "to_" + a_name,
                        "position": [centre[k] - b_origin[k] for k in range(3)],
                        "normal": [-c for c in normal],
                    }
                )
                # One seam per pair: a second axis would be an edge contact,
                # which the overlap test above already rejects.
                break

    return sockets


def recipe_link_points(rule, origin):
    """A part rule's EXPLICIT sockets in part-local space, or `None`.

    Authored in ship space like every other recipe coordinate, so a rule's
    sockets and its `box` read in the same numbers. An explicit list REPLACES
    the generated candidates for that part: the generator proposes, the recipe
    decides.
    """
    if not rule or "link_points" not in rule:
        return None
    points = []
    for index, point in enumerate(rule["link_points"]):
        position = point["position"]
        points.append(
            {
                "id": point.get("id", "socket_%d" % index),
                "position": [position[k] - origin[k] for k in range(3)],
                "normal": list(point["normal"]),
            }
        )
    return points


# ---------------------------------------------------------------------------
# Cap the cut cross-sections
#
# The cutter KNOWS every plane it clipped at (the finite recipe box bounds),
# so capping is plane-aware: per part, per plane, collect the part's boundary
# edges lying IN that plane, weld their endpoints with tolerance, chain them
# into closed loops, and ear-clip each loop in the plane's 2D frame. That
# fixes the three failure modes of the old whole-mesh boundary walk:
# non-planar loops spanning several planes fanned to one 3D centroid (shards
# through the interior), centroid fans on non-convex (L-shaped) sections
# (overlapping triangles), and uncontrolled winding. Winding is deterministic
# here: the cap faces AWAY from the part's material side of the plane.
# `cap_boundary` (the old walk) stays as a fallback pass for boundary loops
# that lie on no cut plane (holes in the source mesh itself).
# ---------------------------------------------------------------------------

_CAP_MATERIAL = "_cap"


def recipe_cut_planes(recipe):
    """Every finite axis plane any recipe rule clips at, as sorted
    (axis, coord) pairs. `claim_part` clips the WHOLE remaining pool at each
    rule's planes, so any part - including the `rest` part - can end at any
    of them. Identity / per-object imports have no recipe rules and therefore
    no cut planes: capping never touches watertight pack pieces."""
    planes = set()
    for rule in recipe.get("parts", []):
        if "box" not in rule:
            continue
        lo, hi = _box_bounds(rule["box"])
        for axis in range(3):
            for p in (lo[axis], hi[axis]):
                if not math.isinf(p):
                    planes.add((axis, p))
    return sorted(planes)


class _Welder:
    """Merge points within `tol` (Chebyshev) via a spatial hash; the first
    point seen in a neighbourhood becomes the representative. Cut edges from
    adjacent triangles meet at float-identical points in theory; welding
    absorbs the last-ulp drift so loops chain instead of breaking open."""

    def __init__(self, tol=1e-5):
        self.tol = tol
        self.cells = defaultdict(list)

    def rep(self, p):
        cell = tuple(int(math.floor(c / self.tol)) for c in p)
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    key = (cell[0] + dx, cell[1] + dy, cell[2] + dz)
                    for q in self.cells.get(key, ()):
                        if max(abs(q[k] - p[k]) for k in range(3)) <= self.tol:
                            return q
        self.cells[cell].append(p)
        return p


def _plane_boundary_segments(triangles, axis, p, welder, on_tol=1e-6):
    """The part's cut cross-section outline at `coord[axis] == p`: welded
    edges lying in the plane and used by an ODD number of triangles (edges
    interior to the surface pair up and cancel). Returns {frozenset(a, b):
    side} where side < 0 means the part's material sits below the plane, so
    the cap must face +axis. `welder` is shared across a part's planes so
    plane-joint corner points unify."""
    count = defaultdict(int)
    side = defaultdict(float)
    for tri in triangles:
        vs = tri.verts()
        on = [abs(v[axis] - p) <= on_tol for v in vs]
        if sum(on) < 2:
            continue
        s = sum(v[axis] - p for v, o in zip(vs, on) if not o)
        for i in range(3):
            j = (i + 1) % 3
            if not (on[i] and on[j]):
                continue
            a, b = welder.rep(vs[i]), welder.rep(vs[j])
            if a == b:
                continue
            key = frozenset((a, b))
            count[key] += 1
            side[key] += s
    return {key: side[key] for key, c in count.items() if c % 2 == 1}


def _chain_loops(segments):
    """Chain welded segments into closed loops. Returns (loops, open_chains)
    where each loop is (vertex list, side sum). Junction vertices (degree > 2,
    e.g. two loops kissing at a corner) take an arbitrary continuation; even
    degree everywhere still closes the walk."""
    adj = defaultdict(list)
    for key in segments:
        a, b = tuple(key)
        adj[a].append(b)
        adj[b].append(a)
    unused = set(segments)
    loops = []
    open_chains = 0
    while unused:
        first = next(iter(unused))
        a, b = tuple(first)
        unused.discard(first)
        loop = [a, b]
        side = segments[first]
        closed = False
        while True:
            cur = loop[-1]
            step = None
            for cand in adj[cur]:
                key = frozenset((cur, cand))
                if key in unused:
                    step = (cand, key)
                    break
            if step is None:
                break
            nxt, key = step
            unused.discard(key)
            side += segments[key]
            if nxt == loop[0]:
                closed = True
                break
            loop.append(nxt)
        if closed and len(loop) >= 3:
            loops.append((loop, side))
        else:
            open_chains += 1
    return loops, open_chains


def _signed_area_2d(pts):
    total = 0.0
    n = len(pts)
    for i in range(n):
        x1, y1 = pts[i]
        x2, y2 = pts[(i + 1) % n]
        total += x1 * y2 - x2 * y1
    return 0.5 * total


def _point_in_poly(pt, pts):
    x, y = pt
    inside = False
    n = len(pts)
    for i in range(n):
        x1, y1 = pts[i]
        x2, y2 = pts[(i + 1) % n]
        if (y1 > y) != (y2 > y):
            t = (y - y1) / (y2 - y1)
            if x < x1 + t * (x2 - x1):
                inside = not inside
    return inside


def _ear_clip(pts):
    """Triangulate a simple CCW polygon (2D points; non-convex and collinear
    vertices allowed) into index triples. T-junction vertices stay corners of
    real triangles, so cap edges keep matching the surface edges exactly.
    Returns None when no ear exists (degenerate input) - callers fall back to
    a centroid fan."""
    idx = list(range(len(pts)))
    tris = []

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    while len(idx) > 3:
        clipped = False
        m = len(idx)
        for k in range(m):
            i0, i1, i2 = idx[k - 1], idx[k], idx[(k + 1) % m]
            a, b, c = pts[i0], pts[i1], pts[i2]
            if cross(a, b, c) <= 1e-12:
                continue  # reflex or collinear corner: not an ear
            blocked = False
            for j in idx:
                if j in (i0, i1, i2) or pts[j] in (a, b, c):
                    continue  # hole-bridge duplicates share coordinates
                if (
                    cross(a, b, pts[j]) >= -1e-12
                    and cross(b, c, pts[j]) >= -1e-12
                    and cross(c, a, pts[j]) >= -1e-12
                ):
                    blocked = True
                    break
            if blocked:
                continue
            tris.append((i0, i1, i2))
            del idx[k]
            clipped = True
            break
        if not clipped:
            return None
    tris.append(tuple(idx))
    return tris


def _bridge_hole(outer, hole):
    """Merge a hole loop into its outer loop through a two-way bridge at the
    nearest vertex pair, yielding one simple polygon for the ear clipper.
    `outer` and `hole` are parallel (pt2, pt3) lists; outer CCW, hole CW."""
    best = None
    for hi, (hp, _) in enumerate(hole):
        for oi, (op, _) in enumerate(outer):
            d = (hp[0] - op[0]) ** 2 + (hp[1] - op[1]) ** 2
            if best is None or d < best[0]:
                best = (d, oi, hi)
    _, oi, hi = best
    return outer[: oi + 1] + hole[hi:] + hole[: hi + 1] + outer[oi:]


def _synthesize_joint_segments(per_plane, notes, line_tol=2e-5):
    """Close the corner gaps between two cut openings of one part. Where two
    planes both cut a part, their openings meet along the plane-plane JOINT
    line, which runs through the part's interior - no surface edge exists
    there, so each plane's outline arrives at the joint and stops (an open
    chain). The chain endpoints are the odd-degree vertices of each plane's
    segment graph; endpoints on a joint line toggle solid/void along it, so
    sorting them by the line's free axis and pairing adjacent ones (even-odd
    rule) yields exactly the missing joint segments. Each segment is added to
    BOTH planes, so the two caps meet flush and the joint edge ends up shared
    by exactly two cap triangles (watertight)."""
    odd = {}
    for key, segments in per_plane.items():
        degree = defaultdict(int)
        for seg in segments:
            for vertex in seg:
                degree[vertex] += 1
        odd[key] = {v for v, d in degree.items() if d % 2 == 1}

    plane_keys = sorted(per_plane)
    for i, pk in enumerate(plane_keys):
        for qk in plane_keys[i + 1 :]:
            if pk[0] == qk[0]:
                continue  # parallel planes never meet
            free = 3 - pk[0] - qk[0]
            on_line = lambda v: (
                abs(v[pk[0]] - pk[1]) <= line_tol and abs(v[qk[0]] - qk[1]) <= line_tol
            )
            points = sorted(
                {v for v in odd[pk] | odd[qk] if on_line(v)},
                key=lambda v: v[free],
            )
            if len(points) % 2 == 1:
                notes.append(
                    "joint %s=%.4g/%s=%.4g: odd endpoint count %d, one dropped"
                    % ("xyz"[pk[0]], pk[1], "xyz"[qk[0]], qk[1], len(points))
                )
                points = points[:-1]
            for a, b in zip(points[0::2], points[1::2]):
                seg = frozenset((a, b))
                for key in (pk, qk):
                    per_plane[key].setdefault(seg, 0.0)


def cap_cut_planes(triangles, planes, min_area=2e-3):
    """Close the cut cross-sections of one part. For each cut plane, chain
    the part's in-plane boundary edges into loops and triangulate each loop
    in the plane's 2D frame (ear clipping - non-convex sections stay inside
    their outline). Corner gaps where two openings meet are closed with
    synthesized joint segments first (see `_synthesize_joint_segments`). A
    loop nested inside another is a HOLE in that section (a tube cut): it is
    bridged into its outer loop and reported. Winding is deterministic: caps
    face away from the material side. Returns (caps, notes); notes list
    anomalies for the run report."""
    caps = []
    notes = []
    welder = _Welder()
    per_plane = {}
    for axis, p in planes:
        segments = _plane_boundary_segments(triangles, axis, p, welder)
        if segments:
            per_plane[(axis, p)] = segments
    _synthesize_joint_segments(per_plane, notes)
    for axis, p in sorted(per_plane):
        segments = per_plane[(axis, p)]
        loops, open_chains = _chain_loops(segments)
        if open_chains:
            notes.append(
                "plane %s=%.4g: %d open chain(s) skipped"
                % ("xyz"[axis], p, open_chains)
            )
        u, v = (axis + 1) % 3, (axis + 2) % 3
        polys = []
        for loop, side in loops:
            pts2 = [(q[u], q[v]) for q in loop]
            if abs(_signed_area_2d(pts2)) < min_area:
                continue
            # Right-handed (u, v, axis) frame: a CCW loop caps toward +axis.
            outward = -1.0 if side > 0.0 else 1.0
            polys.append({"loop": list(zip(pts2, loop)), "outward": outward})
        for poly in polys:
            containers = 0
            for other in polys:
                if other is not poly and _point_in_poly(
                    poly["loop"][0][0], [pt2 for pt2, _ in other["loop"]]
                ):
                    containers += 1
                    poly["outer"] = other
            poly["holes"] = []
            poly["nested"] = containers
            if containers > 1:
                notes.append(
                    "plane %s=%.4g: loop nested %d deep skipped"
                    % ("xyz"[axis], p, containers)
                )
        for poly in polys:
            if poly["nested"] == 1:
                poly["outer"]["holes"].append(poly)
                notes.append(
                    "plane %s=%.4g: cross-section hole bridged into its outer loop"
                    % ("xyz"[axis], p)
                )
        for poly in polys:
            if poly["nested"] != 0:
                continue
            # Ear clipping wants the outer loop CCW and holes CW.
            merged = list(poly["loop"])
            if _signed_area_2d([pt2 for pt2, _ in merged]) < 0.0:
                merged.reverse()
            for hole in poly["holes"]:
                hole_loop = list(hole["loop"])
                if _signed_area_2d([pt2 for pt2, _ in hole_loop]) > 0.0:
                    hole_loop.reverse()
                merged = _bridge_hole(merged, hole_loop)
            pts2 = [pt2 for pt2, _ in merged]
            pts3 = [pt3 for _, pt3 in merged]
            tris = _ear_clip(pts2)
            if tris is None:
                notes.append(
                    "plane %s=%.4g: ear clip failed, centroid fan fallback"
                    % ("xyz"[axis], p)
                )
                n = len(pts3)
                center = tuple(sum(q[k] for q in pts3) / n for k in range(3))
                pts3 = pts3 + [center]
                tris = [(m, (m + 1) % n, n) for m in range(n)]
            for i, j, k in tris:
                a, b, c = pts3[i], pts3[j], pts3[k]
                if poly["outward"] < 0.0:
                    b, c = c, b
                caps.append(Triangle(a, b, c, _CAP_MATERIAL))
    return caps, notes


def boundary_edge_counts(triangles, planes=()):
    """Watertightness report: (on_cut, off_cut) counts of edges used by an
    ODD number of triangles (quantized vertices). `on_cut` edges lie in one
    of the given (axis, p) cut planes - a capped part MUST report zero there
    (an uncovered cut). `off_cut` edges are odd topology away from any cut,
    in practice collinear T-junctions in the source mesh (a large face
    abutting two smaller ones): the surface is fully covered, no hole."""
    quant = lambda v: (round(v[0], 5), round(v[1], 5), round(v[2], 5))
    edge_count = defaultdict(int)
    for t in triangles:
        vs = [quant(t.a), quant(t.b), quant(t.c)]
        for a, b in ((vs[0], vs[1]), (vs[1], vs[2]), (vs[2], vs[0])):
            if a != b:
                edge_count[frozenset((a, b))] += 1
    on_cut = off_cut = 0
    for edge, c in edge_count.items():
        if c % 2 == 0:
            continue
        a, b = tuple(edge)
        if any(
            abs(a[axis] - p) <= 1e-4 and abs(b[axis] - p) <= 1e-4
            for axis, p in planes
        ):
            on_cut += 1
        else:
            off_cut += 1
    return on_cut, off_cut


def count_boundary_edges(triangles):
    """Total odd-count boundary edges: zero for a watertight mesh."""
    return sum(boundary_edge_counts(triangles))


def _loop_area(loop):
    nx = ny = nz = 0.0
    n = len(loop)
    for i in range(n):
        a, b = loop[i], loop[(i + 1) % n]
        nx += (a[1] - b[1]) * (a[2] + b[2])
        ny += (a[2] - b[2]) * (a[0] + b[0])
        nz += (a[0] - b[0]) * (a[1] + b[1])
    return 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)


def cap_boundary(triangles, min_area=2e-3):
    """FALLBACK pass for boundary loops on no cut plane (holes in the source
    mesh itself): walk boundary edges (used by exactly one triangle) into
    closed loops and fan each loop to its own centroid. Open chains and
    sub-`min_area` specks are skipped. Winding is NOT controlled here (the
    glb materials are double-sided) - cut cross-sections must go through
    `cap_cut_planes` instead."""
    quant = lambda v: (round(v[0], 5), round(v[1], 5), round(v[2], 5))
    edge_count = defaultdict(int)
    for t in triangles:
        vs = [quant(t.a), quant(t.b), quant(t.c)]
        for a, b in ((vs[0], vs[1]), (vs[1], vs[2]), (vs[2], vs[0])):
            edge_count[frozenset((a, b))] += 1
    remaining = set()
    adj = defaultdict(set)
    for edge, c in edge_count.items():
        if c == 1 and len(edge) == 2:
            a, b = tuple(edge)
            remaining.add(edge)
            adj[a].add(b)
            adj[b].add(a)
    caps = []
    while remaining:
        a, b = tuple(next(iter(remaining)))
        remaining.discard(frozenset((a, b)))
        loop = [a, b]
        prev, cur = a, b
        closed = False
        while True:
            nxts = [
                n for n in adj[cur] if n != prev and frozenset((cur, n)) in remaining
            ]
            if not nxts:
                break
            nxt = nxts[0]
            remaining.discard(frozenset((cur, nxt)))
            if nxt == loop[0]:
                closed = True
                break
            loop.append(nxt)
            prev, cur = cur, nxt
        if not (closed and len(loop) >= 3 and _loop_area(loop) > min_area):
            continue
        n = len(loop)
        center = (
            sum(p[0] for p in loop) / n,
            sum(p[1] for p in loop) / n,
            sum(p[2] for p in loop) / n,
        )
        for m in range(n):
            caps.append(Triangle(loop[m], loop[(m + 1) % n], center, _CAP_MATERIAL))
    return caps


# ---------------------------------------------------------------------------
# Materials (the .glb container itself lives in nova_glb)
# ---------------------------------------------------------------------------


def build_materials(colours):
    """(glTF material list, {name: index}); `_default` catches unknown
    materials, `_cap` is the flat cut-face fill."""
    materials = [
        pbr_material("_default", (0.6, 0.6, 0.6)),
        # Cut faces read as INTENTIONAL bulkheads, not glitches: one dark
        # neutral across every part of a ship (a per-part darkened hull colour
        # would make sibling caps disagree). Sits just below the Kenney "dark"
        # trim (Kd 0.27/0.30/0.34) so it reads as interior.
        pbr_material(_CAP_MATERIAL, (0.16, 0.17, 0.20), roughness=0.9),
    ]
    index = {None: 0, _CAP_MATERIAL: 1}
    for name in sorted(colours):
        metal = "metal" in name.lower()
        index[name] = len(materials)
        materials.append(
            pbr_material(
                name,
                colours[name],
                metallic=0.6 if metal else 0.1,
                roughness=0.4 if metal else 0.8,
            )
        )
    return materials, index


# ---------------------------------------------------------------------------
# Output verification: re-open the emitted glb and decode it
# ---------------------------------------------------------------------------


def verify_part(path, expected_tris, expected_bbox, eps=1e-4):
    """Re-open one emitted part and check triangle count and bounds."""
    doc, positions = read_glb(path)
    tri_count = sum(
        doc["accessors"][p["indices"]]["count"] // 3
        for mesh in doc["meshes"]
        for p in mesh["primitives"]
    )
    if tri_count != expected_tris:
        raise ValueError(
            "%s: %d triangles in glb, expected %d" % (path, tri_count, expected_tris)
        )
    lo = [min(p[k] for p in positions) for k in range(3)]
    hi = [max(p[k] for p in positions) for k in range(3)]
    for k in range(3):
        if abs(lo[k] - expected_bbox[0][k]) > eps or abs(hi[k] - expected_bbox[1][k]) > eps:
            raise ValueError(
                "%s: decoded bbox %s..%s != manifest %s" % (path, lo, hi, expected_bbox)
            )
    return doc, (lo, hi)


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def run(args):
    recipe = {}
    if args.recipe:
        with open(args.recipe, "r", encoding="utf-8") as handle:
            recipe = json.load(handle)

    scale = args.scale if args.scale is not None else recipe.get("scale", 1.0)
    yaw = args.yaw if args.yaw is not None else recipe.get("yaw", 0.0)

    triangles, mtl_path = parse_obj(args.obj)
    colours = parse_mtl(mtl_path)
    materials, material_index = build_materials(colours)

    soup = scale_triangles(triangles, scale)
    soup = rotate_y_triangles(soup, yaw)
    original_area = sum(triangle_area(t) for t in soup)
    ship_lo, ship_hi = bounds(soup)

    # Cut planes exist only in recipe mode; identity / per-object imports MUST
    # see none (watertight pack pieces get no plane caps by construction).
    planes = []
    if args.per_object:
        parts = segment_per_object(soup)
    elif recipe.get("parts"):
        parts = segment(soup, recipe)
        planes = recipe_cut_planes(recipe)
    else:
        stem = os.path.splitext(os.path.basename(args.obj))[0].lower()
        parts = [(stem, None, soup)]

    cut_area = sum(triangle_area(t) for _, _, tris in parts for t in tris)
    conserved = abs(cut_area - original_area) <= 1e-6 * max(1.0, original_area)

    os.makedirs(args.out, exist_ok=True)
    manifest = {
        "source": args.obj,
        "scale": scale,
        "yaw": yaw,
        "ship_bbox": {"min": list(ship_lo), "max": list(ship_hi)},
        "parts": [],
    }
    cap_notes = {}
    watertight = {}
    part_rules = {}
    for name, rule, tris in parts:
        origin = part_anchor(tris, rule)
        local = [recentre(t, origin) for t in tris]
        local_planes = [(axis, p - origin[axis]) for axis, p in planes]
        open_before = count_boundary_edges(local)
        if args.caps:
            caps, notes = cap_cut_planes(local, local_planes)
            local.extend(caps)
            # Fallback: closed boundary loops on NO cut plane (source-mesh
            # holes). Adds nothing to watertight input.
            local.extend(cap_boundary(local))
            cap_notes[name] = notes
        on_cut, off_cut = boundary_edge_counts(local, local_planes)
        watertight[name] = (open_before, on_cut, off_cut, len(local) - len(tris))
        lo, hi = bounds(local)
        blob = write_glb(local, materials, material_index, GENERATOR)
        path = os.path.join(args.out, name + ".glb")
        with open(path, "wb") as handle:
            handle.write(blob)

        area = defaultdict(float)
        for t in tris:
            area[t.material] += triangle_area(t)
        dominant = max(area, key=lambda m: area[m]) if area else None
        manifest["parts"].append(
            {
                "name": name,
                "file": name + ".glb",
                "origin": list(origin),
                "bbox": {"min": list(lo), "max": list(hi)},
                "size": [hi[k] - lo[k] for k in range(3)],
                "triangles": len(local),
                "cap_triangles": watertight[name][3],
                "open_cut_edges": watertight[name][1],
                "source_odd_edges": watertight[name][2],
                "materials": sorted({t.material for t in tris if t.material}),
                "dominant_material": dominant,
                # Primitive auto-fit suggestion for SectionCollider: the
                # tightest AABB cuboid. A hull/decomposition fit is a game-side
                # decision; the bbox is always available from here.
                "collider_cuboid_size": [hi[k] - lo[k] for k in range(3)],
            }
        )
        part_rules[name] = rule

    # Structural socket candidates, once every part's bounds are known: a seam
    # is a relation between two parts, not a property of one.
    boxes = [
        (
            entry["name"],
            entry["origin"],
            entry["bbox"]["min"],
            entry["bbox"]["max"],
        )
        for entry in manifest["parts"]
    ]
    generated = link_point_candidates(boxes)
    overridden = []
    for entry in manifest["parts"]:
        explicit = recipe_link_points(part_rules.get(entry["name"]), entry["origin"])
        if explicit is not None:
            overridden.append(entry["name"])
        entry["link_points"] = (
            explicit if explicit is not None else generated[entry["name"]]
        )

    manifest_path = os.path.join(args.out, "manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2)
        handle.write("\n")

    # --- verification: re-open everything we just wrote ---
    print("input:          %s (%d triangles)" % (args.obj, len(triangles)))
    print("scale/yaw:      x%.3g / %.1f deg" % (scale, yaw))
    print(
        "ship bounds:    x[%.2f,%.2f] y[%.2f,%.2f] z[%.2f,%.2f]"
        % (ship_lo[0], ship_hi[0], ship_lo[1], ship_hi[1], ship_lo[2], ship_hi[2])
    )
    print(
        "area-conserved: %.6f cut vs %.6f original -> %s"
        % (cut_area, original_area, "OK" if conserved else "MISMATCH")
    )
    if planes:
        print(
            "cut planes:     %s"
            % ", ".join("%s=%g" % ("xyz"[axis], p) for axis, p in planes)
        )
    else:
        print("cut planes:     none (identity import: no plane caps)")
    ok = conserved
    for entry in manifest["parts"]:
        path = os.path.join(args.out, entry["file"])
        try:
            expected = (entry["bbox"]["min"], entry["bbox"]["max"])
            verify_part(path, entry["triangles"], expected)
            status = "OK"
        except ValueError as err:
            status = "FAIL (%s)" % err
            ok = False
        open_before, on_cut, off_cut, caps = watertight[entry["name"]]
        if on_cut > 0:
            ok = False  # a cut face the caps did not close is a regression
        print(
            "  part %-18s origin (%6.2f,%6.2f,%6.2f)  size %.2fx%.2fx%.2f  "
            "%4d tris  caps %3d  open cut edges %d (pre-cap %d, source-odd %d)  "
            "mats %-30s glb %s"
            % (
                entry["name"],
                *entry["origin"],
                *entry["size"],
                entry["triangles"],
                caps,
                on_cut,
                open_before,
                off_cut,
                ",".join(entry["materials"]),
                status,
            )
        )
        for note in cap_notes.get(entry["name"], ()):
            print("       note: %s" % note)
    total_sockets = sum(len(entry["link_points"]) for entry in manifest["parts"])
    print(
        "link points:    %d candidate socket(s)%s"
        % (
            total_sockets,
            "" if not overridden else " (%s authored by the recipe)" % ", ".join(overridden),
        )
    )
    print("manifest:       %s (%d parts)" % (manifest_path, len(manifest["parts"])))
    return 0 if ok else 1


# ---------------------------------------------------------------------------
# Self-test
# ---------------------------------------------------------------------------


def self_test():
    """Exercise box clipping, selectors, anchors and the glb round-trip."""
    # A quad straddling x=0.5 splits cleanly: area conserved, both sides
    # populated, no fragment crossing the plane.
    quad = [
        Triangle((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), "a"),
        Triangle((1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0), "a"),
    ]
    rule = {"name": "left", "box": [[None, None, None], [0.5, None, None]]}
    claimed, rest = claim_part(quad, rule)
    assert claimed and rest
    total = sum(triangle_area(t) for t in claimed + rest)
    assert abs(total - 1.0) < 1e-9, total
    assert all(t.centroid()[0] <= 0.5 for t in claimed)
    assert all(t.centroid()[0] >= 0.5 for t in rest)
    for t in claimed:
        assert max(v[0] for v in t.verts()) <= 0.5 + 1e-9

    # A face COPLANAR with a box bound is boundary skin, not interior: no box
    # bounded at that plane claims it, whatever side float noise puts its
    # centroid on (the yaw-180 residue is ~1e-16). A box NOT bounded there
    # claims it as usual.
    noise = 1e-16
    wall = [
        Triangle((0.5 + noise, 0.0, 0.0), (0.5 - noise, 1.0, 0.0), (0.5, 0.0, 1.0), "a"),
        Triangle((0.5 - noise, 1.0, 0.0), (0.5 + noise, 1.0, 1.0), (0.5, 0.0, 1.0), "a"),
    ]
    for box in ([[0.5, None, None], [None, None, None]], [[None, None, None], [0.5, None, None]]):
        claimed, rest = claim_part(wall, {"name": "side", "box": box})
        assert not claimed and len(rest) >= 2, (box, len(claimed), len(rest))
    claimed, rest = claim_part(wall, {"name": "front", "box": [[None, None, None], [None, None, 2.0]]})
    assert len(claimed) == 2 and not rest

    # Material filter: non-matching fragments stay in the pool.
    two = [
        Triangle((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), "glass"),
        Triangle((2.0, 0.0, 0.0), (3.0, 0.0, 0.0), (2.0, 1.0, 0.0), "metal"),
    ]
    claimed, rest = claim_part(two, {"name": "canopy", "materials": ["glass"]})
    assert len(claimed) == 1 and claimed[0].material == "glass"
    assert len(rest) == 1 and rest[0].material == "metal"

    # Object filter mirrors the multi-object pack path.
    objs = [
        Triangle((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), "a", "hull"),
        Triangle((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), "a", "fin"),
    ]
    parts = segment_per_object(objs)
    assert [name for name, _, _ in parts] == ["hull", "fin"]

    # Anchors snap to the half-unit grid; explicit anchors win.
    tri = [Triangle((0.9, 0.0, 0.0), (1.4, 0.0, 0.0), (0.9, 0.6, 0.0), "a")]
    assert part_anchor(tri) == (1.0, 0.5, 0.0), part_anchor(tri)
    assert part_anchor(tri, {"anchor": [2, 0, 0]}) == (2.0, 0.0, 0.0)

    # Unclaimed triangles land in the rest part.
    parts = segment(quad, {"parts": [{"name": "left", "box": [[None, None, None], [0.5, None, None]]}], "rest": "body"})
    assert [name for name, _, _ in parts] == ["left", "body"]

    # --- link-point candidates ---

    # Two boxes meeting at x=1: one socket each, at the centre of the shared
    # face, normals opposed, positions LOCAL to each part's origin.
    boxes = [
        ("nose", (0.0, 0.0, 0.0), (0.0, -0.5, -0.5), (1.0, 0.5, 0.5)),
        ("body", (2.0, 0.0, 0.0), (-1.0, -0.5, -0.5), (0.0, 0.5, 0.5)),
    ]
    sockets = link_point_candidates(boxes)
    assert [p["id"] for p in sockets["nose"]] == ["to_body"], sockets
    assert [p["id"] for p in sockets["body"]] == ["to_nose"], sockets
    assert sockets["nose"][0]["position"] == [1.0, 0.0, 0.0], sockets["nose"]
    assert sockets["nose"][0]["normal"] == [1.0, 0.0, 0.0], sockets["nose"]
    # Same world point from the other part's origin, facing back.
    assert sockets["body"][0]["position"] == [-1.0, 0.0, 0.0], sockets["body"]
    assert sockets["body"][0]["normal"] == [-1.0, 0.0, 0.0], sockets["body"]

    # A 0.01 recipe offset outside the natural seam still counts as touching -
    # every shipped recipe cuts that way on purpose.
    offset = [
        ("nose", (0.0, 0.0, 0.0), (0.0, -0.5, -0.5), (0.99, 0.5, 0.5)),
        ("body", (2.0, 0.0, 0.0), (-1.0, -0.5, -0.5), (0.0, 0.5, 0.5)),
    ]
    assert len(link_point_candidates(offset)["nose"]) == 1

    # Parts that only meet along an EDGE share no face: a structural mate is a
    # surface. (These two touch at x=1 but their y ranges only meet at y=0.5.)
    edge = [
        ("a", (0.0, 0.0, 0.0), (0.0, -0.5, -0.5), (1.0, 0.5, 0.5)),
        ("b", (2.0, 1.0, 0.0), (-1.0, -0.5, -0.5), (0.0, 0.5, 0.5)),
    ]
    assert link_point_candidates(edge) == {"a": [], "b": []}, link_point_candidates(edge)

    # Parts that are simply apart get nothing either.
    apart = [
        ("a", (0.0, 0.0, 0.0), (0.0, -0.5, -0.5), (1.0, 0.5, 0.5)),
        ("b", (5.0, 0.0, 0.0), (-1.0, -0.5, -0.5), (0.0, 0.5, 0.5)),
    ]
    assert link_point_candidates(apart) == {"a": [], "b": []}

    # An explicit recipe list is authored in SHIP space and comes back local.
    rule = {
        "name": "nose",
        "link_points": [{"id": "dock", "position": [1.0, 0.25, 0.0], "normal": [1, 0, 0]}],
    }
    explicit = recipe_link_points(rule, (0.0, 0.0, 0.0))
    assert explicit == [
        {"id": "dock", "position": [1.0, 0.25, 0.0], "normal": [1, 0, 0]}
    ], explicit
    assert recipe_link_points(rule, (1.0, 0.0, 0.0))[0]["position"] == [0.0, 0.25, 0.0]
    # A rule with no list defers to the generator.
    assert recipe_link_points({"name": "nose"}, (0.0, 0.0, 0.0)) is None
    assert recipe_link_points(None, (0.0, 0.0, 0.0)) is None

    # --- capping ---

    def quads(faces, material="hull"):
        return [
            Triangle(q[0], q[1], q[2], material)
            for a, b, c, d in faces
            for q in ((a, b, c), (a, c, d))
        ]

    def cube_tris(lo, hi):
        """A closed axis box with OUTWARD winding on every face."""
        x0, y0, z0 = lo
        x1, y1, z1 = hi
        return quads(
            [
                ((x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)),  # +z
                ((x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0)),  # -z
                ((x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1)),  # +x
                ((x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0)),  # -x
                ((x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0)),  # +y
                ((x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)),  # -y
            ]
        )

    # Cube cut at x=0.5: both halves close watertight, every cap faces
    # OUTWARD from its half (deterministic winding), cap area = the 1x1
    # cross-section.
    cube = cube_tris((0.0, 0.0, 0.0), (1.0, 1.0, 1.0))
    assert count_boundary_edges(cube) == 0
    left, right = claim_part(
        cube, {"name": "left", "box": [[None, None, None], [0.5, None, None]]}
    )
    for half, out_sign in ((left, 1.0), (right, -1.0)):
        caps, notes = cap_cut_planes(half, [(0, 0.5)])
        assert caps and not notes, (len(caps), notes)
        assert count_boundary_edges(half + caps) == 0
        assert abs(sum(triangle_area(t) for t in caps) - 1.0) < 1e-9
        for t in caps:
            normal = flat_normal(t)
            assert normal[0] * out_sign > 0.999, (normal, out_sign)

    # Corner part (two orthogonal planes): the openings meet along an
    # interior joint line with no surface edges. Joint synthesis closes both
    # planar caps flush - watertight from plane caps ALONE, deterministic
    # winding on both openings, no 3D fan involved.
    corner_rule = {"name": "corner", "box": [[None, None, None], [0.5, None, 0.5]]}
    corner, remainder = claim_part(cube, corner_rule)
    for part, signs in ((corner, {0: 1.0, 2: 1.0}),):
        caps, notes = cap_cut_planes(part, [(0, 0.5), (2, 0.5)])
        assert not notes, notes
        assert count_boundary_edges(part + caps) == 0
        # Each opening is a 0.5 x 1 rectangle: total cap area 1.0.
        assert abs(sum(triangle_area(t) for t in caps) - 1.0) < 1e-9
        for t in caps:
            normal = flat_normal(t)
            axis = max(range(3), key=lambda k: abs(normal[k]))
            assert normal[axis] * signs[axis] > 0.999, normal
    # The remainder (an L-shaped solid) must also close from plane caps.
    caps, notes = cap_cut_planes(remainder, [(0, 0.5), (2, 0.5)])
    assert not notes, notes
    assert count_boundary_edges(remainder + caps) == 0

    # Non-convex (L-shaped) cross-section: ear clipping keeps every cap
    # triangle INSIDE the outline (a centroid fan would spill outside).
    outline = [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (1.0, 1.0), (1.0, 2.0), (0.0, 2.0)]
    ears = _ear_clip(outline)
    assert ears is not None and len(ears) == len(outline) - 2
    lid = [
        Triangle(
            (*outline[i], 1.0), (*outline[j], 1.0), (*outline[k], 1.0), "hull"
        )
        for i, j, k in ears
    ]
    floor = [Triangle(t.a, t.c, t.b, "hull") for t in lid]
    floor = [
        Triangle(
            (t.a[0], t.a[1], -1.0),
            (t.b[0], t.b[1], -1.0),
            (t.c[0], t.c[1], -1.0),
            "hull",
        )
        for t in floor
    ]
    walls = quads(
        [
            (
                (*outline[i], -1.0),
                (*outline[(i + 1) % len(outline)], -1.0),
                (*outline[(i + 1) % len(outline)], 1.0),
                (*outline[i], 1.0),
            )
            for i in range(len(outline))
        ]
    )
    prism = lid + floor + walls
    assert count_boundary_edges(prism) == 0
    halves = defaultdict(list)
    for tri in prism:
        for frag in split_triangle(tri, 2, 0.0):
            halves[frag.centroid()[2] >= 0.0].append(frag)
    for half in halves.values():
        caps, notes = cap_cut_planes(half, [(2, 0.0)])
        assert not notes, notes
        assert count_boundary_edges(half + caps) == 0
        assert abs(sum(triangle_area(t) for t in caps) - 3.0) < 1e-9
        for t in caps:
            cen = t.centroid()
            assert _point_in_poly((cen[0], cen[1]), outline), cen

    # Cross-section with a HOLE (a square tube cut mid-length): the hole is
    # detected, bridged, loudly noted, and no cap covers the bore.
    ring = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
    bore = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
    tube = quads(
        [
            (
                (*ring[i], -1.0),
                (*ring[(i + 1) % 4], -1.0),
                (*ring[(i + 1) % 4], 1.0),
                (*ring[i], 1.0),
            )
            for i in range(4)
        ]
        + [
            (
                (*bore[(i + 1) % 4], -1.0),
                (*bore[i], -1.0),
                (*bore[i], 1.0),
                (*bore[(i + 1) % 4], 1.0),
            )
            for i in range(4)
        ]
    )
    bottom = [
        frag
        for tri in tube
        for frag in split_triangle(tri, 2, 0.0)
        if frag.centroid()[2] < 0.0
    ]
    caps, notes = cap_cut_planes(bottom, [(2, 0.0)])
    assert any("hole" in note for note in notes), notes
    assert abs(sum(triangle_area(t) for t in caps) - 3.0) < 1e-9
    for t in caps:
        cen = t.centroid()
        assert not (abs(cen[0]) < 0.5 and abs(cen[1]) < 0.5), cen

    # Watertight input stays untouched: no cut planes -> no plane caps, and
    # the fallback walk finds no boundary to fan (the identity-import path).
    caps, notes = cap_cut_planes(cube, [])
    assert caps == [] and notes == []
    assert cap_boundary(cube) == []

    # glb round-trip: write, re-open, decoded bounds match.
    import tempfile

    mats, index = build_materials({"a": (1.0, 0.0, 0.0)})
    blob = write_glb(quad, mats, index, GENERATOR)
    with tempfile.NamedTemporaryFile(suffix=".glb", delete=False) as handle:
        handle.write(blob)
        path = handle.name
    try:
        doc, positions = read_glb(path)
        assert len(positions) == 6
        lo = [min(p[k] for p in positions) for k in range(3)]
        hi = [max(p[k] for p in positions) for k in range(3)]
        assert lo == [0.0, 0.0, 0.0] and hi == [1.0, 1.0, 0.0], (lo, hi)
        verify_part(path, 2, ((0.0, 0.0, 0.0), (1.0, 1.0, 0.0)))
    finally:
        os.unlink(path)

    print("self-test OK")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Cut a monolithic .obj spaceship into semantic parts (.glb) via a JSON recipe."
    )
    parser.add_argument("obj", nargs="?", help="input .obj path")
    parser.add_argument("--out", help="output folder for the part .glb meshes + manifest.json")
    parser.add_argument("--recipe", help="per-ship part recipe (JSON)")
    parser.add_argument(
        "--per-object",
        action="store_true",
        help="one part per obj o/g group (multi-object parts packs)",
    )
    parser.add_argument(
        "--scale", type=float, default=None, help="uniform scale (overrides recipe; default 1.0)"
    )
    parser.add_argument(
        "--yaw",
        type=float,
        default=None,
        help="degrees about Y before cutting (overrides recipe; default 0)",
    )
    parser.add_argument(
        "--caps",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="cap the open cut edges with a flat fill (default on)",
    )
    parser.add_argument("--self-test", action="store_true", help="run internal checks and exit")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()
    if not args.obj:
        parser.error("the obj argument is required (or pass --self-test)")
    if not args.out:
        parser.error("--out is required")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
