# Thruster shells: sizes, side link points, and a showcase

Design spike for task 20260816-200255, sibling to the greeble spike
(20260816-194637, cited as GREEBLES below). Deliverables: this document, the
`thruster_gallery` example, and the renders in this folder. No engine code
changed in this lane; the side-link-point change is designed here and built in
the follow-ups.

## 1. Audit: the drive today, and why it reads simple

### 1.1 The prototype

`basic_thruster_section` (`crates/nova_authoring/src/base_content/sections/
standard.rs`): mass 1.0, health 70 (deliberately the most fragile section -
exposed propulsion takes MORE damage), magnitude 1.0, `render_mesh: None`,
and ONE link point: `base` at `-Z * 0.5`, normal `-Z`.

### 1.2 The render

No authored mesh exists. `insert_thruster_section_render`
(`crates/nova_ship/src/sections/thruster_section.rs`) builds the whole look
from two primitives at spawn:

- a grey `Cylinder::new(0.4, 0.4)` barrel at `z = -0.3`,
- a red-orange `Cone::new(0.5, 0.5)` bell opening `+Z`,
- plus the exhaust cone (shader-driven, invisible at zero throttle).

Why it reads simple, itemised: one size only; two flat-colour untextured
primitives; no housing, rim, piping or greeble; a plain cone for a bell; and
the SKIN IGNORES IT COMPLETELY - see 3.1. Every drive on every ship, from a
shuttle to the wfc capitals, is this same one-cell object repeated.

### 1.3 Clearance and the one-socket doctrine

`crates/nova_ship/src/sections/clearance.rs`: `exit_normal` says a thruster's
business points `+Z` (thrust is applied along `-Z`), and the whole LANE in
front of the bell must be void - no structure in it, and nothing beside it
OFFERING A SOCKET into it, because an offered socket is exactly what makes
the skin close a cell over (`cladding_cells` reads the same fact).

Why one socket, in the record's own words (`standard.rs` comment,
`examples/screenshots/wfc_ships.rs` module doc): the drive once carried six
sockets, and "a builder would bolt a hull slab onto the barrel or plate one
across the nozzle". The generator used to carry a special-case mask for it;
the fix was to make the part say what it is in its own link points. The test
`the_thruster_sockets_only_the_face_it_bolts_on_by` pins the single socket.

That reasoning has TWO halves, and the side-socket design must split them:

- "plate one across the nozzle" is the EXHAUST face. Permanently right. The
  exit face must never carry a socket - `wfc_ships` already asserts a part
  may not carry a socket on the face it fires through, and clearance depends
  on it. Nothing proposed here touches this half.
- "bolt a hull slab onto the barrel" was a bug ONLY because the barrel is a
  proud cylinder that nothing can visually join. The owner's shell idea makes
  the flank a flat, claddable slab - a face that a hull mate and a skin plate
  genuinely make sense against. With that art, the flank socket stops being a
  lie about the geometry and becomes the truth about it. The doctrine "every
  part says what it is in its own link points" SURVIVES; what changes is what
  the part is.

## 2. Candidate looks

All three sources below are already in the repo, and all three render in the
`thruster_gallery` example (renders in this folder).

### 2.1 Fertile Soil "Spaceship Blocks" - the purpose-built thruster family

`art/spaceship-blocks/` carries a dedicated propulsion family:
Thruster_Single_Small, Thruster_Triple_Small (each with Housing_Front /
Housing_Mid companion pieces), Thruster_Triple_Large, Hyperdrive_Rearmount
S/M/L, Hyperdrive_Sidemount S/M. Five are already converted to flat-Kd `.glb`
under `art/part-candidates/blocks/`. These are the closest match to the shell
idea: boxy housings with recessed nozzle faces, made to be stacked into
banks. The housing pieces are literally "the shell without the nozzle".

### 2.2 Kenney cast cuts

The corvette and racer engines cut from the Kenney craft
(`art/part-candidates/cargoa/engine_*.glb`, `racer/engine_*.glb`,
`cargob/engine_*.glb`): nacelle silhouettes with intake/fin detail. Good
reference for a mid-size look; as parts they are ship-specific nacelles, not
grid shells.

### 2.3 Quaternius cuts

Spitfire engine pods (x4) and striker nacelles
(`art/part-candidates/quaternius/...`): rounded pods, strong silhouettes,
flat-Kd bakes. Same position as the Kenney cuts: reference and possible
one-off promotions, not the grid family.

### 2.4 Licence status

- Quaternius Ultimate Space Kit and Kenney packs: verified CC0 by the greeble
  spike - `tasks/20260816-194637/GREEBLES.md` section 4.2 (not re-verified
  here, per that record).
- Fertile Soil Spaceship Blocks Collection: CC0, verified on the itch.io page
  2026-08-12; `art/README.md` is the record (the zip ships no licence file).
- The proposed shell meshes themselves are recipe-generated primitives:
  original work, MIT with the repo - the same licence position GREEBLES 4.1
  established for the greeble kit.

### 2.5 Sourcing position

Same as GREEBLES 4.2, for the same three reasons (determinism contract,
density mismatch, cell-frame rework): the packs are SILHOUETTE REFERENCE; the
production route for the shell family is JSON recipes through a generator
script (`scripts/nova_glb.py` writer, the `gen-greebles.py` pattern), with
committed deterministic `.glb` output and a `--check` gate. The Fertile Soil
housing-plus-nozzle split is the silhouette to steal: a flat-sided housing
box, a recessed nozzle plate, bells inset so the rim of the housing stays a
clean slab for the skin to meet.

## 3. The core question: can the skin clad thruster flanks?

Owner's idea: "create just the SHELL of the thruster with different sizes and
then let the style shell actually make it look good ... let the thrusters
connect on sides too and leave only the exhaust to void."

### 3.1 Today: no, and by design

Two functions in `crates/nova_ship/src/sections/shell_skin.rs` decide it:

- `stands` (line 339): a plate may only bolt down through a face that OFFERS
  A SOCKET (`structure.offers(...) == Some(true)`). A drive's flanks offer
  none, so no plate can ever anchor to a thruster. The skin cannot dress a
  drive at all today - not its flanks, not its barrel.
- `walls` (line 450): structure only reads as a wall the skin must climb if
  it offers a socket in the skin's own plane. A drive never does, so plating
  beside it runs PAST at its own height and the nozzle sticks proud - the
  documented fix for "a fin beside every nozzle".

So today's drive is invisible to the whole style system: plates, greebles,
`near_fitting` halos all happen AROUND it, never ON it.

### 3.2 With flank sockets: yes, with zero skin-code change

The skin is a pure function of `SkinStructure`, which reads each section's
rotated link-point normals (`insert_section`). Give the drive sockets on its
four flanks and, mechanically:

- `stands` turns true for the empty cells beside each flank, so
  `cladding_cells` claims them and `plate_for` anchors a plate TO THE DRIVE
  (`SkinPlate.anchor` is the drive's cell - the plate is parented to the
  section and dies with it, exactly like hull cladding).
- `walls` starts reading the drive as something neighbouring plates close
  against, so a drive sunk into a hull line joins the surface instead of
  poking through it.
- The exhaust face still carries NO socket, so `exit_pocket` keeps the lane
  cell bare and `boundary_heights` makes the surrounding skin STOP DEAD at
  the cell boundary - the vertical rim around a drive well that the module
  doc describes and `the_skin_plates_around_a_muzzle_without_closing_its_lane`
  tests. The exhaust stays a hole in the skin, not an end of it.
- Decor mostly follows: `skin_reading`'s `near_fitting` distance is measured
  to cells that FIRE (`fires_at_all`), and the deck plates AROUND a drive
  keep their halo (the drive's cell stays in their plane), so louvres,
  beacons and hooks keep dressing drive surrounds. One caveat found reading
  `fitting_distance`: it walks only IN THE PLATE'S OWN PLANE, and a shroud
  plate's plane does not contain the drive it bolts to (the drive sits one
  step along the plate's out axis). So the shroud plates themselves would
  read `near_fitting` as far unless the walk also counts the plate's own
  anchor - a one-line rule repair that belongs in the style-pass follow-up.
  "The style makes it look good" is otherwise shipped machinery, not
  speculation.

Verdict: the shell idea works, and it works through the existing derivation.
The engine change is confined to the drive's link points (plus multi-cell
support for the big sizes, section 4); the skin needs nothing.

### 3.3 Consequences that must be priced in

1. WFC mating. The adjacency rule is "a socket may never press into a face
   that has none". Today a bank of drives is legal precisely because flanks
   carry NO socket - blank meets blank. After the change, drive-flank meets
   drive-flank is socket-meets-socket: still legal, now a MATE (banks become
   structurally joined, which also feeds integrity). But drive-flank against
   a face that stays blank - a PDC mount's side, a torpedo bay's side -
   becomes ILLEGAL where it was legal. The collapse loses mixed
   fitting-beside-drive adjacencies unless the other fittings' flanks are
   socketed too. Decision to take in the follow-up: either extend the shell
   doctrine to bays (their flanks are also boxy shells) or accept sparser
   mixed banks. Mounts should keep blank flanks - a gun well is not a slab.
2. Clearance's socket clause. A flank socket offered INTO another drive's
   lane blocks that lane (`BlockedExitReason::Cladding`). Flush banks stay
   legal - each flank socket looks into the neighbour's FILLED cell. But
   STAGGERED drives (one bell a cell proud of the next) become illegal,
   because the proud drive's flank socket demands cladding inside the
   neighbour's lane. This is coherent, not collateral: today that stagger
   leaves bare structure facing the lane; after the change the rule set says
   plainly "a flank means skin, and skin may not stand in a lane". The wfc
   erode pass and the editor refusal both enforce it already - no new code,
   but generated hulls will lose staggered nozzle clusters, and `wfc_ships`
   seeds should be re-photographed in the follow-up.
3. Editor placement. Two new verbs appear: bolt hull onto a drive's flank
   (embedded drives, flush transoms) and bolt a drive BY its flank onto hull
   (slung engines under a wing, aimed aft while mounted sideways). Both go
   through the existing mating flow; `placement_blocks_an_exit` already
   answers the lane questions. One behaviour to verify in the follow-up lane:
   what the editor does when a socketed flank merely TOUCHES a blank face
   (wfc refuses it; the editor's mate-driven placement may simply never offer
   it, which would be consistent).
4. Mass and balance. Flank plates add skin mass around every drive
   (`SKIN_DENSITY` 0.25 per plate volume) and move the COM aft on typical
   sterns. The balancer reads live sections and forces, so nothing breaks,
   but flight-feel numbers (the 40.0 torque budget note) deserve a re-check
   on the bench. Note also `SKIN_HEALTH_PER_CELL` 80 vs thruster health 70:
   a drive's shroud is currently tougher than the drive. Acceptable (the
   shroud is armour), worth a deliberate look.
5. The one-socket test. `the_thruster_sockets_only_the_face_it_bolts_on_by`
   pins exactly one link point. The follow-up replaces it with the invariant
   that actually matters and generalises: NO socket on the exit face, flanks
   socketed, exhaust face never. `wfc_ships` already asserts the first half
   for every part.

## 4. The size family

### 4.1 The grid

Sizes are cells (width x height x length, exhaust face = width x height,
length along +Z toward the bell). The owner named 3x3x1, 5x5x3, x1 and x5.

| Shell | Cells | Exhaust lanes | Reads as |
| --- | --- | --- | --- |
| 1x1x1 | 1 | 1 | today's drive, re-shelled |
| 2x2x1 | 4 | 4 | corvette block |
| 3x3x1 | 9 | 9 | frigate plate drive |
| 5x5x1 | 25 | 25 | capital flat array |
| 5x5x3 | 75 | 25 | capital drive block |
| 1x1x5 | 5 | 1 | long nacelle spine |

The flat x1 variants are exhaust AREA with minimal hull depth; the long x5
variant is one lane bought with a long claddable flank - the two extremes the
owner's "x1 or x5" names. Intermediate longs (2x2x3, 3x3x2) fall out of the
same recipe parameters; the table is the family to SHOW, not a cap.

### 4.2 Mass and thrust stance

- Mass = cell volume at the standard 1.0 per cell (75 for the 5x5x3). No
  special casing; every section already weighs its volume.
- Thrust magnitude = cell volume too, so thrust/mass is CONSTANT across the
  family at the tuned baseline. The family is a GEOMETRY choice, not a power
  ladder: what a bigger shell buys is fewer joints (integrity), more
  claddable flank (style), and one part instead of nine placements; what it
  costs is a bigger void field aft (25 lanes behind a 5x5 face) and a bigger
  target (health scales with volume at the thruster's fragile per-cell
  rate). Any per-size bonus (e.g. +10% magnitude per cell on capital shells
  to reward the keep-out zone) is a later balance pass measured on the
  bench, not part of the first landing.
- Exhaust: one bell per exhaust cell (the gallery mocks this); the plume is
  already authorable per section (`ThrusterExhaust`, `Rect` geometry sized by
  width/height), so a 5x5 face can burn one rectangular sheet instead of 25
  cones if the render cost asks for it.

### 4.3 The engine gap the sizes expose

A section occupies ONE cell today: `PlacedPart` has one position, `read_cells`
buckets it once, the wfc grid holds "one section at one rotation" per cell.
No shipped section is bigger (the PDC is half-size). So every shell above
1x1x1 needs MULTI-CELL SECTIONS: a section that fills a WxHxL box of cells,
presents per-cell faces to the skin (the `SkinStructure` reading half is
ready - cells are already buckets that take arbitrary normals), carries link
points on every exposed cell face, and registers one exit lane per exhaust
column in clearance. That is the largest follow-up below, and it is
prerequisite for the family - NOT for the side-socket change, which pays off
on the 1x1x1 immediately.

## 5. The showcase

`examples/screenshots/thruster_gallery.rs` (registered in `Cargo.toml` beside
`shape_bench`): four named rows on the game's sky under the standard
three-point rig - the shipped drive rendered by the game's own observer, the
size family as primitive mocks (same two colours as the shipped drive, one
bell per exhaust cell, so the row reads as "the drive, grown"), the Fertile
Soil propulsion family, and the cast cuts. Idle orbit until the free-fly rig
is touched; fleet capture idiom (`NOVA_AUTOPILOT=1` smoke,
`NOVA_CAPTURE=1` shoots the full gallery plus a close pass on the size
family). Renders in this folder:

- `thruster-gallery.png` - the full four-row gallery.
- `thruster-gallery-sizes.png` - the size family close pass.

Everything in it is display-only: no prototypes, no sockets, no ships.

## 6. Follow-up tasks, ordered

Sizes: S = about half a lane-day, M = one to two lane-days, L = several.

1. **Drive flank sockets** (M) - the engine link-point change. Add the four
   flank link points to `basic_thruster_section` (builders + `content --
   gen`); replace the one-socket test with "flanks socketed, exit face
   never"; re-run the wfc seeds and re-photograph (`wfc_ships`), expecting
   flush banks kept, staggers gone; verify the editor's socket-on-blank
   touch behaviour; bench a fitted subject with an embedded drive and read
   the skin + near_fitting numbers. DONE WHEN: a clad bench subject shows
   plated drive flanks with an open rimmed exhaust well, and lint/clearance
   hold on every shipped scenario.
2. **The 1x1x1 shell look** (M) - recipe-generated shell mesh replacing the
   barrel+cone primitives: `scripts/gen-thruster-shells.py` (nova_glb.py
   writer, deterministic bytes, `--check`, committed output under
   `assets/base/gltf/` like the greebles) + `render_mesh` on the prototype.
   Silhouette per the Fertile Soil housing split: flat slab flanks, recessed
   nozzle plate, inset bell. DONE WHEN: the gallery row and a clad bench
   subject both read it, and the exhaust cone still sits on the bell.
3. **Multi-cell sections** (L) - one section spanning WxHxL cells: reading
   (`PlacedPart` grows a footprint), wfc tiles, clearance (one lane per
   exhaust column), integrity, collider, editor placement. Designed
   separately; this spike only fixes its requirement.
4. **The size family content** (M, needs 2 and 3) - recipes and prototypes
   for the named grid with the section-4 mass/thrust stance; editor listing;
   plume choice per size (bell array vs one Rect sheet). DONE WHEN: the
   gallery's mock row is replaced by the real prototypes and a wfc seed
   grows a capital stern with a 3x3x1.
5. **Style pass on shrouded drives** (S-M, needs 1) - tune the near_fitting
   pieces on engine shrouds per style on the bench; decide the bay-flank
   question from 3.3.1 with a render either way.
6. **Cast cut promotion** (S, optional) - if the owner wants a pack look for
   a mid-size one-off, promote a cut through the recipe route into `assets/`
   with a credits entry.
