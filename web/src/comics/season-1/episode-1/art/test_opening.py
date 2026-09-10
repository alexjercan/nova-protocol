"""Check the opening's drawn continuity without assigning dates, dimensions, or blame."""

from pathlib import Path
import sys
import unittest
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from nova_illustration.faces import FACES
from nova_illustration.scenes import render_scene
from opening import SCENES, assignment_window, coffee_break

OFFICE = ('assignment-window', 'delivery-record', 'borrowed-mug')


def drawn(name):
    """Render a registered scene the way the comic build does."""
    return ET.fromstring(render_scene(SCENES[name]()))


def faces(root):
    """Every likeness the scene draws, in document order."""
    return [n for n in root.iter() if n.get('data-face')]


class OpeningDrawingTests(unittest.TestCase):
    def test_every_scene_draws_only_registered_faces_looking_forward(self):
        for name in SCENES:
            with self.subTest(scene=name):
                root = drawn(name)
                drawings = faces(root)
                for face in drawings:
                    self.assertIn(face.get('data-face'), FACES)
                    self.assertEqual(face.get('data-gaze'), 'forward')
                protected = [n.get('data-protect-face') for n in root.iter() if n.get('data-protect-face')]
                self.assertEqual(sorted(protected), sorted(n.get('data-face') for n in drawings))

    def test_the_only_words_drawn_into_the_art_are_the_three_ship_names(self):
        for name in SCENES:
            with self.subTest(scene=name):
                words = {n.text for n in drawn(name).iter() if n.text and n.text.strip()}
                self.assertLessEqual(words, {'BAIKAL', 'EBRO', 'KAVERI'})

    def test_the_coffee_break_is_the_one_scene_with_an_authored_expression(self):
        expressions = {n.get('data-face'): n.get('data-expression')
                       for n in faces(drawn('coffee-break'))}
        self.assertEqual(expressions, {'rina': 'amused', 'jonah': 'wry'})
        for name in SCENES:
            if name == 'coffee-break':
                continue
            with self.subTest(scene=name):
                self.assertFalse([n for n in faces(drawn(name)) if n.get('data-expression')])

    def test_the_office_looks_out_through_a_pressure_window_not_an_open_bay(self):
        views = [n for n in drawn('assignment-window').iter()
                 if n.get('data-prop') == 'pressure-window']
        self.assertEqual(len(views), 1)
        self.assertTrue(views[0].get('clip-path'))
        self.assertIn('EBRO', [n.text for n in views[0].iter() if n.text])

    def test_elena_is_drawn_in_her_office_and_never_on_the_processing_floor(self):
        for name in SCENES:
            with self.subTest(scene=name):
                cast = {n.get('data-face') for n in faces(drawn(name))}
                self.assertEqual('elena' in cast, name in OFFICE)

    def test_the_page_eighteen_window_is_the_page_three_window_unchanged(self):
        self.assertEqual(assignment_window(), SCENES['assignment-window']())
        self.assertEqual(coffee_break(), SCENES['coffee-break']())


if __name__ == '__main__':
    unittest.main()
