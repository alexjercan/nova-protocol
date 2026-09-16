# Phase 1 spike: docking port mesh candidates

Three candidate ports, the generator checks that grade them, and the gallery
example that renders the poses the owner judges. Phase 2 (the `Docking` section
kind, joints, the `DOCK` command) is not in this spike.

## The candidates

| id | voice | tris | file |
| --- | --- | --- | --- |
| `dock_collar` | plain industrial: proud collar, bolted flange, steel sleeve | 396 | `scripts/section-part-recipes/dock_collar.json` |
| `dock_bellows` | heavy hard-dock: armour ring, corner clamps, two-stage sleeve | 444 | `scripts/section-part-recipes/dock_bellows.json` |
| `dock_flush` | sealed hatch: nothing proud of the cell, graphite sleeve | 392 | `scripts/section-part-recipes/dock_flush.json` |

Built the same way as the hull, railgun and PDC parts: a JSON recipe through
`scripts/gen-section-parts.py` to a deterministic `.glb` under
`art/part-candidates/sections/`. All three sit under the 450 triangle per cell
budget. `--check` reported 28 parts byte for byte at selection time; it
reports 26 now that the two rejected recipes are gone.

## The one new primitive

A docking port is a tube another tube slides INSIDE. `cylinder`, `taper` and
`disc` all cap solid, so a nested part hides behind a lid. `sleeve` in
`scripts/gen-greebles.py` is an open-ended annular tube: annulus rims instead
of discs, a real bore, and an optional `radius_top`/`bore_top` taper.

## Geometry contract, as checked

`_check_dock` in `scripts/gen-section-parts.py` fails the build unless:

- every moving node is named `dock_tube*` and carries no rotation, so the
  runtime track can stay a plain `Translate`;
- retracted `lo[2] == -0.5` - the port face IS the cell face;
- extended `lo[2] == -1.0` - travel is exactly `0.5`, never fitted to a gap;
- extended `hi[2]` equals retracted `hi[2]` - the inner end is anchored;
- the extended pose still fits the cell in x and y.

Generator output: `size 0.96-0.98 x 0.91-0.98 x 0.950, z -0.500..0.450`
retracted; the gallery logs `depth 0.95` retracted and `depth 1.45` extended.

## The seam finding

Two identical mirrored profiles facing each other always cross somewhere. At
symmetric overlap the crossing sits at the midpoint. A seam is therefore
unavoidable by construction, not a defect of any one candidate.

What the candidates do about it is make the crossing read as a seal joint: a
single monotone, near-uniform taper (0.325 mouth to 0.345 root) puts the
crossing on ONE clean circle instead of a chewed band. Below a `0.5` face gap
each mouth enters the other's static barrel, so every candidate carries a dark
throat and a barrel bore (0.355, or 0.39 on the bellows) wide enough to swallow
a mating sleeve.

`dock_bellows` is the exception worth looking at: its two-stage sleeve reads as
a visible waist at gap `1.0`. That is the cost of the stage step, and it is the
thing to accept or reject deliberately.

## The gallery

`screenshot_docking_gallery` under `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`. Five
rows, three columns: retracted, extended, and mated with a copy of itself at
face gaps `1.0`, `0.5` (worst case, sleeves fully overlap) and `0.1`. The wire
box is the 1x1x1 cell. That example is GONE - Phase 2 removed it with the
rejected candidates (see below); the frames it shot are the six `.png` files
kept beside this file.

- `docking-gallery.png` - all five rows
- `docking-gallery-retracted.png`, `-extended.png`
- `docking-gallery-mated-10.png`, `-mated-05.png`, `-mated-01.png`

`examples/screenshots/shared/glb.rs` gained `read_glb_posed`, which displaces a
named node exactly as `SectionAnimationMotion::Translate` composes it at
runtime (`rest.translation + rest.rotation * offset`), so the extended pose in
these shots is the pose the game will play.

## Selected: `dock_flush`

Owner selection on 2026-09-15, off the frames listed above. The sealed-hatch
port: nothing proud of the cell retracted, a graphite sleeve that is the only
thing that leaves the cell.

`dock_collar` and `dock_bellows` were kept for the selection so the gallery
still showed what the choice was made against.

## Phase 2 cleanup, 2026-09-16

Phase 1 step 6, done with the implementation:

- `scripts/section-part-recipes/dock_collar.json` and `dock_bellows.json` are
  deleted, and with them the two candidate `.glb` files under
  `art/part-candidates/sections/`.
- `dock_flush` is PROMOTED: it joins `PROMOTED_STEMS` in
  `scripts/gen-section-parts.py`, so the recipe stays the source and
  `assets/base/gltf/dock_flush.glb` is its committed build. It is declared in
  `assets/base/base.bundle.ron`, which content lint requires.
- `examples/screenshots/screenshot_docking_gallery.rs` is deleted, with its
  `Cargo.toml` block and its CI shard entry. The selected port is shown by the
  existing all-sections gallery instead: `screenshot_section_gallery` gained a
  `docking` row carrying `dock_flush` retracted and extended, posed through
  `read_glb_posed` exactly as the runtime track composes it.
- The candidate frames above stay here as the record of the choice.

No change requests came with the selection.
