"""Episode-local scene artwork. No story, dialogue, or page assembly."""

import math
from nova_illustration.colors import INK, JADE, MINT, INTERIOR, WORK
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener, work_inspection, work_portrait
from nova_illustration.scenery import stars, saturn, baikal
from nova_illustration.ships import render_ship
from nova_illustration.svg import ellipse, group, path, rect, tag
from opening_props import mug_hand
from nova_illustration.scenes import Scene, text_slot

def warm_room(w, h):
    """Place the exchange with cropped wall panels and a work surface, not an empty hall."""
    art = rect(0, 0, w, h, INTERIOR['wall'])
    art += path(f'M0 0H{w * 0.58}L{w * 0.43} {h}H0Z', INTERIOR['wall_light'], 'none')
    art += path(f'M{w * 0.84} 0H{w}V{h}H{w * 0.89}Z', INTERIOR['panel'], width=2)
    art += path(f'M{w * 0.87} 0V{h}M{w * 0.85} {h * 0.68}H{w}', stroke=INTERIOR['panel_line'], width=1.8)
    art += path(f'M0 {h * 0.84}L{w * 0.3} {h * 0.7}L{w * 0.67} {h}H0Z', INTERIOR['worktop'], width=2)
    art += path(f'M0 {h * 0.86}L{w * 0.3} {h * 0.72}L{w * 0.48} {h * 0.87}', stroke=INTERIOR['edge'], width=2)
    return art

def machinery_wall(w, h):
    """Use sparse service pipes as anchors behind the people and stopped pump."""
    art = rect(0, 0, w, h, WORK['wall'])
    art += path(f'M0 0H{w * 0.36}L{w * 0.18} {h}H0Z', WORK['wall_shadow'], 'none')
    for x in (48, w * 0.38, w * 0.88):
        art += path(f'M{x} 0V{h}', stroke=WORK['wall_shadow'], width=25)
        art += path(f'M{x - 6} 0V{h}', stroke=WORK['pipe'], width=6)
    art += path(f'M0 {h * 0.65}H{w}', stroke=WORK['wall_shadow'], width=12)
    return art

def pump(x, y, scale):
    """Return rear pipes and a foreground housing for this scene's painter order."""
    art = path('M-320-30H-90V-205H248', stroke=INK, width=96)
    art += path('M-320-30H-90V-205H248', stroke=WORK['pipe'], width=80)
    art += path('M-320-50H-109V-219H248', stroke=WORK['pipe_light'], width=8)
    pipes = art
    art = rect(-178, 111, 350, 39, WORK['wall_shadow'], INK, 3)
    art += ellipse(28, 0, 195, 158, WORK['wall_shadow'], INK, 3)
    art += ellipse(0, 0, 195, 158, WORK['pump'], INK, 3)
    art += ellipse(0, 0, 156, 127, WORK['pump_light'], INK, 2.5)
    art += ellipse(0, 0, 114, 92, WORK['wall_shadow'], INK, 2.5)
    art += ellipse(0, 0, 88, 72, WORK['pipe'], INK, 2)
    for i in range(10):
        a = math.tau * i / 10
        art += ellipse(math.cos(a) * 177, math.sin(a) * 143, 5, 5, INK, WORK['pipe_light'], 1.5)
    art += path('M-166 132H160', stroke=WORK['warning'], width=6, stroke_dasharray='12 12')
    art += rect(-68, -48, 136, 96, WORK['screen'], INK, 2, 6)
    art += path('M-47-22H46M-47 0H15M-47 22H35', stroke=WORK['pipe_light'], width=3)
    transform = f'translate({x} {y}) scale({scale})'
    return (group(pipes, transform), group(art, transform))

def baikal_work():
    """Draw the baikal work scene."""
    w = 1416
    art = rect(0, 0, w, 360, 'url(#space)') + stars(w, 360) + saturn(1210, -83, 0.9)
    art += baikal(644, 227, 0.63) + render_ship('ebro', 'forward-quarter', 845, 304, 0.23, False) + render_ship('kaveri', 'forward-quarter', 440, 308, 0.19, False)
    return Scene(1416, 360, art)

def coffee_break():
    """Draw the coffee break scene."""
    w = 1416
    art = warm_room(w, 477)
    art += rect(50, 210, 204, 114, WORK['screen'], INK, 2, 4) + text_slot('label-1') + path('M72 270H214M72 290H163', stroke=INTERIOR['edge'], width=3)
    art += group(work_portrait('rina', expression='amused'), 'translate(470 161) scale(.87)')
    art += group(jonah_listener(expression='wry'), 'translate(952 161) scale(.87)') + mug_hand(942, 421, 0.87)
    return Scene(1416, 477, art)

def pump_inspection():
    """Draw the pump inspection scene."""
    w = 1416
    art = machinery_wall(836, 857)
    pipes, housing = pump(245, 735, 1.1)
    body, hands = work_inspection('leila')
    art += pipes + group(body, 'translate(200 185) scale(1.06)')
    art += housing + group(hands, 'translate(200 185) scale(1.06)')
    art += group(jonah_listener(), 'translate(555 474) scale(1.05)')
    return Scene(836, 857, art)

def residential_console():
    """Draw the residential console scene."""
    w = 1416
    art = machinery_wall(560, 364)
    body, hands = work_inspection('samir')
    art += group(body, 'translate(295 20) scale(.65)')
    art += path('M20 253L531 253L557 310V364H8V306Z', WORK['screen'], width=3)
    art += rect(29, 269, 252, 82, WORK['safe_back'], JADE, 2, 5)
    art += text_slot('label-1')
    art += text_slot('label-2')
    art += path('M332 300H512M332 321H512M346 285V340M385 285V340M424 285V340M463 285V340M502 285V340', stroke=WORK['pipe'], width=2)
    art += group(hands, 'translate(295 20) scale(.65)')
    return Scene(560, 364, art)

def repair_choice():
    """Draw the repair choice scene."""
    w = 1416
    return Scene(560, 473, machinery_wall(560, 473) + group(work_portrait('leila'), 'translate(265 164) scale(1.02)'))

def assignment_window():
    """Draw the assignment window scene."""
    w = 1416
    art = warm_room(w, 415)
    view = rect(425, 16, 990, 337, 'url(#space)') + stars(w, 353)
    view += path('M440 349L714 282L759 259', stroke=INTERIOR['structure'], width=18)
    view += path('M444 344L714 276L759 254', stroke=INTERIOR['structure_light'], width=4)
    view += render_ship('ebro', 'forward-quarter', 846, 254, 0.64, False)
    art += tag('defs', tag('clipPath', rect(425, 16, 990, 337, 'white', radius=72), id='window')) + group(view, clip_path='url(#window)')
    art += rect(419, 10, 1002, 349, 'none', INK, 21, 78) + rect(421, 12, 998, 345, 'none', INTERIOR['window_frame'], 13, 76)
    art += path('M476 322V110Q476 61 515 61M1378 19V350', stroke=INTERIOR['window_light'], width=3)
    art += path('M410 372H1416', stroke=INTERIOR['panel'], width=19)
    art += group(elena_gesture(), 'translate(163 146) scale(.64)') + group(jonah_listener(), 'translate(1067 136) scale(.91)')
    return Scene(1416, 415, art)

def delivery_record():
    """Draw the delivery record scene."""
    w = 1416
    art = warm_room(680, 422) + group(elena_close(), 'translate(340 126) scale(.94)')
    record = rect(-120, -65, 240, 153, WORK['screen'], INK, 3, 7) + text_slot('label-1') + text_slot('label-2') + text_slot('label-3')
    art += group(record, 'translate(164 312) rotate(-8)')
    return Scene(680, 422, art)

def borrowed_mug():
    """Draw the borrowed mug scene."""
    w = 1416
    art = warm_room(716, 422) + group(elena_close(), 'translate(322 131)') + mug_hand(177, 340, 0.9)
    return Scene(716, 422, art)

def departure_stations():
    """Draw the departure stations scene."""
    w = 1416
    art = rect(0, 0, w, 407, WORK['wall_shadow']) + rect(113, -35, 1190, 352, 'url(#space)', INK, 18, 60) + stars(w, 300) + baikal(522, 159, 0.29)
    art += path('M107 0L207 326H1220L1324 0M874 0L852 326', stroke=INTERIOR['structure_light'], width=12)
    art += group(work_portrait('tomas'), 'translate(482 147) scale(.74)') + group(jonah_listener(), 'translate(1034 141) scale(.86)')
    art += path('M0 407L192 368H551L876 407Z', WORK['screen'], width=3)
    art += path('M244 378H455L478 400H222Z', INTERIOR['panel'], width=2) + path('M268 389H421M507 389H549M580 389H632', stroke=MINT, width=3)
    return Scene(1416, 407, art)

def outbound_baikal():
    """Draw the outbound baikal scene."""
    w = 1416
    art = rect(0, 0, w, 430, 'url(#space)') + stars(w, 430) + saturn(1260, -158, 1.2) + baikal(285, 197, 0.46)
    art += render_ship('ebro', 'forward-quarter', 421, 257, 0.16, False) + render_ship('kaveri', 'aft-quarter', 1031, 279, 1.15, True)
    return Scene(1416, 430, art)

SCENES = {
    'baikal-work': baikal_work,
    'coffee-break': coffee_break,
    'pump-inspection': pump_inspection,
    'residential-console': residential_console,
    'repair-choice': repair_choice,
    'assignment-window': assignment_window,
    'delivery-record': delivery_record,
    'borrowed-mug': borrowed_mug,
    'departure-stations': departure_stations,
    'outbound-baikal': outbound_baikal,
}
