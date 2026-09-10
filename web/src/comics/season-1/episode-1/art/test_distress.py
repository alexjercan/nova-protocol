"""Check that the call keeps its point of view: Gantry is heard, not seen."""

from pathlib import Path
import sys
import unittest
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from distress import SCENES
from nova_illustration.faces import FACES
from nova_illustration.portraits import GANTRY_CREW
from nova_illustration.scenes import render_scene

CALLS = ('answering-gantry', 'cost-refusal', 'familiar-signal', 'recording-report')


def drawn(name):
    """Render a registered scene the way the comic build does."""
    return ET.fromstring(render_scene(SCENES[name]()))


def cast(root):
    """Every character the scene draws a likeness of."""
    return {n.get('data-face') for n in root.iter() if n.get('data-face')}


class DistressDrawingTests(unittest.TestCase):
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

    def test_gantrys_crew_is_heard_across_the_whole_sequence_and_drawn_in_none_of_it(self):
        for name in SCENES:
            with self.subTest(scene=name):
                self.assertFalse(cast(drawn(name)) & set(GANTRY_CREW))

    def test_a_scene_that_carries_a_voice_draws_the_speaker_it_comes_from(self):
        for name in CALLS:
            with self.subTest(scene=name):
                root = drawn(name)
                self.assertTrue([n for n in root.iter() if n.get('data-prop') == 'audio-grille'])
                self.assertTrue([n for n in root.iter() if n.get('data-story-slot')])
        for name in set(SCENES) - set(CALLS):
            with self.subTest(scene=name):
                self.assertFalse([n for n in drawn(name).iter() if n.get('data-prop') == 'audio-grille'])

    def test_neither_end_of_the_earthworks_refusal_is_drawn(self):
        root = drawn('cost-refusal')
        self.assertEqual(cast(root), {'samir'})
        slots = {n.get('data-story-slot') for n in root.iter() if n.get('data-story-slot')}
        self.assertEqual(slots, {'baikal', 'elena', 'earthworks', 'operations', 'daniel'})

    def test_the_route_is_shown_as_an_ordering_the_reader_can_read(self):
        slots = {n.get('data-story-slot') for n in drawn('arrival-order').iter()
                 if n.get('data-story-slot')}
        self.assertEqual(slots, {'ordering', 'kaveri', 'reserves', 'recovery'})


if __name__ == '__main__':
    unittest.main()
