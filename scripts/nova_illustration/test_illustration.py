"""Unit checks for the shared art sources; no browser or asset writes."""

import ast
import math
import re
import runpy
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
from nova_illustration.lettering import labelled_speech
from nova_illustration.portraits import elena_close, elena_gesture, gantry_portrait, gripping_arm, reaching_arm, jonah_listener, work_inspection, work_portrait, supported_owen, freefall_guide, stretcher_grip, rail_grip_hand
from nova_illustration.scenery import aquila, transfer_hall
from nova_illustration.ships import DRAW_TOLERANCE, Face, MODELS, VIEWS, bounds, box, contour_segments, dot, draw_precision, hull_segment, normal, painter_order, render_faces, render_ship, ship_faces, split_surface, sub
from nova_illustration.styles import present


class IllustrationTests(unittest.TestCase):
    """Protect identity, projection, palette ownership, and deterministic exports."""

    def test_supported_body_keeps_owens_original_head_without_equipment_or_injury(self):
        art = supported_owen()
        self.assertEqual(art, supported_owen())
        self.assertIn(frontal_head('owen'), art)
        root = ET.fromstring(art)
        self.assertEqual(root.get('data-pose'), 'supported-recline')
        self.assertFalse(any(n.get('data-prop') or n.get('data-expression') for n in root.iter()))
        self.assertNotIn('rotate(', art)
        for scheme in ('comic', 'lore'):
            self.assertIn(frontal_head('owen'), present(art, scheme, 'supported'))

    def test_freefall_guides_continue_the_existing_bodies_without_replacing_the_faces(self):
        for name, original in [('rina', work_portrait('rina')), ('ivo', gantry_portrait('ivo'))]:
            art = freefall_guide(name)
            self.assertIn(original, art)
            self.assertEqual(art, freefall_guide(name))
            self.assertEqual(ET.fromstring(art).get('data-pose'), 'freefall-guide')
        with self.assertRaises(KeyError):
            freefall_guide('unknown')

    def test_support_grips_own_no_heads_or_rails_and_reject_unknown_people(self):
        for name in ('rina', 'ivo', 'samir'):
            for draw in (stretcher_grip, rail_grip_hand):
                art = draw(name)
                self.assertEqual(art, draw(name))
                root = ET.fromstring(art)
                self.assertFalse(any(n.get('data-face') or n.get('data-prop') for n in root.iter()))
            self.assertEqual(ET.fromstring(stretcher_grip(name)).get('data-grip'), '430 433')
        for draw in (stretcher_grip, rail_grip_hand):
            with self.assertRaises(KeyError):
                draw('unknown')

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
        for name in ('leila', 'rina', 'samir', 'tomas'):
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
        for name in ('unknown',''):
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
        poses += [(name,lambda expression='original', name=name: work_portrait(name,expression)) for name in ('leila','rina','samir','tomas')]
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
        for name in ('elena','leila','samir','tomas'):
            self.assertEqual(expression_names(name),('original',))
        for name in ('unknown',''):
            with self.assertRaises(KeyError):
                expression_names(name)
        for name, expression in [('rina','wry'),('jonah','amused'),('elena','amused'),('leila','focused'),('samir','amused'),('tomas','attentive'),('rina',''),('jonah','typo')]:
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

    def test_samir_reuses_the_lore_face_and_all_five_public_portraits_stay_exact(self):
        exporter = runpy.run_path(str(SCRIPTS/'gen-lore-portraits.py'))
        samir = next(p for p in exporter['PORTRAITS'] if p.name == 'Samir Bell')
        for field in ('head','back_hair','front_hair','features'):
            self.assertEqual(getattr(samir,field),getattr(FACES['samir'],field))
        for portrait in exporter['PORTRAITS']:
            slug = portrait.name.lower().replace(' ','-')
            asset = SCRIPTS.parent/f'web/src/assets/lore/{slug}-portrait-concept.svg'
            self.assertEqual(exporter['render'](portrait),asset.read_text())

    def test_inspection_layers_keep_the_face_and_allow_a_surface_between_body_and_hands(self):
        for name in ('leila','rina','samir','tomas'):
            for expression in expression_names(name):
                body,hands = work_inspection(name,expression)
                self.assertEqual((body,hands),work_inspection(name,expression))
                self.assertEqual(body,work_portrait(name,expression))
                self.assertIn(frontal_head(name,expression),body)
                self.assertNotIn('data-face',hands)
                self.assertNotIn('<text',hands)
                drawing = body+'<rect data-work-surface="true"/>'+hands
                roots = [ET.fromstring('<svg>'+present(drawing,scheme,scheme)+'</svg>') for scheme in ('comic','lore')]
                self.assertEqual([n.attrib for n in roots[0].iter('path')],[n.attrib for n in roots[1].iter('path')])
        with self.assertRaises(KeyError):
            work_inspection('unknown')
        with self.assertRaises(KeyError):
            work_inspection('samir','amused')

    def test_gantry_condition_only_replaces_the_two_authored_service_covers(self):
        intact = ship_faces('gantry')
        stranded = ship_faces('gantry','stranded')
        replaced = {'gantry-service-cover','gantry-service-roof'}
        self.assertEqual(tuple(f for f in intact if f.component not in replaced),stranded[:len(intact)-sum(f.component in replaced for f in intact)])
        self.assertTrue(all(f.component not in replaced for f in stranded))
        for component in ('crew-module','gantry-transfer-collar','gantry-transfer-hatch','gantry-cargo-body','gantry-frame-crossbar'):
            self.assertEqual([f for f in intact if f.component==component],[f for f in stranded if f.component==component])
        for f in stranded:
            self.assertIn(f.material,MATERIALS)
            n = normal(f.vertices)
            for v in f.vertices:
                self.assertTrue(all(math.isfinite(c) for c in v))
                self.assertAlmostEqual(dot(n,sub(v,f.vertices[0])),0)
        for view in VIEWS:
            drawing = render_ship('gantry',view,0,0,1,False,'stranded')
            self.assertEqual(drawing,render_ship('gantry',view,0,0,1,False,'stranded'))
            self.assertEqual(ET.fromstring(drawing).get('data-state'),'stranded')
            self.assertEqual(bounds('gantry',view),bounds('gantry',view,'stranded'))
            variants = [ET.fromstring('<svg>'+present(drawing,s,s)+'</svg>') for s in ('comic','lore')]
            self.assertEqual([n.attrib for n in variants[0].iter('polygon')],[n.attrib for n in variants[1].iter('polygon')])

    def test_ship_conditions_and_thrust_are_explicit_and_invalid_pairs_fail(self):
        for name in MODELS:
            self.assertEqual(render_ship(name,'side',0,0,1,False),render_ship(name,'side',0,0,1,False,'intact'))
        for name,state in [('unknown','intact'),('kaveri','stranded'),('ebro','stranded'),('gantry','destroyed'),('gantry','')]:
            with self.assertRaises(KeyError):
                ship_faces(name,state)
            with self.assertRaises(KeyError):
                render_ship(name,'side',0,0,1,False,state)
        with self.assertRaises(ValueError):
            render_ship('gantry','side',0,0,1,True,'stranded')
        self.assertNotEqual(render_ship('gantry','side',0,0,1,True),render_ship('gantry','side',0,0,1,False))

    def test_gantry_portraits_share_heads_but_have_distinct_civilian_bodies(self):
        bodies = []
        for name in ('nadia','owen','ivo'):
            self.assertEqual(expression_names(name),('original',))
            drawing = gantry_portrait(name)
            head = frontal_head(name)
            self.assertEqual(drawing,gantry_portrait(name,'original'))
            self.assertEqual(drawing.count(head),1)
            bodies.append(tuple(n.get('d') for n in ET.fromstring(drawing.replace(head,'')).iter('path')))
            roots = [ET.fromstring('<svg>'+present(drawing,s,s)+'</svg>') for s in ('comic','lore')]
            self.assertEqual([n.attrib for n in roots[0].iter('path')],[n.attrib for n in roots[1].iter('path')])
            with self.assertRaises(KeyError):
                gantry_portrait(name,'amused')
        self.assertEqual(len(set(bodies)),3)
        self.assertEqual(len({FACES[n].head for n in ('nadia','owen','ivo')}),3)
        with self.assertRaises(KeyError):
            gantry_portrait('samir')

    def test_public_gantry_exports_contain_only_starting_identities(self):
        exporter = runpy.run_path(str(SCRIPTS/'gen-lore-designs.py'))
        sheet = exporter['ship_sheet']('gantry','lore')
        self.assertNotIn('data-state',sheet)
        self.assertNotIn('scorch',sheet)
        self.assertNotIn('stranded',sheet.lower())
        self.assertEqual(len(ET.fromstring(sheet).findall('.//*[@data-ship="gantry"]')),4)
        for name in ('nadia','owen','ivo'):
            drawing = exporter['crew_portrait'](name,'lore')
            self.assertIn(frontal_head(name),drawing)
            for marker in ('injury','rescued','stranded','pirate'):
                self.assertNotIn(marker,drawing.lower())

    def test_props_share_ship_occlusion_without_changing_empty_cargo_renders(self):
        cargo = tuple(box((-30,0,58),(90,80,40),'accent','test-cargo'))
        for view in VIEWS:
            bare = render_ship('kaveri',view,0,0,1,False)
            self.assertEqual(bare,render_ship('kaveri',view,0,0,1,False,cargo=()))
            loaded = render_ship('kaveri',view,0,0,1,False,cargo=cargo)
            self.assertIn('test-cargo',loaded)
            self.assertNotEqual(bare,loaded)
            self.assertEqual(loaded,render_ship('kaveri',view,0,0,1,False,cargo=cargo))
            prop = ET.fromstring(render_faces(cargo,view,0,0,1))
            self.assertTrue(prop.findall('.//polygon'))
        self.assertEqual(len(cargo),6)

    def test_exported_precision_follows_the_drawn_size_rather_than_the_model(self):
        self.assertEqual([draw_precision(s) for s in (.16,.19,.23,.64,1.15,2.4,3.4)],[0,0,1,1,1,2,2])
        for scale in (.05,.16,.5,1,2.4,17,240):
            digits = draw_precision(scale)
            self.assertLessEqual(scale*10**-digits,DRAW_TOLERANCE)
            self.assertGreater(scale*10**(1-digits),DRAW_TOLERANCE if digits else 0)
        for name in MODELS:
            for scale in (.16,1.15,3.4):
                quantum = 10**draw_precision(scale)
                drawing = render_ship(name,'forward-quarter',0,0,scale,False)
                emitted = [float(n) for p in ET.fromstring(drawing).iter('polygon')
                           for pair in p.get('points').split() for n in pair.split(',')]
                self.assertTrue(emitted)
                for value in emitted:
                    self.assertAlmostEqual(value*quantum,round(value*quantum),places=6)
            self.assertLess(len(render_ship(name,'forward-quarter',0,0,.16,False)),
                            len(render_ship(name,'forward-quarter',0,0,3.4,False)))
        for scale in (0,-1,float('nan'),float('inf')):
            with self.assertRaises(ValueError):
                draw_precision(scale)

    def test_a_surface_smaller_than_its_own_drawn_precision_leaves_no_sliver(self):
        speck = tuple(box((0,0,0),(.02,.02,.02),'metal','speck'))
        self.assertFalse(ET.fromstring(render_faces(speck,'top',0,0,DRAW_TOLERANCE)).findall('.//polygon'))
        self.assertTrue(ET.fromstring(render_faces(speck,'top',0,0,200)).findall('.//polygon'))

    def test_invalid_prop_surfaces_and_placement_fail_before_svg_is_written(self):
        valid = tuple(box((0,0,0),(10,10,10),'metal','prop'))
        for x,y,scale in ((float('nan'),0,1),(0,float('inf'),1),(0,0,-1),(0,0,0)):
            with self.assertRaises(ValueError):
                render_faces(valid,'top',x,y,scale)
        for vertices in ((),((0,0,0),(1,1,1)),((0,0,0),(1,0,0),(0,float('nan'),0))):
            with self.assertRaises(ValueError):
                render_faces((Face(vertices,'metal','bad'),),'top',0,0,1)
        with self.assertRaises(KeyError):
            render_faces((Face(valid[0].vertices,'unknown','bad'),),'top',0,0,1)
        with self.assertRaises(KeyError):
            render_faces(valid,'unknown',0,0,1)

    def test_handhold_arms_do_not_own_heads_expressions_or_fixtures(self):
        for name in FACES:
            for draw in (gripping_arm,reaching_arm):
                art = draw(name)
                self.assertEqual(art,draw(name))
                root = ET.fromstring(art)
                self.assertEqual(root.get('data-character'),name)
                for attribute in ('data-face','data-expression','data-prop'):
                    self.assertFalse(root.findall(f'.//*[@{attribute}]'))
                self.assertFalse(root.findall('.//text'))
                self.assertFalse(root.findall('.//filter'))
            self.assertNotEqual(gripping_arm(name),reaching_arm(name))
        with self.assertRaises(KeyError):
            gripping_arm('unknown')
        with self.assertRaises(KeyError):
            reaching_arm('unknown')

    def test_aquila_scenery_keeps_ships_out_and_presentation_color_only(self):
        art = aquila(300,200,.7)+transfer_hall(600,400)
        comic = ET.fromstring('<svg>'+present(art,'comic','aquila-comic')+'</svg>')
        lore = ET.fromstring('<svg>'+present(art,'lore','aquila-lore')+'</svg>')
        places = lambda root: [ET.tostring(n) for n in root.findall('.//*[@data-place]')]
        self.assertEqual(places(comic),places(lore))
        self.assertFalse(comic.findall('.//*[@data-ship]'))
        self.assertFalse(comic.findall('.//*[@data-face]'))
        self.assertEqual(comic.find('.//*[@data-gravity]').get('data-gravity'),'freefall')
        self.assertEqual(art,aquila(300,200,.7)+transfer_hall(600,400))

    def test_labelled_balloons_keep_speaker_and_words_with_four_authored_tail_sides(self):
        for side in ('top','bottom','left','right'):
            art = labelled_speech(0,0,250,'Rina & Jonah',['Keep it clear.'],(280,70),side)
            root = ET.fromstring(art)
            self.assertEqual([n.text for n in root.findall('text')],['RINA & JONAH','Keep it clear.'])
            self.assertEqual(root.get('data-height'),'69')
            self.assertEqual(root.get('class'),'dialogue')
        with self.assertRaises(ValueError):
            labelled_speech(0,0,250,'Rina',['Keep it clear.'],(0,0),'unknown')

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
