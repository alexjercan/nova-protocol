"""One opposed-heading docking arrangement in unscaled illustration coordinates.

Shared hulls stay intact as sources. The episode owns the short connector and
relative placement, not a docking standard, physical dimensions, or simulation.
"""

import math
from nova_illustration.colors import WORK
from nova_illustration.portraits import work_portrait
from nova_illustration.scenery import stars
from nova_illustration.scenes import Scene, text_slot
from nova_illustration.ships import Face, ship_faces, render_faces, normal, dot
from nova_illustration.svg import group, path, rect
from distress import cabin_wall, console
from props import assembly_faces

SLEEVE_SPAN = 28
BORE_RADIUS = 20
COLLARS = {'kaveri': 'transfer-collar', 'gantry': 'gantry-transfer-collar'}


def collar_mouth(name):
    """Derive the outward collar plane and center from the existing hull geometry."""
    vertices = [v for face in ship_faces(name) if face.component == COLLARS[name] for v in face.vertices]
    low = tuple(min(v[k] for v in vertices) for k in range(3))
    high = tuple(max(v[k] for v in vertices) for k in range(3))
    return ((low[0] + high[0]) / 2, low[1], (low[2] + high[2]) / 2)


def gantry_point(point, gap):
    """Place Gantry opposite Kaveri in a Kaveri-relative frame, without resizing it."""
    kx, ky, kz = collar_mouth('kaveri')
    gx, gy, gz = collar_mouth('gantry')
    x, y, z = point
    return (kx + gx - x, ky + gy - SLEEVE_SPAN - gap - y, kz - gz + z)


def moved_faces(faces, transform, prefix):
    """Preserve material, winding, and original outlines under a rigid placement."""
    return tuple(Face(tuple(transform(v) for v in f.vertices), f.material, prefix + f.component,
                      tuple(transform(v) for v in f.outline)) for f in faces)


def outward_face(vertices, direction, material, component):
    """Wind a connector surface toward its outside or toward the hollow bore."""
    if dot(normal(vertices), direction) < 0:
        vertices = vertices[::-1]
    return Face(tuple(vertices), material, component)


def tube_section(y0, y1, outer, component):
    """Draw an open annular section; no end disk blocks the passage."""
    x, _, z = collar_mouth('kaveri')
    faces = []
    def point(radius, angle, y):
        return (x + radius * math.cos(angle), y, z + radius * math.sin(angle))
    for i in range(16):
        a, b = math.tau * i / 16, math.tau * (i + 1) / 16
        radial = (math.cos((a + b) / 2), 0, math.sin((a + b) / 2))
        for radius, direction, material, part in [
            (outer, radial, 'metal', component),
            (BORE_RADIUS, tuple(-n for n in radial), 'dark', 'connector/bore'),
        ]:
            vertices = (point(radius, a, y0), point(radius, b, y0), point(radius, b, y1), point(radius, a, y1))
            faces.append(outward_face(vertices, direction, material, part))
        for y, direction in [(y0, (0, -1, 0)), (y1, (0, 1, 0))]:
            vertices = (point(BORE_RADIUS, a, y), point(outer, a, y), point(outer, b, y), point(BORE_RADIUS, b, y))
            faces.append(outward_face(vertices, direction, 'steel', 'connector/rim'))
    return tuple(faces)


def connector_faces():
    """Keep the same short hollow sleeve attached to Kaveri throughout the approach."""
    _, near, _ = collar_mouth('kaveri')
    far = near - SLEEVE_SPAN
    return (tube_section(far, far + 3, 26, 'connector/gantry-cuff')
            + tube_section(far + 3, near - 3, 23.5, 'connector/sleeve')
            + tube_section(near - 3, near, 26, 'connector/kaveri-cuff'))


def paired_faces(gap):
    """Move only the relative separation; both headings and the connector stay fixed."""
    if isinstance(gap, bool) or not isinstance(gap, (int, float)) or not math.isfinite(gap) or gap < 0:
        raise ValueError('Docking illustration gap must be finite and nonnegative')
    kaveri = moved_faces(ship_faces('kaveri') + assembly_faces(), lambda v: v, 'kaveri/')
    # Cache the anchor transform rather than regenerate both hulls for every vertex.
    tx, ty, tz = gantry_point((0, 0, 0), gap)
    gantry = moved_faces(ship_faces('gantry', 'stranded'), lambda v: (tx - v[0], ty - v[1], tz + v[2]), 'gantry/')
    return kaveri + gantry + connector_faces()


def tilted_point(point):
    """Give the whole arrangement one elevated camera, not separately rotated SVGs."""
    x, y, z = point
    angle = math.radians(-30)
    return (x, y * math.cos(angle) - z * math.sin(angle), y * math.sin(angle) + z * math.cos(angle))


def docking_pair(gap, x, y, scale):
    """Render both original hulls and the connector in one shared occlusion pass."""
    faces = moved_faces(paired_faces(gap), tilted_point, '')
    return group(render_faces(faces, 'top', x, y, scale), data_ship_pair='kaveri-gantry',
                 data_gantry_state='stranded', data_motion='coast', data_connector='short-rigid', data_gap=gap)


def holding_at_port():
    """Show the remaining gap with both hull identities and the covered load visible."""
    art = rect(0, 0, 900, 857, 'url(#space)') + stars(900, 857)
    art += docking_pair(120, 340, 290, .92)
    art += text_slot('kaveri') + text_slot('gantry')
    return Scene(900, 857, art)


def checked_alignment():
    """Keep Leila's assessment and Tomas's control work in the same cabin."""
    art = cabin_wall(496, 350)
    art += group(work_portrait('leila'), 'translate(35 137) scale(.58)')
    art += group(work_portrait('tomas'), 'translate(302 217) scale(.52)')
    art += console(229, 278, 102, 54)
    art += path('M242 295H316M242 309H292M242 323H310', stroke=WORK['pipe'], width=2)
    return Scene(496, 350, art)


def checking_seal():
    """Close in on the same joined collars; both hatches remain closed for checks."""
    art = rect(0, 0, 496, 487, 'url(#space)') + stars(496, 487)
    cx, cy, cz = collar_mouth('kaveri')
    px, py, _ = tilted_point((cx, cy - SLEEVE_SPAN / 2, cz))
    art += docking_pair(0, 205 - px * 3.4, 306 + py * 3.4, 3.4)
    return Scene(496, 487, art)


SCENES = {
    'holding-at-port': holding_at_port,
    'checked-alignment': checked_alignment,
    'checking-seal': checking_seal,
}
