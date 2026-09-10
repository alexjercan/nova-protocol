"""Check transfer continuity and restraints, not medicine or physical dimensions."""

from pathlib import Path
import sys
import unittest
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from transfer import SCENES, TRANSFER_PLACEMENT, RECEIVING_PLACEMENT, stretcher_rail, placed_rail, moving_together, securing_support, transfer_port
from nova_illustration.scenes import render_scene


class TransferDrawingTests(unittest.TestCase):
    def test_scenes_are_safe_wordless_art_with_original_forward_faces(self):
        expected = {'meeting-at-opening': 3, 'moving-together': 3, 'securing-support': 2,
                    'last-across': 2, 'transfer-side-clear': 2, 'leaving-gantry': 0}
        for name, draw in SCENES.items():
            with self.subTest(scene=name):
                root = ET.fromstring(render_scene(draw()))
                self.assertFalse(''.join(root.itertext()).strip())
                faces = [n for n in root.iter() if n.get('data-face')]
                self.assertEqual(len(faces), expected[name])
                self.assertTrue(all(n.get('data-gaze') == 'forward' and not n.get('data-expression') for n in faces))

    def test_both_guides_hold_frame_anchors_during_the_feet_first_crossing(self):
        root = ET.fromstring(moving_together().art)
        self.assertEqual(root.get('data-transfer'), 'feet-first')
        contacts = {n.get('data-holder'): tuple(map(float, n.get('data-contact').split()))
                    for n in root.iter() if n.get('data-holder')}
        self.assertEqual(contacts, {'rina': placed_rail(TRANSFER_PLACEMENT, 'left', .79),
                                    'ivo': placed_rail(TRANSFER_PLACEMENT, 'right', .52)})

    def test_both_guides_retain_control_while_samir_secures_two_room_restraints(self):
        root = ET.fromstring(securing_support().art)
        holders = [n for n in root.iter() if n.get('data-holder')]
        self.assertEqual({n.get('data-holder') for n in holders}, {'rina', 'ivo', 'samir'})
        self.assertTrue(all(n.get('data-control') == 'maintained' for n in holders))
        self.assertEqual(root.get('data-receiving'), 'secured-before-release')
        ties = [n for n in root.iter() if n.get('data-prop') == 'receiving-restraint']
        self.assertEqual(len(ties), 2)
        self.assertTrue(all(n.get('data-latch') == 'closed' and n.get('data-mechanism') == 'rigid-clamp' for n in ties))
        sx, sy = placed_rail(RECEIVING_PLACEMENT, 'left', .8)
        samir = next(n for n in holders if n.get('data-holder') == 'samir')
        self.assertEqual(tuple(map(float, samir.get('data-contact').split())), ((sx+220)/2, (sy+486)/2))

    def test_one_padded_restrained_patient_continues_through_all_three_views(self):
        for name in ('meeting-at-opening', 'moving-together', 'securing-support'):
            root = ET.fromstring('<g>' + SCENES[name]().art + '</g>')
            patients = [n for n in root.iter() if n.get('data-prop') == 'padded-stretcher']
            self.assertEqual(len(patients), 1)
            self.assertEqual(patients[0].get('data-patient'), 'owen')
            self.assertEqual(patients[0].get('data-body-restraints'), 'secured')
            ports = [n for n in root.iter() if n.get('data-prop') == 'sealed-transfer-passage']
            self.assertEqual(len(ports), 1)
            self.assertEqual(ports[0].get('data-hatches'), 'open')
            self.assertEqual(ports[0].get('data-gravity'), 'freefall')

    def test_closing_precedes_separation_with_the_same_loaded_unpowered_pair(self):
        closed = ET.fromstring('<g>' + SCENES['transfer-side-clear']().art + '</g>')
        self.assertEqual(next(n for n in closed.iter() if n.get('data-hatches')).get('data-hatches'), 'closed')
        exterior = ET.fromstring('<g>' + SCENES['leaving-gantry']().art + '</g>')
        pair = next(n for n in exterior.iter() if n.get('data-ship-pair'))
        self.assertEqual(pair.get('data-gap'), '210')
        self.assertEqual(pair.get('data-motion'), 'coast')
        self.assertEqual(pair.get('data-gantry-state'), 'stranded')
        self.assertIn('kaveri/replacement-housing', {n.get('data-component') for n in pair.iter()})

    def test_unknown_hatch_states_and_out_of_range_grips_fail(self):
        with self.assertRaises(ValueError):
            transfer_port(0, 0, 100, 'maybe')
        for side, fraction in [('unknown', .5), ('left', -1), ('right', 2), ('left', float('nan'))]:
            with self.subTest(side=side, fraction=fraction), self.assertRaises(ValueError):
                stretcher_rail(side, fraction)


if __name__ == '__main__':
    unittest.main()
