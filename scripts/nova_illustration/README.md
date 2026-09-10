# Reusable illustration sources

This standard-library Python package keeps a drawing's identity separate from
its scene composition and color presentation. It is not a game asset pipeline.

The [comic authoring workflow](../../web/src/comics/README.md) uses the real website
reader. This package supplies scene geometry and colors; the reader owns
measured dialogue layout and its matching SVG export.

## Owners

| Module | Owns |
| --- | --- |
| `colors.py` | Named palette constants, character colors, material colors, and lore channel settings. |
| `styles.py` | `comic` and `lore` presentation. Color changes do not change geometry. |
| `ships.py` | Kaveri, Ebro, and Gantry models, named views, a Gantry condition, and shared prop/cargo projection. |
| `faces.py` | Nine forward-facing identities, expression selection, and original features outside the Jonah/Rina variants. |
| `expressions.py` | Original and variant facial features for Jonah and Rina, with character-specific linework. |
| `portraits.py` | Close/conversational poses, work bodies and arms, Gantry's civilian bodies, and a supported Owen pose. |
| `scenery.py` | Provisional Baikal/Aquila silhouettes, a local transfer-hall wall, Saturn, and a repeatable star field. |
| `svg.py` | Shared SVG primitives in illustration coordinates. |
| `lettering.py` | Speech balloons built from authored lines, separate from colored artwork. |

Add colors in `colors.py`, not as local hex literals in drawing functions.
Elena's poses share one palette; the ship models share material roles. These
colors remain art choices, not approved skin tones or company livery.

The original five encyclopedia portraits retain their template in
`../gen-lore-portraits.py`. It imports all five crew heads from `faces.py`,
including Samir's original curls, glasses, and features. All five rendered
assets remain byte-for-byte unchanged. The legacy template colors have not
been migrated. This package does not claim a complete cast library.

The current comic treatment uses the original frontal heads, not profiles or
three-quarter faces. Body poses, necks, clothing folds, and local framing can
change without replacing a character's face. New head angles require separately
authored drawings and visual approval; a flat head cannot rotate automatically.

## Facial expressions

```python
from nova_illustration.faces import expression_names, frontal_head
from nova_illustration.portraits import jonah_listener, work_portrait

assert expression_names('rina') == ('original', 'amused')
rina = work_portrait('rina', expression='amused')
jonah = jonah_listener(expression='wry')
head_only = frontal_head('jonah', expression='wry')
```

Head-bearing portrait helpers accept `expression`, which defaults to `original`. This
means the retained drawing, not a universal neutral mood. Omitting it keeps
existing default renders byte-for-byte unchanged. SVG heads mark only an
explicit variant with `data-expression`.

Rina currently has `original` and `amused`; Jonah has `original` and `wry`.
Elena, Leila, Samir, Tomas, Nadia, Owen, and Ivo currently have only `original`.
Unknown heads or expression names raise `KeyError`, including a known expression
not authored for that character.

Variants replace brows, eyes, mouth, and nearby acting lines. They reuse the
same nose, head contour, hair, ears, shadow planes, and body pose. Author each
character's features in `expressions.py`; do not paste a generic smile over the
old mouth or distort the entire head. These are selected drawings, not a face
rig, automatic head rotation, or a complete emotion catalog. The original
Jonah/Rina features also live there, so public portraits and scenes share them.

Use `present` on the resulting artwork for comic or lore colors. Expression
selection does not change the palette or include lettering.

## Inspection staging

`work_portrait` accepts Leila, Rina, Samir, and Tomas. `work_inspection` accepts
the same names and expression argument. It returns `(body, forearms)` so the
scene can put its console or machinery between those layers. The body retains
the shared frontal head; the hands contain no face, tool, lettering, or filter.
Apply the same placement to both layers. Comic and lore presentation remain
color-only. No new head angle or expression is implied by a working pose.

`gantry_portrait(name, expression='original')` accepts `nadia`, `owen`, or `ivo`.
Each has a separately authored civilian body and one registered frontal head.
These starting identities have no injury variant or military rank marking.
The same head/body drawing supplies comic scenes and the public lore portrait.

`gripping_arm(name)` and `reaching_arm(name)` are separate forearm/hand layers.
They contain no head, expression, or fixture. The closed grip meets a scene-owned
rail at `(5, 425)` in the work-bust drawing frame; place that rail above the body
and below the hand. The open reaching pose signals movement without rotating a
flat likeness. These drawing coordinates are not physical measurements.

`supported_owen()` keeps his original frontal head on a foreshortened reclining
body, viewed from the foot end. The scene owns padding, body straps, receiving
restraints, and any continuing care. The pose selects no injury or treatment.
`freefall_guide(name)` extends Rina's or Ivo's existing work body below the bust;
use an actual foreground boundary when a distant person's lower body is hidden.

`stretcher_grip(name)` reaches from the work-body shoulder to a scene rail at
`(430, 433)`. `rail_grip_hand(name)` supplies only a closed hand at the local
origin for a cropped foreground arm. Mirror an arm or hand layer, never the
frontal head. Place the frame before the gripping layer and use the same anchor
for both. These are illustration coordinates, not physical dimensions. The
existing public portraits and earlier pose functions remain unchanged.

## Shared assets versus local scene work

Keep recurring characters, named ships, and recurring locations in the library.
Scene generators own panel artwork, one-off background details, and incidental
props. The [comic TS page language](../../web/src/comics/README.md) owns story,
dialogue, screen text, and page composition. A mug and its hand belong with the
episode's `art/` sources, not in the portrait module. Drawing an object twice
within one conversation does not give it a series-wide identity.

Add a shared props module only when an established recurring prop needs a
consistent design across independent scenes. Keep that module separate from
characters, ships, and scenery. Do not preemptively catalog every drawn object.
Local drawings still use `colors.py` and the common SVG primitives.

`scenes.py` supplies `Scene(width, height, art)` and blank `text_slot(name)`
insertion points. The slot preserves painter order but owns no words or text
styling. `render_scene` supplies shared sky definitions and face-contour markers,
and rejects unsafe SVG before raw scene assets can be opened. It adds no page
frame, dialogue, or transcript. Episode art modules register named drawing
functions in `SCENES`; the shared web builder calls only referenced scenes.

## Reuse

With `scripts/` on the Python import path:

```python
from nova_illustration.ships import render_ship
from nova_illustration.styles import present

hull = render_ship('kaveri', 'aft-quarter', 400, 300, 0.8, False)
comic_art = present(hull, 'comic', 'kaveri-scene')
lore_art = present(hull, 'lore', 'kaveri-sheet')
```

Each `present` call needs a unique SVG-safe instance key. Keep captions, labels,
and lettering outside this group. The lore scheme applies the encyclopedia's
luminance-to-green channel treatment to a child SVG group, not the root SVG.
The same filter definition can drive an interactive preview. Geometry and
underlying full-color values do not change.

`render_ship` accepts the registered names and views in `MODELS` and `VIEWS`.
Its first six arguments are ship name, view, drawing x/y placement, drawing
scale, and an explicit thrust-visual flag. Optional `state='intact'` preserves
the original exports; `bounds` accepts the same state for layout. `project`
places annotations with the same camera as the hull.

`render_faces(surfaces, view, x, y, scale)` projects an authored prop with the
same materials, flat lighting, and plane-partition occlusion. Optional keyword
`cargo=()` on `render_ship` means no load by default. Supplied faces use the
ship's drawing coordinates and join its painter pass. The scene owns the prop
and placement; this is not inventory, capacity validation, or a cargo manifest.
Existing no-cargo renders stay exact.

Only Gantry has an authored `stranded` condition. `ship_faces` derives it from
the intact hull by replacing two service-cover components. The crew module,
cargo body, handling frame, and transfer collar retain their geometry. This is
an art condition, not procedural damage or a physical fault simulation.
Unknown ship/state pairs raise `KeyError`; requesting main-drive thrust for the
stranded condition raises `ValueError`. Plumes follow the ship's authored bell
layout. The old Kaveri/Ebro renders remain unchanged.

Public lore exports always select intact starting identities. A future damage
view belongs to an unpublished story study until its relevant installment is
released; a reusable renderer does not authorize its publication.

Ship coordinates are unscaled drawing proportions. They are not meters,
build-grid cells, or engine coordinates. The external-form proposals have no
approved dimensions. Never print these coordinates as fictional measurements
or copy them into game content without a separate physical design decision.

The hulls use faceted plating, sloping crew brows, segmented engine cowls, and
selected industrial fixtures. The game screenshots and greeble recipes inform
their visual language, not their dimensions or a mapping to game ship content.

The projector culls back faces and uses plane partitions to order overlapping
surfaces. It splits polygons where needed, but inks only their original edges.
This keeps a broad plate behind its hatches and hazard bands. Each projection is
exported at the precision its placement can show: `draw_precision(scale)` gives
the decimals a placement earns, no exported point moves further than
`DRAW_TOLERANCE` of a drawn unit, and a surface that this quantisation collapses
is dropped instead of inked as a sliver. A ship drawn small therefore costs a
small export, and rescaling an existing drawing changes its exported bytes. It
remains a small flat-surface illustration renderer, not a CAD kernel. Inspect new forms and
views for overlaps. It does not provide construction tolerances,
interior plans, rigging, simulation behavior, or automatic damage states.
Baikal and Aquila are reusable two-dimensional silhouettes, not multi-view
solids. Aquila retains its existing two residential rings and broad freight
spine. `transfer_hall(w, h)` draws the wall of a pressurized, non-rotating work
area. The scene supplies windows onto exterior berths, handholds, and a closed
freight lock. This local freefall staging does not establish a station plan.

`labelled_speech` adds speaker labels and authored `top`, `bottom`, `left`, or
`right` tails. It retains literal text separately from art. Inspect wrapping and
face clearance in the rendered scene; a bounded tail does not validate layout.

## Lore exports

From the repository root:

```bash
python3 scripts/gen-lore-designs.py
python3 scripts/gen-lore-designs.py --check
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/nova_illustration -p 'test_*.py'
```

The exporter owns these files in `web/src/assets/lore/`:

- `kaveri-design-concept.svg`
- `ebro-design-concept.svg`
- `gantry-design-concept.svg`
- `elena-ward-portrait-concept.svg`
- `nadia-sen-portrait-concept.svg`
- `owen-park-portrait-concept.svg`
- `ivo-marin-portrait-concept.svg`

Ship sheets use plan, side, forward, and aft three-quarter views of the same
model. The three orthographic views share a drawing scale within each sheet;
the three-quarter view is fitted separately. They are shape proposals, not
engineering drawings. Elena's lore export uses the same close-up drawing that
scenes can render in full color. Gantry's three portraits use their shared
frontal heads and separate civilian bodies, with no later injuries.

Change the sources and regenerate. Commit source and outputs together. Public
exports import this library only, never production-task files. The package
itself performs no file writes on import.
