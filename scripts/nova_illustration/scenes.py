"""Standalone scene artwork. Story text and page composition belong to TypeScript."""

from dataclasses import dataclass
from math import isfinite
import re
from xml.etree import ElementTree as ET

from .colors import SKY, SATURN
from .faces import FACES
from .svg import tag, group


@dataclass(frozen=True)
class Scene:
    """Art in panel-local drawing coordinates, without a page frame or story text."""
    width: float
    height: float
    art: str


def text_slot(name):
    """Reserve a text insertion point in painter order without owning its words."""
    if not re.fullmatch(r'[a-z][a-z0-9-]*',name):
        raise ValueError(f'Invalid story slot: {name}')
    return group('',data_story_slot=name)


def space_definitions():
    """Supply the shared sky, planet-lighting gradients, and globe clip."""
    sky = tag('linearGradient',''.join(tag('stop',stop_color=SKY[k],offset=o) for k,o in [('top',0),('middle',.6),('bottom',1)]),id='space',x2='.8',y2='1')
    planet = tag('linearGradient',''.join(tag('stop',stop_color=SATURN[k],offset=o) for k,o in [('light',0),('middle',.58),('shadow',1)]),id='planet',x2='.9',y2='.6')
    return tag('defs',sky+planet+tag('clipPath',tag('circle',r=310),id='globe'))


def render_scene(scene):
    """Wrap scene art and annotate face contours for reader diagnostics."""
    if not all(isinstance(n,(int,float)) and not isinstance(n,bool) and isfinite(n) and 0<n<=10000 for n in (scene.width,scene.height)):
        raise ValueError('Scene dimensions must be positive, finite, and bounded')
    if len(scene.art)>2_000_000 or re.search(r'<!DOCTYPE|<!ENTITY',scene.art,re.I):
        raise ValueError('Invalid comic scene document')
    ET.register_namespace('','http://www.w3.org/2000/svg')
    root = ET.fromstring(f'<svg xmlns="http://www.w3.org/2000/svg" width="{scene.width}" height="{scene.height}" viewBox="0 0 {scene.width} {scene.height}">{space_definitions()}{scene.art}</svg>')
    tags = {'svg','title','desc','defs','g','path','rect','ellipse','circle','polygon','polyline','line','text','linearGradient','radialGradient','stop','clipPath'}
    ids = set()
    slots = set()
    for element in root.iter():
        if not element.tag.startswith('{http://www.w3.org/2000/svg}') or element.tag.split('}')[-1] not in tags:
            raise ValueError('Unsafe comic SVG element')
        identifier = element.get('id')
        if identifier:
            if identifier in ids:
                raise ValueError('Duplicate comic SVG id')
            ids.add(identifier)
        slot = element.get('data-story-slot')
        if slot:
            if slot in slots:
                raise ValueError('Duplicate story text slot')
            slots.add(slot)
        for key,value in element.attrib.items():
            key = key.split('}')[-1].lower()
            if key.startswith('on') or key in ('href','style') or ('url(' in value.lower() and not re.fullmatch(r'url\(#[a-zA-Z0-9_-]+\)',value)):
                raise ValueError('Unsafe comic SVG attribute')
    for face in root.iter():
        name = face.get('data-face')
        if name:
            contour = next(child for child in face if child.get('d')==FACES[name].head)
            contour.set('data-protect-face',name)
    return ET.tostring(root,encoding='unicode')+'\n'
