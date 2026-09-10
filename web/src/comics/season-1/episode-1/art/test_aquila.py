"""Check the Aquila stop's drawn continuity without claiming freight or medical procedure."""

from pathlib import Path
import sys
import unittest
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from aquila import SCENES, held_person
from nova_illustration.faces import FACES
from nova_illustration.scenes import render_scene

HALL = 'aquila-transfer-hall'


def drawn(name):
    """Render a registered scene the way the comic build does."""
    return ET.fromstring(render_scene(SCENES[name]()))


def marked(root, prop):
    """Every group the art tags with a story-safety prop name."""
    return [n for n in root.iter() if n.get('data-prop') == prop]


class AquilaDrawingTests(unittest.TestCase):
    def test_every_scene_draws_only_registered_faces_looking_forward(self):
        for name in SCENES:
            with self.subTest(scene=name):
                root = drawn(name)
                drawings = [n for n in root.iter() if n.get('data-face')]
                for face in drawings:
                    self.assertIn(face.get('data-face'), FACES)
                    self.assertEqual(face.get('data-gaze'), 'forward')
                    self.assertIsNone(face.get('data-expression'))
                protected = [n.get('data-protect-face') for n in root.iter() if n.get('data-protect-face')]
                self.assertEqual(sorted(protected), sorted(n.get('data-face') for n in drawings))

    def test_gantry_is_whole_everywhere_it_appears_at_aquila(self):
        seen = []
        for name in SCENES:
            for ship in [n for n in drawn(name).iter() if n.get('data-ship') == 'gantry']:
                seen.append(name)
                self.assertIsNone(ship.get('data-state'), name)
        self.assertEqual(sorted(seen), ['approach-aquila', 'gantry-departure', 'shared-workspace'])

    def test_the_transfer_hall_is_freefall_with_something_to_hold(self):
        halls = [name for name in SCENES
                 if any(n.get('data-place') == HALL for n in drawn(name).iter())]
        self.assertEqual(len(halls), 5)
        for name in halls:
            with self.subTest(scene=name):
                root = drawn(name)
                place = next(n for n in root.iter() if n.get('data-place') == HALL)
                self.assertEqual(place.get('data-gravity'), 'freefall')
                self.assertTrue(marked(root, 'handhold') or marked(root, 'freight-lock'))

    def test_the_hall_sees_space_through_a_pressure_window_and_the_lock_stays_shut(self):
        for name in SCENES:
            root = drawn(name)
            for window in marked(root, 'pressure-window'):
                with self.subTest(scene=name):
                    self.assertTrue(window.get('clip-path') or window.find('*') is not None)
            for lock in marked(root, 'freight-lock'):
                with self.subTest(scene=name):
                    self.assertEqual(lock.get('data-door'), 'closed')

    def test_a_person_at_a_rail_can_be_drawn_for_either_crew(self):
        for name in ('leila', 'rina', 'samir', 'tomas', 'jonah', 'nadia', 'owen', 'ivo'):
            with self.subTest(character=name):
                art = held_person(name, 0, 0, 1)
                self.assertIn(f'data-face="{name}"', art)
                self.assertIn('data-prop="handhold"', art)


if __name__ == '__main__':
    unittest.main()
