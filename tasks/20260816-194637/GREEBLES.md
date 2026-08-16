# Greebles: a shared vocabulary across styles

Design spike for task 20260816-194637. No engine code changed; this document
plus four renders is the deliverable. Placement research lives in
`tasks/20260815-231945/PLATING-AND-GREEBLES.md` (cited as PG below) and is not
repeated here.

## 1. Audit: what exists

### 1.1 Models on disk

`assets/base/gltf/greebles/` holds 27 `.glb`, one per fixture, all GENERATED
from committed JSON recipes by `scripts/gen-greebles.py` (primitives: `box`,
`cylinder`, `taper`, `disc`, `ribs`; deterministic bytes; 200-triangle cap;
`--check` fails CI on a stale commit). Nothing is hand-modelled.

Correction to the brief that spawned this spike: the rules do NOT share
placeholder models. Every rule owns its own authored mesh (23 authored + 4
magenta placeholders). The variety problem is not model sharing - it is CLASS
COVERAGE (whole object classes exist in only one style) and RULE STARVATION
(signature pieces whose filters never fire on real builds). Both are measured
below.

### 1.2 Fixtures per style (from `crates/nova_authoring/src/base_content/styles.rs`)

| Style | Fixture | Zone it owns | Key filter | Density |
| --- | --- | --- | --- | --- |
| industrial | `industrial_stack` | high ground | Ridge/Peak/Spur/Step, seat Any, Up | chance 0.35 |
| industrial | `industrial_hazard_band` | straight edges | Brink, min_run 3, align Run | chance 1.0 (a line) |
| industrial | `industrial_radiator` | flat panels | Flat, min_run 2, stride 2 | patch 3 |
| industrial | `industrial_duct` | deck shoulders | Step/Flat, min_run 3, align Run | patch 4 |
| industrial | `industrial_louvre` | around fittings | near_fitting 1, stride 2 | chance 0.55 |
| industrial | `industrial_hatch` | thick body | min_depth 2, stride 2 | chance 0.3, patch 5 |
| industrial | `industrial_ribbing` | filler | min_depth 2 | chance 0.45, patch 3 |
| armoured | `armoured_sensor` | flat panels | Flat, min_run 2, min_depth 2 | patch 6 (floor only) |
| armoured | `armoured_cap` | tips and corners | Spur, seat Any, stride 2 | full share on lattice |
| armoured | `armoured_strake` | straight edges | Brink, min_run 2, align Run | chance 1.0 (a line) |
| armoured | `armoured_hatch` | filler | Flat/Step, stride 2 | chance 0.6 |
| civilian | `civilian_windows` | flanks | Flat/Step, facing Side, min_depth 2 | chance 0.8, patch 4 |
| civilian | `civilian_fin` | high ground | Spur/Ridge/Peak, seat Any, Outward | chance 0.5 |
| civilian | `civilian_stripe` | straight edges | Brink, min_run 3, align Run | chance 1.0 (a line) |
| civilian | `civilian_fairing` | panels | Flat/Step, stride 2 | chance 0.8, patch 3 |
| civilian | `civilian_beacon` | around fittings | near_fitting 1, stride 2 | chance 0.6 |
| salvage | `salvage_whip` | high ground | Ridge/Peak/Spur/Step, seat Any, Outward | chance 0.18 |
| salvage | `salvage_drum` | broad decks | Flat/Step, min_height 2, min_depth 2 | chance 0.10, patch 6 |
| salvage | `salvage_hook` | around fittings | near_fitting 1, stride 2 | chance 0.5 |
| salvage | `salvage_weld_seam` | straight edges | Brink, min_run 3, align Run | chance 1.0 (a line) |
| salvage | `salvage_patch_strip` | around fittings | near_fitting 1 | chance 0.35 |
| salvage | `salvage_patch_plate` | thick body | min_depth 2 | chance 0.4 |
| salvage | `salvage_patch_scab` | filler | min_height 1 | chance 0.05, patch 10 |
| placeholder | mast / vent / block / blister | one per vocabulary reading | - | - |

Kit sizes: industrial 7, armoured 4, civilian 5, salvage 7. Tests pin the caps
(6-8) and the doctrine: mismatch comes from placement and material, not piece
count.

### 1.3 Class coverage today (the gap the owner is naming)

| Class | industrial | armoured | civilian | salvage |
| --- | --- | --- | --- | --- |
| edge line | hazard_band | strake | stripe | weld_seam |
| tall piece | stack | - | fin | whip |
| flat panel piece | radiator | sensor | windows | drum |
| pipe / duct | duct | - | - | - |
| vent / grille | louvre | - | - | - |
| hatch | hatch | hatch | - | - |
| fitting halo | louvre | - | beacon | hook, patch_strip |
| filler | ribbing | hatch | fairing | patch_scab, patch_plate |
| wires / cables | - | - | - | - |
| battery / tank | - | - | - | drum (tank) |
| wheels / cogs | - | - | - | - |
| markings / stripes | (band is edge-only) | - | (stripe is edge-only) | - |

Only ONE class is implemented four ways: the edge line. That is exactly the
class the render shows degenerating into "same rule, different paint" (1.5).
Every other row is a hole in at least two styles. The owner's named classes
(antenna, wires, cables, tubes, vent pipes, wheels/cogs, ammo stripes,
batteries) map onto the empty cells of this table almost one to one.

### 1.4 Render evidence

Four shots in this folder, shot on the shape bench (fleet capture idiom,
`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`, matched pose, `freeze_bodies`):

- `bench-industrial.png`
- `bench-armoured.png`
- `bench-civilian.png`
- `bench-salvage.png`

Critique against the owner's ranking (salvage first, industrial close second,
armoured/civilian flat and samey):

- SALVAGE reads best for the reason the style comments predict: three patch
  materials a HUE apart (steel, rust, livery green) put per-plate variety on
  the body. Rust and green read from full frame distance. But the thin
  subjects (owners_l, tee, cross, runs) are BARE brown rock - the kit places
  zero pieces on them (1.5).
- INDUSTRIAL's yellow is the only accent that reads across the whole frame;
  `run_5_thick` wearing three dashed yellow rails is the best single subject
  on the bench. Thin subjects: bare. The kit's other six pieces are invisible
  at frame distance - the look is carried by the band plus the seam palette.
- ARMOURED is the only style that dresses thin shapes (caps on every other
  corner of tee/cross), which is worth keeping as the pattern to copy. But
  the whole row is one value of grey: zero accent colour, pale pieces on dark
  plate, and at frame distance it reads as undecorated. "Flat" is accurate.
- CIVILIAN reads as bare pale rock. The cobalt stripe lands only on
  `run_5_thick`; everything else wears one to four specks. The style's whole
  identity currently hangs on one piece that most builds never earn.

### 1.5 The numbers under the renders (from the bench report, one run)

Pieces placed per subject (industrial / armoured / civilian / salvage):

| Subject | ind | arm | civ | sal |
| --- | --- | --- | --- | --- |
| owners_l | 1 | 0 | 1 | 0 |
| lone_cell | 0 | 0 | 0 | 0 |
| run_2 | 0 | 4 | 0 | 0 |
| run_5 | 0 | 8 | 2 | 0 |
| run_5_thick | 25 | 34 | 24 | 24 |
| tee | 0 | 10 | 2 | 0 |
| cross | 0 | 18 | 3 | 0 |
| plate_open | 2 | 23 | 4 | 8 |
| inside_corner | 3 | 1 | 3 | 8 |
| fitted_hull | 17 | 4 | 4 | 15 |

Three findings that should drive the follow-up tasks:

1. **The one-line-piece degeneracy.** On `run_5_thick` all four styles are
   carried almost entirely by their edge line at x24 of 24 brinks (industrial
   25 = band 24 + stack 1; armoured 34 = strake 24 + cap 10; civilian 24 =
   stripe 24 alone; salvage 24 = seam 24 alone). On a plain thick hull the
   styles ARE one rule each, distinguished by paint. This is the mechanical
   form of "they have not that much variety".
2. **Signature pieces with zero reach.** Across all ten subjects:
   `civilian_windows` x0 of 0 (never eligible ANYWHERE), `armoured_sensor`
   x0 of 0 (same), `industrial_radiator` reaches 2 plates on one subject
   only. The rarest, most characterful piece of each clean style is exactly
   the piece a hand-built hull never earns - their Flat + min_run + min_depth
   filters describe generated hulls, not player builds.
3. **Thin shapes are dressed by one style.** Confirms 20260816-142330:
   armoured's cap (Spur + seat Any + stride, no min_depth) is the only rule
   shape that fires on one-cell-thick builds. Industrial, civilian and
   salvage all gate their fillers on `min_depth 2` or on Flats that thin
   shapes do not have. Any new vocabulary MUST give every style at least one
   "thin-shape carrier" with the cap's filter shape.

## 2. Per-style art direction

The tone split, sharpened so armoured and civilian stop being "kind of equal".
The two cleans are different KINDS of clean:

- **Armoured clean is SUPPRESSION.** Function hidden under armour. Every
  piece low, flush, bolted, matte; detail clusters at corners and edges
  (structure), never mid-panel; colour budget: gunmetal plus ONE stencil
  white for markings, nothing lit, nothing specular. A piece earns its place
  by saying "sealed against fire". Fewer, sharper classes - the kit should
  stay the smallest of the four.
- **Civilian clean is FINISH.** Function hidden under styling. Pieces are
  faired, hull-coloured shells and painted lines; detail flows lengthwise
  along the hull (aero grammar), never clustered; colour budget: cobalt for
  paint, amber for anything lit, both already established. A piece earns its
  place by saying "designed, sold, branded". Windows and livery do the
  talking; machinery never shows.
- **Industrial is EXPOSURE.** Everything serviceable is on the outside with a
  part number. Yellow keeps its three-use discipline (edges, collars,
  handles). New pieces must be things a fitter would unbolt - no ornament.
  Detail clusters at fittings, because that is where the machinery is.
- **Salvage is ACCUMULATION, and the ham has a budget.** Mismatch is carried
  by MATERIAL (three patch hues) and OFF-AXIS authoring, never by piece
  count - the style comments already prove a deterministic scatter reads as
  haphazard through territories, crossing axes and off-centre recipes. The
  ham limit: at most one new accent hue beyond steel/rust/green, silhouette
  breakers stay rationed (whip at 0.18), and a region must keep sharing a
  dominant material - the moment adjacent cells stop agreeing on anything,
  it reads as confetti, which is the failure all the placement research
  exists to avoid. Scrap gets the most CLASSES, not the most pieces per
  plate.

## 3. The vocabulary matrix

Rows are object classes; cells are the class in that style's voice. Fixtures
are visual only - "function" is pretend, but every cell states its fiction
because that is what makes the same object read differently. `-` is a
DELIBERATE blank: restraint is the armoured/civilian look. `(exists)` marks a
shipped piece; `(stretch)` marks a cell that is designed but NOT in the first
batch, so the kit caps stay honest - a stretch cell only lands if the bench
render asks for it. Everything else is new and first-batch.

### 3.1 Shared core

Five classes are implemented by all four styles (edge line, tall piece, vent,
power cell, marking). Pipe and hatch carry stretch cells on the clean styles
and salvage - a warship and a sold ship HIDE their runs, which is the tone
split doing its job.

| Class | industrial | armoured | civilian | salvage |
| --- | --- | --- | --- | --- |
| **edge line** | hazard band (exists): yellow dashes on every straight edge | strake (exists): thick armour belt | stripe (exists): cobalt livery rail | weld seam (exists): hand-run bead |
| **tall piece** (antenna/mast) | stack (exists): heat stack with yellow collar | NEW `armoured_mast`: stub sensor spike, shortest of the four, raked, one matte radome - a warship hides its silhouette | fin (exists): raked aero blade | whip (exists): kinked aerial leaning overboard |
| **vent / grille** | louvre (exists): bolted grille bank | NEW `armoured_intake`: flush shuttered slit, armoured louvres angled shut | NEW `civilian_vent`: faired hull-coloured scoop, amber-lit slot | NEW `salvage_grille`: mismatched grate, one slat bent, sooted rim |
| **pipe / tube** | duct (exists): external conduit down the deck shoulder | `armoured_race` (stretch): square cable race under a bolted cover, low profile | `civilian_channel` (stretch): recessed service molding, hull-coloured cover strip | NEW `salvage_hose`: taped hose bundle sagging between two crude clamps |
| **hatch / access** | hatch (exists): round wheel-handle hatch | hatch (exists): flush square plate | NEW `civilian_door`: flush door, cobalt outline, amber dome light | `salvage_hatch` (stretch): hatch scavenged off another ship, wrong hue, welded proud |
| **power cell** (battery/tank) | NEW `industrial_cells`: open battery rack, yellow terminal collar | NEW `armoured_magazine`: low bolted box, one white stencil | NEW `civilian_tank`: faired tank blister, hull-coloured, seam line only | drum (exists); `salvage_jerry` (stretch): jerry-can cell lashed on with straps |
| **marking** (ammo stripes/stencils) | NEW `industrial_stencil`: unit number + hazard diamond panel | NEW `armoured_ammo_stripes`: white rounds-count stripes beside gun wells (the near_fitting slot, placed LAST) | NEW `civilian_registry`: registry pinstripe + maker's plate, cobalt | NEW `salvage_kills`: kill marks and painted-over older marks, misaligned |

The marking class is FLAT geometry like the hazard band (a decal drawn as a
thin box), so it is cheap, it works on any seat, and it is the class that
carries "ammo stripes" from the owner's brief. PG section 3.6's craft rule
supports the whole split: flat detail (panels, piping, marks) may sit
anywhere; chunky detail (drums, racks, cogs) wants recesses and clumps.

### 3.2 Per-style exclusives (deliberate asymmetry)

| Class | Style | Piece |
| --- | --- | --- |
| radiator bank | industrial | exists - stays exclusive, it IS the working-ship read |
| corrugated ribbing | industrial | exists (filler) |
| wheels / cogs | industrial | NEW `industrial_winch`: winch drum with cog flank, near fittings - deck machinery |
| corner boss | armoured | exists (`cap`) - stays exclusive, armour is about corners |
| sensor blister | armoured | exists - rule needs repair (1.5) |
| window row | civilian | exists - rule needs repair (1.5) |
| nav beacon | civilian | exists |
| smooth fairing | civilian | exists (filler) |
| patches x3 | salvage | exist - the identity, untouchable |
| tow cleat | salvage | exists (`hook`) |
| wheels / cogs | salvage | NEW `salvage_cog_patch`: a scavenged gear bolted flat over a hole - a cog used as armour |
| scorch | salvage | `salvage_scorch` (stretch): flat soot fan licking from a fitting - damage the patches answer |

Wheels/cogs is deliberately a two-style class: machinery on a working ship,
junk-as-armour on a raider. On the two clean styles it stays blank - a sold
ship and a warship have no exposed gears, and the blank IS the tone split.

### 3.3 Kit size after the matrix, against the pinned caps

First-batch counts (stretch cells excluded):

- industrial 7 + cells, stencil, winch = 10
- armoured 4 + mast, intake, magazine, ammo_stripes = 8
- civilian 5 + vent, door, tank, registry = 9
- salvage 7 + grille, hose, kills, cog_patch = 11

The caps (6-8) exist because "forty generated meshes were deleted from this
repo once". The doctrine - mismatch comes from placement and material, not
count - survives: these are CLASSES with one piece each, not variants. The
cap tests should be raised deliberately in the batch tasks to: armoured 8,
civilian 9, industrial 10, salvage 11, and pinned again. The relative order
(armoured smallest, salvage biggest) is itself the art direction and worth a
test assertion. Stretch cells (race, channel, salvage hatch, jerry, scorch)
spend the remaining headroom only if the tuning pass shows a hole.

### 3.4 Placement notes per new class (so the batch tasks start right)

- **Thin-shape carriers** (finding 1.5.3). One per style, copying the
  armoured cap's filter shape (cone-friendly, seat Any or Spur, min_height 1,
  NO min_depth 2): industrial_stencil (flat, any seat), civilian_registry,
  salvage_kills / salvage_cog_patch, armoured already has the cap. Markings
  are ideal carriers because a flat piece lies on anything (PG 3.6).
- **Pipes/races/channels** are LINES: follow the duct recipe (align Run,
  min_run, stride 1, near-full-cell length so neighbours join).
- **Power cells** are chunky: min_depth 2, patch floor, LOW share (the drum's
  measured lesson: 0.28 gave 13-14 drums, a scrapyard).
- **Ammo stripes** use near_fitting 1 and go LAST in the armoured list - the
  documented carpet trap, already pinned by tests in the other styles.
- **Vents** take the louvre's slot shape per style; civilian_vent should NOT
  use near_fitting (beacon already owns that slot there).
- **Zero-reach repairs** ride with the batches: windows and sensor need Step
  widening or a Brink side-facing variant; radiator needs a patch floor that
  actually fires on hand-built flats. Acceptance is measured on the bench
  report, not reasoned about.

## 4. Sourcing and production plan

### 4.1 Primitives first - the pipeline already exists and is the answer

The repo already generates every greeble from JSON recipes
(`scripts/greeble-recipes/*.json` -> `gen-greebles.py` -> deterministic
`.glb`, box/cylinder/taper/disc/ribs, 200-tri cap, CI `--check`). 27 shipped
recipes prove a solo dev maintains this. All ~17 new pieces in section 3 are
expressible in the current primitive set; the only candidate addition is an
`elbow`/bend primitive if the hose/pipe classes want visible turns (defer
until a recipe actually needs it).

This is also the licence position: recipes are original work, MIT with the
repo, no third-party art enters `assets/base`.

### 4.2 External CC0/CC-BY kits - reference, not ingestion

Verified this session:

- Kenney Space Kit: CC0, ~150 models (kenney.nl/assets/space-kit). The
  project already ships Kenney-derived ship parts, so the licence path is
  proven.
- Quaternius Ultimate Space Kit: CC0, 92 models, glTF among formats
  (quaternius.com); several more sci-fi/modular packs on the same site.
- Kay Lousberg "Space Base Bits" (kaylousberg.com): licence NOT verified
  this session - check before any use.

Position: use packs as SILHOUETTE REFERENCE for recipes, do not ingest the
meshes into the greeble folder. Three reasons: (a) the folder's contract is
"nothing here is hand-modelled" with a determinism check - foreign binaries
break it; (b) pack detail density and material style do not match the
flat-shaded one-primitive-per-colour kit look; (c) the cell frame (+Y out,
centred, half-cell footprint budget) means every import needs rework anyway.
A webmod remains free to ship pack-based greebles through `dep://` - the
contract already allows it, and that is where pack ingestion belongs.

Red flags: Blender's Discombobulator greeble add-on is GPL - never vendor its
code (its OUTPUT would be fine, but the recipe pipeline makes it pointless).
PG's licence table stays the single point for everything it already lists.

### 4.3 The craft vocabulary (why a shared class set is the right design)

The film lineage says greebles were always a KIT reused across subjects:
Star Wars models "began as simple shapes... given visual complexity by
attaching greebles taken from commercially-available model kits", used "to
imply mechanical function without necessarily having any real purpose" and
"to create an illusion of scale" (Wikipedia, "Greeble"; facts only, CC-BY-SA
text not reused). Different ships on one film shared one parts bin - the
same object appearing in different paint and context is the historical norm,
which is exactly the owner's "similar objects looking different". The
per-style rules already banked in PG (recess chunky detail, clump heights,
avoid even spacing, scale families per Blevins) apply to every new recipe and
are not repeated here.

## 5. The faction angle (brainstorm only, no commitment)

Styles as factions if the campaign grows:

- UNLOCKS: instant IFF at range (the render already proves hue reads before
  silhouette); campaign factions become content overlays (a style id plus a
  kit) with zero engine work; webmods get "new faction" as a content-only
  mod; the matrix above IS the faction design language - each faction is a
  column.
- COSTS: a new faction is a full column (7-10 recipes + rules + tuning on
  the bench) - call it one style-batch task each; styles are cosmetic today,
  so faction MEANING (AI, allegiance, spawn tables) is separate work the
  skin should not be mistaken for; four voices are distinct now, but each
  additional faction makes hue collisions likelier (rust vs industrial
  yellow-brown already flirt).

Block ships as the mainline cast instead of the Kenney casts:

- UNLOCKS: one art pipeline for player builds AND cast ships - every skin
  improvement pays across the whole game; fixtures have health, so greebles
  shoot off derived hulls (damage state for free); scale coherence (cast
  ships and player builds share the cell); the editor's build vocabulary and
  the campaign's art stop being two systems.
- COSTS: silhouette ceiling - blocky hulls cannot yet match a modelled
  yacht's swept lines; the bench expansion (section 7) exists precisely to
  test whether hand-placed blocks reach "this can be a ship"; the shipped
  casts (cargoa corvette, racer yacht) would need block rebuilds and the
  scenario art would visibly change; plate meshes cost more triangles than
  one authored glb per ship (measured cheap so far, but a fleet of big
  blocks is a new measurement).
- HONEST MIDDLE: nothing forces a binary. The cast can stay Kenney while
  block ships take over generated/NPC traffic; promote block casts one ship
  at a time when a bench shape earns it.

## 6. Spec: greeble catalog example

`examples/screenshots/greeble_catalog.rs` - present every available greeble,
parts-preview style.

- App shape: `with_game_plugins` (needs the asset scheme and `GameStyles`),
  the shape_bench pattern. Read the MERGED `GameStyles` after load and lay
  out every `StyleFixtureConfig` - a mod's fifth style appears with no code
  change.
- Layout: one ROW per style (authored order, placeholder last), one column
  per fixture. Each piece stands on a one-cell pedestal PLATE tinted with
  that style's `Top` surface colour and roughness - a greeble is only
  judgeable against the plate it will stand on. Label under each: fixture id;
  a second line with collider extents and health (both from the config).
- Camera: the shape_bench `IdleOrbit` verbatim (resource armed when not
  `capturing()`, `stop_orbit_on_input` on the WASD rig, orbit system in
  `PostUpdate` after `WASDCameraSystems::Sync`, before
  `TransformSystems::Propagate`).
- Focus mode, the parts_viewer idiom: arrows move a selection ring; Enter
  focuses one piece large on a turntable (`spin_focused`:
  `transform.rotate_y(rate * dt)`); left/right steps within the style; Esc
  returns to the wall.
- Keys: `L` snaps the camera to the next style row; `C` toggles the
  pedestals (piece against void); `G` toggles a unit-cell wireframe so the
  half-cell footprint budget is visible - the recipe habit the README
  documents becomes checkable by eye.
- Harness: `NOVA_AUTOPILOT=1` smoke (load, frame, exit);
  `NOVA_CAPTURE=1` shoots `greeble-catalog.png` (the wall) plus one
  `greeble-catalog-<style>.png` per row, scripted steps with the shared
  deadline constants.
- Report: one line per fixture - id, model path, collider, health, and the
  rule summary (relief list, seat, align, share/floor) - so the catalog is
  the one place a reviewer sees model and rule together.
- Definition of done: every fixture in merged content appears labelled; a
  content-only mod style appears without touching the example; one command
  produces the wall shot.

## 7. Spec: bench expansion - the building blocks roster

The current roster (`examples/screenshots/shape_bench.rs`) is ten DIAGNOSTIC
subjects - primitive neighbourhoods, up to 5x2x2. The owner's bar for the new
set is different: "this thing looks nice, it can be used for an actual ship".
Add a second SET, selected by `--set diagnostics|blocks` (default
diagnostics, so every existing capture and number stays comparable).

Proposed blocks roster (cells; all wear fittings where noted so the pocket
rules read):

| Subject | Cells | What it tests |
| --- | --- | --- |
| `wedge_8` | 8x2x3 body, bow stepping to 1 wide over the last 3 cells | the destroyer bow; Brink runs meeting a taper |
| `spine_freighter` | 2x2x9 spine + three 3x2x2 saddle blocks | the classic freighter; repeated modules; long edge lines |
| `outrigger` | 6x2x2 body + two 4x1x1 pontoons on 1-cell struts | thin appendages on a thick body; both regimes on one ship |
| `tower_ship` | 3x3x2 under 2x2x2 under 1x1x1, vertical | the Homeworld-style vertical mothership silhouette |
| `carrier_deck` | 6x1x4 deck over a 4x2x2 hull, drive aft | the biggest Flat field a build can make; where radiators/windows/sensors must finally fire |
| `trench_hull` | 7x3x3 with a 1-wide, 1-deep dorsal trench | PG 3.6's recess rule: chunky greebles down IN the trench, flat detail on top |
| `owners_l_2x` | the owner's L with every cell doubled to 2x2x2 | the same shape at the thickness where Flat/Brink exist |
| `asym_gunship` | 5x2x2 with a 1x1x3 boom on one flank + PDC on the boom | deliberate asymmetry; fittings on thin structure |

- Spacing: the blocks set needs its own COLUMN/ROW spacing derived from the
  set's max extent (9 cells + margin); keep the existing camera-span math,
  feed it per-set constants.
- Everything else inherits: labels, freeze_bodies, `L`/`C` keys, the
  per-style report, the capture script (shot name gains the set:
  `shape-bench-blocks.png`).
- Definition of done: one command renders the blocks set clad in any style;
  every subject passes the exit lint; the per-style report covers the new
  subjects; the owner can point at >=3 subjects and say "cast-worthy" (that
  judgment call, not this task, decides the faction/mainline question in
  section 5).

## 8. Recommended follow-up tasks, ordered

Sizes: S = about half a lane-day, M = one to two lane-days.

1. **Style rule repair** (S, rules only, no new art). Fix the three
   zero-reach signatures (windows, sensor, radiator) and give civilian +
   salvage a thin-shape carrier by loosening one existing filler each.
   DONE WHEN: bench report shows every style placing >=1 piece on every
   subject except lone_cell, and the three signatures reach >0 on at least
   three subjects.
2. **Bench expansion: blocks roster** (M). Section 7. Build it before the
   art batches so they are judged on ship-worthy shapes.
   DONE WHEN: its own definition of done above.
3. **Greeble catalog example** (S-M). Section 6.
   DONE WHEN: its own definition of done above.
4. **Vocabulary batch A: the clean styles** (M). Armoured mast, intake,
   magazine, ammo stripes; civilian vent, door, tank, registry. Eight
   recipes + rules + cap-test raise (armoured 8, civilian 9).
   DONE WHEN: catalog shows the pieces; blocks-set renders per style pass
   owner review; ammo stripes sit beside gun wells on `asym_gunship`.
5. **Vocabulary batch B: the working styles** (M). Industrial cells,
   stencil, winch; salvage grille, hose, kills, cog_patch. Seven recipes +
   rules + cap raise (industrial 10, salvage 11).
   DONE WHEN: same bar as batch A; salvage thin subjects no longer bare.
6. **Art-direction tuning pass** (S). Densities, shares and palette nudges
   on the full blocks bench; re-pin the kit-size ordering test (armoured <
   civilian < industrial < salvage).
   DONE WHEN: four matched renders of the blocks set land in the closing
   task record and the owner re-ranks the styles.

The faction / block-mainline question is deliberately NOT a task: it is a
decision the owner takes after task 2 and the batches show what block ships
look like dressed.
