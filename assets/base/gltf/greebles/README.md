# Greebles

A greeble is a small decorative fixture bolted ON a hull plate - a vent, a rib
run, a blister, a mast. The base mod GENERATES its own: every `.glb` here is
build output written by `scripts/gen-greebles.py` from a committed JSON recipe
in `scripts/greeble-recipes/`, and the recipe is the source you edit. Nothing
in this folder is hand-modelled, so authoring a new piece is writing a recipe.

```sh
python3 scripts/gen-greebles.py             # write every .glb
python3 scripts/gen-greebles.py --check     # verify, write nothing
python3 scripts/gen-greebles.py --self-test # internal checks, no I/O
```

The build is deterministic: the same recipes always produce the same bytes, so
`--check` fails on a stale commit instead of letting generated art churn in an
unrelated diff.

## What is here now

Four authored KITS and the scaffolding. Each `<kit>_` prefix is worn by the
style of that name.

`armoured_*.glb` - a belt (`strake`) that runs the length of a hull's straight
edges, a corner boss (`cap`), a flush access hatch, a faceted sensor blister,
and the vocabulary batch: a stub raked mast, a shuttered intake, a ready
magazine, a chaff tube, an applique tile grid and a white rounds-count tally
(`ammo_stripes`). Ten pieces and still the smallest kit BY DOCTRINE: everything
low, flush and bolted, gunmetal plus ONE stencil white, nothing lit.

`civilian_*.glb` - the racer's look, a ship built to be sold: a livery rail
(`stripe`), a cabin window row (`windows`), a raked aero fin (`fin`), a smooth
fairing over whatever a working ship would leave exposed (`fairing`), and a nav
beacon (`beacon`). Two accent colours across the whole kit and nothing else:
cobalt for paint, amber for anything lit.

`industrial_*.glb` - seven pieces a fitter would have a part number for: a
radiator bank, a conduit run, a corrugated panel, a louvred grille, an access
hatch, a heat stack and a painted hazard band.

`salvage_*.glb` - mismatched patches in three materials, a hand-run weld bead, a
lashed drum, a kinked whip antenna and a tow cleat. Seven pieces, and the cap is
deliberate: the look comes from where they land and what they are made of, never
from how many of them there are.

`placeholder_*.glb` are PLACEHOLDERS in garish magenta, shipped only to prove
the pipeline runs end to end. They make no art decision (task
`20260815-225748`, Phase A).

Two recipe habits the kits share, and both are answers to the same constraint -
the scatter stands a piece at the CENTRE of its plate and offers no jitter:

- A piece meant to draw a LINE raises its own footprint budget toward a whole
  cell (`armoured_strake`, `civilian_stripe`, the industrial band and pipe, the
  salvage patches), so consecutive plates of one run butt together instead of
  photographing as a dashed row. A full-cell piece still cannot spill onto a
  neighbour, because it is centred.
- A piece meant to read as UNPLANNED is authored off-centre in its own footprint
  (the whole salvage kit), because a centred piece repeated is a tile and there
  is no jitter to break it.

## The frame a greeble is authored in

A hull plate is one cell - the unit cube, out along `+Y` (see
`crates/nova_ship/src/sections/shell_shape.rs`). A greeble uses that same
frame:

- `+Y` is out of the plate and `y = 0` is the mounting face. Nothing sits
  behind it, so placing a piece is a translate to the plate face plus the
  rotation that takes `+Y` to the plate normal.
- The footprint is centred on the origin and stays inside half a cell by
  default, so a piece scattered off the centre of a plate cannot spill across
  the seam onto its neighbour. A recipe raises its own budget when it needs to
  (a mast is tall).
- Flat-shaded, low-poly, untextured, one primitive per flat colour - the same
  look as the ship parts next door, which run 52-251 triangles each. A piece
  over 200 triangles is refused: decoration is scattered many times over a
  hull.

## Shipping and referencing

These files are declared in the base bundle's `resources`
(`assets/base/base.bundle.ron`), so base content references one as
`self://gltf/greebles/<id>.glb` and a mod references one as
`dep://base/gltf/greebles/<id>.glb` - the same contract every other base asset
has. A mod can ship its own greeble `.glb` (generated however it likes) and
reference it exactly the same way.
