# Refusing a crowded cell is wrong, and nothing had to arbitrate

## The answer

**No. A crowded cell must be clad.** The refusal guarded nothing.

`stands` asked three things of a direction: something to bolt to, no structure
ahead, and NOTHING ALREADY CLAD ahead. The third is the one that dropped the
owner's corner, and it protects against a collision that cannot happen.

A plate is bounded by its own cell (`plate_collider` cuts a box inside the unit
footprint) and it bolts to STRUCTURE. So for a cell `A` facing `d` whose cell
ahead `B = A + d` carries a plate facing `e`:

- `e == d` is impossible. `B`'s bolt face would be `A`, and `A` is empty.
- `e == d ^ 1` is a SLOT two cells wide. Each plate hugs its own wall and the
  gap between them is the slot.
- `e` perpendicular is a CONCAVE CORNER. Each plate hugs a different wall of it.

In all three the two plates stand in different cells, on different structure,
showing different directions. There is no pair to arbitrate between, so the
question the rule was asking has no answer to give. Pinned in
`a_plate_and_the_plate_it_faces_never_stand_on_the_same_wall`.

Removing the clause also removes the fixpoint: `stands` no longer reads the skin
being built, so `cladding_cells` drops from "claim, then drop repeatedly until
nothing moves" to one pass. The `touches_socket` test went with it - `stands`
already requires a socket offered into the cell, so it was the same claim twice.

## What happens to the owner's L

It is CLAD, and the corner plate is a fillet the alphabet already has.

The inside angle `(1,1,0)` bolts down onto the run and rises to the WHOLE CELL
along the edge where it closes against the upright, dying to the floor away from
it: `shell_0044_2242`, 0.5625 of a cell. Both hull faces that looked into the
angle are covered - the L now has zero hull faces looking at vacuum, checked
exhaustively over all five cells in
`the_inside_angle_of_an_l_is_clad_and_closes_against_the_upright`.

Three cells thick, the same thing happens once per slice (`shell_0442_2431`,
`shell_2244_2343`, `shell_0244_1342`), each bolted to the run under it.

**No new shape is needed.** This is worth stating plainly for the shape task
(20260816-112429): the inside corner is a corner sample at `FULL` on the two
slots against the wall and `0` on the two away from it, which is an ordinary
member of the vocabulary and comes out of `boundary_heights` unchanged. The
`walls()` test already reads the upright as something to climb.

## The 12 crowded faces on the generated row

All twelve are CLAD. `wfc_ships`, seeds 20260815..20260817, industrial, frame
35, read back out of `NOVA_PERF_SNAPSHOT`:

| | plates | shapes | coplanar | mean flat area | bare hull faces |
| --- | --- | --- | --- | --- | --- |
| before | 526 | 9 / 15 / 18 | 308 | 0.790 / 0.725 / 0.581 | 142 |
| after | 538 | 7 / 15 / 12 | 344 | 0.799 / 0.734 / 0.674 | 118 |

The 24 bare faces are exactly the crowded ones. **`fires_into` (56 cells, 60
faces) and `no_socket` (48 cells, 58 faces) are unchanged**, and nothing became
newly bare. The twelve cells are six mirrored pairs on one hull (WFC 20260817).

The other numbers moved because of the second half of the same clause: `stands`
is also what `plate_for` filters its candidate outs with, so 26 plates across
the three hulls had a direction hidden from them and were forced to face a
shallower one. They now go to the deepest structure they can reach, which is
what `plate_for` says it does. The row lost 8 distinct shapes (42 -> 34 meshes)
and gained 36 flat-topped plates for it.

Determinism holds: `NOVA_PERF_SNAPSHOT_FRAMES=35,35` gives two byte-identical
captures of the frozen frame.

## Two behaviours that follow, both wanted

- **A slot two cells wide is plated on both walls.** Each side could only ever
  show the other, so the pair used to knock each other out in the same drop pass
  and both walls came back bare. Pinned in
  `a_slot_two_cells_wide_is_plated_on_both_of_its_walls`.
- **A sealed pocket two cells long fills with two full-cell plates.** Invisible
  mass, but consistent: a sealed void THREE cells wide was already clad before
  this change, because its middle cell touches no structure and so never blocked
  anything. Zero occurrences on the generated row. Refusing enclosed cells would
  want a flood fill, which is a different rule and not this defect.

## `BareReason::Crowded` is gone

Not deprecated - deleted. A cell that touches a socket and is not claimed now
has structure on the far side of every direction it could bolt through, which is
`NoFooting` and nothing else. The dump's reason vocabulary is `no_socket`,
`fires_into`, `no_footing`, `no_shape`.

## Verified

- `cargo check --workspace --all-targets`, `cargo fmt --check`.
- `cargo test --lib -p nova_ship` (615), `-p nova_editor` (66), `-p nova_probe`.
- Live `wfc_ships` render, before and after, on the hull that changed: the
  gouge on its starboard quarter closes and nothing else moves.
- `camera::handback::tests::handback_blends_the_anchor_instead_of_snapping`
  fails under full-suite load and passes in isolation on master and on this
  branch. Unrelated, and not introduced here.
