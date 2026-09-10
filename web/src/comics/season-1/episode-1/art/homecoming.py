"""The survivors' return and the ordinary work that resumes at Baikal.

Existing faces, support, machinery, ships, and the borrowed mug keep their
sources. These scenes add no medical outcome, new ship, or company capability.
"""

from nova_illustration.colors import INK, FREIGHT, WORK, INTERIOR
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener, gantry_portrait, work_inspection
from nova_illustration.scenery import stars
from nova_illustration.scenes import Scene, text_slot
from nova_illustration.ships import render_ship
from nova_illustration.svg import group, path, rect, ellipse, tag
from aquila import held_person, window
from distress import cabin_wall, console, audio_grille
from intercept import first_aid_case
from opening import machinery_wall, pump, warm_room, assignment_window
from opening_props import mug_hand
from props import assembly
from transfer import stretcher, placed_rail, guide_layers, receiving_clamps


def asking_for_record():
    """Keep the actual comms record and Owen's continuing support in the same scene."""
    art = cabin_wall(1416, 510)
    art += group(gantry_portrait('ivo'), 'translate(35 168) scale(.62)')
    art += held_person('samir', 480, 164, .62)
    art += held_person('nadia', 1000, 159, .63)
    placement = (781, 215, .35)
    art += path('M735 493H956', stroke=FREIGHT['frame'], width=18)
    art += path('M742 491H949', stroke=FREIGHT['rail'], width=9)
    art += stretcher(placement) + receiving_clamps(placement, ((742, 491), (949, 491)))
    art += first_aid_case(450, 412, 206, 76)
    art += console(283, 288, 163, 102)
    art += text_slot('record')
    return Scene(1416, 510, art)


def quiet_thanks():
    """Let the captain's acknowledgement stay small, during the coast rather than at home."""
    art = cabin_wall(1416, 327)
    art += window('return-coast', 557, 129, 295, 165, stars(1416, 327))
    art += held_person('nadia', 240, 118, .63)
    art += held_person('jonah', 892, 115, .65)
    return Scene(1416, 327, art)


def welcome_to_baikal():
    """Meet the arrivals after transfer, without inventing another docking arrangement."""
    art = warm_room(1416, 360)
    art += path('M0 342H1416', stroke=INTERIOR['structure'], width=12)
    art += group(elena_gesture(), 'translate(20 135) scale(.67)')
    art += group(jonah_listener(), 'translate(347 135) scale(.64)')
    art += group(gantry_portrait('nadia'), 'translate(620 157) scale(.55)')
    placement = (1120, 91, .62)
    samir, hand = guide_layers('samir', placed_rail(placement, 'left', .52), .55, 'left')
    art += samir + stretcher(placement) + hand
    art += rect(929, 15, 444, 61, INTERIOR['panel'], INK, 2, 5)
    art += text_slot('arrival')
    return Scene(1416, 360, art)


def covers_still_on():
    """Stage the same covered replacement beside the isolated line before work begins."""
    art = machinery_wall(896, 477)
    pipes, housing = pump(726, 319, .52)
    art += pipes + housing
    art += held_person('rina', 50, 142, .67)
    body, hands = work_inspection('leila')
    art += group(body, 'translate(588 142) scale(.67)')
    art += path('M203 466H639L657 477H190Z', WORK['wall_shadow'], width=3)
    art += path('M225 470H618', stroke=WORK['pipe'], width=5)
    art += group(assembly(450, 458, 1.42), data_prop='replacement-service-cradle', data_restraint='secured')
    for x in (299, 526):
        art += rect(x, 457, 18, 14, FREIGHT['frame'], INK, 2, 2)
        art += path(f'M{x+9} 461V471', stroke=FREIGHT['rail'], width=3)
    art += path('M588 414H864V477H578Z', WORK['screen'], width=3)
    art += group(hands, 'translate(588 142) scale(.67)')
    art += console(313, 154, 225, 60) + text_slot('isolated')
    return Scene(896, 477, art)


def line_back_in_service():
    """Separate the completed repair from the still-pending production and delivery."""
    art = machinery_wall(500, 477)
    art += text_slot('later')
    art += console(20, 209, 460, 88)
    art += text_slot('line') + text_slot('checks') + text_slot('load')
    art += audio_grille(438, 155)
    view = stars(500, 477) + render_ship('ebro', 'forward-quarter', 279, 423, .31, False)
    art += window('ebro-waits', 20, 341, 460, 116, view)
    return Scene(500, 477, art)


def penalty_at_window():
    """Return to the page-three framing, without a Foundation or Mara cutaway."""
    return assignment_window()


def carrying_the_cost():
    """Keep the burden with Elena's decision, rather than accusing the returning crew."""
    art = warm_room(680, 422)
    art += group(jonah_listener(), 'translate(-28 136) scale(.68)')
    art += group(elena_close(), 'translate(397 161) scale(.68)')
    return Scene(680, 422, art)


def mug_back_home():
    """Set the same mug within reach, with Ebro and Baikal's work still outside the glass."""
    art = warm_room(716, 422)
    view = rect(30, 132, 650, 180, 'url(#space)') + stars(716, 422)
    view += path('M30 302L143 273L181 245', stroke=INTERIOR['structure'], width=12)
    view += render_ship('ebro', 'forward-quarter', 250, 239, .32, False)
    clip = tag('defs', tag('clipPath', rect(30, 132, 650, 180, 'white', radius=35), id='home-window'))
    art += clip + group(view, clip_path='url(#home-window)', data_prop='pressure-window')
    art += rect(30, 132, 650, 180, 'none', INK, 17, 35)
    art += rect(30, 132, 650, 180, 'none', INTERIOR['window_frame'], 10, 35)
    art += group(elena_close(), 'translate(438 132) scale(.67)')
    art += path('M0 388L154 346H400L468 422H0Z', INTERIOR['worktop'], width=3)
    art += path('M146 349H396', stroke=INTERIOR['edge'], width=3)
    art += ellipse(278.4, 367.3, 55, 5, INTERIOR['structure'], 'none')
    art += group(mug_hand(246, 262, .9), data_prop='returned-borrowed-mug', data_contact='worktop')
    return Scene(716, 422, art)


SCENES = {
    'asking-for-record': asking_for_record,
    'quiet-thanks': quiet_thanks,
    'welcome-to-baikal': welcome_to_baikal,
    'covers-still-on': covers_still_on,
    'line-back-in-service': line_back_in_service,
    'penalty-at-window': penalty_at_window,
    'carrying-the-cost': carrying_the_cost,
    'mug-back-home': mug_back_home,
}
