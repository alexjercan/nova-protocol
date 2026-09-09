"""Kaveri's rescue assessment, with scene art separate from the TS script."""

from nova_illustration.colors import INK, INTERIOR, WORK, FREIGHT
from nova_illustration.portraits import elena_close, jonah_listener, work_inspection, work_portrait
from nova_illustration.scenery import stars
from nova_illustration.scenes import Scene, text_slot
from nova_illustration.svg import group, path, rect, tag, ellipse
from aquila import held_person, window
from props import assembly


def cabin_wall(w, h):
    """Crop the familiar work-cabin panels without asserting a floor or cabin plan."""
    art = rect(0, 0, w, h, WORK['wall'])
    art += path(f'M0 0H{w * .32}L{w * .17} {h}H0Z', WORK['wall_shadow'], 'none')
    for x in (w * .36, w * .91):
        art += path(f'M{x} 0V{h}', stroke=WORK['wall_shadow'], width=18)
        art += path(f'M{x - 5} 0V{h}', stroke=WORK['pipe'], width=3)
    art += path(f'M0 {h * .82}H{w}', stroke=WORK['wall_shadow'], width=10)
    return art


def audio_grille(x, y):
    """Make the sound source visible without drawing words or a fictional waveform."""
    art = rect(x, y, 36, 30, WORK['wall_shadow'], INK, 1.5, 4)
    for row in range(4):
        art += path(f'M{x + 7} {y + 6 + row * 6}H{x + 29}', stroke=WORK['pipe'], width=2)
    return group(art, data_prop='audio-grille')


def console(x, y, w, h):
    """Supply a fixed screen surround and restrained controls, not a loose tablet."""
    art = rect(x - 10, y - 10, w + 20, h + 24, FREIGHT['frame'], INK, 3, 9)
    art += rect(x, y, w, h, WORK['screen'], INK, 2, 5)
    art += path(f'M{x + 12} {y + h + 6}H{x + w - 12}', stroke=WORK['pipe'], width=3)
    return art


def elena_display(key, x, y, w, h, portrait_x, portrait_y, scale):
    """Show Elena's accepted face in a comms display, without a new identity or tint."""
    clip = tag('defs', tag('clipPath', rect(x, y, w, h, 'white', radius=5), id=key))
    image = rect(x, y, w, h, INTERIOR['wall'])
    image += path(f'M{x} {y}H{x + w * .4}V{y + h}H{x}Z', INTERIOR['wall_light'], 'none')
    image += group(elena_close(), f'translate({portrait_x} {portrait_y}) scale({scale})')
    return console(x, y, w, h) + clip + group(image, clip_path=f'url(#{key})')


def familiar_signal():
    """Put the incoming call on Samir's fixed console, with Jonah already attending."""
    art = cabin_wall(698, 390)
    art += group(jonah_listener(), 'translate(215 137) scale(.52)')
    body, hands = work_inspection('samir')
    art += group(body, 'translate(448 152) scale(.70)')
    art += console(26, 278, 357, 91) + audio_grille(324, 323)
    art += text_slot('channel') + text_slot('request')
    art += path('M435 374H681V390H419Z', WORK['screen'], width=2)
    art += group(hands, 'translate(448 152) scale(.70)')
    return Scene(698, 390, art)


def answering_gantry():
    """Hold the point of view aboard Kaveri as Jonah answers the report."""
    art = cabin_wall(698, 390)
    art += held_person('jonah', 39, 160, .85)
    art += console(405, 259, 259, 111) + audio_grille(607, 324)
    art += text_slot('channel') + text_slot('audio')
    return Scene(698, 390, art)


def recording_report():
    """Separate Samir's received report from Tomas's ongoing route work."""
    art = cabin_wall(1416, 447)
    body, hands = work_inspection('samir')
    art += group(body, 'translate(65 164) scale(.84)')
    pilot, controls = work_inspection('tomas')
    art += group(pilot, 'translate(765 150) scale(.50)')
    art += held_person('jonah', 1084, 172, .72)
    art += console(394, 260, 330, 157)
    art += console(749, 330, 271, 87)
    art += audio_grille(674, 280) + text_slot('report')
    art += text_slot('reserves') + text_slot('port')
    art += path('M752 351H981M752 374H867M752 395H922', stroke=WORK['pipe'], width=3)
    art += path('M8 413H366L387 447H0Z', WORK['screen'], width=3)
    art += group(hands, 'translate(65 164) scale(.84)')
    art += group(controls, 'translate(765 150) scale(.50)')
    return Scene(1416, 447, art)


def arrival_order():
    """Use an ordinal assessment, not an orbit plot, scale, or countdown."""
    art = cabin_wall(660, 857)
    art += held_person('tomas', 31, 190, .85)
    art += held_person('jonah', 386, 291, .80)
    art += console(28, 659, 604, 169)
    art += text_slot('ordering')
    for x, slot in ((68, 'kaveri'), (250, 'reserves'), (452, 'recovery')):
        art += ellipse(x, 750, 8, 8, WORK['pipe_light'], INK, 1.5)
        art += text_slot(slot)
    art += path('M88 750H218L206 742M218 750L206 758M270 750H422L410 742M422 750L410 758', stroke=WORK['pipe'], width=2)
    return Scene(660, 857, art)


def transfer_assessment():
    """Keep the restrained spare outside the pressure window and the crew inside."""
    art = cabin_wall(736, 500)
    view = stars(736, 500)
    view += path('M234 345H478M246 204V345M466 204V345', stroke=FREIGHT['frame'], width=13)
    view += assembly(370, 347, 1.00)
    art += window('cradle-window', 221, 199, 272, 162, view)
    art += held_person('rina', 12, 173, .58)
    art += held_person('leila', 521, 190, .64)
    art += text_slot('cradle')
    art += path('M232 403V472M251 403V472M483 403V472M502 403V472', stroke=FREIGHT['rail'], width=5)
    return Scene(736, 500, art)


def honest_warning():
    """Keep Tomas at the shared work station, with Jonah taking his warning seriously."""
    art = cabin_wall(736, 337)
    art += group(work_portrait('tomas'), 'translate(30 168) scale(.62)')
    art += group(jonah_listener(), 'translate(484 141) scale(.65)')
    art += console(267, 269, 193, 49) + text_slot('work')
    return Scene(736, 337, art)


def asking_clearwell():
    """Keep the route work visible while Jonah reports to Elena on screen."""
    art = cabin_wall(1416, 305)
    art += group(jonah_listener(), 'translate(65 132) scale(.60)')
    art += group(work_portrait('tomas'), 'translate(530 130) scale(.46)')
    art += console(440, 260, 418, 35)
    art += path('M462 277H658M693 277H836', stroke=WORK['pipe'], width=3)
    art += elena_display('elena-initial', 1000, 149, 390, 136, 1190, 147, .30)
    art += text_slot('contact') + text_slot('location')
    return Scene(1416, 305, art)


def cost_refusal():
    """Monitor a labelled audio exchange without inventing a face for Daniel."""
    art = cabin_wall(840, 532)
    art += group(work_portrait('samir'), 'translate(597 29) scale(.75)')
    art += console(578, 277, 238, 230)
    art += text_slot('baikal') + text_slot('elena')
    art += path('M594 355H802', stroke=WORK['pipe'], width=2)
    art += text_slot('earthworks') + text_slot('operations') + text_slot('daniel')
    art += audio_grille(752, 310) + audio_grille(752, 459)
    return Scene(840, 532, art)


def taking_intercept():
    """Frame Jonah from Tomas's station, with no automatic approval display."""
    art = cabin_wall(556, 532)
    art += elena_display('elena-return', 26, 165, 178, 158, 56, 167, .32)
    art += text_slot('contact')
    art += group(jonah_listener(), 'translate(280 179) scale(.90)')
    art += path('M0 491L155 475L262 532H0Z', WORK['screen'], width=3)
    art += path('M14 509H110M30 522H141', stroke=WORK['pipe'], width=3)
    return Scene(556, 532, art)


SCENES = {
    'familiar-signal': familiar_signal,
    'answering-gantry': answering_gantry,
    'recording-report': recording_report,
    'arrival-order': arrival_order,
    'transfer-assessment': transfer_assessment,
    'honest-warning': honest_warning,
    'asking-clearwell': asking_clearwell,
    'cost-refusal': cost_refusal,
    'taking-intercept': taking_intercept,
}
