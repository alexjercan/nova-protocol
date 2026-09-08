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
from nova_illustration.expressions import EXPRESSIONS, facial_features
from nova_illustration.faces import FACES, expression_names, frontal_head
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener, work_portrait
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
        for name, drawing in FACES.items():
            root = ET.fromstring(frontal_head(name))
            self.assertEqual(root.get('data-gaze'), 'forward')
            self.assertIn(drawing.head, [p.get('d') for p in root.findall('.//path')])

    def test_work_busts_reuse_frontal_heads_and_only_change_color_under_lore_presentation(self):
        for name in ('leila', 'rina', 'tomas'):
            drawing = work_portrait(name)
            self.assertEqual(drawing,work_portrait(name))
            self.assertIn(frontal_head(name),drawing)
            root = ET.fromstring(drawing)
            self.assertEqual(root.get('data-character'),name)
            comic = ET.fromstring('<svg>'+present(drawing,'comic','color')+'</svg>')
            lore = ET.fromstring('<svg>'+present(drawing,'lore','green')+'</svg>')
            paths = lambda root: [n.attrib for n in root.findall('.//path')]
            self.assertEqual(paths(comic),paths(lore))

    def test_unregistered_heads_and_work_busts_are_errors(self):
        for name in ('unknown','samir'):
            with self.assertRaises(KeyError):
                frontal_head(name)
            with self.assertRaises(KeyError):
                work_portrait(name)
        with self.assertRaises(KeyError):
            work_portrait('elena')

    def test_omitted_expression_keeps_the_original_features(self):
        for name, face in FACES.items():
            drawing = frontal_head(name)
            self.assertEqual(drawing,frontal_head(name,'original'))
            self.assertIn(face.features,drawing)
            self.assertNotIn('data-expression',drawing)
        for name in EXPRESSIONS:
            self.assertEqual(FACES[name].features,facial_features(name,'original'))

    def test_expressions_replace_features_without_replacing_the_likeness(self):
        for name, variants in EXPRESSIONS.items():
            original = frontal_head(name)
            for expression, features in variants.items():
                with self.subTest(name=name,expression=expression):
                    drawing = frontal_head(name,expression)
                    self.assertEqual(drawing,frontal_head(name,expression))
                    self.assertEqual(drawing.count(features),1)
                    retained = drawing.replace(features,'FEATURES',1)
                    retained = retained.replace(f' data-expression="{expression}"','')
                    self.assertEqual(retained,original.replace(FACES[name].features,'FEATURES',1))
                    root = ET.fromstring(drawing)
                    self.assertEqual(root.get('data-gaze'),'forward')
                    if expression != 'original':
                        self.assertNotEqual(features,FACES[name].features)
                        self.assertNotIn(FACES[name].features,drawing)
                        self.assertEqual(root.get('data-expression'),expression)

    def test_body_helpers_pass_expressions_without_changing_the_pose(self):
        poses = [('elena',elena_close),('elena',elena_gesture),('jonah',jonah_listener)]
        poses += [(name,lambda expression='original', name=name: work_portrait(name,expression)) for name in ('leila','rina','tomas')]
        for name, pose in poses:
            for expression in expression_names(name):
                head = frontal_head(name,expression)
                drawing = pose(expression=expression)
                self.assertIn(head,drawing)
                self.assertEqual(drawing.replace(head,'HEAD',1),pose().replace(frontal_head(name),'HEAD',1))

    def test_expression_instances_keep_geometry_under_color_changes_and_do_not_collide(self):
        for name in FACES:
            for expression in expression_names(name):
                drawing = frontal_head(name,expression)
                comic = ET.fromstring('<svg>'+present(drawing,'comic','comic')+'</svg>')
                lore = ET.fromstring('<svg>'+present(drawing,'lore','left')+present(drawing,'lore','right')+'</svg>')
                self.assertEqual(ET.tostring(comic.find('.//*[@data-face]')),ET.tostring(lore.find('.//*[@data-face]')))
                ids = [n.get('id') for n in lore.iter() if 'id' in n.attrib]
                self.assertEqual(len(ids),len(set(ids)))
                self.assertEqual(len(lore.findall('.//filter')),2)

    def test_expression_names_are_character_specific_and_unknown_pairs_fail(self):
        self.assertEqual(expression_names('rina'),('original','amused'))
        self.assertEqual(expression_names('jonah'),('original','wry'))
        for name in ('elena','leila','tomas'):
            self.assertEqual(expression_names(name),('original',))
        for name in ('unknown','samir'):
            with self.assertRaises(KeyError):
                expression_names(name)
        for name, expression in [('rina','wry'),('jonah','amused'),('elena','amused'),('leila','focused'),('tomas','attentive'),('rina',''),('jonah','typo')]:
            with self.assertRaises(KeyError):
                frontal_head(name,expression)
        with self.assertRaises(KeyError):
            jonah_listener(expression='amused')
        with self.assertRaises(KeyError):
            work_portrait('rina',expression='wry')
        with self.assertRaises(TypeError):
            EXPRESSIONS['rina'] = {}
        with self.assertRaises(TypeError):
            EXPRESSIONS['rina']['amused'] = 'replacement'

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
