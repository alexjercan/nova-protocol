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

`placeholder_*.glb` are PLACEHOLDERS in garish magenta, shipped only to prove
the pipeline runs end to end. They make no art decision and the authored kit
replaces them (task `20260815-225748`, Phase B).

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
