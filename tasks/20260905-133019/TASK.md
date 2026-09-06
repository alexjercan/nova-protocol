# WFC in the editor: generated hulls, template maps, and Save As

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.13.0,editor,generation

## Goal

The wave-function-collapse generator is the most interesting thing in the
repository that the game cannot reach. It lives in
`examples/playable/shared/wfc.rs` behind a `#[path]` include, so only
`wfc_ships` and `wfc_arena` can run it. `wfc_arena` also dresses the best map
the project has, and no one can start a document from it. Meanwhile the editor
seeds every new document with one hardcoded sandbox range and writes every save
over one hardcoded mod id.

Four strands, one arc: make the collapse base-game code, put it behind an
editor verb, let a new document start from a map worth starting from, and let a
save name its own bundle.

## 1. The collapse becomes a crate

`examples/playable/shared/wfc.rs` is 2043 lines and builds no App. It needs
`GameSections`, the scenario config vocabulary, the lint, and `rand`. That is a
library, not an example include.

A new `nova_wfc` crate over `nova_ship` + `nova_scenario` + `nova_events` +
`rand`. `nova_editor` already depends on all three, so the editor picks it up
with one line. What moves: `Tile`, `tile_set`, `wfc_hull`, `style_at`, and the
placement/erosion machinery behind them. What does NOT move: the arena's own
stamps. `stamp_large_drives` and `stamp_spinal_lance` are proof-of-concept
stamps applied outside the grammar, and the module doc already says neither is
the future ship grammar. They stay with `wfc_arena` until something decides
they are content. `refuse_broken_ships` is an example assertion over
`lint_scenario`; the crate exposes the lint pass, the example keeps the panic.

THE DRAW TABLE IS THE REAL DESIGN QUESTION. Adjacency already comes from
content: `tile_set` reads the link points off `GameSections`, so a mod that
adds a section already changes what may sit next to what. But `PARTS` is a
const array of six `&'static str` prototype ids with a hand-tuned `weight` and
an `aim` face. A base-game generator that a mod cannot join is a generator with
the catalog hardcoded into it. The weight and the aim belong in authored
content, per the explicit-authoring rule: an unknown id is an error at lint,
then at load, and a fifth section joins the draw the same way a fifth style
joins the rotation today. Decide whether that is a field on the section config
or a separate authored draw table, and write the choice down before the move.

Both examples must still run and still produce the hulls they produce now. The
seeds are reproducible, so this is checkable: same seed, same hull.

## 2. WFC in the editor

The editor is where a builder wants a hull they did not draw by hand. Put the
collapse behind a verb: generate a hull into the document as ordinary section
nodes, then edit it like any other ship.

The output is `ShipHull`, and the document already lifts a hull into nodes -
`insert_lifted_section` in `node.rs`, which is what `lift_content` uses to read
a saved bundle back. So a generated hull is the same road as an opened file,
short one parse. That is the whole integration.

What the verb needs on screen: a seed (shown, editable, rerollable - the arena
lobby at `examples/playable/wfc_arena/lobby.rs` already has this exact control
and is worth reading), a style, and a cladding toggle. Where it lands: at the
placement cursor, or as a new ship node beside the others. It must NOT silently
replace the ship being edited; a generate is an add.

A generated hull must pass `lint_scenario` before it enters the document, and
the editor must say so when it does not, in the status line. The generator is
seeded and the editor is not; a hull the collapse produced but the game would
refuse is a bug in the generator, and swallowing it in the editor hides it.

## 3. Template maps

Today `File > New Scenario` always founds the same document:
`default_world_objects` + `default_script` in `crates/nova_editor/src/scenario.rs`
- two rock belts, a hulk corridor, three dormant pickets, two beacons, one
planetoid, one light rig. It is a good range. It is also the only one.

Make the seed a CHOICE. `New Scenario` opens a template picker; the current
sandbox stays the default entry.

The second entry is the arena map. `wfc_arena` dresses rock rings below the
fight plane for parallax, one pinned planetoid, derelict junk scattered into
three flank blobs, and a three-point rig - `rock_ring`, `derelicts`,
`planetoid` in `examples/playable/wfc_arena.rs`. Every one of those is already
an `EventActionConfig::SpawnScenarioObject`, which is exactly what the editor's
layout lowers to. So a template is a `Vec<ScenarioObjectConfig>` plus a script,
and both existing seeds are already that shape.

WHERE A TEMPLATE LIVES is the decision. A const in `nova_editor` is the cheap
answer and the wrong one: a template is content, and a mod should be able to
ship one. Authored content, read through the merged catalog like everything
else, is the answer that matches the rest of the project. Cost it before
committing; a const with a written note that says why is acceptable for the
first pass if the authored form is a task of its own.

Derelict junk is generated from seeds today. A template that bakes 30 fragment
hulls into a document is a document nobody can read on the rail. Either the
template seeds a scatter action and stays small, or junk is left out of the
template and offered as a separate "scatter wreckage" verb. Prefer the scatter
action: the script half of the document already carries that kind of thing.

## 4. Save As

`SAVE_MOD_ID` is `"editor_save"`, one slot, and the doc comment in
`crates/nova_editor/src/bundle.rs` already names this as its own task: "A file
browser and a name field are their own task." `File > Save As...` exists in the
menu today, greyed with a `soon` tail (`crates/nova_editor/src/ui/mod.rs`).
Build it.

There is no OS folder dialog in this project and there should not be one: the
editor runs on the web too, and the store there is IndexedDB. The unit is
already right - a save is an installed MOD BUNDLE, and `install_local`,
`read_index`, `upsert_index_record` and `remove_mod` in
`crates/nova_assets/src/mod_cache.rs` are the whole filesystem the editor
needs. So "Save As" is a bundle picker over the mod index plus a name field,
not a folder tree.

What that costs:

- A document remembers WHICH bundle it came from. `Save` writes there, `Save
  As` asks. A document that was never saved sends `Save` to the ask.
- The id is derived from the name and must be a safe id (`is_safe_id` is
  already in `mod_cache`). A collision is an overwrite and must be confirmed.
- `File > Open` becomes the same picker, listing editor-written bundles. It
  reads one file today.
- The read-only rule was structural: the editor could only ever write one id,
  so a hand-written mod was out of reach. That property is now a check, not a
  shape. Editor-written bundles carry a marker in their `ModMeta` and the
  picker only offers those; opening a hand-authored mod bundle for edit is NOT
  in scope.
- `SAVED_RANGE.id` and the retry-scenario id inside the document are tied to
  `editor_save` (see `bundle.rs` and `scenario.rs:1183`). Both become the
  chosen id, and the existing test at `bundle/tests.rs:221` covers the failure
  this causes if they drift.

WEB IS THE OPEN QUESTION. `write_save` is `Err("saving is not available on the
web yet")` under `cfg(target_arch = "wasm32")` because the cache is async and
the save is not. Save As does not fix that and must not pretend to. Native
first; say plainly in the task's proof whether the web path is deferred.

## Order

1 unblocks 2. 3 and 4 are independent of both and of each other. 4 is the one a
builder feels every session, so it is the one to land first if the arc is split.

## Proof

- Both wfc examples run and produce the same hulls for the same seeds after the
  crate move.
- Unit: a generated hull lifts into document nodes and lowers back unchanged.
- Unit: a generated hull that fails the lint does not enter the document, and
  the status line says why.
- Unit: each template founds a document whose lowered layout matches the
  template.
- Unit: Save As writes under the chosen id; Save then rewrites the same id; a
  colliding id is refused without confirmation.
- A live editor run: generate a hull on the arena template, save it to a named
  bundle, leave to the main menu, and find the range in the Scenarios picker.

## What landed (2026-09-05)

Four commits on `wfc-editor`, in the planned order.

### 1. `nova_wfc`, over an authored grammar (`a036f58a`)

The draw table question got the answer the explicit-authoring rule asks for: a
`Grammar` content item. `PARTS`, the grid extents and the keel/stern roles were
consts inside the include; they are now authored fields, so a mod that ships a
grammar joins the draw and a mod that ships a section can be drawn. An unknown
prototype id is an error, not a skip.

Every failure inside the collapse is a `Result<_, String>` the caller can put
on a status line. The include panicked or retried; a crate a UI calls cannot.

`stamp_spinal_lance` and `stamp_large_drives` did NOT move. They are applied
outside the grammar, the old module doc already said neither is the future ship
grammar, and they stay with the bench that proves them.

### 2. Generate, in the rail (`a8685cc4`)

A Generate block on the scenario context: a seed you can read, type and reroll,
and a button. The look is NOT here - the style picker and the cladding toggle
are the ship's own, in Ship Settings, and the collapse reads them off the node
it is building for.

The order is collapse, LINT, then spawn. A hull `lint_scenario` refuses never
becomes nodes and the refusal goes to the status line, which is the task's
requirement that the editor not swallow a generator bug.

A generate is an ADD. The new ship lands beside what was built and is not
entered, so the block stays up for the next roll.

### 3. Templates (`27b9637f`)

`New Scenario` asks. Free-Flight Range (the old hard-coded seed) is the
default, Duelling Arena stands beside it, Empty Scenario beside that.

`DestructiveVerb::New` lost its confirm step. The template row IS the answer -
a confirm followed by a picker is two modals for one decision, and the picker
already carries the "no undo" line.

Templates are Rust in `crates/nova_editor/src/template.rs`, not authored
content. The task called a const "the cheap answer and the wrong one" and
allowed it for a first pass with the reason written down: a template is a
`Vec<ScenarioObjectConfig>` plus a script, so the authored form needs a content
item, a lint arm, a merge rule and a picker that reads the catalog. That is its
own task. What this pass owed was the shape, and `ScenarioTemplate::objects` /
`::script` return exactly what a loaded file lifts, so an authored template
swaps in behind them without touching the picker.

The arena's dressing is two scatter ACTIONS, not two dozen rocks in the tree -
the option the task preferred, for the reason the task gave.

### 4. Save As

A document remembers its file. `DocumentSlot(Option<SaveSlot>)` holds the id
and the name; `Save` writes there, `Save As` asks, an unnamed `Save` (menu or
Ctrl+S) goes to the ask, and `New Scenario` forgets it so a fresh range cannot
land on the range the builder just left.

The picker is an IN-GAME window (`crates/nova_editor/src/ui/files.rs`, on
`window_frame`), at the owner's call. There is no OS dialog and no folder tree:
a save is an installed mod bundle, so the window lists saved ranges and takes a
name.

Divergences from the plan, each deliberate:

- **The id is derived, not validated by `is_safe_id`.** That function is
  `pub(crate)` to `nova_assets` and unreachable from the editor. The derivation
  is `editor_` plus the name's lowercase ASCII alphanumerics with every other
  run collapsed to one `_`, which is strictly narrower than what `is_safe_id`
  accepts, and `install_local` validates again on the way in.
- **A collision is not a second dialog.** The readout under the name field says
  which slot the name derives and, in amber, which saved range it overwrites,
  with the id. The window is already the confirm; a modal over a modal to
  re-ask the question the reader is looking at buys nothing.
- **No editor marker in `ModMeta`.** The picker lists by the `editor_` id
  prefix. The editor is the only writer of that prefix, so the read-only rule
  the task wanted holds without a new field: a hand-authored bundle cannot
  appear in the picker.
- **`DestructiveVerb::Open` is gone.** Open is the same window with the rows as
  the answers, exactly parallel to the template picker, and carries its own
  "this replaces everything on the stage, there is no undo" line.

The range name and the file name are ONE string. `on_save` writes the typed
name onto the root `ScenarioNode` before raising the request, so the name in
the Scenarios picker and the name on the file cannot drift.

One thing only a live run found: the parts gallery is a MODE, not a panel. It
hides the whole `EditorChrome` root, the window layer with it, so a window
raised from under it is a window nobody can answer no matter where it sits on
the Z ladder. `open_file_window` puts the gallery down before it spawns.

**The web path is DEFERRED**, at the owner's call. `write_save` and `read_save`
still return `Err("... is not available on the web yet")`, `saved_bundles()` is
empty there, and the window's empty line reads "Saving is not available on the
web yet." rather than pretending there is a store. `cargo check -p nova_editor
--lib --target wasm32-unknown-unknown` is clean.

## Proof

- `one_seed_names_one_hull` in `crates/nova_wfc/src/tests.rs` pins the seed
  contract the move had to keep. Both benches still build
  (`cargo check --features debug --example wfc_arena --example wfc_ships`).
- `cargo test -p nova_editor --lib`: 462 passed, 0 failed. Thirteen of them are
  new, in `crates/nova_editor/src/ui/files/tests.rs`: the id a name derives,
  two names that collapse to one slot, the offered name for a named and an
  unnamed document, an empty name greying Save, a taken name reading as an
  overwrite, a row as the answer in Open and as a name in Save As, cancel, and
  a second ask not stacking a second window.
- `cargo check -p nova_editor --all-targets` and
  `cargo check -p nova_editor --lib --target wasm32-unknown-unknown` clean.
- The `system_ship_editor` autopilot walk passes end to end under Xvfb, 13.1s.
  It now covers the whole arc in one run: Ctrl+S from under the parts gallery
  raises the name window and the gallery stands down; the named save writes and
  the second save goes straight to its file; `File > Open` reopens 16 nodes on
  the same ids and poses; then `New Scenario` founds a document on the DUELLING
  ARENA, `Generate` collapses a hull onto it, and `Save As` writes it under a
  name of its own - the run ends holding both `editor_saved_range` and
  `editor_arena` as enabled mods. Both a fresh id and an overwrite of an
  existing one were run.
- Inspected on disk: `editor_arena.bundle.ron` carries `name: "Arena"`, and its
  content holds the arena's four objects beside the generated ship.

Not proven: the saved range LISTING in the main-menu Scenarios picker. The walk
ends inside the editor and asserts the save enabled the mod, which is the
mechanism the picker reads; the listing itself is not walked.

## Follow-up: Generate is a ship verb, and what it lays comes out flyable

The generator moved INSIDE the ship. It used to be a scenario verb that ADDED a
ship, which made every reroll a new draft to delete; now you add a ship, go
inside it, and Generate replaces the hull that ship holds. Rerolling is a dial.

The rail block names what the collapse may draw from. The list is the whole
merged section catalog, one ticked row each, and the ticks ARE the setting -
there is no resource behind them. The shipped `standard_hull` grammar is left
alone so the benches keep their tuned taste; a ticked section the grammar does
not price joins the draw at a stated weight of 1 with no aim, which the block
says on screen rather than hiding.

Every part the collapse lays comes out bound, by kind: Space thrusts, LMB the
PDCs, `F` the torpedoes, `R` the railgun. Set the ship to Player and fly it.
This also changes what a hand-PLACED torpedo or railgun binds to - they shared
one key before - and no two kinds now answer to the same button on either
device.

### Decisions

- A grammar is UNTRUSTED input now: a mod ships one and the editor builds one
  from ticks, so every arithmetic precondition the collapse relies on is a
  checked refusal in `nova_wfc::runnable` - grid dimensions, vacuum pricing,
  part weights, and at least one part priced above zero. `Collapse::draw` grew
  a floor beneath it, because a cell whose taste cancels out to a zero total
  used to hand `rand` an empty range and panic.
- Two catalog parts CANNOT be drawn, and the refusal says so by name rather
  than failing silently: `capital_thruster_section` (5x5x3) and
  `vector_thruster_section` (3x3x2). `segment_tiles` models a part that runs
  along its own z axis, and a 5-wide part cannot straddle the mirrored half
  grid at all. Railguns (1x1x3) do draw. **Superseded below: both draw now.**
- The rail keeps the Scene tree FIRST. Putting the ship blocks above it was
  tried and reverted: the tree is the navigation spine, and pushing it below
  two blocks put the scenario row off the fold, so leaving a ship stopped
  working. The cost is that a generated ship's 164-row tree buries the Generate
  block until you scroll. Bounding the tree needs nested scroll panes, and
  `scroll_viewports` gives the wheel to EVERY hovered viewport - hover
  propagates to ancestors, so a nested pane would scroll the rail behind it.
  That is a change to a shared widget and is left alone here.

## Proof

- `cargo test -p nova_editor --lib`: 468 passed. Ten rewritten in
  `generate/tests.rs` - generate lays into the ship being edited, a second
  generate REPLACES rather than stacks (and mints no duplicate ids), every
  weapon comes out bound, one seed lifts one hull, a refused roll leaves the
  ship alone for three separate reasons, generate outside a ship says where to
  stand, and an unpriced section joins at the stated weight. Two in `ui`: the
  draw list offers the catalog with the grammar's own parts ticked, and one row
  ticks without disturbing the others.
- `cargo test -p nova_wfc --lib`: 7 passed. Eight bent grammars are each
  refused rather than run, proven non-vacuous by commenting the gate out and
  watching it fail; and a cell whose taste cancelled out still draws something.
- `cargo test -p nova_ui --lib`: 56 passed.
- `cargo check -p nova_editor --lib --target wasm32-unknown-unknown` clean;
  `cargo check --features debug --example wfc_arena --example wfc_ships
  --example system_ship_editor` clean.
- The walk passes end to end under Xvfb: it adds a ship, enters it, generates
  164 sections into it, and asserts the raised keybind chips carry `Space` and
  `LMB` - 42 chips over a hull nobody drew.
- Inspected rendered: the block reads SHIP SETTINGS then GENERATE HULL, seed,
  Reroll Seed, Generate, then DRAW FROM over the ticked catalog. The Generate
  button sits ABOVE the 17-row list because below it the button was off the
  fold and unclickable. Two note strings were wrapped across source lines and
  so carried the indentation into the text - one rendered a phantom gap, the
  other a floating "it" - and both are now single-line literals.

### A fix the walk found, not the tests

`scroll_viewport()` never included `Hovered`, so `any_hovered` in
`scroll_viewports` was always false and one wheel notch moved EVERY pane. It
was invisible while the rail's in-ship content was short enough to clamp at 0;
the draw list made the rail long, and the inspector's wheel roll started
scrolling the rail out from under the walk. This shipped in v0.12.0 and has a
Fixes entry.

## Follow-up: the big drives, on multi-cell sockets

`vector_thruster_section` (3x3x2) and `capital_thruster_section` (5x5x3) are
drawn and seeded now. `segment_tiles` builds a 3D BLOCK instead of a 1x1xN
chain: one tile per cell of the footprint, `joints` in all three axes, the
authored `drive_mount_points` link points resolved to the cell whose face
centre each one lands on, and the exit taken by the whole muzzle LAYER rather
than by one tile.

### Decisions

- `exit` and `aims` are now two different facts, and conflating them was the
  whole bug. `exit` is a CELL's clearance - only the muzzle layer carries it,
  because only the muzzle layer has anything in front of it. `aims` is the
  PART's firing direction and every one of its cells carries it. `aim_allowed`
  read `exit`, so a grammar that aims a drive aft struck the drive's entire
  MOUNT layer as pointing nowhere, propagation then took the muzzle layer with
  it, and a big drive stood in zero cells of any grid. Found by measuring
  opening domains against post-propagation domains rather than by reading.
- The big drive is SEEDED, not merely offered. A free roll cannot place it: it
  needs `half_width >= span.x + 1`, because its socketless flank cannot face
  the keel hull at `x = 0`, and `erode_blocked_exits` - which delegates to the
  game's own `blocked_exits` - correctly kills a drive whose 9 or 25 exhaust
  lanes are not all clear. That is only true at the transom. `seed_stern`
  therefore lays a deck plate the size of the drive's mount face and stands the
  drive on it; for a 1x1x1 drive the generalised code is byte-identical to what
  was there. The codebase already seeded the small drive for this reason.
- `seam_flush` is authored per tile rather than inferred from the offset. A
  block's segments sit at their own cell centres, so the old `offset.x == 0`
  test called every segment of a wide part non-flush and barred the part from
  the centreline. The flag says what the test meant: does the body reach the
  cell's `-x` face.
- `mirror_symmetric` now compares socket POSITIONS as well as normals. A part
  several cells across can carry a socket over one cell and not over that
  cell's reflection, which the normals alone cannot see.
- The editor grows the grid rather than refusing the tick. `holding()` raises
  `half_width`, `height` and `length` to hold the biggest ticked thruster and
  never shrinks them, so ticking the capital drive widens the hull instead of
  dropping the section.

## Proof

- `cargo test -p nova_wfc --lib`: 10 passed. Three new: a block part's tiles
  span the authored footprint at every rotation (sorted, because `span` is in
  grid axes and permutes), a seeded capital drive lands in a grown grid, and a
  grammar too small for its own seeded drive is refused by name.
- `cargo test -p nova_editor --lib`: 469 passed. The two tests that asserted
  the big thrusters were refused now assert they are laid.
- Seed sweep: 12/12 seeds lay the drive for both sizes, every clad hull passing
  the game's own `lint_scenario` and the unmated-contact check.
- `cargo check --features debug --example wfc_arena --example wfc_ships
  --example system_ship_editor` clean.
- The editor walk passes end to end under Xvfb: 160 sections generated into the
  ship, 18 keybind chips raised, `cycle complete, no panic (t=13.0s)`.
- Inspected rendered. `wfc_ships` was temporarily pointed at a grammar whose
  `stern_drive` is the big part, shot clad and bare, and reverted. Both sizes
  come out as a mirrored stern PAIR with the skin closing over them: the
  capital drive in a grown 6x5x13 grid, and the vector thruster in the SHIPPED
  4x5x11 grid with no grid change at all.

### What the walk found, and why the assertion moved

The walk asserted `LMB` on every generated hull. `seam_flush` makes more
layouts legal, which shifts the RNG stream, and the walk's seed then rolled a
hull with no PDC on it. That is seed variance, not a regression - the generator
never promised a PDC. The beat now asserts what it does promise: the drive is
seeded, so `Space` is always there, and every chip raised is one of `Space`,
`LMB`, `F`, `R`.

## Follow-up: rules per section, and a railgun at the front

A grammar could say which way a part POINTS (`GrammarPart::aim`) and nothing
about where it STANDS. It says both now, in two shapes that answer two
different questions.

- `GrammarPart::zone` is a soft rule for parts that ROLL: `Bow`, `Amidships`,
  `Stern`, `Dorsal`, `Ventral`, `Flank`, resolved against whatever grid the
  generator was handed rather than against a cell range.
- `GrammarKeel::bow_gun` is a SEEDED role, symmetric with `stern_drive`.

### Decisions

- A zone alone cannot put a railgun at the front, and shipping only the zone
  would have looked like it did. A spinal gun fires down its own axis, and
  `erode_blocked_exits` takes off anything whose lane is not clear - a lane
  three cells long is only ever clear at an END of the hull, and the bow is the
  sparsest part of the grid, where a drawn gun has almost nothing to mate to.
  So the gun is seeded, exactly as the stern drive is.
- It stands IN the keel column rather than beside it. That is what makes the
  lane free without carving anything: the muzzle cell is the bow-most row, so
  the lane in front of it leaves the grid. The bench stamp this replaces had to
  delete whatever the collapse had hung past the bow face; nothing is deleted
  now. The mirror gives a tight PAIR either side of the centreline, which keeps
  the recoil on the ship's axis.
- Both fields are `Option` with `skip_serializing_if`, and the absence of each
  is documented - "free to stand anywhere the mating rule allows", "this hull
  plan seats no spinal gun". So this is additive to content: `content gen`
  reproduces `assets/base/**` byte for byte and no mod migrates. That is a
  deliberate choice against a required field, not an oversight of the
  explicit-authoring rule.
- The standard hull seats NO bow gun. A lance is a decision about what kind of
  ship this is, and the base warship is not one; nothing about a shipped
  silhouette moves. The editor seats one when a builder ticks it.
- The editor DERIVES both seeded roles from the ticks, the way `main_engine`
  already derived the stern drive. A builder learns no second control: ticking
  the lance puts a lance on the nose the way ticking the capital thruster puts
  a big drive on the tail. The HULL PLAN line is the feedback that says so.
- `tiles::build` now gives every role the grammar NAMES its tiles, at weight
  zero, rather than requiring the grammar to also list it as a draw. Without
  that a grammar that seeds a part it does not draw fails deep in the seed with
  "cannot stand on the grid at all", which names the symptom. It also deletes
  the hand-rolled version of the same padding from the editor.
- The zone chip is a button INSIDE the row's button. Safe because
  `on_part_choice` rules on `activate.entity` - the button the press names -
  so a press on the chip is not also a press on the tick. The alternative was
  an eight-state cycle on one row, where unticking a part would take seven
  presses.
- A bow gun wider than one cell is refused BY NAME. A spinal gun stands in the
  keel column and a block cannot be centred on a column; the catalog's one
  railgun is 1x1x3, and a 3x3x2 asked for as one now says why it cannot be.

### What this retires

`stamp_spinal_lance` and its three tests are gone. Its module doc said the
quiet part - "a gun the whole SHIP aims is a placement the grammar has no
vocabulary for" - and that is the vocabulary. `wfc_arena` names the lance as
its grammar's `bow_gun` and the collapse seeds the pair itself.

`stamp_large_drives` stays. The arena benches one capital drive against two or
three vector drives on the same hull, and a grammar seeds ONE stern drive, so
that comparison is not something a grammar can express.

## Proof

- `cargo test -p nova_wfc --lib`: 13 passed. Three new - a seeded bow gun
  reaches the nose of all eight seeds as a pair sitting exactly on the bow
  face, a zoned turret stands only dorsal or only ventral over eight seeds
  (with a guard that the rule was not vacuous), and the two refusals name what
  is wrong.
- `cargo test -p nova_editor --lib`: 472 passed. Three new - ticking a spinal
  gun seats it on the bow and unticking it takes it away, a row's zone reaches
  the grammar and holds in the rolled hull, and the chip cycles without
  ticking the row under it and disappears when the row is unticked.
- `cargo test -p nova_ship --lib`: 860 passed. `nova_scenario` lint: 57 passed.
- `cargo test --features debug --example wfc_arena`: 13 passed. This target
  never compiled before: `stamps.rs` carried a `use nova_ship::...` that the
  example crate cannot resolve, and `cargo check --example` does not build the
  test target. Removed.
- `content gen` rewrites `assets/base/**` with no diff; `content lint` is 0
  errors, 0 warnings, 0 findings.
- `cargo check -p nova_editor --lib --target wasm32-unknown-unknown` clean, and
  the three examples check clean.
- The editor walk passes end to end under Xvfb with a new beat: it ticks the
  lance through the row's own observer, waits for the HULL PLAN line to name
  Railgun Lance, generates 188 sections and asserts the chips carry `R` as well
  as `Space`.
- Inspected rendered, twice. The Generate block first drew HULL PLAN over four
  wrapped lines and the chip pushed long section names onto a second row, in a
  190px rail. Fixed: the line names only the two roles the ticks decide (the
  keel and bridge are the same on every hull), and the chip labels are short
  (`any`, `mid`). `wfc_ships` was temporarily pointed at a grammar with a bow
  gun and dorsal turrets, turned to face its bow, shot, and reverted - the
  lances come out as a spinal pair on the nose.
