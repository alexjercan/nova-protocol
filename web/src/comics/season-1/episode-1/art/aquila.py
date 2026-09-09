"""Episode-local scene artwork. No story, dialogue, or page assembly."""

from nova_illustration.colors import INK, MINT, FREIGHT, WORK, RINA
from nova_illustration.portraits import gantry_portrait, gripping_arm, reaching_arm, jonah_listener, work_inspection, work_portrait
from nova_illustration.scenery import aquila, stars, transfer_hall
from nova_illustration.ships import render_ship
from nova_illustration.svg import group, path, rect, tag
from props import assembly, assembly_faces, freight_lock, handhold
from nova_illustration.scenes import Scene, text_slot

def window(key, x, y, w, h, view):
    """Keep space behind a framed pressure window, never an open cargo doorway."""
    inside = rect(x, y, w, h, 'url(#space)') + view
    clip = tag('defs', tag('clipPath', rect(x, y, w, h, 'white', radius=30), id=key))
    art = clip + group(inside, clip_path=f'url(#{key})')
    art += rect(x, y, w, h, 'none', INK, 14, 30)
    art += rect(x, y, w, h, 'none', FREIGHT['rail'], 8, 30)
    art += path(f'M{x + 18} {y + h - 22}V{y + 43}Q{x + 18} {y + 18} {x + 45} {y + 18}', stroke=FREIGHT['edge'], width=2)
    return group(art, data_prop='pressure-window')

def held_person(name, x, y, scale, reach=False):
    """Anchor a frontal person at a drawn rail without rotating their likeness."""
    body = gantry_portrait(name) if name == 'nadia' else jonah_listener() if name == 'jonah' else work_portrait(name)
    art = group(body, f'translate({x} {y}) scale({scale})') + handhold(x, y, scale)
    arm = reaching_arm(name) if reach else gripping_arm(name)
    return art + group(arm, f'translate({x} {y}) scale({scale})')

def clear_signal(x, y):
    """Let Rina's orange sleeve and open signal enter the edge of the working shot."""
    art = path('M-20 80L-14 37L11 35L28 80Z', RINA['coat'], width=2)
    art += path('M-12 40L-16 15L-13-11Q-10-19-5-13L-5 8L0-20Q5-27 9-19L7 7L16-13Q21-18 24-11L18 15L26 3Q34 0 31 8L20 36L10 45Z', RINA['skin'], width=1.8)
    art += path('M-4 18L0 33M7 17L8 32', stroke=RINA['lines'], width=1.2)
    return group(art, f'translate({x} {y})', data_prop='rina-clear-signal')

def approach_aquila():
    """Draw the approach aquila scene."""
    w = 1416
    art = rect(0, 0, w, 350, 'url(#space)') + stars(w, 350)
    art += aquila(974, 150, 0.64) + render_ship('gantry', 'forward-quarter', 1117, 318, 0.23, False)
    art += render_ship('kaveri', 'forward-quarter', 565, 169, 0.55, False)
    return Scene(1416, 350, art)

def collection_check():
    """Draw the collection check scene."""
    w = 1416
    art = transfer_hall(700, 487)
    art += window('collection-window', 210, 142, 250, 147, stars(700, 300) + render_ship('kaveri', 'forward-quarter', 342, 232, 0.29, False))
    art += held_person('rina', 39, 136, 0.64) + held_person('samir', 453, 147, 0.65)
    art += assembly(365, 482, 1.68)
    art += path('M190 385Q176 356 171 342M466 405Q493 381 510 350', stroke=FREIGHT['stripe'], width=4)
    art += path('M610 357Q652 376 596 434', stroke=FREIGHT['stripe'], width=3)
    art += rect(477, 324, 143, 62, WORK['screen'], INK, 2, 5)
    art += text_slot('label-1') + text_slot('label-2')
    return Scene(700, 487, art)

def assembly_inspection():
    """Draw the assembly inspection scene."""
    w = 1416
    art = transfer_hall(696, 487)
    body, hands = work_inspection('leila')
    art += handhold(390, 145, 0.72) + group(body, 'translate(390 145) scale(.72)')
    art += assembly(485, 576, 1.78) + group(hands, 'translate(390 145) scale(.72)')
    art += path('M388 363Q346 346 324 351', stroke=FREIGHT['stripe'], width=4)
    return Scene(696, 487, art)

def shared_workspace():
    """Draw the shared workspace scene."""
    w = 1416
    art = transfer_hall(w, 400)
    art += window('gantry-berth', 857, 125, 531, 243, stars(w, 400) + render_ship('gantry', 'forward-quarter', 1128, 287, 0.74, False))
    art += held_person('nadia', 247, 123, 0.68)
    body, hands = work_inspection('rina')
    art += group(body, 'translate(603 99) scale(.64)') + assembly(794, 478, 1.42) + group(hands, 'translate(603 99) scale(.64)')
    return Scene(1416, 400, art)

def introductions():
    """Draw the introductions scene."""
    w = 1416
    art = transfer_hall(680, 437) + freight_lock(225, 126, 0.77)
    art += held_person('jonah', 8, 125, 0.62) + held_person('nadia', 444, 132, 0.6)
    return Scene(680, 437, art)

def safe_trip():
    """Draw the safe trip scene."""
    w = 1416
    art = transfer_hall(716, 437) + freight_lock(261, 162, 0.51)
    art += held_person('nadia', 17, 151, 0.64, True) + held_person('jonah', 426, 142, 0.64)
    art += clear_signal(354, 409)
    return Scene(716, 437, art)

def gantry_departure():
    """Draw the gantry departure scene."""
    w = 1416
    art = rect(0, 0, w, 350, WORK['wall_shadow'])
    view = stars(w, 350) + path('M22 0V280H205', stroke=FREIGHT['frame'], width=25)
    view += render_ship('gantry', 'forward-quarter', 492, 227, 0.89, True)
    art += window('departure-window', 24, 28, 813, 292, view)
    art += held_person('rina', 875, 138, 0.59) + held_person('samir', 1156, 142, 0.52)
    art += rect(1090, 286, 298, 57, WORK['screen'], INK, 2, 5)
    art += text_slot('label-1')
    art += text_slot('label-2')
    return Scene(1416, 350, art)

def outbound_aquila():
    """Draw the outbound aquila scene."""
    w = 1416
    art = rect(0, 0, 680, 487, 'url(#space)') + stars(680, 487) + aquila(166, 126, 0.29)
    art += render_ship('kaveri', 'aft-quarter', 438, 371, 0.81, True, cargo=assembly_faces())
    return Scene(680, 487, art)

def thoughts_of_home():
    """Draw the thoughts of home scene."""
    w = 1416
    art = rect(0, 0, 716, 487, WORK['wall'])
    art += path('M286 0V487M652 0V487', stroke=WORK['wall_shadow'], width=18)
    art += held_person('leila', 24, 159, 0.7) + held_person('jonah', 413, 168, 0.65)
    art += rect(431, 369, 243, 89, WORK['screen'], INK, 2, 5)
    art += text_slot('label-1') + text_slot('label-2')
    return Scene(716, 487, art)

SCENES = {
    'approach-aquila': approach_aquila,
    'collection-check': collection_check,
    'assembly-inspection': assembly_inspection,
    'shared-workspace': shared_workspace,
    'introductions': introductions,
    'safe-trip': safe_trip,
    'gantry-departure': gantry_departure,
    'outbound-aquila': outbound_aquila,
    'thoughts-of-home': thoughts_of_home,
}
