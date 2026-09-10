"""Check the drawing arrangement without claiming engineering compatibility."""

from collections import defaultdict
import math
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from docking import BORE_RADIUS, SLEEVE_SPAN, collar_mouth, connector_faces, gantry_point, paired_faces, tilted_point
from nova_illustration.ships import ship_faces, split_surface
from props import assembly_faces


def component_bounds(faces):
    """Conservative component boxes; disjoint boxes prove disjoint drawing solids."""
    vertices = defaultdict(list)
    for face in faces:
        vertices[face.component].extend(face.vertices)
    return {name: tuple((min(v[k] for v in points), max(v[k] for v in points)) for k in range(3))
            for name, points in vertices.items()}


def nearest_projected_edge(face):
    """Find the bore-axis distance to an annular sector's projected convex boundary."""
    cx, _, cz = collar_mouth('kaveri')
    points = [(v[0] - cx, v[2] - cz) for v in face.vertices]
    signs, distances = [], []
    for a, b in zip(points, points[1:] + points[:1]):
        dx, dz = b[0] - a[0], b[1] - a[1]
        length = dx * dx + dz * dz
        if length < 1e-12:
            continue
        signs.append(dx * -a[1] - dz * -a[0])
        t = max(0, min(1, -(a[0] * dx + a[1] * dz) / length))
        distances.append(math.hypot(a[0] + t * dx, a[1] + t * dz))
    area = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(points, points[1:] + points[:1]))
    if abs(area) > 1e-8 and (all(s >= -1e-8 for s in signs) or all(s <= 1e-8 for s in signs)):
        return 0
    return min(distances)


class DockingDrawingTests(unittest.TestCase):
    def test_mating_planes_come_from_the_existing_collars(self):
        self.assertEqual(collar_mouth('kaveri'), (112, -97.5, 42))
        self.assertEqual(collar_mouth('gantry'), (133, -105, 43))
        k = collar_mouth('kaveri')
        g = gantry_point(collar_mouth('gantry'), 0)
        self.assertEqual(g, (k[0], k[1] - SLEEVE_SPAN, k[2]))
        sleeve = [v for f in connector_faces() for v in f.vertices]
        self.assertEqual(min(v[1] for v in sleeve), g[1])
        self.assertEqual(max(v[1] for v in sleeve), k[1])

    def test_shared_hulls_damage_and_load_are_not_resized_or_replaced(self):
        pair = paired_faces(0)
        kaveri = [f for f in pair if f.component.startswith('kaveri/')]
        original = ship_faces('kaveri') + assembly_faces()
        self.assertEqual(len(kaveri), len(original))
        for actual, expected in zip(kaveri, original, strict=True):
            self.assertEqual(actual.vertices, expected.vertices)
            self.assertEqual(actual.material, expected.material)
            self.assertEqual(actual.component, 'kaveri/' + expected.component)
        gantry = [f for f in pair if f.component.startswith('gantry/')]
        original = ship_faces('gantry', 'stranded')
        self.assertEqual(len(gantry), len(original))
        tx, ty, tz = gantry_point((0, 0, 0), 0)
        for actual, expected in zip(gantry, original, strict=True):
            self.assertEqual(actual.vertices, tuple((tx - x, ty - y, tz + z) for x, y, z in expected.vertices))
            self.assertEqual(actual.material, expected.material)
            self.assertEqual(actual.component, 'gantry/' + expected.component)

    def test_opposing_headings_share_the_same_up_direction(self):
        origin = gantry_point((0, 0, 0), 0)
        for local, expected in [((1, 0, 0), (-1, 0, 0)), ((0, 0, 1), (0, 0, 1))]:
            self.assertEqual(tuple(a - b for a, b in zip(gantry_point(local, 0), origin)), expected)
        a, b = (23, -87, 65), (213, 99, -30)
        self.assertAlmostEqual(math.dist(a, b), math.dist(tilted_point(a), tilted_point(b)))

    def test_approach_changes_only_the_relative_gap(self):
        for a, b in zip(paired_faces(0), paired_faces(120), strict=True):
            self.assertEqual(a.component, b.component)
            delta = -120 if a.component.startswith('gantry/') else 0
            for expected, actual in zip(a.vertices, b.vertices, strict=True):
                self.assertAlmostEqual(actual[0], expected[0], places=10)
                self.assertAlmostEqual(actual[1], expected[1] + delta, places=10)
                self.assertAlmostEqual(actual[2], expected[2], places=10)

    def test_hull_boxes_clear_each_other_at_contact_and_through_closure(self):
        boxes = component_bounds(paired_faces(0))
        kaveri = {k:v for k,v in boxes.items() if k.startswith('kaveri/')}
        gantry = {k:v for k,v in boxes.items() if k.startswith('gantry/')}
        for a, ka in kaveri.items():
            for b, gb in gantry.items():
                fixed_clearance = any(ka[k][1] < gb[k][0] or gb[k][1] < ka[k][0] for k in (0, 2))
                opening_clearance = gb[1][1] < ka[1][0]
                self.assertTrue(fixed_clearance or opening_clearance, (a, b))

    def test_connector_is_hollow_and_the_shared_hatches_remain_closed(self):
        clearance = min(nearest_projected_edge(f) for f in connector_faces())
        self.assertGreaterEqual(clearance, BORE_RADIUS * math.cos(math.pi / 16) - 1e-7)
        names = {f.component for f in paired_faces(0)}
        self.assertIn('kaveri/transfer-hatch', names)
        self.assertIn('gantry/gantry-transfer-hatch', names)

    def test_sleeve_clears_surrounding_plating_and_encloses_the_closed_hatches(self):
        cx, near, cz = collar_mouth('kaveri')
        far = near - SLEEVE_SPAN
        for face in paired_faces(0):
            if face.component.startswith('connector/'):
                continue
            if min(v[1] for v in face.vertices) >= near or max(v[1] for v in face.vertices) <= far:
                continue
            inside_near, _ = split_surface(face, (0, 1, 0), near)
            _, clipped = split_surface(inside_near, (0, 1, 0), far)
            bounds = component_bounds([clipped])[face.component]
            closest = math.hypot(max(bounds[0][0] - cx, 0, cx - bounds[0][1]),
                                 max(bounds[2][0] - cz, 0, cz - bounds[2][1]))
            farthest = max(math.hypot(v[0] - cx, v[2] - cz) for v in clipped.vertices)
            self.assertTrue(closest > 26 or farthest < BORE_RADIUS * math.cos(math.pi / 16), face.component)

    def test_invalid_gaps_fail_instead_of_overlapping_the_pair(self):
        for value in (-1, float('nan'), float('inf'), True, 'contact'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                paired_faces(value)


if __name__ == '__main__':
    unittest.main()
