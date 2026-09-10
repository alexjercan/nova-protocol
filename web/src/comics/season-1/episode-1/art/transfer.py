"""Supported freefall transfer through the chosen short connection.

The episode owns the temporary stretcher, restraints, and local receiving view.
These drawings specify neither an injury nor a permanent cabin or medical bay.
"""

from nova_illustration.colors import INK, FREIGHT, WORK, MATERIALS, RINA, IVO
from nova_illustration.portraits import gantry_portrait, standing_body, supported_owen, freefall_guide, gripping_arm, stretcher_grip, rail_grip_hand
from nova_illustration.scenery import stars
from nova_illustration.scenes import Scene, text_slot
from nova_illustration.svg import group, path, rect, tag
from aquila import held_person
from distress import cabin_wall, console
from docking import docking_pair
from props import handhold


def stretcher_rail(side, fraction):
    """Locate a grip on a side rail in the stretcher's foreshortened drawing frame."""
    if side not in ('left', 'right') or not 0 <= fraction <= 1:
        raise ValueError('A stretcher rail needs a side and an in-range position')
    start, end = ((30, -36), (-86, 724)) if side == 'left' else ((304, -36), (422, 724))
    return tuple(a + (b - a) * fraction for a, b in zip(start, end))


def placed_rail(placement, side, fraction):
    """Use the same anchor for the visible frame and its controlling hand."""
    x, y, scale = placement
    a, b = stretcher_rail(side, fraction)
    return (x + a * scale, y + b * scale)


def stretcher(placement):
    """Put padding behind Owen and body restraints above him, with both feet supported."""
    x, y, scale = placement
    art = path('M30-36H304L422 724H-86Z', FREIGHT['frame'], width=4)
    art += path('M45-19H289L404 705H-69Z', WORK['pipe_light'], width=3)
    art += path('M59-5H276L381 683H-44Z', FREIGHT['wall'], width=2)
    art += path('M71 4H263L275 208H57Z', FREIGHT['wall_light'], width=2)
    art += path('M32 324H303M10 460H325M-20 613H355', stroke=FREIGHT['stripe'], width=2)
    art += supported_owen()
    for height, half in [(359, 160), (467, 180), (601, 207)]:
        art += path(f'M{167-half} {height}Q167 {height+24} {167+half} {height}L{167+half+3} {height+20}Q167 {height+44} {164-half} {height+20}Z', FREIGHT['panel_shadow'], width=2)
        art += rect(150, height + 12, 34, 23, FREIGHT['rail'], INK, 2, 3)
        art += path(f'M159 {height+17}V{height+30}H177', stroke=FREIGHT['frame'], width=3)
    for side in ('left', 'right'):
        a, b = stretcher_rail(side, 0), stretcher_rail(side, 1)
        art += path(f'M{a[0]} {a[1]}L{b[0]} {b[1]}', stroke=FREIGHT['rail'], width=11)
    return group(art, f'translate({x} {y}) scale({scale})', data_prop='padded-stretcher', data_patient='owen', data_body_restraints='secured')


def transfer_port(cx, cy, radius, state):
    """Show one pressure passage, with a short lined bore rather than an open space door."""
    if state not in ('open', 'closed'):
        raise ValueError('Unknown transfer hatch state')
    art = ''
    for ratio, fill in [(1, MATERIALS['metal']), (.95, MATERIALS['dark']), (.9, MATERIALS['steel']), (.78, FREIGHT['panel_shadow'])]:
        art += tag('circle', cx=cx, cy=cy, r=radius*ratio, fill=fill, stroke=INK, stroke_width=3)
    if state == 'open':
        art += path(f'M{cx-radius*.67} {cy-radius*.35}V{cy+radius*.35}M{cx+radius*.67} {cy-radius*.35}V{cy+radius*.35}', stroke=FREIGHT['rail'], width=6)
        for direction in (-1, 1):
            art += path(f'M{cx+direction*radius*.7} {cy-radius*.42}L{cx+direction*radius*.86} {cy-radius*.5}M{cx+direction*radius*.7} {cy+radius*.42}L{cx+direction*radius*.86} {cy+radius*.5}', stroke=FREIGHT['frame'], width=5)
    else:
        art += tag('circle', cx=cx, cy=cy, r=radius*.78, fill=MATERIALS['steel'], stroke=INK, stroke_width=3)
        art += path(f'M{cx-radius*.5} {cy}H{cx+radius*.5}', stroke=MATERIALS['hazard'], width=9)
    return group(art, data_prop='sealed-transfer-passage', data_hatches=state, data_gravity='freefall')


def guide_layers(name, grip, scale, side):
    """Put the body behind the patient, then its controlling arm above the frame."""
    if side not in ('left', 'right'):
        raise ValueError('Unknown guide side')
    mirrored = side == 'right'
    wrist_x = -96 if mirrored else 430
    x, y = grip[0] - wrist_x * scale, grip[1] - 433 * scale
    body = freefall_guide(name) if name in ('rina', 'ivo') else standing_body(name)
    wall = handhold(0, 0, 1) + gripping_arm(name)
    arm = stretcher_grip(name)
    if mirrored:
        wall = group(wall, 'translate(334 0) scale(-1 1)')
        arm = group(arm, 'translate(334 0) scale(-1 1)')
    transform = f'translate({x} {y}) scale({scale})'
    body = group(body + wall, transform)
    arm = group(group(arm, transform), data_holder=name, data_control='maintained', data_contact=f'{grip[0]} {grip[1]}')
    return body, arm


def meeting_at_opening():
    """Enter Gantry with Rina, after checks, and let Owen speak for himself."""
    art = cabin_wall(1416, 320)
    art += transfer_port(228, 327, 220, 'open')
    art += held_person('rina', 80, 105, .48)
    art += group(gantry_portrait('ivo'), 'translate(550 154) scale(.58)')
    art += stretcher((1055, 42, .7))
    art += text_slot('gantry')
    return Scene(1416, 320, art)


TRANSFER_PLACEMENT = (357, 137, .46)
RECEIVING_PLACEMENT = (222, 180, .42)


def moving_together():
    """Bring the feet through first, with Rina near and Ivo controlling the head end."""
    art = cabin_wall(896, 517) + transfer_port(500, 295, 221, 'open')
    rina_body, rina_arm = guide_layers('rina', placed_rail(TRANSFER_PLACEMENT, 'left', .79), .58, 'left')
    ivo_body, ivo_arm = guide_layers('ivo', placed_rail(TRANSFER_PLACEMENT, 'right', .52), .34, 'right')
    clip = tag('defs', tag('clipPath', tag('circle', cx=500, cy=295, r=221*.9), id='gantry-crew-space'))
    art += clip + group(ivo_body, clip_path='url(#gantry-crew-space)')
    art += rina_body + stretcher(TRANSFER_PLACEMENT) + rina_arm + ivo_arm
    return Scene(896, 517, group(art, data_transfer='feet-first'))


def retained_hands(placement):
    """Keep both guides' sleeves continuous to the crop while Samir secures the frame."""
    rx, ry = placed_rail(placement, 'left', .37)
    ix, iy = placed_rail(placement, 'right', .24)
    rina = path(f'M-40 393L-40 441L128 410L{rx+16} {ry+16}L{rx-6} {ry-8}L111 375Z', RINA['coat'], width=2.5)
    rina += path(f'M-20 419L119 392L{rx-1} {ry+18}', stroke=RINA['coat_light'], width=2)
    rina += group(rail_grip_hand('rina'), f'translate({rx} {ry}) scale(.58)')
    ivo = path(f'M530 {iy-42}L524 {iy+1}L{ix+57} {iy+29}L{ix-1} {iy+12}L{ix+2} {iy-13}L{ix+54} {iy-4}Z', IVO['shirt'], width=2.5)
    ivo += path(f'M513 {iy-19}L{ix+57} {iy+12}L{ix+16} {iy}', stroke=IVO['coat_light'], width=2)
    ivo += group(rail_grip_hand('ivo'), f'translate({ix} {iy}) scale(-.45 .45)')
    return group(rina, data_holder='rina', data_control='maintained', data_contact=f'{rx} {ry}') + group(ivo, data_holder='ivo', data_control='maintained', data_contact=f'{ix} {iy}')


def securing_support():
    """Close two temporary rigid frame clamps before either guide releases control."""
    art = cabin_wall(500, 517)
    art += transfer_port(353, 187, 128, 'open')
    art += path('M204 488H468', stroke=FREIGHT['frame'], width=18)
    art += path('M210 486H462', stroke=FREIGHT['rail'], width=9)
    sx, sy = placed_rail(RECEIVING_PLACEMENT, 'left', .8)
    samir_body, samir_hand = guide_layers('samir', ((sx+220)/2, (sy+486)/2), .53, 'left')
    art += samir_body + stretcher(RECEIVING_PLACEMENT)
    art += receiving_clamps(RECEIVING_PLACEMENT, ((220, 486), (450, 486)))
    art += retained_hands(RECEIVING_PLACEMENT) + samir_hand
    return Scene(500, 517, group(art, data_receiving='secured-before-release'))


def receiving_clamps(placement, anchors):
    """Reuse the two closed rigid clamps at a scene's fixed receiving rail."""
    art = ''
    for side, end in zip(('left', 'right'), anchors, strict=True):
        sx, sy = placed_rail(placement, side, .8)
        ex, ey = end
        clamp = path(f'M{sx} {sy}L{ex} {ey}', stroke=INK, width=14)
        clamp += path(f'M{sx} {sy}L{ex} {ey}', stroke=FREIGHT['rail'], width=9)
        for ax, ay in [(sx, sy), (ex, ey)]:
            clamp += rect(ax-7, ay-7, 14, 14, MATERIALS['steel'], INK, 2, 2)
        clamp += rect((sx+ex)/2-8, (sy+ey)/2-10, 16, 20, MATERIALS['metal'], INK, 2, 2)
        clamp += path(f'M{(sx+ex)/2-4} {(sy+ey)/2+3}L{(sx+ex)/2+5} {(sy+ey)/2-3}', stroke=FREIGHT['stripe'], width=3)
        art += group(clamp, data_prop='receiving-restraint', data_mechanism='rigid-clamp', data_latch='closed')
    return art


def last_across():
    """Receive Nadia last, with a restrained record case rather than a plot dossier."""
    art = cabin_wall(856, 407) + transfer_port(213, 297, 207, 'open')
    art += held_person('nadia', 36, 140, .64)
    art += held_person('jonah', 557, 126, .66)
    case = path('M190 332L100 381', stroke=FREIGHT['stripe'], width=7)
    case += path('M116 344L245 334L255 388L122 400Z', FREIGHT['frame'], width=3)
    case += path('M125 352L237 344L245 381L130 391Z', WORK['pipe_light'], width=2)
    art += group(case, data_prop='crew-record-case', data_restraint='secured')
    return Scene(856, 407, art)


def transfer_side_clear():
    """Keep Rina's clear passage separate from Leila's closing checks."""
    art = cabin_wall(540, 407) + transfer_port(237, 323, 128, 'closed')
    art += held_person('rina', 4, 146, .57)
    art += held_person('leila', 328, 152, .55)
    art += console(332, 333, 189, 64)
    art += text_slot('closing')
    return Scene(540, 407, art)


def leaving_gantry():
    """Withdraw along the same connection axis; leave the stranded hull intact in space."""
    art = rect(0, 0, 1416, 430, 'url(#space)') + stars(1416, 430)
    art += docking_pair(210, 680, 127, .53)
    return Scene(1416, 430, art)


SCENES = {
    'meeting-at-opening': meeting_at_opening,
    'moving-together': moving_together,
    'securing-support': securing_support,
    'last-across': last_across,
    'transfer-side-clear': transfer_side_clear,
    'leaving-gantry': leaving_gantry,
}
