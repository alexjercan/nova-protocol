"""Episode-local cargo and access equipment, not a series-wide hardware standard."""
from nova_illustration.colors import INK, FREIGHT, WORK
from nova_illustration.ships import box, cylinder, render_faces
from nova_illustration.svg import group, path, rect
from nova_illustration.scenes import text_slot

def assembly_faces():
    """Build one covered assembly and handling frame in Kaveri's drawing coordinates.

These proportions let the same prop appear in close inspection and in the
external cradle. They establish no measurements, mass, or shipping standard.
"""
    faces = box((-30, 0, 33), (144, 124, 8), 'steel', 'assembly-frame')
    for x in (-98, 38):
        for y in (-57, 57):
            faces += box((x, y, 65), (8, 8, 60), 'metal', 'assembly-frame')
        faces += box((x, 0, 96), (8, 122, 7), 'plate_light', 'assembly-frame')
    faces += cylinder((-30, 0, 70), 32, 78, 0, 'assembly', 'replacement-housing', 32)
    for x in (-72, 12):
        faces += cylinder((x, 0, 70), 35, 6, 0, 'assembly_face', 'replacement-flange', 35)
        faces += cylinder((x + (-4 if x < 0 else 4), 0, 70), 23, 4, 0, 'dark', 'protective-cover', 23)
    faces += cylinder((-28, 0, 112), 17, 20, 2, 'metal', 'replacement-connection', 17)
    faces += cylinder((-28, 0, 124), 20, 5, 2, 'dark', 'protective-cover', 20)
    faces += box((-28, 0, 128), (22, 5, 3), 'hazard', 'cover-handle')
    for x in (-52, -8):
        faces += box((x, 0, 105), (5, 67, 4), 'jade', 'assembly-restraint')
        for y in (-34, 34):
            faces += box((x, y, 69), (5, 4, 68), 'jade', 'assembly-restraint')
    faces += box((-53, -64, 40), (57, 3, 13), 'dark', 'identification-plate')
    return tuple(faces)

def assembly(x, y, scale):
    """Render the intact prop without duplicating its geometry for close views."""
    return group(render_faces(assembly_faces(), 'forward-quarter', x, y, scale), data_prop='replacement-assembly')

def handhold(x, y, scale):
    """Place a wall-mounted rail behind the shared gripping hand."""
    art = rect(-7, 368, 26, 112, FREIGHT['frame'], INK, 2, 8)
    art += path('M6 376V471', stroke=FREIGHT['rail'], width=10)
    art += path('M3 380V468', stroke=FREIGHT['edge'], width=2)
    return group(art, f'translate({x} {y}) scale({scale})', data_prop='handhold')

def freight_lock(x, y, scale):
    """Show only the closed pressure-side door; no simultaneous open path to space."""
    art = rect(0, 0, 270, 280, FREIGHT['frame'], INK, 4, 26)
    art += rect(15, 15, 240, 250, FREIGHT['rail'], INK, 2, 20)
    art += rect(28, 28, 214, 224, FREIGHT['panel'], INK, 3, 12)
    art += path('M134 29V251M29 229H241', stroke=FREIGHT['frame'], width=5)
    art += path('M105 115V161M164 115V161', stroke=FREIGHT['rail'], width=6)
    art += rect(60, 44, 150, 36, WORK['screen'], INK, 2, 4)
    art += text_slot('freight-lock')
    art += path('M31 237H240', stroke=FREIGHT['stripe'], width=9, stroke_dasharray='12 9')
    return group(art, f'translate({x} {y}) scale({scale})', data_prop='freight-lock', data_door='closed')
