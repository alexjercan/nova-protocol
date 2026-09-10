"""The approach and receiving preparations, before the docking-hardware decision."""

from nova_illustration.colors import INK, WORK, FREIGHT
from nova_illustration.portraits import jonah_listener, work_portrait, work_inspection
from nova_illustration.scenery import stars
from nova_illustration.scenes import Scene, text_slot
from nova_illustration.ships import render_ship, project
from nova_illustration.svg import group, path, rect, tag
from aquila import held_person, window
from distress import cabin_wall, console, audio_grille
from props import assembly_faces


def first_aid_case(x, y, w, h):
    """Restrain a portable case without assigning contents or medical capability."""
    art = path(f'M{x - 18} {y + 14}H{x + w + 18}M{x - 18} {y + h - 10}H{x + w + 18}', stroke=FREIGHT['rail'], width=7)
    art += rect(x, y, w, h, WORK['pipe_light'], INK, 3, 9)
    art += rect(x + 14, y - 4, 13, h + 8, WORK['wall_shadow'], INK, 1.5, 2)
    art += rect(x + w - 28, y - 4, 13, h + 8, WORK['wall_shadow'], INK, 1.5, 2)
    art += path(f'M{x + w * .37} {y}V{y - 12}H{x + w * .63}V{y}', stroke=INK, width=5)
    art += text_slot('first-aid')
    return group(art, data_prop='first-aid-case', data_restraint='secured')


def across_the_gap():
    """Leave distance around the loaded workship during the coast."""
    art = rect(0, 0, 1416, 440, 'url(#space)') + stars(1416, 440)
    art += render_ship('kaveri', 'forward-quarter', 976, 280, .55, False, cargo=assembly_faces())
    return Scene(1416, 440, art)


def gantry_in_sight():
    """Reveal the retained damage model through Kaveri's pressure window."""
    art = cabin_wall(1416, 397)
    view = stars(1416, 397) + render_ship('gantry', 'forward-quarter', 722, 289, .69, False, state='stranded')
    art += window('gantry-approach', 335, 139, 679, 226, view)
    art += group(work_portrait('tomas'), 'translate(26 159) scale(.67)')
    art += held_person('jonah', 1130, 162, .70)
    art += path('M3 348H298L321 397H0Z', WORK['screen'], width=3)
    art += path('M35 365H230M48 382H262', stroke=WORK['pipe'], width=3)
    art += audio_grille(372, 358) + text_slot('channel')
    return Scene(1416, 397, art)


def visible_port():
    """Crop the real external hull geometry, not an internal-system scan."""
    art = cabin_wall(1416, 348)
    art += held_person('leila', 50, 153, .69)
    art += console(496, 148, 862, 174)
    clip = tag('defs', tag('clipPath', rect(508, 190, 838, 121, 'white', radius=4), id='port-camera'))
    px, py, _ = project((133, -107, 43), 'forward-quarter')
    view = rect(508, 190, 838, 121, 'url(#space)')
    view += render_ship('gantry', 'forward-quarter', 940 - px * 2.4, 254 - py * 2.4, 2.4, False, state='stranded')
    art += clip + group(view, clip_path='url(#port-camera)', data_prop='external-camera-view')
    art += text_slot('camera') + audio_grille(1298, 153)
    return Scene(1416, 348, art)


def receiving_space():
    """Prepare a restrained kit and handhold route without choosing transfer hardware."""
    art = cabin_wall(846, 489)
    body, hands = work_inspection('samir')
    art += group(body, 'translate(140 164) scale(.67)')
    art += path('M802 285V359', stroke=FREIGHT['frame'], width=17)
    art += path('M802 293V350', stroke=FREIGHT['rail'], width=9)
    art += held_person('rina', 510, 195, .65, reach=True)
    art += first_aid_case(224, 377, 203, 95)
    art += group(hands, 'translate(140 164) scale(.67)')
    art += audio_grille(40, 184) + audio_grille(465, 185)
    return Scene(846, 489, art)


def ready_to_help():
    """Keep Samir's readiness distinct from a diagnosis or a promise of treatment."""
    art = cabin_wall(550, 489)
    art += held_person('jonah', 8, 117, .65)
    art += held_person('samir', 315, 210, .68)
    art += first_aid_case(156, 385, 183, 84)
    return Scene(550, 489, art)


SCENES = {
    'across-the-gap': across_the_gap,
    'gantry-in-sight': gantry_in_sight,
    'visible-port': visible_port,
    'receiving-space': receiving_space,
    'ready-to-help': ready_to_help,
}
