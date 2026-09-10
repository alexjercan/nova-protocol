"""Check the ending's continuity without assigning medicine, dimensions, or dates."""

from pathlib import Path
import sys
import unittest
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from homecoming import SCENES, asking_for_record, welcome_to_baikal, covers_still_on, penalty_at_window, mug_back_home
from nova_illustration.scenes import render_scene
from opening import assignment_window, pump
from opening_props import mug_hand
from props import assembly
from transfer import stretcher, placed_rail, receiving_clamps


def elements(scene):
    """Read scene-owned geometry before the renderer supplies page-independent definitions."""
    return ET.fromstring('<g>' + scene.art + '</g>')


class HomecomingDrawingTests(unittest.TestCase):
    def test_all_eight_scenes_are_safe_art_with_the_original_forward_faces(self):
        expected = {'asking-for-record': 4, 'quiet-thanks': 2, 'welcome-to-baikal': 5,
                    'covers-still-on': 2, 'line-back-in-service': 0, 'penalty-at-window': 2,
                    'carrying-the-cost': 2, 'mug-back-home': 1}
        for name, draw in SCENES.items():
            with self.subTest(scene=name):
                root = ET.fromstring(render_scene(draw()))
                self.assertTrue(all(word == 'EBRO' for word in ''.join(root.itertext()).split()))
                faces = [n for n in root.iter() if n.get('data-face')]
                self.assertEqual(len(faces), expected[name])
                self.assertTrue(all(n.get('data-gaze') == 'forward' and not n.get('data-expression') for n in faces))
                self.assertFalse(any(n.get('data-face') in ('mara', 'daniel') for n in faces))

    def test_owen_keeps_the_same_restrained_support_during_the_return_and_arrival(self):
        original = list(ET.fromstring(stretcher((0, 0, 1))))
        for draw in (asking_for_record, welcome_to_baikal):
            patients = [n for n in elements(draw()).iter() if n.get('data-prop') == 'padded-stretcher']
            self.assertEqual(len(patients), 1)
            self.assertEqual(patients[0].get('data-body-restraints'), 'secured')
            self.assertEqual([ET.tostring(n) for n in patients[0]], [ET.tostring(n) for n in original])

    def test_owen_rests_in_closed_rigid_clamps_with_first_aid_nearby(self):
        root = elements(asking_for_record())
        clamps = [n for n in root.iter() if n.get('data-prop') == 'receiving-restraint']
        self.assertEqual(len(clamps), 2)
        self.assertTrue(all(n.get('data-mechanism') == 'rigid-clamp' and n.get('data-latch') == 'closed' for n in clamps))
        self.assertTrue(any(n.get('data-prop') == 'first-aid-case' and n.get('data-restraint') == 'secured' for n in root.iter()))

    def test_samir_controls_the_frame_at_arrival_instead_of_owen_arriving_unaided(self):
        root = elements(welcome_to_baikal())
        holders = [n for n in root.iter() if n.get('data-holder')]
        self.assertEqual(len(holders), 1)
        self.assertEqual(holders[0].get('data-holder'), 'samir')
        self.assertEqual(holders[0].get('data-control'), 'maintained')
        self.assertEqual(tuple(map(float, holders[0].get('data-contact').split())), placed_rail((1120, 91, .62), 'left', .52))

    def test_the_covered_assembly_and_old_pump_share_the_processing_view(self):
        scene = covers_still_on()
        self.assertIn(assembly(450, 458, 1.42), scene.art)
        pipes, housing = pump(726, 319, .52)
        self.assertIn(pipes, scene.art)
        self.assertIn(housing, scene.art)
        parts = {n.get('data-component') for n in elements(scene).iter()}
        self.assertTrue({'replacement-housing', 'protective-cover', 'assembly-restraint'} <= parts)

    def test_the_penalty_returns_to_the_same_window_and_waiting_ship(self):
        self.assertEqual(penalty_at_window(), assignment_window())
        for name in ('line-back-in-service', 'penalty-at-window', 'mug-back-home'):
            ships = [n.get('data-ship') for n in elements(SCENES[name]()).iter() if n.get('data-ship')]
            self.assertEqual(ships, ['ebro'])

    def test_the_last_object_is_the_same_mug_not_a_new_prop_or_threat(self):
        scene = mug_back_home()
        self.assertIn(mug_hand(246, 262, .9), scene.art)
        self.assertTrue(any(n.get('data-prop') == 'returned-borrowed-mug' and n.get('data-contact') == 'worktop' for n in elements(scene).iter()))
        faces = {n.get('data-face') for n in elements(scene).iter() if n.get('data-face')}
        self.assertEqual(faces, {'elena'})

    def test_reusing_receiving_clamps_still_requires_both_anchors(self):
        for anchors in ((), ((0, 0),), ((0, 0), (1, 1), (2, 2))):
            with self.subTest(anchors=anchors), self.assertRaises(ValueError):
                receiving_clamps((0, 0, 1), anchors)


if __name__ == '__main__':
    unittest.main()
