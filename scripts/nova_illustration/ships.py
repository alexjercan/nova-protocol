"""Unscaled ship shape proposals, projected from one authored solid model per ship.

Coordinates are dimensionless drawing proportions, not meters, build cells, or
engine units. No physical length or gameplay configuration is inferred here.
"""

import math
from dataclasses import dataclass

from .colors import MATERIALS, SHIP_INK, REGISTRY_PAINT, EXHAUST
from .svg import group, path, tag, text


@dataclass(frozen=True)
class Face:
    """One outward-wound surface with a named material and component."""
    vertices: tuple
    material: str
    component: str
    outline: tuple = ()
    """Unsplit ink contour; empty means this face's own vertices, before projection."""


def sub(a, b):
    """Subtract drawing vectors."""
    return tuple(x-y for x, y in zip(a, b))


def dot(a, b):
    """Project a drawing vector onto a basis vector."""
    return sum(x*y for x, y in zip(a, b))


def cross(a, b):
    """Find the oriented perpendicular for a surface."""
    return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])


def average(points):
    """Locate a surface or component center."""
    return tuple(sum(p[k] for p in points)/len(points) for k in range(3))


def normal(vertices):
    """Return an outward unit normal for a non-degenerate face."""
    n = cross(sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0]))
    length = math.sqrt(dot(n, n))
    if length < 1e-9:
        raise ValueError("Degenerate illustration face")
    return tuple(v/length for v in n)


def solid(vertices, indices, material, component):
    """Wind a convex primitive outward so every view uses the same surfaces."""
    center = average(vertices)
    result = []
    for ids in indices:
        face = tuple(vertices[i] for i in ids)
        if dot(normal(face), sub(average(face), center)) < 0:
            face = tuple(reversed(face))
        result.append(Face(face, material, component))
    return result


def box(at, size, material, component):
    """Build a box from its drawing center and extents."""
    if any(s <= 0 for s in size):
        raise ValueError("Box extents must be positive")
    vertices = tuple(tuple(at[k]+side[k]*size[k]/2 for k in range(3)) for side in (
        (-1,-1,-1), (1,-1,-1), (1,1,-1), (-1,1,-1),
        (-1,-1,1), (1,-1,1), (1,1,1), (-1,1,1),
    ))
    return solid(vertices, ((0,1,2,3),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)), material, component)


def cylinder(at, radius, length, axis, material, component, end_radius):
    """Build a cylinder or tapered bell with sixteen radial faces."""
    if min(radius, end_radius, length) <= 0 or axis not in (0,1,2):
        raise ValueError("Invalid illustration cylinder")
    radial = [k for k in range(3) if k != axis]
    vertices = []
    for end, r in [(-1, radius),(1, end_radius)]:
        for i in range(16):
            v = list(at)
            v[axis] += end*length/2
            v[radial[0]] += r*math.cos(math.tau*i/16)
            v[radial[1]] += r*math.sin(math.tau*i/16)
            vertices.append(tuple(v))
    indices = [tuple(range(16)), tuple(range(16,32))]
    indices += [(i,(i+1)%16,(i+1)%16+16,i+16) for i in range(16)]
    return solid(vertices, indices, material, component)


def hull_segment(x0, x1, half0, half1, bottom, top0, top1, bevel, material, component):
    """Loft a chamfered hull section: plating changes its silhouette, not just its color."""
    if x1 <= x0 or bevel <= 0 or min(half0,half1) <= bevel or min(top0,top1)-bottom <= 2*bevel:
        raise ValueError('Invalid plated hull segment')
    vertices = []
    for x,half,top in ((x0,half0,top0),(x1,half1,top1)):
        vertices.extend((x,y,z) for y,z in (
            (-half+bevel,bottom),(half-bevel,bottom),(half,bottom+bevel),
            (half,top-bevel),(half-bevel,top),(-half+bevel,top),
            (-half,top-bevel),(-half,bottom+bevel),
        ))
    indices = [tuple(range(8)),tuple(range(8,16))]
    indices += [(i,(i+1)%8,(i+1)%8+8,i+8) for i in range(8)]
    return solid(vertices,indices,material,component)


def shifted(faces, at):
    """Place a shared fixture without maintaining another view-specific drawing."""
    return [Face(tuple(tuple(v[k]+at[k] for k in range(3)) for v in f.vertices),f.material,f.component) for f in faces]


def louvre(at):
    """Set four sloping blades in a recessed mounting tray, following the industrial kit."""
    f = box((0,0,2),(35,29,4),'dark','louvre-tray')
    for y in (-9,-3,3,9):
        f.append(Face(((-15,y-2,4),(15,y-2,4),(15,y+2,7),(-15,y+2,7)),'steel','louvre-blade'))
    return shifted(f,at)


def radiator(at):
    """Stand a fin bank proud of its frame, rather than paint parallel lines on a box."""
    f = box((0,0,2),(43,35,4),'dark','radiator-frame')
    for x in (-16,-8,0,8,16):
        f += box((x,0,14),(3,31,24),'steel','radiator-fin')
    return shifted(f,at)


def hatch(at):
    """Layer a dark access cover and a small safety-colored handle on its mounting plate."""
    f = box((0,0,1),(31,31,2),'metal','access-plate')
    f += cylinder((0,0,3),12,2,2,'dark','access-cover',12)
    f += box((0,0,6),(15,4,4),'hazard','access-handle')
    return shifted(f,at)


def hazard_edge(x0, x1, y, z):
    """Keep the industrial accent on working edges, not across whole machines."""
    f = box(((x0+x1)/2,y,z),(x1-x0,7,1),'hazard','hazard-band')
    for x in range(x0+5,x1-5,14):
        f.append(Face(((x,y-3.5,z+.6),(x+5,y-3.5,z+.6),(x+9,y+3.5,z+.6),(x+4,y+3.5,z+.6)),'dark','hazard-stripe'))
    return f


def machinery():
    """Share a faceted lower hull, plated engine cowls, and exposed aft bell stacks."""
    f = []
    for x0,x1,w0,w1 in [(-235,-150,75,98),(-148,-40,98,98),(-38,70,98,93),(72,214,93,53)]:
        f += hull_segment(x0,x1,w0,w1,-36,20,20,17,'plate','lower-hull-plating')
    for y in (-105,105):
        f += shifted(hull_segment(-264,-210,31,34,-22,43,50,12,'plate','engine-cowl'),(0,y,0))
        f += shifted(hull_segment(-208,-143,34,27,-22,50,33,12,'plate_light','engine-cowl'),(0,y,0))
        f += cylinder((-248,y,13),24,37,0,'shadow','engine-drum',24)
        f += cylinder((-279,y,13),36,40,0,'metal','engine-bell',23)
        f += cylinder((-301,y,13),33,5,0,'steel','engine-rim',33)
        f += cylinder((-304,y,13),27,1,0,'dark','engine-throat',27)
        for x in (-290,-278,-266):
            radius = 36-(x+299)*13/40
            f += cylinder((x,y,13),radius+1.4,2,0,'steel','bell-rib',radius+.8)
        f += louvre((-239,y,47))
    return f


def crew_module(x0, split, end, half, nose_half, top):
    """Shape a plated crew compartment with a sloping brow and narrow forward windows."""
    f = hull_segment(x0,split,half,half,17,top,top,12,'paint','crew-module')
    f += hull_segment(split+1,end,half,nose_half,17,top,top-27,12,'plate_light','crew-brow')
    for side in (-1,1):
        for a,b in ((.08,.43),(.48,.9)):
            points = []
            for t,z in ((a,-29),(b,-29),(b,-17),(a,-17)):
                x = split+(end-split)*t
                y = side*(half+(nose_half-half)*t+.6)
                points.append((x,y,top-27*t+z))
            if side > 0:
                points.reverse()
            f.append(Face(tuple(points),'dark','window-frame'))
            f += box((x0+19,side*(half+.8),35),(24,1.5,7),'jade','livery')
    f += box((end+.7,0,top-47),(1,2*nose_half-17,10),'dark','forward-windows')
    f += box((end+1.3,0,top-44),(1,2*nose_half-21,2),'glass','window-glint')
    f += hatch(((x0+split)/2,0,top+.3))
    return f


def kaveri():
    """Keep the workship layout, with plated shoulders, an open cradle, and stowed machinery."""
    f = machinery()
    f += hull_segment(-224,-174,78,78,19,105,105,17,'plate','service-module')
    f += hull_segment(-172,-119,78,74,19,105,94,17,'plate_light','service-module')
    f += radiator((-198,0,105.3))
    f += louvre((-145,0,100.5))
    f += box((-202,-80,59),(33,4,24),'dark','side-service-recess')
    for x in (-215,-207,-199,-191):
        f += box((x,-83,59),(3,3,20),'steel','side-service-rib')
    for y in (-51,51):
        f += cylinder((-199,y,108),4,34,0,'steel','service-duct',4)
        f += cylinder((-179,y,108),6,5,0,'metal','duct-collar',6)
    f += crew_module(76,153,216,73,44,94)
    f += cylinder((112,-84,42),23,27,1,'metal','transfer-collar',23)
    f += cylinder((112,-99,42),19,3,1,'dark','transfer-hatch',19)
    f += cylinder((112,-101,42),14,2,1,'steel','hatch-inner',14)
    f += box((112,-103,44),(13,2,4),'hazard','hatch-handle')
    f += box((93,-101,66),(8,3,4),'mint','hatch-light')
    for x in (-89,-30,29):
        f += box((x,0,24),(56,147,8),'dark','work-cradle')
    for y in (-88,88):
        for x in (-89,-27,35):
            f += shifted(hull_segment(x-29,x+29,12,12,2,44,44,8,'plate','cradle-bulwark'),(0,y,0))
        f += hazard_edge(-115,63,y,44.7)
    for x in (-103,43):
        f += box((x,0,35),(9,158,13),'metal','cross-rail')
        for y in (-68,68):
            f += box((x,y,44),(23,15,6),'steel','load-clamp')
            f += box((x,y,48),(10,8,2),'hazard','clamp-tab')
    f += box((34,-109,29),(31,31,10),'dark','arm-base')
    f += cylinder((34,-109,48),12,28,2,'steel','arm-pedestal',12)
    f += cylinder((34,-109,65),11,19,1,'metal','arm-joint',11)
    f += box((-24,-109,65),(115,10,10),'steel','stowed-arm')
    f += box((-20,-109,72),(102,5,4),'metal','arm-stiffener')
    f += cylinder((-82,-109,65),9,16,1,'dark','arm-joint',9)
    f += box((-87,-109,51),(9,8,22),'steel','stowed-hook')
    f += box((-81,-109,43),(18,8,7),'metal','stowed-hook')
    f += box((-92,-109,33),(28,24,7),'dark','hook-rest')
    return tuple(f)


def ebro():
    """Frame the exposed vessels in a plated hauler, with service fittings and a tapered cab."""
    f = machinery()
    f += hull_segment(-229,-174,79,108,19,106,106,16,'plate','aft-service-housing')
    f += radiator((-201,0,106.3))
    for x in (-141,-69,3,75):
        f += hull_segment(x-34,x+34,122,122,-14,39,39,16,'plate','tank-bed-plating')
    for y in (-77,0,77):
        for x in (-119,-35,49):
            f += cylinder((x,y,72),31,82,0,'tank','water-vessel',31)
        for x in (-137,63):
            f += cylinder((x,y,72),34,8,0,'metal','vessel-band',34)
        f += cylinder((-165,y,72),25,9,0,'steel','vessel-end',31)
        f += cylinder((96,y,72),31,10,0,'metal','vessel-end',24)
        f += cylinder((103,y,72),13,5,0,'dark','vessel-cap',13)
        f += box((107,y,72),(3,15,4),'hazard','vessel-handle')
    for x in (-169,109):
        for y in (-116,116):
            f += box((x,y,73),(11,11,88),'metal','tank-frame')
            f += box((x,y,34),(24,24,9),'steel','frame-foot')
        for z in (31,113):
            f += box((x,0,z),(11,241,11),'metal','tank-frame')
        for y in (-82,0,82):
            f += box((x,y,121),(16,24,4),'steel','frame-gusset')
    for y in (-118,118):
        for x in (-111,-39,33):
            f += cylinder((x,y,47),3,68,0,'steel','service-pipe',3)
            f += cylinder((x-32,y,47),5,4,0,'metal','pipe-collar',5)
        f += hazard_edge(-152,92,y,39.8)
    f += crew_module(123,170,222,60,38,83)
    f += cylinder((146,-74,37),19,24,1,'metal','transfer-collar',19)
    f += cylinder((146,-88,37),15,3,1,'dark','transfer-hatch',15)
    f += box((146,-91,38),(12,2,4),'hazard','hatch-handle')
    return tuple(f)


def gantry():
    """Give the supply workship a broad cargo body, raised frame, and single aft bell."""
    f = hull_segment(-223,88,106,106,-35,23,23,17,'plate','gantry-keel')
    f += hull_segment(90,223,106,48,-35,23,23,17,'plate_light','gantry-forefoot')
    f += hull_segment(-214,-132,95,95,22,108,108,14,'dark','gantry-service-core')
    f += hull_segment(-218,-129,97,97,90,114,114,8,'plate','gantry-service-roof')
    f += box((-173,-98,58),(78,5,64),'plate','gantry-service-cover')
    f += box((-173,98,58),(78,5,64),'plate','gantry-service-starboard')
    f += radiator((-176,34,114.4))
    for z in (50,63,76):
        f += cylinder((-174,-97,z),2,45,0,'accent','gantry-service-feed',2)
    for x0,x1 in [(-126,-57),(-55,14),(16,85)]:
        f += hull_segment(x0,x1,101,101,22,118,118,16,'paint','gantry-cargo-body')
        for side in (-1,1):
            f += box(((x0+x1)/2,side*102,66),(x1-x0-9,2,56),'plate_light','gantry-cargo-door')
            f += box(((x0+x1)/2,side*103.5,46),(17,2,4),'dark','gantry-door-latch')
    for x in (-119,76):
        for y in (-109,109):
            f += box((x,y,75),(15,14,116),'steel','gantry-frame-leg')
            f += box((x,y,28),(29,28,11),'metal','gantry-frame-foot')
        f += box((x,0,140),(15,232,14),'metal','gantry-frame-crossbar')
        f += box((x,0,131),(25,34,6),'dark','gantry-stowed-carriage')
        f += box((x,0,126),(12,17,5),'hazard','gantry-handling-fixture')
        for y in (-109,109):
            f += box((x,y,148),(25,24,2),'steel','gantry-frame-cap')
            for dx in (-7,7):
                f += cylinder((x+dx,y,150),2,2,2,'metal','gantry-frame-fastener',2)
    f += louvre((-87,24,118.4))
    f += hatch((36,-34,118.4))
    for y in (-109,109):
        f += box((-20,y,135),(197,10,10),'steel','gantry-frame-rail')
        f += hazard_edge(-114,72,y,140.5)
        for x in (-70,-20,30):
            f += cylinder((x,y,-7),2.5,43,0,'steel','gantry-lower-service-pipe',2.5)
            f += cylinder((x-20,y,-7),4,3,0,'metal','gantry-lower-pipe-collar',4)
    f += crew_module(105,168,231,76,48,92)
    f += box((131,-77,69),(42,2,11),'accent','gantry-crew-band')
    f += cylinder((133,-91,43),23,28,1,'metal','gantry-transfer-collar',23)
    f += cylinder((133,-107,43),19,3,1,'dark','gantry-transfer-hatch',19)
    f += box((133,-110,44),(13,2,4),'hazard','gantry-transfer-handle')
    f += box((112,-108,67),(8,3,4),'mint','gantry-transfer-light')
    f += hull_segment(-259,-224,56,72,-29,74,91,16,'plate','gantry-drive-cowl')
    f += cylinder((-264,0,18),39,34,0,'steel','gantry-drive-drum',39)
    f += cylinder((-291,0,18),48,34,0,'metal','gantry-drive-bell',33)
    f += cylinder((-311,0,18),46,5,0,'steel','gantry-drive-rim',46)
    f += cylinder((-314,0,18),38,1,0,'dark','gantry-drive-throat',38)
    for y in (-75,75):
        f += box((-240,y,34),(39,24,38),'shadow','gantry-aft-auxiliary')
        f += louvre((-240,y,54))
    return tuple(f)


MODELS = {'kaveri': kaveri, 'ebro': ebro, 'gantry': gantry}


def ship_faces(name, state='intact'):
    """Return one hull with an explicitly authored condition, not a second damage model.

Only Gantry has a stranded condition. This is illustration staging, not damage
simulation, a destruction sequence, or an engineering explanation of the fault.
"""
    faces = MODELS[name]()
    if state == 'intact':
        return faces
    if (name,state) != ('gantry','stranded'):
        raise KeyError(f'Unknown ship condition: {name}/{state}')
    removed = {'gantry-service-roof','gantry-service-cover'}
    kept = tuple(face for face in faces if face.component not in removed)
    tears = (
        Face(((-213,-101,28),(-143,-101,28),(-153,-101,44),(-183,-101,36),(-201,-101,60),(-213,-101,76)),'steel','gantry-torn-service-cover'),
        Face(((-213,-101,93),(-197,-101,79),(-186,-101,93),(-162,-101,80),(-135,-101,91),(-135,-101,103),(-213,-101,103)),'plate','gantry-torn-service-cover'),
        Face(((-215,-95,112),(-180,-95,112),(-191,-70,112),(-169,-57,112),(-191,-31,112),(-182,-10,112),(-215,-10,112)),'steel','gantry-torn-service-roof'),
        Face(((-193,-95.6,50),(-181,-95.6,46),(-163,-95.6,64),(-149,-95.6,61),(-140,-95.6,83),(-167,-95.6,88),(-193,-95.6,72)),'scorch','gantry-service-scorch'),
    )
    roof_remainder = box((-173,49,111),(89,96,6),'plate','gantry-retained-roof')
    return kept+tuple(roof_remainder)+tears
VIEWS = {'top': (-90,90), 'side': (-90,0), 'front': (0,0), 'forward-quarter': (-55,28), 'aft-quarter': (-125,27)}


def basis(view):
    """Return right, up, and toward-camera axes for a registered view."""
    az, el = (math.radians(a) for a in VIEWS[view])
    toward = (math.cos(el)*math.cos(az),math.cos(el)*math.sin(az),math.sin(el))
    right = (-math.sin(az),math.cos(az),0)
    return right, cross(toward,right), toward


def project(point, view):
    """Project an authored anchor with exactly the same camera as its model."""
    right, up, toward = basis(view)
    return dot(point,right), -dot(point,up), dot(point,toward)


def bounds(name, view, state='intact'):
    """Measure an authored condition without assigning physical dimensions."""
    points = [project(v,view) for f in ship_faces(name,state) for v in f.vertices]
    return min(p[0] for p in points),min(p[1] for p in points),max(p[0] for p in points),max(p[1] for p in points)


def shade(color, value):
    """Apply one of a few flat lighting levels without a material gradient."""
    return '#'+''.join(f'{min(255,round(int(color[i:i+2],16)*value)):02x}' for i in (1,3,5))


def split_surface(face, n, offset):
    """Split an intersecting surface at a painter partition, retaining its material."""
    sides = ([], [])
    vertices = face.vertices
    for a,b in zip(vertices,vertices[1:]+vertices[:1]):
        da,db = dot(n,a)-offset,dot(n,b)-offset
        if da <= 1e-7:
            sides[0].append(a)
        if da >= -1e-7:
            sides[1].append(a)
        if (da < -1e-7 and db > 1e-7) or (da > 1e-7 and db < -1e-7):
            point = tuple(a[k]+(b[k]-a[k])*da/(da-db) for k in range(3))
            sides[0].append(point)
            sides[1].append(point)
    return tuple(Face(tuple(v),face.material,face.component,face.outline or face.vertices) for v in sides)


def painter_order(faces):
    """Order visible surfaces with plane splits so plating cannot hide its own fixtures.

Every retained normal faces the orthographic camera. A partition's back side
therefore paints before its plane, then its front side. Whole-face midpoint
sorting is insufficient for a long plate under a small hatch or hazard band.
"""
    if not faces:
        return []
    def area(entry):
        face,n = entry
        return abs(sum(dot(cross(a,b),n) for a,b in zip(face.vertices,face.vertices[1:]+face.vertices[:1])))
    partition,n = max(faces,key=area)
    offset = dot(n,partition.vertices[0])
    back,plane,front = [],[],[]
    for face,face_normal in faces:
        distances = [dot(n,v)-offset for v in face.vertices]
        lo,hi = min(distances),max(distances)
        if lo >= -1e-7 and hi <= 1e-7:
            plane.append((face,face_normal))
        elif hi <= 1e-7:
            back.append((face,face_normal))
        elif lo >= -1e-7:
            front.append((face,face_normal))
        else:
            a,b = split_surface(face,n,offset)
            back.append((a,face_normal))
            front.append((b,face_normal))
    return painter_order(back)+plane+painter_order(front)


def contour_segments(face):
    """Keep original surface edges, not the artificial seams introduced by painter splits."""
    original = face.outline or face.vertices
    edges = tuple(zip(original,original[1:]+original[:1]))
    result = []
    for a,b in zip(face.vertices,face.vertices[1:]+face.vertices[:1]):
        for start,end in edges:
            direction = sub(end,start)
            if all(dot(cross(sub(p,start),direction),cross(sub(p,start),direction)) < 1e-8 for p in (a,b)):
                result.append((a,b))
                break
    return result


DRAW_TOLERANCE = .2
"""Largest coordinate error the export may leave in the placed drawing, in drawn units."""


def draw_precision(scale):
    """Give a projection the decimals its placement can show, not a fixed count.

A model carries the same coordinates however small it is drawn. At scale one a
third decimal of a model unit is a thousandth of a drawn unit, and below that it
buys nothing at all, so the emitted precision follows the placement.
"""
    if not math.isfinite(scale) or scale <= 0:
        raise ValueError('Finite placement and a positive drawing scale are required')
    return max(0,math.ceil(math.log10(scale/DRAW_TOLERANCE)))


def _placed(value, digits):
    """Quantise one projected coordinate, without exporting a negative zero."""
    return round(value,digits) or 0.


def _quantised(points, digits):
    """Quantise a projected outline and drop the neighbours it collapses together."""
    corners = [(_placed(a,digits),_placed(b,digits)) for a,b in points]
    return [corner for i,corner in enumerate(corners) if corner != corners[i-1]]


def render_faces(surfaces, view, x, y, scale):
    """Project an authored prop with the same occlusion and ink as ship surfaces.

Coordinates and scale are drawing proportions, not physical measurements.
"""
    if not all(math.isfinite(v) for v in (x,y,scale)) or scale <= 0:
        raise ValueError('Finite placement and a positive drawing scale are required')
    return group(_surface_art(surfaces,view,draw_precision(scale)),f'translate({x} {y}) scale({scale})')


def _surface_art(surfaces, view, digits):
    _, _, toward = basis(view)
    faces = []
    for f in surfaces:
        if f.material not in MATERIALS:
            raise KeyError(f.material)
        if len(f.vertices)<3:
            raise ValueError('An illustration face needs at least three vertices')
        if any(len(v)!=3 or not all(math.isfinite(c) for c in v) for v in (*f.vertices,*f.outline)):
            raise ValueError('Finite three-coordinate illustration vertices are required')
        n = normal(f.vertices)
        if dot(n,toward) > 1e-7:
            faces.append((f,n))
    art = ''
    for f, n in painter_order(faces):
        corners = _quantised([project(v,view)[:2] for v in f.vertices],digits)
        if len(corners) < 3:
            continue
        points = ' '.join(f'{a:.{digits}f},{b:.{digits}f}' for a,b in corners)
        lighting = 1.05 if n[2] > .5 else (0.87 if n[1] < -.3 else 0.73)
        fill = shade(MATERIALS[f.material],lighting)
        if not f.outline:
            art += tag('polygon', points=points, fill=fill, stroke=SHIP_INK, stroke_width=1.2, stroke_linejoin='round', data_component=f.component)
        else:
            art += tag('polygon', points=points, fill=fill, stroke=fill, stroke_width=1.2, data_component=f.component)
            edges = ''
            for a,b in contour_segments(f):
                ends = _quantised([project(a,view)[:2],project(b,view)[:2]],digits)
                if len(ends) == 2:
                    (ax,ay),(bx,by) = ends
                    edges += f'M{ax:.{digits}f} {ay:.{digits}f}L{bx:.{digits}f} {by:.{digits}f}'
            if edges:
                art += path(edges,stroke=SHIP_INK,width=1.2)
    return art


def render_ship(name, view, x, y, scale, thrust, state='intact', *, cargo=()):
    """Project a registered ship view into SVG, separate from its color scheme.

The default intact condition preserves existing exports. Only Gantry has an
authored stranded condition, which cannot show main-drive thrust. Plumes are
visuals, not performance claims. Use styles.present for comic or lore colors.
Cargo defaults to no load. Caller-authored faces use the ship's drawing
coordinates and share its painter pass; this does not validate physical fit.
"""
    if not all(math.isfinite(v) for v in (x,y,scale)) or scale <= 0:
        raise ValueError('Finite placement and a positive drawing scale are required')
    _, _, toward = basis(view)
    digits = draw_precision(scale)
    surfaces = ship_faces(name,state)+tuple(cargo)
    if state != 'intact' and thrust:
        raise ValueError('A stranded ship cannot show main-drive thrust')
    art = ''
    if thrust:
        nozzles = ((-314,0,18,34,360),) if name == 'gantry' else ((-304,-105,13,22,371),(-304,105,13,22,371))
        for aft,side,height,radius,length in nozzles:
            outline = [(aft,side-radius,height),(aft-length,side,height),(aft,side+radius,height)]
            plume = _quantised([project(v,view)[:2] for v in outline],digits)
            art += path('M'+'L'.join(f'{a:.{digits}f} {b:.{digits}f}' for a,b in plume)+'Z',EXHAUST,'none',opacity=0.35)
    art += _surface_art(surfaces,view,digits)
    if toward[1] < -0.1:
        anchor = (-46,-103,80) if name == 'gantry' else (-55,-74,-20)
        origin = project(anchor,view)
        along = sub(project((anchor[0]+1,anchor[1],anchor[2]),view),origin)
        up = sub(project((anchor[0],anchor[1],anchor[2]-1),view),origin)
        art += group(text(0,0,name.upper(),11,REGISTRY_PAINT,font_weight='bold',letter_spacing=2),f'matrix({along[0]:.4f} {along[1]:.4f} {up[0]:.4f} {up[1]:.4f} {origin[0]:.4f} {origin[1]:.4f})')
    condition = {} if state == 'intact' else {'data_state':state}
    return group(art,f'translate({x} {y}) scale({scale})',data_ship=name,data_view=view,**condition)
