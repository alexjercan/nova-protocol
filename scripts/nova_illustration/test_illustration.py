"""Unit checks for the shared art sources; no browser or asset writes."""

import ast
import math
import re
import sys
import unittest
from pathlib import Path
from xml.etree import ElementTree as ET

sys.dont_write_bytecode = True
SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(SCRIPTS))
from nova_illustration.colors import ELENA, MATERIALS
from nova_illustration.faces import ELENA_FACE, JONAH_FACE, frontal_head
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener
from nova_illustration.ships import Face, MODELS, VIEWS, bounds, contour_segments, dot, hull_segment, normal, painter_order, render_ship, split_surface, sub
from nova_illustration.styles import present


class IllustrationTests(unittest.TestCase):
    """Protect identity, projection, palette ownership, and deterministic exports."""

    def test_ship_models_have_finite_planar_faces_and_known_materials(self):
        for name,build in MODELS.items():
            self.assertEqual(build(),build(),name)
            for face in build():
                self.assertIn(face.material,MATERIALS)
                self.assertTrue(face.component)
                self.assertTrue(all(math.isfinite(c) for p in face.vertices for c in p))
                n = normal(face.vertices)
                self.assertAlmostEqual(dot(n,n),1)
                for p in face.vertices:
                    self.assertAlmostEqual(dot(n,sub(p,face.vertices[0])),0)

    def test_plate_partitions_occlude_crossing_surfaces_instead_of_sorting_their_centers(self):
        deck = Face(((-10,-10,0),(10,-10,0),(10,10,0),(-10,10,0)), 'plate', 'deck')
        ramp = Face(((-2,-3,-1),(2,-3,1),(2,3,1),(-2,3,-1)), 'metal', 'ramp')
        ordered = painter_order([(ramp,normal(ramp.vertices)),(deck,normal(deck.vertices))])
        self.assertEqual([f.component for f,_ in ordered], ['ramp','deck','ramp'])
        self.assertTrue(all(v[2] <= 1e-7 for v in ordered[0][0].vertices))
        self.assertTrue(all(v[2] >= -1e-7 for v in ordered[2][0].vertices))

    def test_painter_splits_do_not_add_inked_seams(self):
        face = Face(((-10,-10,0),(10,-10,0),(10,10,0),(-10,10,0)), 'plate', 'deck')
        parts = split_surface(face,(1,0,0),0)
        edges = [edge for part in parts for edge in contour_segments(part)]
        self.assertEqual(len(edges),6)
        self.assertAlmostEqual(sum(math.dist(a,b) for a,b in edges),80)
        for a,b in edges:
            self.assertFalse(a[0] == b[0] == 0)

    def test_plating_has_sloped_surfaces_and_invalid_bevels_fail(self):
        faces = hull_segment(0,100,40,30,0,60,40,10,'plate','test')
        self.assertTrue(any(sum(abs(c)>1e-7 for c in normal(f.vertices))>=2 for f in faces))
        with self.assertRaises(ValueError):
            hull_segment(0,100,40,30,0,60,40,25,'plate','test')

    def test_orthographic_views_preserve_the_same_longitudinal_extent(self):
        for name in MODELS:
            top,side = bounds(name,'top'),bounds(name,'side')
            self.assertAlmostEqual(top[2]-top[0],side[2]-side[0])

    def test_every_registered_view_is_deterministic_and_contains_visible_surfaces(self):
        for name in MODELS:
            for view in VIEWS:
                first = render_ship(name,view,0,0,1,False)
                self.assertEqual(first,render_ship(name,view,0,0,1,False))
                self.assertTrue(ET.fromstring(first).findall('.//polygon'))

    def test_color_schemes_preserve_all_ship_surface_coordinates(self):
        for name in MODELS:
            drawing = render_ship(name,'aft-quarter',400,300,.8,False)
            comic = ET.fromstring('<svg>'+present(drawing,'comic','comic-view')+'</svg>')
            lore = ET.fromstring('<svg>'+present(drawing,'lore','lore-view')+'</svg>')
            surfaces = lambda root: [n.attrib for n in root.findall('.//polygon')]
            self.assertEqual(surfaces(comic),surfaces(lore))
            self.assertEqual(len(lore.findall('.//filter')),1)
            self.assertNotIn('filter',lore.attrib)
            self.assertEqual(len(comic.findall('.//filter')),0)

    def test_portraits_have_distinct_authored_poses_and_share_the_color_treatment(self):
        self.assertNotEqual(elena_close(),elena_gesture())
        for pose in (elena_close,elena_gesture,jonah_listener):
            drawing = pose()
            comic = ET.fromstring('<svg>'+present(drawing,'comic','color')+'</svg>')
            lore = ET.fromstring('<svg>'+present(drawing,'lore','green')+'</svg>')
            paths = lambda root: [n.attrib for n in root.findall('.//path')]
            self.assertEqual(paths(comic),paths(lore))
        with self.assertRaises(TypeError):
            ELENA['skin'] = 'another color'

    def test_both_elena_poses_reuse_the_same_forward_head(self):
        face = frontal_head('elena')
        self.assertIn(face, elena_close())
        self.assertIn(face, elena_gesture())
        for name, drawing in [('elena', ELENA_FACE), ('jonah', JONAH_FACE)]:
            root = ET.fromstring(frontal_head(name))
            self.assertEqual(root.get('data-gaze'), 'forward')
            self.assertIn(drawing.head, [p.get('d') for p in root.findall('.//path')])

    def test_unknown_ship_view_scheme_and_unsafe_instance_are_errors(self):
        for args in [('unknown','top'),('kaveri','unknown')]:
            with self.assertRaises(KeyError):
                render_ship(*args,0,0,1,False)
        with self.assertRaises(ValueError):
            present('','unknown','valid')
        with self.assertRaises(ValueError):
            present('','lore','not an SVG key')

    def test_color_literals_live_only_in_the_palette_module(self):
        sources = list(Path(__file__).parent.glob('*.py'))+[SCRIPTS/'gen-lore-designs.py']
        for source in sources:
            if source.name=='colors.py':
                continue
            tree = ast.parse(source.read_text())
            for node in ast.walk(tree):
                if isinstance(node,ast.Constant) and isinstance(node.value,str):
                    self.assertIsNone(re.search(r'#[0-9a-fA-F]{6}\b',node.value),f'{source}:{node.lineno}')

    def test_nonfinite_placement_and_mirrored_scales_are_rejected(self):
        for x,y,scale in [(float('nan'),0,1),(0,float('inf'),1),(0,0,0),(0,0,-1)]:
            with self.assertRaises(ValueError):
                render_ship('kaveri','top',x,y,scale,False)

    def test_multiple_lore_instances_do_not_duplicate_filter_ids(self):
        drawing = elena_close()
        root = ET.fromstring('<svg>'+present(drawing,'lore','left')+present(drawing,'lore','right')+'</svg>')
        ids = [n.attrib['id'] for n in root.iter() if 'id' in n.attrib]
        self.assertEqual(len(ids),len(set(ids)))


if __name__=='__main__':
    unittest.main()
