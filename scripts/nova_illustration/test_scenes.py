"""Standalone comic scene boundary checks."""

import unittest
from xml.etree import ElementTree as ET

from nova_illustration.faces import FACES, frontal_head
from nova_illustration.scenes import Scene, render_scene, text_slot


class SceneTests(unittest.TestCase):
    def test_scene_has_local_dimensions_without_page_furniture(self):
        scene = ET.fromstring(render_scene(Scene(200,100,'<rect width="200" height="100"/>')))
        self.assertEqual(scene.get('viewBox'),'0 0 200 100')
        self.assertFalse(any(node.tag.endswith('text') for node in scene.iter()))

    def test_text_slot_reserves_painter_order_without_owning_words(self):
        source = render_scene(Scene(200,100,text_slot('display')))
        root = ET.fromstring(source)
        slot = next(n for n in root.iter() if n.get('data-story-slot'))
        self.assertEqual(slot.get('data-story-slot'),'display')
        self.assertEqual(len(slot),0)
        with self.assertRaises(ValueError):
            text_slot('../display')

    def test_face_diagnostics_mark_the_contour_not_the_hair(self):
        root = ET.fromstring(render_scene(Scene(500,500,frontal_head('leila'))))
        protected = [n for n in root.iter() if n.get('data-protect-face')]
        self.assertEqual(len(protected),1)
        self.assertEqual(protected[0].get('d'),FACES['leila'].head)

    def test_dimensions_fail_before_export_when_invalid(self):
        for width in (0,-1,float('inf'),float('nan'),True,'200',10001):
            with self.subTest(width=width),self.assertRaises(ValueError):
                render_scene(Scene(width,100,''))

    def test_raw_exports_reject_executable_or_external_svg(self):
        for art in ('<script>bad()</script>','<rect onload="bad()"/>','<rect xmlns:x="urn:test" x:onload="bad()"/>','<rect fill="url(https://invalid.example/a)"/>','<g style="color:red"/>','<g href="#target"/>','<g xmlns=""/>','<!DOCTYPE svg>'):
            with self.subTest(art=art),self.assertRaises(ValueError):
                render_scene(Scene(200,100,art))

    def test_art_naming_a_missing_definition_fails_instead_of_rendering_blank(self):
        for art in ('<g clip-path="url(#absent)"><rect width="10" height="10"/></g>','<rect width="10" height="10" fill="url(#absent)"/>'):
            with self.subTest(art=art),self.assertRaises(ValueError):
                render_scene(Scene(200,100,art))
        defined = '<defs><clipPath id="present"><rect width="10" height="10"/></clipPath></defs><g clip-path="url(#present)"><rect width="10" height="10"/></g>'
        self.assertIn('url(#present)',render_scene(Scene(200,100,defined)))
        self.assertIn('url(#space)',render_scene(Scene(200,100,'<rect width="10" height="10" fill="url(#space)"/>')))

    def test_a_face_marker_needs_a_registered_head_and_its_own_contour(self):
        for art in ('<g data-face="nobody"><path d="M0 0"/></g>','<g data-face="leila"><path d="M0 0"/></g>'):
            with self.subTest(art=art),self.assertRaises(ValueError):
                render_scene(Scene(200,100,art))

    def test_scene_ids_and_story_slots_are_unique(self):
        for art in ('<g id="same"/><g id="same"/>',text_slot('display')+text_slot('display')):
            with self.subTest(art=art),self.assertRaises(ValueError):
                render_scene(Scene(200,100,art))


if __name__=='__main__':
    unittest.main()
