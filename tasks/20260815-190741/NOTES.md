# Notes

Working record for the derived ship skin. Everything the prototype learned,
including the dead ends, so the reimplementation does not rediscover them.

## Where the work lives

| Thing | Value |
| --- | --- |
| Branch | `wfc-shells` |
| Worktree | `/home/alex/.cache/sprouts/nova-protocol/wfc-shells` |
| Task association | `git config --worktree sprout.task 20260815-190741` |
| Sync master in | `sprout sync wfc-shells` |
| Land as one commit | `sprout land wfc-shells -m "<subject>"` |

The branch lands on master as a single squash commit. Nothing on it needs
backward compatibility with anything else on it. Owner directive, verbatim:
"if it breaks the previous version GOOD it means we make progress".

## Prototype commits

Oldest first. Read as a record of how the design moved, not as history worth
keeping.

- `439c710a` grow random ships by wave function collapse over link points
- `080e3b68` clad the collapsed ships in a second pass of hull shells
- `f52ba753` cut the hull shells from a cube's midpoints
- `72409cda` bolt the drive on by one face, close the steps in the skin
- `0d6fd74b` put the shells in the editor palette
- `c7c11967` catch a socket pressed into a body with nothing to mate
- `33fe40de` rebuild the shells as a corner height field
- `077d1842` tidy after the corner-height rebuild
- `892a744d` sample the boundary at its edges as well as its corners
- `25213efd` give five more shell codes a reading
- `775d5926` give every shell only the sockets its own surface can back
- `a0e5f3fe` make the eight boundary samples the whole of the shape
- `9a3bdc60` cut a shell into one mesh per material, not vertex colours
- `bc75a6c5` build the cladding shells instead of shipping them
- `319f5a57` stop collapsing the skin, and let blind faces touch
- `b21c02e5` derive a ship's skin from the structure it wraps
- `6e368af8` let the corners decide a clad edge, and spawn the derived skin

## The shape system

### Eight boundary samples

A cladding tile is described by 4 corner heights plus 4 edge-midpoint heights.
Id spells the samples: `shell_<CCCC>_<MMMM>`.

A corner belongs to the 4 cells meeting at it. A midpoint belongs to the 2 cells
across it. Neighbours therefore compute IDENTICAL shared samples, so their
surfaces meet exactly. No seam geometry is generated and none is needed. This is
the single property the whole design rests on.

### Rotation only, never reflection

Canonicalisation is under C4, not D4. A section is placed with a `Quat`, and no
rotation produces a mirror image. Folding reflections gave 954 classes of which
711 could not be placed. Rotation-only gives 1665 from 6561 raw at the
three-height alphabet.

Do not "optimise" this back to D4. It was tried and it was wrong.

### Quarter cells

Three heights cannot express a half-step edge. Its true midpoint is a quarter
cell, so with integer means a `0 -> half` edge sags to the floor and every rim
tile grows a facet. The render made this obvious; `cargo check` and 18 passing
tests did not.

The alphabet moves to 0..4 in quarter cells. The corner rule still emits only
0, 2 and 4, and midpoints become exact means, so every live edge is straight:

```
(0,4) -> 2    (0,2) -> 1    (2,4) -> 3    (2,2) -> 2
```

Affordable only because nothing enumerates the roster any more. At five values
the raw space is 5^8 = 390625 and the rotation classes are ~97k, which is
irrelevant when a whole ship touches a couple of dozen shapes and each mesh is
built on demand.

### Sockets

Bolt face (`negative_y`) always. A side is socketed exactly when that side's
midpoint sample is at least half, because the crest at the middle of a face IS
its midpoint sample. Verified against the actual triangles, not asserted: a test
parses the generated mesh and checks material presence at each socket point, and
eight mutations of the rule fail it.

### Mating

`compatible` is "a socket may never press into a face that has none":

```rust
here.faces[face] == there.faces[face ^ 1]
```

It deliberately does NOT model clearance. A muzzle or nozzle needing empty space
in front is content's business; the content already knows where plumes point.

Blind faces MAY touch blind faces. Two thrusters side by side is legal. An
earlier version restricted the `(false, false)` case to shells and cut thruster
placements from 46 per row to 16.

## Why this stopped being WFC

The skin is a LOOKUP, not a constraint solve. Every cell's shape follows from
its neighbourhood, deterministically, in one pass. That removes:

- the tileset totality risk, which the research named as the single biggest
  danger in the constraint-solving approach;
- the minimal-perturbation problem, since there is nothing to perturb;
- any need for a universal filler tile.

This mirrors Townscaper's driver-layer/generated-layer split, which Boris the
Brave recommends for exactly this reason: the player edits the driver layer, and
the generated layer is a pure function recomputed from it.

## Research already done

Do not re-run these. Findings only.

### Constraint solving

- Merrell model synthesis, "modifying in blocks".
- Dynamic arc consistency (DnAC-4, DnAC-6, AC|DC-2i) is NOT needed. Retraction
  here is LIFO depth-1, so snapshot and restore is enough.
- Verfaillie and Schiex on minimal perturbation: the cheap value-ordering trick
  beats sophisticated solvers.
- Townscaper uses a strict global priority order, not a seeded RNG.
- Arc consistency is confluent, so a propagate-only preview cannot flicker
  (miWFC). Relevant if interactive adaptation is ever built.
- Nobody ghosts. Townscaper commits instantly and fakes the morph with two
  shader globals.

### Decoration continuity

- Our eight shared samples ARE the corner-tile system (Lagae and Dutre 2006),
  with Barrett's mid-edge fix already built in.
- Canonicalise decoration by EQUALITY PATTERN, not by value: 625 collapses
  to 15.
- Townscaper REMOVED decoration from tiles. It stencil-cuts windows and scatters
  props by reading the neighbourhood.
- Corner matching degrades straight features. Panel lines want trim or decals,
  not corner-matched texture.
- Winged tiles (Carlson). Neyret and Cani zero-gradient-at-vertex.
- Hardspace: Shipbreaker made panel lines BE the module boundaries, which is the
  cheapest honest answer.

### Interactive adaptation

Owner's stated long-term vision: hover a thruster in the editor and watch the
ship reshape to accept it. Out of scope here, but the driver-layer split is what
makes it reachable later, and confluence means the preview is stable.

## Rendering constraints found the hard way

- CORRECTED. This entry used to claim that bevy's PBR fragment ASSIGNS
  `base_color` from vertex colour and therefore vertex colours would clobber
  `damage_tint`. That is WRONG, and one mesh per surface role was chosen partly
  on the strength of it. `pbr_fragment.wgsl` does both:

  ```wgsl
  pbr_input.material.base_color = in.color;      // line 55  - assign
  pbr_input.material.base_color *= base_color;   // line 101 - multiply by the uniform
  ```

  They COMPOSE. `damage_tint` writes the uniform, so a vertex-coloured mesh
  still grades. Two consequences worth acting on: a plate's surface roles could
  collapse into ONE mesh with vertex colours and a single shared material
  (roughly 1200 mesh entities per clad ship down to 400), and it is safe
  regardless because fixtures are exempt from tinting anyway. It cannot go
  further than per-plate - plates are individually destructible, so each needs
  its own entity.

  Related, from the market-research survey: WebGL2 has no `BASE_VERTEX`, so
  distinct meshes NEVER share a batch set. Fewer, larger meshes matter more on
  web than native.
- A child carrying meshes needs its parent to carry `Visibility`, or bevy warns
  and drops them.
- `unwrap_or` evaluates eagerly. Using it in the surface accumulator pushed a
  new mesh slot per facet: 16 slots instead of 3. Use `match`.

## Mesh builder specifics

`MeshFaces` dedups repeated vertices and CHECKS winding against the desired
outward normal rather than trusting vertex order.

Walls split at the midpoint, because a V-notch wall is not convex and a naive
fan papers straight across the dip. The floor fans over the same 8 footprint
points to avoid T-junctions with the walls.

Caught by the closed-solid test, which runs over all shapes.

## Bugs hit, and what they actually were

1. "Cell 0 collapsed to nothing." A shell picked an `out` with only cladding
   beneath it. Fixed by `stands()`.
2. Collapse still failing: `compatible` demanded BOTH sides socket. Wrong for
   cladding. Became "a socket may never press into a face that has none".
3. `refuse_unmated_contacts` was stricter than the rule it guarded. Added an
   `offers()` exemption for pairs where neither side has a socket.
4. Blind faces could not touch. Owner corrected this. See Mating above.
5. `keel_component` walked plain adjacency, which no longer implies mating, so
   ships came apart in the link-point graph. Now checks both sides' faces.
6. D4 canonicalisation produced 711 unplaceable classes. See Rotation only.
7. V-notch walls are not convex. See Mesh builder.
8. `unwrap_or` eager evaluation. See Rendering constraints.
9. Vertex colours vs `damage_tint`. See Rendering constraints.
10. THE SPIKE FIELD. The first working render was a bed of spikes: 160 of 410
    plates were `shell_0011_0111`. A corner needs all four of its cells clad to
    read 1, but a midpoint needed only the one cell across, so at every rim the
    midpoints rode at half between corners sitting on the floor. Fixed in
    `6e368af8`: midpoints interpolate their corners, and only a DEAD edge (both
    corners on the floor) gets a vote of its own, which is what still turns a
    one-cell spine into a ridge. Shapes used dropped 36 -> 24.
11. Screenshots looked hung. Cause was omitting `--features dev`
    (`dev = ["debug"]`), so the probe host and capture systems never linked. A
    display IS required; with none, `WinitPlugin::build` panics.
12. THE PHANTOM SPINE. A spawned ship's cells cannot be read off the integers.
    The wfc row mirrors a half grid about x = 0, so every section stands at a
    half cell on that axis; rounding put the two halves in cells one apart with
    an empty column between them, they never touched, and both skins would have
    died to the floor down the spine of every ship. `lattice_phase` reads the
    offset most of a ship's sections share, per axis, and the cells come out of
    the ship. Caught before the render, by reasoning about the example's own
    `HULL_GRID.origin`; pinned by
    `a_ship_standing_half_a_cell_off_the_integers_is_clad_the_same`.
13. THE BLOCKED EXIT. Owner's report: "there is a torpedo bay with exit upwards
    and on diagonal up there is a hull, that's illegal because hull requires a
    neighbor, but the torpedo bay requires void". Real, and worse than reported.
    Reproduced on the default row: seed 20260816 held a bay at cell `(1, 2, 8)`
    firing `+Y` with hull at `(0, 3, 8)`, DIAGONALLY up, wanting cladding in the
    muzzle cell `(1, 3, 8)`; seed 20260815 held a bay at `(0, 1, 9)` firing `-Z`
    down the length of its OWN SHIP, through eight cells of hull, with the
    torpedo's birth cell (`spawn_offset` is 2 cells) solid. 10 of 30 exits over
    three ships.

    The conflict is genuine and has no third answer. A lane cell either gets a
    plate (the bay fires into its own plating) or is refused one because the
    bay's blind face keeps the skin out (the hull beside it faces vacuum bare).
    So it cannot be fixed in the skin: the STRUCTURE must not be there. The rule
    is that an exit's whole lane is VOID - no structure in it, and nothing
    beside it offering a socket into it, which is exactly what `cladding_cells`
    reads to decide a cell is clad.

    `compatible` now carries the local half (an exit face demands vacuum across
    it), which is a binary constraint the propagation can use and which can
    never empty a domain, since vacuum is compatible with everything. The rest
    of the lane cannot be a binary constraint at all, so `erode_blocked_exits`
    takes off the parts that cannot fire where they stand, one per pass, beside
    `erode_studs`; `fill_pits` is handed the surviving lanes so packing a dent
    cannot re-block one. `refuse_blocked_exits` fails the run if any of that
    misses. NO DEADLOCK: nothing here can starve the collapse.

    Cost, and it is real: over 3 seeds, bays 18 -> 4 and drives 46 -> 28 (12
    mounts unchanged, hull 364 -> 382). Over 12 seeds it settles at 3.3 bays and
    15 drives a ship. A fitting now only survives where it stands on a locally
    flat outward surface, which is what "it has to be able to fire" means on a
    lumpy hull. If the fleet wants more guns, the knob is the draw weights.
14. THE PROPORTIONAL CORNER, tried and reverted. Holding a rim corner up in
    proportion to how many of its four cells are clad (`HALF * clad / 4`) does
    NOT remove the shark fins it was aimed at: a fin comes from the STRUCTURE
    branch, where one hull cell standing proud pulls its corner to `FULL` while
    the rim around it sits on the floor, and no corner-softening rule touches
    that. What it does do is make every edge LIVE, so the dead-edge midpoint
    vote never fires and a one-cell run flattens from a ridge (corners 0,
    midpoints `[HALF, 0, HALF, 0]`) into a flat quarter-height slab (corners 1,
    midpoints 1). Rendered at a matched pose: fins and jagged flanks both
    survive, and the whole hull gains a crumpled quarter-cell facet field.
    Pinned by `a_one_cell_run_of_skin_reads_as_a_ridge`, which fails under the
    proportional rule with corners `[1, 1, 1, 1]`.

    The narrower variant (soften only 3 of 4) preserves the ridge but is close
    to a no-op: a STRAIGHT rim corner has exactly two of its four cells clad, so
    only reflex corners of a skin patch ever lift. Rendered too; it is the
    all-or-nothing image with a few inner corners rounded.
15. RENDERS THAT DO NOT COMPARE. `wfc_ships` was missing `freeze_bodies`, which
    every other capture example runs behind `capturing`. A subject is a dynamic
    body, so a spawn impulse turns the whole row while the harness settles, and
    `frames(SETTLE_FRAMES)` is a FRAME count: under a busy box the same seeds
    were photographed after 14 s of physics instead of 1.4 s, at a visibly
    different attitude. Two runs of the same binary agreed, so it read as "the
    skin changed the ships" until the step timings gave it away. A/B of anything
    rendered is worthless without this.

16. THE PYRAMIDS BESIDE THE NOZZLES. Owner's report, with a screenshot of a
    thruster bank: "it's basically not adding ridges with void next to those
    thrusters on the hull section, and it also adds those corners next to
    thrusters which makes it impossible to have the actual wfc of the skin
    work".

    Reproduced before touching a rule, on a hand-built transom (4x5x3 of hull
    with five drives bolted across it, each blind on five faces - now
    `the_skin_between_a_bank_of_drives_is_flat_plate`). The blind faces refuse
    cladding to every cell around a drive, so the transom skin comes back in
    lone cells and one-cell strips, and what those cells wore was:

    - `shell_0000_0000`, every sample on the floor, whose centre falls back to
      half a cell: a CONE. That is the pyramid, one per gap in the bank.
    - `shell_0004_2022` and friends: one corner at the WHOLE CELL with the rest
      on the floor. A fin, standing right beside a nozzle.

    Two causes, and the hypothesis in the task named the first:

    - a rim corner read 0, so a plate at the edge of a patch ramped away to
      nothing instead of carrying its height to the cell boundary. HELD. A rim
      corner now holds the running height, the plate's own wall drops
      vertically at the boundary, and the edge of a skin is a ridge the mesh
      builder was already able to draw.
    - a corner read the WHOLE CELL from `filled` alone. A drive is filled, so
      the plate diagonally off one climbed a whole cell to close against a
      nozzle there is nothing to close against. Structure now only walls a
      corner where it offers a socket IN THE SKIN'S OWN PLANE: a hull cube
      does, a drive (one socket, on the end it bolts down through) does not.

    The FIRST answer took the rim rule at face value and held EVERY rim corner
    at the running height. It works and it is wrong. Every corner set holds the
    plate's own clad cell, so no corner can be 0, no edge can be dead, and the
    whole hull comes out an offset shell half a cell thick with square steps:
    3-4 shapes a ship instead of 15-21, and one shape (`shell_2222_2222`) end to
    end over a flat face. Rendered, and it is a clean plated hull - but the
    owner saw it and asked "did we remove the corners there? I kind of liked
    ridges and corner looking things", having said earlier "I like having
    studs". The pointy vocabulary is WANTED. It was the crowding beside the
    nozzles that was not.

    The SECOND answer splits the rim in two, which is what the owner's sentence
    said all along - "not adding ridges WITH VOID next to those thrusters":

    - a rim facing OPEN SPACE tapers away as it always did. Spines keep their
      tents, lone cells keep their studs, the silhouette keeps its facets.
    - a rim ending AGAINST something stops dead at the cell boundary and its
      own wall is the ridge. Against something is three things, all "the
      surface cannot carry on past here": the cell is clad, or it holds
      structure, or it is a POCKET a blind face keeps the cladding out of.

    That last clause is the whole fix, and it is free: `blind_pocket` is the
    predicate `cladding_cells` already used to refuse the cell, lifted out and
    read from both places. The cells between a bank of nozzles are surrounded by
    pockets, so they stand up square; the cells at the edge of the ship are
    surrounded by vacuum, so they taper.

    Renders at a matched pose over the default row, four of them, in
    `/tmp/shots-edges-before` (baseline), `/tmp/shots-walls-only`,
    `/tmp/shots-edges` (rim held everywhere) and `/tmp/shots-pocket` (shipped).
    The `walls()` fix ALONE does not do it: the full-cell fins go, but every
    cell between the nozzles is still surrounded by unclad cells and still
    derives `shell_0000_0000`, whose middle rides at half a cell - a CONE. The
    pyramid field survives, at half the height.

    What is NOT fixed, and was not attempted: the VOID itself. A blind face
    still keeps the skin out of all five cells around a drive, so a bank of them
    still opens a well of bare hull art. It now reads as a recessed bay framed
    by a raised rim rather than as a hole full of spikes. Narrowing that to the
    EXIT face alone is the obvious next move and is not free: the skin would
    have to be told which face a part fires through, and it reads link points
    and nothing else today.
17. THE HIP THAT IS NOT A BUG. A plate DIAGONALLY off a hull block standing
    proud in its plane still rises to the whole cell at that one corner, and
    reads as a triangular facet. It cannot be anything else: the corner is
    SHARED with the two plates edge-on to the block, which are at full height
    along their whole edge, so a diagonal plate holding the running height there
    would tear the surface. Facets at a proud corner are the design; facets
    everywhere were the bug.

18. THE FITTINGS CAME BACK BY PRICE, NOT BY RULE. Exit clearance is correct and
    was not touched. What it costs is that a fitting no longer competes for a
    CELL, it competes for a cell with ROOM AROUND IT, and most of what the draw
    offers is taken back off by `erode_blocked_exits`. So the answer is the one
    the record already named: the draw weights.

    Measured over 48 seeds (four blocks of 12: 20260815, 1, 999, 20260901),
    before -> after, at `2.5 / 0.6 / 0.9` and `VACUUM_BASE 0.3` ->
    `3.2 / 1.4 / 1.6` and `VACUUM_BASE 0.22`:

    | | hull | drives | bays | mounts |
    | --- | --- | --- | --- | --- |
    | before | 6630 | 636 | 144 | 268 |
    | after | 7126 | 988 | 338 | 526 |

    Per ship: drives 13.3 -> 20.6, bays 3.0 -> 7.0, mounts 5.6 -> 11.0, and the
    hull is DENSER rather than thinner. `refuse_blocked_exits` passes on all 48.

    Two findings worth keeping:

    - the parts compete with EACH OTHER, not just with vacuum. Pricing bays at
      2.5 pulled drives DOWN from 152 to 142 on one block while bays went 32 to
      106. A weight is a share of the same surface, so these have to be tuned
      together and read together.
    - `VACUUM_BASE` is the lever that pays for the rest. Fittings carry their
      own void with them (the lane, and the pocket the skin is refused), so a
      hull full of them thins out on its own; buying that back at the vacuum
      weight is what keeps a well-armed ship from reading as a frame with parts
      hung off it. Without it the same fitting weights cost 260 cells of hull.
    - the ceiling is real. At `5.0 / 1.5 / 2.2` the row comes back a radiator
      bank - 22 drives a ship in dense rows - and the hull loses 8%. Rendered,
      and it is the reason the shipped numbers are lower than the first probe.

19. ONE CLEARANCE RULE, TWO CALLERS. The rule lived in the example, so the
    GENERATOR could not draw a blocked muzzle and a PLAYER could still build
    one by hand. It is `crates/nova_ship/src/sections/clearance.rs` now:
    `exit_normal`, the lane walk, both clauses, and `placement_blocks_an_exit`
    for a builder. The example keeps only the reading of a collapsed, mirrored
    grid and the erosion pass; the counts over 12 seeds are byte-identical
    before and after the move, which is what says the rule did not change.

    Four things worth keeping:

    - the rule reads a `SkinStructure`, the same type the skin does. That is
      not a convenience: the lateral clause IS the skin's rule ("a socket
      offered into a cell is what `cladding_cells` reads"), so sharing the type
      is what stops the two drifting.
    - the lane used to stop at the example's fixed grid. It stops at the extent
      of the ship's own filled cells, grown by one, which is the same answer
      for a hull that fits in a grid and the only available one for an editor
      ship that has no grid at all.
    - the editor's question is INCREMENTAL: "would this part block something",
      not "is this ship legal". A ship that already cannot fire must still take
      a placement, or a builder who makes one mistake can do nothing but
      delete. Findings are counted with the part and without it, on ONE lattice
      taken over the whole set - reading the phase twice would let the extra
      part re-bucket everything and the difference would be about the
      bucketing.
    - the editor learns which way a part fires from the BUILD STATE
      (`PlayerSpaceshipConfig`'s `SectionSource::Inline`), not from the preview
      entity. A preview section carries its sockets and its collider as
      components and nothing that says what kind of part it is.

    Surfaced through the channel that was already there: one more `Refusal`
    variant, "nothing may block an exit". That gets the red status line, the
    red ghost box and the red socket ring for free, and the click is refused by
    the same gate that refuses an occupied socket. Proved live in the editor
    run, which now builds a tower, aims a drive up the lane beside it, reads
    the words back off the screen and shoots
    `editor-placement-blocked-exit.png`.

    Not free: the pocket a ship makes under an overhang is INVISIBLE from the
    editor's camera, so the first three attempts at a live demo hit the section
    in front instead and read a stale status line (`subtree_text` does not
    filter on visibility - a hidden status node still reports its last words).
    The run builds the shape it needs in view of the camera instead.

20. THE WELL ROUND A FITTING. Owner's report: "if we have a face with a 3x3 of
    hulls and in the middle of that face you have a PDC or a thruster (which
    require empty surrounding) there ARE NO shells in the surrounding 3x3 of
    that PDC/thruster which feels wrong". This is the thing 16 named and did not
    fix, and the diagnosis there was right: a blind face refused cladding on all
    FIVE faces a fitting turns to the world when only the EXIT face needs the
    space. The hole is a CROSS - the four cells edge-on to the fitting plus its
    own - because the four diagonal cells were never face-neighbours of it and
    were clad all along.

    The missing fact already existed, one module over: `clearance.rs` reads
    `exit_normal` off the KIND. `SkinStructure` now carries a second `[bool; 6]`
    per cell, the faces something FIRES through, beside the sockets it already
    held, and `blind_pocket` becomes `exit_pocket`. One reading fills both, so
    the skin and the clearance rule cannot disagree about which face a part
    fires through - which is the same argument 19 made for sharing the type.

    Plumbing, and it is most of the diff. `read_structure` takes a `PlacedPart`
    (pose + sockets + exit) instead of a pose/socket pair, which is the struct
    `clearance` already had as `PlacedExit`; the two are now one type in
    `shell_skin`, and `clearance::read_ship` is `read_cells` plus the exits.
    A live section learns its own exit from a new `SectionExit` component, put
    on at spawn where the `SectionKind` is still in hand - a section entity
    carries sockets and a collider and nothing that says what kind of part it is.
    The editor reads it off the build state, exactly as its placement refusal
    already did.

    NOT WEAKENED: the lane. `blocked_exits` is untouched, and the interaction is
    pinned rather than argued (`the_skin_plates_around_a_muzzle_without_closing_
    its_lane`): the lane is held by the SOCKET clause, not by the blind-face
    one, so narrowing the skin cannot open it. A lane cell is clad only if some
    neighbour offers a socket into it, and that is exactly what the rule refuses
    as `BlockedExitReason::Cladding`. On a ship that passes
    `refuse_blocked_exits`, `exit_pocket` is therefore a no-op in
    `cladding_cells` - it still earns its place in `ends_against`, and as the
    answer for a hand-built ship that is already illegal.

    One consequence worth knowing: `PlateReading::pocket` is now
    `PlateReading::fitting`, and it measures the distance to a cell that FIRES
    rather than to a hole in the skin. The old predicate would have found
    nothing at all after the narrowing - a fitting's mouth is one cell out of
    the plate's own plane, so `pocket_distance`, which walks that plane, would
    have returned `REACH` for every plate on a deck. `near_fitting` in a style
    means what it always said it meant.

    RENDERED, matched pose, matched seeds, the only difference being the one
    line of `exit_pocket`. The structure is byte-identical either way - nothing
    in the collapse reads this predicate - so the pair is an A/B of the skin
    alone. Plates over the row 438 -> 520 (162/144/132 -> 196/158/166), shapes
    18/16/21 -> 17/13/22, and the relief histogram gains flat and step where it
    used to have nothing at all.

    Honest reading of the frames: the well is GONE, not smaller. The before
    frame has a trench of bare hull-cube art running the length of every ship
    wherever drives and mounts stand; the after frame plates it, and what is
    still visible between the nozzles is the drive BODIES, which is what a
    nozzle sticking out of a hull looks like. Two things remain and are not this
    bug: the cell each part fires into stays bare by design, and a HALF-SIZE
    part (the PDC mount stands on a cell boundary) still leaves a narrow band of
    hull art at its foot, which is the derivation's whole-cell resolution and
    not the pocket rule.

## Current inventory

New:

- `crates/nova_ship/src/sections/shell_shape.rs` - shape math, canonicalisation,
  socket derivation, mesh generation, `ShellSurface`.
- `crates/nova_ship/src/sections/shell_skin.rs` - `SkinStructure`, `SkinPlate`,
  `derive_skin`, `cladding_cells`, `stands`, `boundary_heights`, `plate_for`,
  plus the spawn half: `ShipSkin`, `ShipSkinMarker`, `spawn_ship_skin`,
  `lattice_phase`, `section_cell`, `plate_body`, `plate_collider`, `SkinAssets`,
  `dress_skin_plate`, `despawn_dead_fixtures`, `ShipSkinPlugin`. Unbounded on
  `IVec3`, `SUPPORT_REACH = 4`.
- `crates/nova_ship/src/sections/fixture.rs` - `SectionFixture`. See Fixtures.

Deleted by the prototype already:

- `crates/nova_authoring/src/base_content/sections/shells.rs`
- `scripts/gen-shell-parts.py`
- `assets/base/gltf/shells/*.glb` (41), `art/shells/*.obj` (41)
- 41 bundle manifest lines; `base.content.ron` lost 2619 lines (6256 -> 3637)

Deleted by steps 2 and 3:

- `shell_shape.rs`: `generated_shell_sections`, `PALETTE`, `palette_reading`,
  `shell_description`, `round_of`, `height_word`, `SHELL_HEALTH`, `SHELL_MASS`,
  `every_canonical_shell` and the Burnside/exhaustiveness tests
- `crates/nova_assets/src/merge.rs`: `with_generated_shells`
- `crates/nova_ship/src/sections/hull_section.rs`: `HullSectionConfig.shell`,
  `HullSectionShell`, `ShellRenderAssets`, the shell render branch

The alphabet is quarter cells now: `SAMPLE_HEIGHTS` is five long, `FULL` is 4
and `HALF` is 2. Corners still emit only 0, `HALF` and `FULL`; every live edge's
midpoint is the exact mean of its corners, which now always lands on a real
sample. `round_digits` takes the alphabet as its radix, so a digit above `FULL`
is refused by construction.

The geometry tests took a shape SPREAD in place of the roster: every corner
setting over `[0, HALF, FULL]` against every midpoint setting over the whole
alphabet, canonicalised - 12720 shapes, and the four tests over them run in
0.11s. A full sweep of 5^8 is pointless now that nothing enumerates anything.

CLOSED by `50d98ae9`: the example emits structure only. Its whole skin half is
gone (`SKIN_GRID`, `cladding_cells`, `plate_for`, `boundary_heights`, `stands`,
`support_depth`, `hull_face_cover`, `turned_corners`, `shell_out_of`,
`mirror_tile`, `turns`, `Grid::cell_at`, the shape histogram), and the two
CHANGELOG entries that described it are rewritten. `--bare` and `C` now flip
`SpaceshipConfig.skin` instead of dropping a pass.

## Wired, and what it measured

`50d98ae9` wires the spawn half. What landed, against the table below:

- `spawn_ship_skin` in `Update`, `.after(build_ship_integrity_graph)`
  (now `pub(crate)`) `.before(IntegritySystems)`, on `Added<SectionLinkPoints>`
  and only for a root carrying `ShipSkin(true)`.
- Each plate is a child of the section in its `anchor` cell - a new `SkinPlate`
  field, `cell + FACES[out ^ 1]`, which the derivation already knew. The pose is
  turned into that section's frame, so a yawed section does not carry a yawed
  plate.
- `SpaceshipConfig.skin: bool`, `serde(default, skip_serializing_if)`, put on
  the root as `ShipSkin(config.skin)`. `Option<B>` is NOT a `Bundle` in bevy
  0.19, so the flag rides in the component rather than the component being
  conditional. Generated content is byte-identical after `content gen`.
- `ShipSkinPlugin { render }` added by `SpaceshipSectionPlugin`. GAMEPLAY:
  `spawn_ship_skin`, `despawn_dead_fixtures`. RENDER: `SkinAssets` and the
  `dress_skin_plate` observer, which hangs one mesh child per surface role.
  `plate_body` is the whole of the gameplay plate.

Cladding is OPT IN (`skin: false` by default, on all 50 existing call sites).
The derivation reads a hull as unit cells; the semantic craft (racer, cargoa,
cargob) are modelled parts of their own sizes standing at half cells, and
default-on would have wrapped every campaign ship in a skin that fits nothing.
That decision is worth revisiting when a hull can say which kind it is.

Measured on the row (RTX 3060 Ti, vulkan, 1920x1080, dev profile):

- 410 plates over 3 ships (146 / 132 / 132), 15-21 distinct shapes each.
- Frame time, clad vs bare, three pairs: clad 50.6 / 59.5 / 60.5 ms mean,
  bare 82.5 / 90.6 / 65.7 ms. The clad row measured FASTER every time. So
  several hundred plate colliders and their meshes are not what this scene
  costs; something else (three ships of ~150 physics sections, plumes) is, and
  the plate cost is inside the run-to-run spread. Not a clean isolation - it is
  a bound, and the bound is "cheap enough to be invisible here".
- Render (`/tmp/shots/wfc-ships-row.png`): coverage is total, no void gaps, no
  cracks or z-fighting between plates. The surface is heavily FACETED, with
  pyramids where the skin runs one cell wide and the odd thin blade. That is the
  resolution limit of a corner-sampled height field on a hull this small, not a
  regression - a one-cell spine still reads as a ridge.

## Wiring insertion points

Surveyed earlier. Line numbers may have drifted; verify before editing. The
`nova_ship` and `spaceship.rs` rows are done; the editor rows are not.

| Where | Point |
| --- | --- |
| `nova_ship` skin spawn | `Update`, `.after(build_ship_integrity_graph).before(IntegritySystems)`, gated on `Added<SectionLinkPoints>` |
| Plate parent | the structural section it bolts to - gives free recursive teardown and free damage tint via `owning_section` |
| `spaceship.rs:232` | `SpaceshipConfig` gains the skin field, `serde(default, skip_serializing_if)` |
| `spaceship.rs:261` | `spaceship_scenario_object` puts the component on the root |
| `scenario.rs:436` | flattens `player_config.sections` on Play - structure only |
| `nova_editor/src/lib.rs:175-198` | chain after `sync_placement_ghost`; gate on `in_state(Editor)` and `player_config.is_changed()`; take `Res`, never `ResMut` |
| `nova_editor/.../preview.rs:68-72` | `PreviewRole::Display` strips `SectionMarker` and `Collider`, so plates stay invisible to the validator, the picker and the Q pipette |
| `nova_editor/src/ui/mod.rs:183-197` | skin toggle in the Tools block |
| `nova_ui/src/widget/chrome.rs:117` | `checkbox(on, skin)` idiom; used at `nova_menu/src/mods.rs:186-198` |
| `nova_editor/src/ui/mod.rs:285-305` | `sync_key_legend` line |
| `gallery/catalog.rs:73-91` | `browsable` filters on `hide_in_editor` |

The editor has NO grid. Positions come from socket mating, so cells must be
bucketed with a `PLACEMENT_SNAP`-style requantiser before derivation.

No authored content contains `shell_`, so there is no data migration.

## Health and mass

`base_section` feeds a section's `mass` field to avian as DENSITY, not absolute
mass, and avian derives real mass as `mass * collider_volume`. So plate mass
needs no tuning: one density constant, and a quarter-height plate weighs a
quarter of a full cube. Scale health by the same volume.

Plates are NOT integrity-graph nodes. Structure alone decides connectivity, or
cladding would hold a severed ship together.

Settled, in `shell_skin.rs`:

- `SKIN_HEALTH_PER_CELL = 80.0`, the figure cladding already carried as a
  section. About twenty PDC rounds for a full cell, ten for the half cell a run
  of skin travels at, against a hull section's 200.
- `SKIN_DENSITY = 0.25`, a quarter of a section's. Every section stands at 1.0
  over a unit cube, and a clad ship carries hundreds of plates against a handful
  of sections: at structural density the skin would outweigh the ship it wraps.
- `ShellShape::volume()` is the mean of the eight samples, which is the same
  number `centre_height` wants, so they are one function. The all-floor stud
  keeps its half-cell fallback - a zero there would be a plate born dead inside
  a collider of no volume.
- The collider is a cuboid across the footprint, `volume` cells tall, wrapped in
  a one-shape `Collider::compound` so it can be OFFSET onto the cell floor. A
  box centred on the entity floats above the surface a shot has to hit.

## Fixtures

A `SectionFixture` (`crates/nova_ship/src/sections/fixture.rs`) is something
attached to a section that is not part of the structure. It has health, mass, a
collider and a look, and is never in the integrity graph, never in the aggregate
health, never in the palette and never a `SectionMarker`. The test is
CAPABILITY: if shooting it off should cost the ship something it can DO, it is a
section. Plates are the first kind; decorations and greebles are the next.

Two answers to the two open questions:

- Tinting does NOT apply to plates. `owning_section` in `damage_tint` stops the
  ChildOf walk at a fixture, so a plate mesh is never marked, never clones a
  `StandardMaterial`, and never reddens. That is behaviour first - a fixture's
  damage read is that it COMES OFF - and it also drops 400 to 1200 material
  clones per clad ship.
- Collider fidelity stays at the cuboid. The meshes are not convex and cladding
  is thin.

A dead plate is despawned by `despawn_dead_fixtures`; nothing else would take
it, since destruction reaches structure through the graph a fixture is not in. A
destroyed SECTION takes its plates with it for free, because they are its
children - pinned by a test, since it is what the parenting decision buys.

Damage still BUBBLES from a plate to the section behind it (`HealthApplyDamage`
auto-propagates through `ChildOf`), so shooting cladding also charges the hull
under it. Left as is; the pierce work is the place to decide whether it should.

## Verification

Render, do not trust exit codes. The spike field passed `cargo check` and 18
tests.

```bash
nix develop --command cargo check
nix develop --command cargo fmt --check
nix develop --command cargo test --lib -p nova_ship shell_
nix develop --command cargo run content lint
```

Screenshots, exactly as the owner runs them, plus a display:

```bash
Xvfb :99 -screen 0 1920x1080x24 &
DISPLAY=:99 NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_SHOT_DIR=/tmp/shots \
  nix develop --command cargo run --example wfc_ships --features dev
```

Then OPEN `wfc-ships-row.png`. Kill Xvfb by recorded PID, and take a display
nobody else is on - a parallel agent may already hold `:99`.

The generator's own gate is worth a sweep as well as a row, since the collapse
is where the illegal ships come from:

```bash
DISPLAY=:99 NOVA_AUTOPILOT=1 nix develop --command \
  cargo run --example wfc_ships --features dev -- --ships 12
```

## The editor half, after the fact

Landed separately, on `editor-skin`. The spawn half was step 7's first row; this
is the rest of it - the build view showing the skin while the ship is still
being built.

`sync_editor_skin` (`crates/nova_editor/src/skin.rs`), in the placement chain
after `sync_placement_ghost`. After, not before, because the PART UNDER THE
POINTER counts as structure: the skin has to be derived from the same solve the
ghost on screen is showing, or the cladding is a frame behind the part.

Five decisions worth keeping:

- The GATE is a hash of the structure, not `player_config.is_changed()`. The
  ghost is not in the build state, so a change gate would show the reflow only
  after the click - and the reflow before the click IS the feature. Hashing
  ~150 sections every frame is 0.1 ms at the size a ship gets to; deriving is
  what has to be avoided, not reading.
- Nothing diffs and nothing is patched. Despawn every plate, derive, respawn.
  The derivation is a pure function, so the cheap answer and the correct answer
  are the same one.
- A REFUSED ghost contributes nothing. The bounds box already says the click
  will build nothing, and cladding it would draw a ship that cannot exist.
- A preview plate is `ShipSkinMarker` + a pose + `Visibility` and NOTHING else:
  no `SectionMarker`, no `Collider`, no health. `dress_skin_plate` still draws
  it (the observer only reads the marker), and the placement validator, the
  pointer and the Q pipette cannot see it. `PreviewRole::Display` was the
  obvious vehicle and is the wrong one - it takes a `SectionConfig`, and a
  plate has no prototype.
- The toggle lives on `PlayerSpaceshipConfig`, not in a view resource: it is a
  property of the SHIP, `scenario.rs` already flattens that resource into the
  `SpaceshipConfig` Play spawns, and one resource to watch is one resource to
  watch. It survives New Ship and the on-enter rebuild for the same reason.

`read_structure` in `shell_skin.rs` is now the one reading of poses + sockets
into a `SkinStructure`, shared by the spawner and the editor. The lattice phase
is what would have drifted otherwise, and a ship clad on two phases is clad two
ways.

### Measured

`--lib` test profile (optimized), synthetic 8x8x8 block: 512 sections, 384
plates.

| | cost |
| --- | --- |
| first derive + spawn | 2.92 ms |
| reflow (despawn 384, derive, respawn 384) | 2.29 ms |
| unchanged frame (hash + count) | 0.15 ms |

Live, in the editor example (5-9 sections, 19-27 plates): 0.11-0.23 ms per
reflow, logged by `sync_editor_skin` at debug. No stutter: the ghost only moves
in whole cells, because placement mates sockets, so dragging the pointer across
a face does not re-derive at all. The render half is not in the 2.29 ms - each
plate hangs 1-3 mesh children, and their meshes are cached per shape - but at a
real build's size it is invisible.

Nothing was pre-optimised. If a 400-plate editor ship ever stutters, the lever
is deriving on a settled placement rather than on every ghost move.

### Rendered

`editor-skin-off.png`, `editor-skin.png`, `editor-skin-drag.png`, from the
`editor` example. Honest reading: on a 5-section build the cladding is a
faceted crystal - most of the skin is one cell wide, so it is nearly all tents
and studs, and the boxes underneath stop being readable. That is the resolution
limit the row render already showed, not a regression. The drag figure works:
the plating closes over the ghost's cell and the green bounds box is what says
where the part is, since the part itself is now UNDER the skin - which is the
words the owner used.

### Pre-existing flake, not caused by this

`examples/ui/editor.rs`'s `raise a tower, first course` beat fails about half
the time on this box: the click at world `(0, 1.5, 1)` builds nothing. Verified
on the BASE commit with this branch's work stashed, so it predates the editor
skin. It is a grazing aim - the ray from the editor camera clears the front-top
edge of the target section by ~0.19 cells - and it is worth re-aiming next time
somebody is in that file.
