# Wiki widgets pass

Widgets and the prose around them. Screenshots and loops are a separate lane.

## Widgets added

Three, registered in `web/src/widgets.ts` and hydrated by `data-widget` key.
Each carries static fallback prose that states its result on its own.

- `ammo-rhythm` - `wiki/combat-weapons.md`, "Ammo & reloading". A magazine as a
  RATE LIMIT. Replays a burst-and-pause trigger pattern against the reload
  rule and plots the level. The point it makes that the prose could not: the
  batch is all-or-nothing on a WHOLE quiet interval, so a pause a tick short
  of the delay returns nothing, and any shot that lands restarts the interval.
- `turret-arc` - `wiki/sections/turret.md`, new "What it can bear on". Traverse
  is unbounded, so the entire blind volume is the cone under the mount's own
  keel. Draws the covered band, the hull sitting in the blind wedge, and the
  slew cost of a swing. This is the geometry `combat-weapons.md` spends three
  paragraphs on under "Point defense" and "Barrel discipline"; both now link
  here instead of re-explaining it.
- `torpedo-run` - `wiki/sections/torpedo-bay.md`, new "The two run-ins". Lance
  and Serpent race one 300 u run-in under one transport clock, with the weave
  tapering out over the terminal band. A defender toggle marks where one stock
  PDC kills each. Arming it is the whole argument: the Lance dies 114 u out,
  the Serpent 40 u out.

No new `:root` tokens and no new mark classes - all three reuse the existing
widget vocabulary, so `tests/theme.test.ts` checks (d)/(e)/(f) are untouched.

## Tables, not widgets

- `wiki/keybinds.md` rebuilt as three columns - action, keyboard & mouse,
  gamepad - with a `<thead>` per table, behind a new `.controls--split`
  modifier in `style.css` (plus a narrow-viewport rule). The old form crammed
  both devices into one cell joined by "or", so a gamepad player had to pick
  their half out of every row. The Editor table stays two columns: the editor
  has no gamepad bindings, and a column of dashes is not information.
- `wiki/settings.md` already leads with a preset table and folds the detail
  behind `show-explanation`. Left alone.

## Pages deliberately given nothing

- `glossary.md` - 25 lines of definitions. Prose is the correct form.
- `getting-started.md` - a walkthrough. Its visuals are the authored capture
  slots (other lane), and a widget would restate the pages it links to.
- `scenarios.md` - a list of authored content. Nothing to compute.
- `sections.md` - overview plus catalog table; the mechanisms live on the two
  child pages that got widgets.
- `sections/hull.md` - a hull is passive. Its only number is health, which the
  catalog table already carries.
- `sections/thruster.md` - REJECTED ON SOURCING. A "more thrusters, more push"
  widget needs acceleration, which needs hull mass; mass is not authored, it
  falls out of section geometry and density. No number to lift, so no widget.
- `nova-os.md` - long (334 lines) but already widget-first: five tables, five
  `show-explanation` folds and a widget. Length here is reference material,
  not a wall of text.

## Numbers lifted, with source

Constants are in `web/src/widgets.ts` with the `file:line` on each comment;
page numbers carry a verification comment in the markdown. Paths below are
under `crates/`.

Turret mount (`nova_authoring/src/base_content/sections/standard.rs`):

- traverse limits `None`/`None` :142-143 (unbounded)
- elevation min `-TURRET_DEPRESSION_LIMIT` :157, that constant `PI / 18` :84
- elevation max `FRAC_PI_2` :158
- hinge speed `PI` rad/s :141,:150
- muzzle speed 100.0 :273, projectile lifetime 2.0 :280
- `nova_ship/src/sections/turret_section/config.rs:124-126` states that product
  IS the reach (200 u)
- fire gate `TURRET_ON_TARGET_RAD = 1.6 / 100` at
  `nova_ship/src/sections/turret_section/aim.rs:19,:24,:47`, the source's own
  comment reading 0.016 rad / 0.92 deg at :26-27
- reachability derived from the elevation hinge alone,
  `nova_ship/src/sections/turret_section/arc.rs:46-102`
- 58.7% sky coverage is DERIVED in the widget from the depression floor,
  `(1 - sin(floor)) / 2`. Not an authored number.

Magazines and reload:

- PDC `ammo_capacity: Some(500)` standard.rs:286, `delay: 3.0` :288,
  `amount: 200` :289, `fire_rate: 100.0` :262
- bay `ammo_capacity: Some(6)` :591, `delay: 10.0` :603, `amount: 1` :604,
  `fire_rate: 1.0` :558
- semantics in `nova_ship/src/sections/ammo.rs`: a successful shot resets the
  clock :136, a whole batch lands at the delay :171-174, clamped at capacity
  :156, empty trigger pulls never reset :134
- sustained rate formula `amount / (delay + amount / rate)` is standard.rs:594's
  own, which works the PDC through it to 40 rounds/s

Torpedo types (`nova_authoring/src/base_content/sections/ordnance.rs`):

- the measured comparison table is the module header, :13-21 - cruise caps,
  weave half-angles, rounds one PDC spends (116 / 390), where it kills each
  (114 u / 40 u), time over a 300 u run-in (9.10 s / 9.78 s), speed along the
  line (31.3 / 29.1 u/s)
- closing rates against a 25 u/s runner (6.3 / 4.1 u/s) at :23-25
- Lance `LANCE_MAX_SPEED = 35.0` :49, weave zeroed :72-73
- Serpent `max_speed: 32.0` / `weave_angle: 0.44` / `weave_rate: 1.4` at
  `nova_ship/src/sections/torpedo_section/mod.rs:347-349`
- the 11.1 u swing off the direct line at that angle and rate, mod.rs:293-303
- weave taper full beyond 3x blast radius to zero at 0.5x,
  `nova_ship/src/sections/torpedo_section/projectile.rs:330-334`
- blast radius 30.0 standard.rs:565
- warhead `projectile_health: 10.0` standard.rs:583, sized above the hardest
  single PDC round with the reasoning at :580-582; siege bay 5000.0 at :509

One modelling decision worth recording: the widget drives each torpedo's
position from the MEASURED arrival time for this run-in, never from the cruise
cap or from the speed along the line. Those are three separate measurements of
three separate questions, and 300 / 9.10 is not 31.3. Dividing one by another
put the Serpent 15 u short of a target the harness says it reaches.

## Numbers already on a page that no longer match the source

Three of these, all the same defect: a world unit read as a metre, when
`nova_events/src/scale.rs:14` puts `METERS_PER_UNIT` at 10.0. All three FIXED.

1. `wiki/combat-weapons.md` - "the fuze fires about a metre off the nearest
   part of that body's skin". `CONTACT_FUZE = 1.0` at
   `nova_ship/src/sections/torpedo_section/projectile.rs:76` is one world unit,
   so it is about TEN metres.
2. `wiki/sections/torpedo-bay.md` - the same claim, same constant, same error.
3. `wiki/combat-weapons.md` - "a round that lands within about a metre of the
   last one joins that crater". `MERGE_MAX = 1.0` at
   `nova_gameplay/src/integrity/carve.rs:128` is one world unit, so ten metres.
   The `[Unreleased]` changelog entry for the same change says "within a unit
   of itself", which is right; the wiki converted it wrongly.

Two documentation-rule defects, also FIXED:

4. `wiki/sections/turret.md` claimed "the ten per-craft `*_turret_*` modules
   ... have been removed". That is history on a reference page (Documentation
   1), and the removal is only in `CHANGELOG.md` `[Unreleased]` (line 28), so
   there is no RELEASED section naming the old prototypes (Documentation 2/3).
   Clause dropped; the page now just says every craft mounts the same gun.
5. `wiki/combat-weapons.md` - "A warhead NOW carries enough hit points that no
   single PDC round can swat it". "Now" frames the reader against the last
   commit rather than the last release. Reworded to state the fact.

One claim left alone but flagged:

6. `wiki/combat-weapons.md` - "the difference between a salvo that costs a
   defender a magazine and one that costs it four". No measurement produces
   "four". The type sweep gives 3.36x (390 / 116, ordnance.rs:13-21); a
   separate 400 u angle sweep gives 4.04x (1245 / 308,
   torpedo_section/mod.rs:280-289); a six-torpedo salvo costs 696 vs 2340
   rounds, which is 1.4 vs 4.7 PDC magazines of 500. Defensible as colour,
   not lifted from a measurement. Retuning prose numbers to fit is exactly
   what Web 2 forbids, so it stays until an owner decides which rig the page
   should quote. The neighbouring "roughly three times the ammunition" IS
   sourced.

## Belongs on another surface

Flagged only - `docs/` is another lane's and neither was changed.

- `wiki/nova-os.md`, "In the WFC arena". Documents
  `cargo run --example wfc_arena`, a developer match bench run from a source
  checkout. A player cannot reach it. This is dev-book material on a player
  page.
- `wiki/scenarios.md`, the "Show the full vocabulary" fold. The events /
  filters / actions list is the authored contract, which is `/create/`'s job
  and has to be exact there. The page already links to `/create/` at the end;
  the fold duplicates it in a place that cannot be held exact.

## Verification

- `npm run ci` in `web/` - format, lint, test, build.
- Rendered and INSPECTED every touched widget in headless Chromium against the
  built `dist/`, including the `torpedo-run` defender state and the
  `ammo-rhythm` dry case. Three collisions were found that way and fixed: a
  depression-floor label over the hull block, torpedo lane labels under the
  weave, and the "ran dry" marker sitting on the trace.
- The site ships ONE skin (phosphor on dark). There is no light theme and no
  `prefers-color-scheme` branch in `style.css`, so there is no second theme to
  check; `tests/theme.test.ts` check (f) is what keeps the retired light-3D
  skin out.

# Section-page widgets pass

Second pass on the same lane. The first attempt was killed mid-flight and its
work landed as an unverified WIP commit; this section is the audit of that work
plus the fixes it needed. Four widgets, three pages.

## What each widget shows

`controller-arm` - `wiki/sections/controller.md`, "What sets how hard a ship
turns". The shipped corvette in plan view with its balance point, a ring at its
structural arm, and the `8 G / arm` curve beside it with the hull's point read
off it. Nine buttons shoot sections off. Losing a part moves the balance point,
moves the arm, moves the point on the curve, and severs anything the structural
graph no longer reaches.

`controller-margin` - same page, under "A hard turn spends the margin". One
slider: the rate the hull is already turning at, against what is left to turn
HARDER with. Flat, then a cliff, then the whole budget back in the over-spun
band so a rammed ship can brake.

`hull-armour` - `wiki/sections/hull.md`, "Variants". The ten catalog hull parts
ranked two ways. Opens on the catalog's own order (health) and the toggle
re-ranks by health per unit of mass, which inverts it.

`thruster-mass` - `wiki/sections/thruster.md`. Acceleration against basic drives
added, one curve per shipped hull, with a hard ceiling at 64 u/s2. Two sliders:
which hull, how many extra drives.

## Why the controller pair answers the complaint

The old `controller-stacking` failed on four counts. Against each:

1. **Log scale over five decades.** Neither replacement uses one. `controller-arm`
   plots ceiling against ARM over 1.0-3.0 u linear, where `8 G / arm` is a
   visible hyperbola and the hull's move along it is a visible move.
   `controller-margin` is linear on both axes and its whole point is the SHAPE
   of one curve.
2. **X axis answered the stacking question.** Both new x axes are the ceiling's
   own variables: structural arm, and current turn rate. Stacking is a
   different question and it still has the measured flip-time table under
   "Stacking controllers", which was left alone.
3. **Subjects were test fixtures.** Both fly `cargoa`, the shipped corvette,
   assembled from its own authored boxes. `thruster-mass` flies all three
   shipped hulls. No rig from `flight/tests/stacking.rs` appears in a widget.
4. **The page's three best claims had no visual.** All three now do.
   "Every shipped ship is held by the STRUCTURAL ceiling with a wide margin" is
   the `what binds` stat and the 41.9-against-2.84 readout. "A wreck turns
   sharper" is the NOSE button: 2.76 u -> 2.43 u, 2.84 -> 3.23 rad/s2, a 180 in
   1.97 s against 2.10 s. "A hard turn spends the margin" is the whole of
   `controller-margin`.

Nothing was cut. Both earn their place: they answer two different claims and
neither could carry the other's.

## Numbers lifted, with source

The model. Paths under `crates/`:

- `nova_events/src/scale.rs`: `METERS_PER_UNIT` 10.0 :14, `LOAD_LIMIT` 8*9.81
  :23.
- `nova_ship/src/physics/attitude.rs`: the two ceilings and the lower one
  :72-93; the arm to the outer FACE of the furthest section :146-166, whose own
  doc reads 2.76 u for the shipped corvette :140-141; assembled mass properties
  :175-186; sustained rate `sqrt(structural)` :108-110; `available(spin)` and
  the vector addition :121-130.
- `nova_ship/src/sections/base_section.rs`: density 1 and not authorable, so a
  section's mass IS its authored box :376; the unit-cube fallback for a section
  with no authored collider :79-85.
- `nova_ship/src/sections/thruster_section.rs`: magnitude is an IMPULSE PER
  FIXED TICK with no `dt` :276-295 and :370-373, at Bevy's own 64 Hz :289-292.
  Ships carry no `LinearDamping` and no speed cap, so `100 / a` is the honest
  sprint time.
- `nova_ship/src/sections/integrity.rs`: a cut that disconnects the graph
  severs it, and the body holding the computers keeps ship identity :231-349.
- `nova_editor/src/attitude.rs:18-70` is the reference implementation the
  widget's `hullState` mirrors: same colliders, same density 1.0, same
  `structural_arm` off the measured centre of mass.

The hulls. `nova_authoring/src/base_content/`:

- `ships/cargo_a.rs` parts :16-96 and structural mates :98-108; `ships/racer.rs`
  parts :13-88 with the base assembly taking the meshed seven :107-115;
  `ships/cargo_b.rs` parts :9-82. All assembled through `ships/shared.rs:44-50`
  (centre and size off the authored box) and `:235` (the collider IS that box).
- `sections/standard.rs`: `PDC_TURRET_SIZE` 0.5 :71 and the PDC's own cube
  collider :240-242, which is what actually lands on a mount POINT;
  `TURRET_BASE_HEALTH` 130 :32; `max_torque` 1501 :384 (and `shared.rs:302` for
  the fuselage computers); thruster `magnitude` 1.0 :354 with no collider :337;
  `reinforced_hull_section` 200 :308 no collider :311; `light_hull_section` 60
  :408 no collider :411.

Derived in the widget, not authored anywhere - which is the argument all four
make:

- corvette mass 15.86, largest principal inertia 35.79, arm 2.7615 u. That arm
  matches `attitude.rs`'s own 2.76 u to two decimals, which is the check that
  the assembly model is the game's. It only matches WITH the two turret mount
  cubes counted; without them it reads 2.74.
- structural ceiling 2.8435, torque ceiling 41.94, ratio 14.8, 180 flip 2.103 s,
  sustained rate 96.6 deg/s.
- nose off: 6 sections held, both turrets adrift, arm 2.431, ceiling 3.2296,
  flip 1.973 s, balance point z 0.235 -> 0.661 (aft).
- margin: 97 % left at 48 deg/s, 83 % at 72, 0 % at 97.
- hull masses 8.28 (racer), 15.86 (cargoa), 18.95 (cargob); two drives each, so
  15.45 / 8.07 / 6.75 u/s2, and `n` drives asymptote to 64.
- armour per mass, best to worst: CargoA Pod 216.1, Reinforced 200, Racer Tail
  136.3, Racer Nose 119.6, Racer Wing 105.3, CargoB Tail 94.6, CargoA Tail 83.0,
  CargoA Nose 72.1, CargoB Nose 61.5, Light 60. Spread 3.6x.

## Defects found in the inherited work

Every number in the WIP commit was recomputed from the Rust and every one of
them held, including the `METERS_PER_UNIT` conversion in the thruster widget's
`in G` stat, which is the trap that has fired three times this cycle. What was
wrong was elsewhere:

1. **`controller-arm`'s plan view was mirrored.** Captioned "from above" and
   drawn from BELOW. A ship faces -Z with starboard at +X, so a plan view with
   the nose at screen left puts starboard at the TOP; the widget mapped +X
   downward. Every port/starboard label was on the wrong side - POD P above the
   fuselage, POD S below, and the same for the drives and the two severed
   turrets. Geometry, arm and ceiling were unaffected (the hull is
   symmetric), which is why nothing else caught it. FIXED: `py` runs against
   +X, the section rect takes its top edge from `+size/2`, and the arm ray's
   cross-axis term is negated to match.
2. **The fallback prose and the live widget disagreed on the same number.** The
   page fallback says the computer offers "41.9"; the widget printed
   `toFixed(0)`, so a reader with JS saw "42" for the identical claim. FIXED:
   the stat and the readout carry one decimal.
3. **`hull-armour` opened on the answer.** It defaulted to PER MASS, while its
   own fallback prose, the sentence above it on the page, and its own header
   ("Switch the ranking and watch it come apart") all put the reader on the
   HEALTH ranking. FIXED: opens on the catalog's order; the switch is the
   argument.
4. **`hull.md` carried two contradictory citations for the same two numbers.**
   The catalog table's comment cites `standard.rs:292` and `:434` for the two
   unit-cell healths; the real lines are :308 and :408, which is what the
   lane's own new comment two lines above says. Pre-existing drift, but the
   page now stated both. FIXED to :308 and :408.

Checked and NOT defects:

- No orphaned prose. Nothing on `controller.md` referred to the removed
  `controller-stacking`, and the measured flip-time table under "Stacking
  controllers" is intact.
- No new `:root` token and no new mark class. All four widgets reuse the
  existing vocabulary, so `tests/theme.test.ts` (d)/(e)/(f) are untouched and
  `style.css` was not opened.
- `.widget-mark--gate` has no `fill: none`, and `controller-arm` sets it inline
  on its curve path - the one place the omission would have flooded the plot.
- All four `data-widget` blocks carry fallback prose that states a result on its
  own, and every number in all four was verified above.
- "fixed tick" and "collider" read as engine words on a player page, but both
  are already the wiki's own vocabulary (`combat-weapons.md:75,:221,:225,:231`),
  so the notes were left in the house voice rather than re-litigated here.

Found while reading, NOT fixed - `crates/` is another lane's:

- `ships/cargo_a.rs`'s module doc says the corvette's turrets sit on "their
  forward shoulders" of the pods. `CARGOA_EDGES` :105-107 and the comment at
  :87-93 both put them on the NOSE cheeks. The doc is stale; the widget follows
  the edges.

## What was rendered and checked

Built `dist/`, served it, and inspected every widget in headless Chromium at
1280 px wide, plus the DOM text of each hydrated block:

- `controller-arm`: intact, NOSE out (severs both turrets), FUSELAGE out (the
  derelict fault state - every section severed, stats blanked, `is-fault`
  readout), and a five-section wreck. Re-rendered all of them after the mirror
  fix.
- `controller-margin`: 0, 72 (the default) and the over-spun band past 97 deg/s,
  where the recovery segment comes back to full authority.
- `hull-armour`: both rankings, before and after the default flip.
- `thruster-mass`: all three hulls, +0 and +8 drives.

One skin only. There is no light theme: `style.css` has no
`prefers-color-scheme` branch and no `data-theme` selector, and
`tests/theme.test.ts` exists to keep the retired light-3D vocabulary
unconsumed. Both themes were asked for; there is one, and it was checked.

## Cut

Nothing. `controller-stacking` was already gone in the inherited commit and its
replacement pair is the right trade - the page's remaining stacking content is a
measured table, which is the correct form for four rows of flown timings.
