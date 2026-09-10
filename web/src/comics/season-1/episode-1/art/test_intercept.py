"""Check the approach: Gantry is stranded, seen at a distance, and never boarded here."""

from pathlib import Path
import sys
import unittest
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from intercept import SCENES
from nova_illustration.faces import FACES
from nova_illustration.portraits import GANTRY_CREW
from nova_illustration.scenes import render_scene

THROUGH = ('pressure-window', 'external-camera-view')


def drawn(name):
    """Render a registered scene the way the comic build does."""
    return ET.fromstring(render_scene(SCENES[name]()))


class InterceptDrawingTests(unittest.TestCase):
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

    def test_gantry_is_stranded_in_every_view_of_it_and_kaveri_is_not(self):
        seen = []
        for name in SCENES:
            for ship in [n for n in drawn(name).iter() if n.get('data-ship')]:
                if ship.get('data-ship') == 'gantry':
                    seen.append(name)
                    self.assertEqual(ship.get('data-state'), 'stranded', name)
                else:
                    self.assertIsNone(ship.get('data-state'), name)
        self.assertEqual(sorted(seen), ['gantry-in-sight', 'visible-port'])

    def test_the_crew_only_ever_see_gantry_through_a_window_or_a_camera(self):
        for name in ('gantry-in-sight', 'visible-port'):
            with self.subTest(scene=name):
                root = drawn(name)
                views = [n for n in root.iter() if n.get('data-prop') in THROUGH]
                self.assertEqual(len(views), 1)
                self.assertTrue([n for n in views[0].iter() if n.get('data-ship') == 'gantry'])

    def test_gantrys_crew_is_still_only_a_voice_before_the_ships_are_joined(self):
        for name in SCENES:
            with self.subTest(scene=name):
                cast = {n.get('data-face') for n in drawn(name).iter() if n.get('data-face')}
                self.assertFalse(cast & set(GANTRY_CREW))

    def test_the_first_aid_case_is_secured_wherever_it_is_brought_out(self):
        cases = []
        for name in SCENES:
            for case in [n for n in drawn(name).iter() if n.get('data-prop') == 'first-aid-case']:
                cases.append(name)
                self.assertEqual(case.get('data-restraint'), 'secured', name)
        self.assertEqual(sorted(cases), ['ready-to-help', 'receiving-space'])


if __name__ == '__main__':
    unittest.main()
