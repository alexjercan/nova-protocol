# Reusable illustration sources

This standard-library Python package keeps a drawing's identity separate from
its scene composition and color presentation. It is not a game asset pipeline.

## Owners

| Module | Owns |
| --- | --- |
| `colors.py` | Named palette constants, character colors, material colors, and lore channel settings. |
| `styles.py` | `comic` and `lore` presentation. Color changes do not change geometry. |
| `ships.py` | One plated shape proposal each for `kaveri` and `ebro`, with industrial fixtures and named projected views. |
| `faces.py` | Original forward-facing head contours, hair, and features for Jonah and Elena. |
| `portraits.py` | Close and conversational body poses, using the shared frontal heads. |
| `scenery.py` | The provisional Baikal silhouette, Saturn, and a repeatable star field. |
| `svg.py` | Shared SVG primitives in illustration coordinates. |
| `lettering.py` | Speech balloons built from authored lines, separate from colored artwork. |

Add colors in `colors.py`, not as local hex literals in drawing functions.
Elena's poses share one palette; the ship models share material roles. These
colors remain art choices, not approved skin tones or company livery.

The original five encyclopedia portraits retain their template in
`../gen-lore-portraits.py`. That exporter now imports Jonah's face from
`faces.py`; its five rendered assets remain byte-for-byte unchanged. The other
four heads and legacy template colors have not been migrated. This package does
not claim a complete cast library.

The current comic treatment uses the original frontal heads, not profiles or
three-quarter faces. Body poses, necks, clothing folds, and local framing can
change without replacing a character's face. New head angles require separately
authored drawings and visual approval; a flat head cannot rotate automatically.

## Shared assets versus local scene work

Keep recurring characters, named ships, and recurring locations in the library.
Scene generators own framing, dialogue, one-off background details, and incidental
props. A mug and the hand holding it belong with their scene, not in the portrait
module. Drawing an object twice within one conversation does not give it a
series-wide identity.

Add a shared props module only when an established recurring prop needs a
consistent design across independent scenes. Keep that module separate from
characters, ships, and scenery. Do not preemptively catalog every drawn object.
Local drawings still use `colors.py` and the common SVG primitives.

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
The six arguments are ship name, view, drawing x/y placement, drawing scale,
and an explicit thrust-visual flag. An unknown name or view is an error.
`bounds` measures the projected illustration for layout; `project` places
annotations using the same camera as the hull.

Ship coordinates are unscaled drawing proportions. They are not meters,
build-grid cells, or engine coordinates. The external-form proposals have no
approved dimensions. Never print these coordinates as fictional measurements
or copy them into game content without a separate physical design decision.

The hulls use faceted plating, sloping crew brows, segmented engine cowls, and
selected industrial fixtures. The game screenshots and greeble recipes inform
their visual language, not their dimensions or a mapping to game ship content.

The projector culls back faces and uses plane partitions to order overlapping
surfaces. It splits polygons where needed, but inks only their original edges.
This keeps a broad plate behind its hatches and hazard bands. It remains a small
flat-surface illustration renderer, not a CAD kernel. Inspect new forms and
views for overlaps. It does not provide construction tolerances,
interior plans, rigging, simulation behavior, or automatic damage states.
Baikal is still a reusable two-dimensional silhouette, not a multi-view solid.

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
- `elena-ward-portrait-concept.svg`

Ship sheets use plan, side, forward, and aft three-quarter views of the same
model. The three orthographic views share a drawing scale within each sheet;
the three-quarter view is fitted separately. They are shape proposals, not
engineering drawings. Elena's lore export uses the same close-up drawing that
scenes can render in full color.

Change the sources and regenerate. Commit source and outputs together. Public
exports import this library only, never production-task files. The package
itself performs no file writes on import.
