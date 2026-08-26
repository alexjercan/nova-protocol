# Changelog

All notable changes to this project are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
but groups each release by subsystem (Gameplay & Flight, Combat & Weapons,
Ships & Sections, Scenarios & Objectives, Modding & Mod Portal, Interface & HUD,
Web & Platform, Audio & Visuals, Performance, Fixes, Internals & Tooling) rather
than by Added/Changed/Fixed. This project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Before editing this file, read the Changelog section of `AGENTS.md`.**
It owns the rules - the release baseline, the 200-character limit, and what
does NOT get an entry - and it is the only place they are written down.

## [Unreleased]

### Modding & Mod Portal

- A range saved in the editor is an ordinary mod: each built ship a design, the
  range a scenario naming them. Editing a design moves every instance of it.
- F5 reads every content file off disk behind a loading screen, as does leaving
  the editor or the mods panel: a range saved in the editor reaches the
  Scenarios list without a restart.

### Interface & HUD

- The editor holds a scenario of ships under a real scene tree: a click picks a
  node and two enter one, entering isolates that ship in the tree and on the
  stage, and Play compiles the scenario root.
- Add Ship starts a blank ship - a pad marker shows where the first armed
  part founds it - and dragging a ship at the scenario node slides it on the
  ground plane.
- The sandbox range is part of the document: its planetoid, hulks, pickets,
  beacons and lights come up as scene-tree nodes, and Play flies the range the
  tree holds rather than a fixed one.
- The top bar's Add menu places anchors, asteroids, beacons, salvage crates and
  lights in front of the camera, and dragging or deleting a world object works
  exactly as it does for a ship.
- A click in the world selects: a section inside a ship, a ship outside; two on
  a world object frame it. Rebinding a section's key is Ship > Rebind Key, on
  the selection.
- The selected ship or object carries move and turn handles: drag an arrow to
  slide it along one axis, a ring to spin it about one - so a node can go
  somewhere other than the ground plane.
- The stage stands on a world grid - cells stepping by decades as the camera
  pulls back, the origin's own X and Z lines, a plumb line under the selection.
  View > World Grid turns it off.
- Objects draw what they have no body to show: a beacon's or a crate's trigger
  sphere, a lamp's reach, a sun's direction, each in its own colour. View >
  Object Volumes turns them off.
- Play flies the whole document: every ship stands where it was dragged and
  faces where it was turned, and every non-empty AI design spawns as a neutral
  AI ship beside the player's.
- The stage camera snaps to what you edit - the entered ship, or the whole
  scenario back at the root.
- F, View > Frame Selection and a click on a scene-tree row all put the camera
  on that node, framing its whole size rather than the point it was founded on.
- The top bar reads the context back on its right - SCENARIO or SHIP, the path,
  the selection - and the footer legend matches the level.
- File, Edit, View, Add and Ship drop real menus, with Play dead centre. Ship
  holds what used to crowd the top right: the parts gallery and Rebind Key.
  Escape closes an open menu.
- Scene tree rows step in by depth and print a section as its part name, with
  the ordinal in a column the rail cannot clip, so six alike hulls stay apart.
- Every tree row wears an icon for what it is, and resting on one reveals the
  kind that icon stands for and the whole id the row had to clip.
- The right-hand Inspector lists a node's own fields - a thruster's, a rock's,
  a ship's driver - read off its config by reflection; typing into one writes
  it back, so a placed object keeps what you set.
- Inspector rows are typed to what they hold: a colour paints a swatch beside
  its hex, and an enum of bare names becomes a row of choices you can switch
  rather than a readout you cannot.
- The Inspector opens on what a kind is authored through - a turret's fire rate
  and ammunition, not its joint offsets - and View > All Fields brings the whole
  config back.
- The Inspector says a node's id under its title, and View > Ids reads the whole
  tree as ids rather than names: what an event's filter will name them by.
- A seeded spacecraft IS a ship: it wears the driver's mark, opens on a double
  click like any other, and its panel reads the hull it flies, who flies it and
  which side it is on.
- Backspace leaves the node you are inside, and so does one click on any rung
  above it: going up stopped needing a double click on the root.
- Every vector is three axis-tinted boxes: a node's position, its rotation in
  yaw/pitch/roll degrees, and every offset in a config. There is no scale row -
  Nova sections mate, they do not stretch.
- The Inspector reaches into a node's lists - a turret's fire rate lives on a
  muzzle inside its joint tree - and draws them as an indented tree of ruled
  headings, not a column of long paths.
- Both side panels are wider and scroll: a bar beside each says how far down
  the list runs and can be dragged, and a wheel notch moves three lines of the
  list rather than one.
- Pressing a colour's swatch opens a floating picker: four channel sliders over
  a live preview, dragged by its bar and closed by pressing the swatch again.
  It belongs to that row and goes away with it.
- Back to Main Menu ends the session and deletes the scenario draft; Play
  round-trips still return to it.
- File > Save writes the whole document, ships and range together, and File >
  Open reads it back on the same ids and poses. Ctrl+S saves; the rail's line
  reports what happened.
- Every menu row shows the key that runs it, and the footer legend is cut to
  the gestures no other surface can carry. A row that is not built yet reads
  `soon`.
- What is selected is what Delete deletes, at any depth: a trash on the tree
  row, the Del key, or Edit > Delete. The delete brush that had to be armed is
  gone.
- Add obeys where you are: the world palette greys inside a ship, and Add
  offers that ship's own parts instead.
- Every refusal answers on the editor's own line - a rebind conflict names the
  action already holding the key - instead of only in a log nobody is reading.
- Nothing throws a document away without asking: File > New Scenario and Back
  to Main Menu both put the question up first.
- The selected node wears a phosphor outline in every context, and resting on
  a tree row lights the thing on the stage - and the other way round.
- A field knows what it holds: a unit, a floor and the step it drags by. A
  number is scrubbed by pulling its row's name, the pointer wrapping at the
  screen edge; a typed negative radius is refused.
- A ship carries a name of its own, and the tree shows it with the minted id
  on hover. The Key row is the button that arms the rebind.
- The scenario node reports what the document holds: how many ships, how many
  objects, and which ship the player flies.
- The placement verdict is said beside the ghost rather than at the window
  edge, and every refusal names the key that resolves it.
- Link points read at any distance, the socket the ghost would take is marked
  apart from the rest, and a part's keybind chip stands off it on a leader.
- The parts gallery is framed to its parts: each thumbnail fitted to the
  part's own bounds, sockets drawn on the focus preview, and the filter
  visible while it is set.
- The rail reads the ship back: turn rate, mass, thrust, hull and part count,
  and it names what holds the turn down - shorten the hull, or fit another
  computer.
- The path in the top bar is a control: press a rung to go there. View holds
  front, side, top and iso presets, and an axis rose in the corner says which
  way is up.
- Hulls carry their name on the stage, and anything else takes one while it is
  selected or under the pointer, so five derelicts stop being five identical
  grey shapes.
- Labels pinned to something in the world - a keybind chip, a NOVA OS blip -
  sat a label's height off target on a HiDPI screen. They point where they mean
  at any scale and landscape size.

### Ships & Sections

- Sections keep a stable name for as long as they exist, so a ship read back
  after a trip out to flight is the ship that was built.

### Internals & Tooling

- The editor publishes its build state - armed tool, placement, gallery, edited
  ship, inspector rows, the status line - as read-only data the driven ranges
  wait on instead of counting frames.
- Editor previews carry mesh and collider only: the behaviour half of a ship
  section is no longer inserted on something that will never fly.
- Autopilot scripts can wait on a UI node laying out, on the picking pointer
  registering a press or a release, and on either of two conditions (`or`).
- Autopilot scripts can press a widget twice as one gesture, so a driven walk
  can spell a double-click.
- One arbiter owns the keyboard: a verb answers in normal mode alone, and a
  mode - a focused field, the parts gallery, a keybind capture - takes every key
  from the verbs, which never hear of it.

## [0.11.0] - 2026-08-23

### Combat & Weapons

- **(breaking)** Torpedoes fuze 3 u from the locked body's nearest skin, not half
  a blast radius from its centre: near-full pressure, hull-side craters, and no
  contact duds.
- Rock is its own material, ten times softer than cladding: a radius-3 asteroid
  takes 2.4 minutes of held PDC fire instead of 24, and a torpedo bowls out every
  shipped rock size instead of nothing.
- A crater captures a following hit only within a unit of itself, so the hole
  follows the aim instead of the first place the rock was shot.
- The turret catalog is the two PDC mounts. The Better, the Light and the ten per-craft turret prototypes are gone; every craft mounts the kinetic PDC. **(breaking)**
- Make ship sections shield against blast pressure. **(breaking)**
- Weapon reloads require a quiet interval: PDCs recover partial batches, torpedo launches restart rearming, and incoming rounds pulse on a persistent ammo gauge. **(breaking)**
- Turrets fire only when the barrel is ON its aim point (0.92 deg, derived from
  what still hits a hull at 1 km), so a mount that cannot bear holds fire and
  the ones that can keep shooting.
- Rounds leave the muzzle inside a 0.1 deg cone, so a stream reads as a gun
  rather than a laser line. Nine times narrower than the fire gate, so it moves
  no hit rate.
- Point defense is assigned PER TURRET: each mount holds the most imminent
  torpedo it can bear on, so a battery no longer dogpiles one torpedo.
- Turrets lead on a FILTERED track of the target's course instead of the
  velocity of one step, and a mount handed a different target starts its track
  over.
- The Flight Computer works your IDLE PDCs against inbound torpedoes, drawing a
  thin line to each pick. Lock or raise and every mount is yours instantly.
- Torpedo bays regrow one torpedo every 10 s, up from every 4 s, so a bay
  refills like every other weapon and a ship is never left with nothing.
- Torpedoes fly a TERMINAL WEAVE: an armed torpedo corkscrews off its guidance
  solution until the fuze, breaking the point-defense lead solution. Authorable
  per bay.
- **(breaking)** A bay loads a torpedo TYPE - name, tint, cruise cap and weave
  in one authored value - replacing loose `weave_angle`/`weave_rate`/`max_speed`.
  Mods author their own.
- Two shipped torpedoes: the fast, straight-running Lance and the slower,
  weaving Serpent. Same warhead, same rack, own colour in flight.
- Evasion costs speed: the Serpent cruises 32 u/s against the Lance's 35, so it
  arrives ~7% later and gains on a runner half as fast.
- The campaign's first torpedo fight (Broadside part two) now throws Lances,
  which point defense can answer; the Final Tally flagship escalates to
  Serpents.
- Gunfights happen at 1-2 km instead of 4-5: shorter round lifetimes
  (PDC 2.0 s, scavenger 3.0 s) and AI ranges to match. No weapon's damage moved.
- **(breaking)** The per-section damage-resistance table is GONE. Damage is one
  number, and a damage type's identity is how the round TRAVELS.
- **(breaking)** The damage roster is cut to `Kinetic`, `Pierce` and
  `Explosive`. `Emp` is removed and `ArmorPiercing` is renamed `Pierce`. No
  aliases.
- **(breaking)** The two bullet types are a punch and a rake: Kinetic carries
  on only through what it DESTROYS, Pierce deals full damage to every section
  it crosses.
- **(breaking)** Both bullet types read the round's CLOSING SPEED, anchored at
  100 u/s: Kinetic turns it into damage per hit, Pierce into raking POWER.
- **(breaking)** A Pierce round pays for travel out of a separate power budget,
  spent on each layer's FULL health rating, under a hard six-layer cap.
- **(breaking)** `SectionDamageClass` is renamed `SectionClass`: with the table
  gone it is the ship computer's section LABEL, not a damage key.
- **(breaking)** `pdc_turret_section` splits into `pdc_kinetic_turret_section`
  and `pdc_pierce_turret_section`: the same mount, punch versus rake.
- **(breaking)** Fixes a live double-damage bug: a round reported its contact
  twice and paid out on both. Real turret damage halves at unchanged authored
  values.
- **(breaking)** Torpedo ordnance stops dying to one bullet: default
  `projectile_health` 1.0 -> 10.0, authorable per bay. The siege bay is
  unchanged at 5000.
- **(breaking)** Standard torpedoes hit like torpedoes: blast damage 100 -> 750
  everywhere, for every bay but the capital-grade siege one. The counter is
  point defense.
- Turrets no longer aim into the hull they stand on: the pitch hinge's
  depression floor tightens from 30 to 10 degrees below level.
- **(breaking)** Neutralization is "no weapons OR no flight computer" now, was
  "no weapons AND no thrusters". Bare emplacements still neutralize on their
  guns alone.
- Retires the "guns and thrusters gone" defeat copy: every neutralized-defeat
  banner now reads "Nothing left to fight with".
- New `heavy_torpedo_section` prototype: a capital-grade siege bay with a
  ship-killing blast and armored ordnance, hidden from the editor gallery.
- New `ForceTorpedoLaunch` event action: script a controller-less ship's
  torpedo bays to launch at a named target on timers.
- **(breaking)** Unlimited player ammunition is a DEBUG-ONLY cheat:
  `infinite_ammo` is honored only under the `debug` feature, so every scenario
  plays real magazines.

### Gameplay & Flight

- **(breaking)** Make an asteroid's remaining material its only durability.
- Let sustained fire grow one crater without remeshing sub-cell hits.
- Asteroids CARVE: a hit takes a real crater out of the rock, mesh and collider
  both, so a shot rock is physically the shape it looks. Craters persist and a
  rock's published radius only ever shrinks.
- **(breaking)** Rocks use a ROCK generator, not the planet one: signed, per-seed
  stretched displacement makes a rock an irregular chunk instead of a sphere with
  growths. Same size, new silhouette.
- Rocks are textured by POSITION, not by mesh UVs. No more per-triangle quilting,
  and a carved rock wears exactly the surface an uncarved one does.
- Fixed: a rock whose seed landed near `u32::MAX` overflowed the noise generator.
  Seeds come from hashing a scenario id, so an unlucky name could hit it.
- Every rock is meshed from its own carve field, so a hit changes the crater and
  nothing else; the field is dropped after meshing and rebuilt on the first hit.
- Carve a rock apart and the piece FLIES OFF: anything a crater cuts free
  becomes a rigid body of its own, carrying the drift and spin it had.
- A carve throws DUST, however big the bite: only geometry a cut really severed
  becomes a body. Debris drifts clear before it collides, so nothing kicks its
  own wreckage.
- **(breaking)** No body ever bursts into generic grey cubes. A wreck wears what
  it was already wearing, and a range asserts the burst never happens.
- A damaged section fractures where it is failing and glows through the cracks,
  instead of the whole body turning red.
- Wreck debris leaves with the wreck: a body's pieces inherit its drift and spin
  instead of hanging where it died.
- **(breaking)** How hard a ship turns is physics now: the lower of its
  computers' torque over its inertia and an 8 G hull limit over its length.
  Small craft turn 2-3x sharper; capitals are barges.
- **(breaking)** Controller `max_torque` is rescaled to 1501. Computers stack
  LINEARLY up to the hull's structural limit, so a small ship is already at
  that limit and gains nothing from more.
- A damaged hull turns sharper: losing sections shortens the arm the structural
  limit is measured over.
- Losing one computer on a stacked hull degrades handling to the smaller stack
  instead of casting the ship adrift; autopilot and neutralization key on "no
  computer LEFT".
- The build screen shows the hull's turn ceiling and which limit sets it while
  sections are being placed.
- Wreck fragments despawn after 30 seconds instead of persisting until scenario
  teardown.
- AI patrol legs steer around sized bodies: a leg blocked by an asteroid's
  geometric radius detours past it instead of flying the GOTO straight
  through the rock.
- AI ships gain an authorable `engage_range` (default 800): the
  hostile-detection distance a passive ship leaves its routine for.
- AI ships gain an authorable `pd_range` (default 400): the distance point
  defense starts engaging inbound torpedoes.
- AI ships gain authorable `waypoint_slack` (default 25) and `arrival_standoff`
  (default 50): how close a patrol presses to its waypoints.
- New `SetAmmo` section modification: a hard magazine (rounds overridden,
  auto-reload stripped) for ships whose ammunition is the scene's clock.

### Ships & Sections

- **(breaking)** Sections lose `base.mass`. A section is solid ship: its mass IS
  its collider volume. Delete the field from mod content, and resize the collider
  to change what a part weighs.
- **(breaking)** Hits are REMEMBERED where they land: a ship stores each one as
  a sphere in its own frame, so an effect can read WHERE it was hit and not only
  how much it cost.
- The plate that stopped the round dies and comes off, leaving a hole in the
  cladding onto the hull underneath.
- Ships do not CARVE. Sections and cladding keep their authored shape and show
  damage through cracks and lost plates. Only asteroids change shape.
- Damage is TWO readings now: how far gone a body is (cracks, sparks) and where
  it was hit (the chips a round knocks off it). Nothing on a ship loses geometry.
- Sections AUTHOR the damage looks they wear (`base.damage_effects`): `Cracks`,
  `Sparks`, `Plume`. Omitted means `[Cracks]`, so a section shows damage without
  saying anything; author `[]` to opt out.
- New PLUME effect: a damaged drive's exhaust guts and flickers without ever
  reading as shut down, and delivers exactly the thrust it authored.
- Bullets CHIP what they hit: a kinetic or pierce round throws chips off ships
  and rocks alike, more of them for a bigger hit. A torpedo throws none - its
  fireball is the cue.
- **(breaking)** Ships collapse structurally: a hull below a fraction of its
  as-built health (0.05 by default) comes apart. Authorable as
  `collapse_threshold`.
- Ships can wear a DERIVED skin: `skin: true` clads the whole hull at spawn
  from the structure alone. Cladding is destructible and never counts toward
  ship health.
- Generated `wfc_ships` hulls have a BACK: the stern is seeded with a drive
  deck, so nozzles all point one way. Design note in
  `docs/ship-layout-sense.md`.
- The derived skin plates around a drive or a gun: only the one cell a part
  FIRES into is left bare, its four flanks are ordinary plating. Exit clearance
  is unchanged.
- `basic_thruster_section` carries ONE socket now, on the forward face it bolts
  by, so nothing mounts on the drive or plates over its exhaust.
- A collapsing ship TEARS ITSELF APART: the outermost sections blow off first
  and the wreck peels inward until the root dies with it.
- A ship coming apart keeps fighting until the sections carrying its guns blow
  off, and its sections fire their own `OnDestroyed`. `OnDefeated` still fires
  exactly once.
- The hull cast swapped: the cargoa is the campaign's armed corvette, the racer
  flies unarmed as the civilian.
- The editor places parts by MATING link points instead of stepping one unit
  along the surface it hit. A placement that cannot mate is refused, and the
  status line says why.
- The semantic Racer, CargoA and CargoB parts join the editor palette - they
  were hidden while placement was a grid step.
- A part now mates the same way up on ANY socket, so parts from different craft
  fit each other: socket FRAMES are mated and authored normals snap to an axis.
- New `pdc_turret_section` ("PDC Turret"): one compact point-defense mount that
  fits any hull face, replacing ten per-craft copies in the editor's Weapons
  tab.
- A turret's base stands on the face of the section it mounts through; the
  joint tree hardcoded the unit cube's -0.5 and sank a small mount's gun into
  its hull.
- `render_mesh_transform` takes a `scale` alongside position and rotation, so
  art resizes without touching the collider, the sockets or the mass.
- The shared PDC is assembled at HALF a section, one number driving its
  collider, its sockets and its art.
- Semantic parts are named for the craft they came off - `CargoB // Nose`
  rather than a third part called `Nose`.
- A clad ship can wear a STYLE: `style: Some("<id>")` beside `skin: true` gives
  its cladding authored plate materials plus destructible decoration.
- Decoration is placed from what the derivation already works out about each
  plate - relief, facing, run, fill, depth, fitting proximity - and claims
  cells on a lattice.
- The scatter is DETERMINISTIC: no RNG, just a hash of the cell, so a ship
  always wears the same greebles and the build view never flickers.
- The base game ships FOUR authored looks - `industrial`, `armoured`,
  `civilian` and `salvage` - with 23 greebles between them. `placeholder` stays
  as scaffolding.
- A ship picks its look in the EDITOR: the cladding toggle lists every loaded
  style. `wfc_ships` cycles them with `L` or takes `--style <id>`.
- Decoration LIES DOWN on the plate it stands on instead of standing upright in
  its cell, and only lands on a plate with a whole flat top. `seat: Any` is the
  high-ground exception.

### Scenarios & Objectives

- The **Asteroid Field** sandbox and its Next relay leave the Scenarios
  picker. The editor's Sandbox button is unaffected.
- Chapters play an outro: a win posts its beat, then two timer-paced comms
  beats over the live world before the overlay. The win locks the instant it
  lands.
- New `Anchor` scenario object: an invisible authored gravity well (radius +
  optional mass, no mesh or collider) for orbit targets and bodiless gravity.
- **(breaking)** Menu backdrops pose their own camera: a `SetCamera` in OnStart
  is the contract, a lint Error without one.
- The main menu is a backdrop CAROUSEL: four scenes hand off Factorio-style.
  Torpedo Gauntlet, Asteroid Weave and Duel Cycle are new; Menu Ambience and
  Scrapyard Drift retire.
- Asteroids gain an optional `seed` pinning the generated silhouette across
  runs; `ScatterObjects` fills it deterministically from the scatter seed.
- **(breaking)** A carvable asteroid grids in WORLD units - a half-unit cell, 16
  to 64 cells - so a PDC round's hole shows on any rock. Small rocks carve 5-8x
  cheaper; rocks past radius 2.4 cost 3-5x.
- The editor's Play sandbox becomes a RANGE: two seeded rock belts, five target
  hulks, three dormant pickets and two skybox beacons. Dying now offers a
  Retry.

### Modding & Mod Portal

- **(breaking)** The mod portal and cache take the `NOVA_MODDING_*` prefix: `NOVA_MOD_CACHE_ROOT` is `NOVA_MODDING_CACHE_ROOT` and `NOVA_PORTAL_URL` is `NOVA_MODDING_PORTAL_URL`.
- **(breaking)** A mod declares its own balance acknowledgments in a
  `balance_acks.ron` beside its manifest; the linter reads them from the bundle
  it lints. No list in this repository names a mod.
- **(breaking)** A new content kind: `Ship`. A scenario spawns a hull by id and overrides sections per spawn; base scenario RON drops 36%.
- A new content kind: `Style`. A mod can author the look a ship's derived
  cladding wears, and a scenario picks one per ship by id. Documented in the
  modding wiki.
- A mod can ship its own greeble `.glb` files and reference them with
  `self://`. The base placeholders are `dep://base/gltf/greebles/<id>.glb`.
- A style's scatter rules speak a wider vocabulary: `Rim` splits into `Bevel`,
  `Brink` and `Spur`, and `align` grew from a flag into `Free` / `Run` /
  `Outward`.
- A scatter rule can state its density in CELLS OF SHIP: `patch: N` guarantees
  a piece in every block of N cubed cells the rule can stand on. Still no RNG.
- `near_fitting` counts steps across the surface instead of rings, so `Some(1)`
  is the four cells beside a nozzle rather than the eight around it.
- The skin log tells a STARVED rule from an IMPOSSIBLE one: each fixture
  reports `taken of reach`, and the build view logs the hull's relief
  histogram.

### Interface & HUD

- Keep WFC arena combat frozen while its NOVA OS is open.
- Let several ship sections share one input when rebinding through NOVA OS.
- Keep the mouse visible and free while NOVA OS and WFC arena result screens are open.
- Add a configurable WFC arena lobby, match pause and result screens, reusable text fields, dynamic ammunition scores, and live NOVA OS section rebinding.
- A scenario swap draws an animated LOADING SCENARIO panel over the stall, and
  rocks build 25x faster: a chapter load was 11 s in one frozen frame, now
  ~0.3 s.
- The editor SHOWS the ship's derived skin while you build it: **Ship Skin**
  clads the build and re-derives as the structure moves. The toggle rides Play.
- The editor grows a **parts gallery**: a full-screen catalog browser with a
  live 3D preview per tile, a category row, a text filter and a focus card.
- **(breaking)** The Components drawer is gone; the parts gallery replaces it.
- The focus card takes direction: drag to turn the part, wheel to zoom, and the
  turntable picks up again from wherever you left it once you stop.
- Link points are visible while a part is armed: every free socket draws a ring
  and a stub, the one under the pointer draws bright, and the mating socket
  draws on the ghost.
- Placement pose control is reversible: the wheel rolls the ghost and
  Shift+wheel cycles its socket. `R` and `F` still step forward.
- `Q` picks up whatever part is under the cursor, Factorio-style, and the
  editor's bottom-left legend lists the keys that apply to what you are holding.
- `Tab` opens and closes the parts gallery from the build view, and `Q` over a
  tile takes that part and hands you back to placing it.
- The gallery's search takes the keyboard only once the field has the caret
  (`/`, or click it), so letters stay shortcuts; `Enter` opens the top hit.
- The socket cycle moved from Shift+wheel to **Ctrl**+wheel: Shift is the
  free-fly rig's descend key, so cycling a socket also sank the camera.
- The preview ship draws its forward direction, which a pile of boxes otherwise
  has no way to show.
- The editor lights its scene with a key and a rim, and a gallery tile is
  fitted to what it DRAWS rather than to its collider.

### Audio & Visuals

- Torpedo detonations are brief white-hot ejecta in vacuum, without the orange
  sphere that drew their damage radius.
- The civilian kit grows to twelve pieces: faired vent, flush door, tank
  blister, comms radome, skylight strip, advert panel and a registry mark
  that lies on any facet a thin hull offers.
- The armoured kit grows 4 to 10: stub mast, shuttered intake, magazine, chaff
  tube, applique tiles and white rounds-count stripes beside every gun well -
  low, bolted, gunmetal plus one stencil white.
- Every skin style now dresses hand-built hulls: the windows, sensor and
  radiator signature pieces reach real builds, and each kit carries one
  cone-friendly piece for one-cell-thick shapes.
- The industrial skin doubles its kit to 14: crane, plate stock, battery
  rack, umbilical row, winch, floodlight and a stencil placard that reaches
  one-cell-thick builds.
- A second skin bench at ship scale: `block_bench` stands eight hand-placed
  building block ships - destroyer wedge, spine freighter, carrier deck -
  clad, labelled and reported like `shape_bench`.
- The salvage kit doubles to 14 pieces: a bent-slat grille, a sagging hose, a
  kill tally, a cog patch, a cargo net, a stolen cobalt dish and a tow chain -
  accumulation, still one hue of ham.
- Turret rounds are modelled per DAMAGE TYPE: a stubby Kinetic tracer, a thin
  Pierce needle, a squat Explosive shell, each in the type's own HUD colour.
- Torpedoes fly nose-first: the warhead is a coned body instead of a
  flat-ended pipe, and one shared mesh serves every launch.
- Camera shake is scoped to the player camera - the free-fly WASD camera never
  shakes - and the default shake is roughly halved, with the rotational kick
  cut hardest.

### Performance

- A gun round is swept math, not a physics body. A saturated point-defence step
  drops 2221 dynamic bodies to 340, and its worst step 17.4 ms to 11.8 - back
  inside the fixed tick.
- The fixed loop runs single-threaded: its schedules are too small to pay for
  the executor's fan-out. A 1v1 arena fight's per-step median drops 7.9 ms to
  6.1 and its 1% low rises 27 fps to 48.
- A section's placeholder body and its exhaust flames are built ONCE and
  shared, not minted per entity. Three WFC hulls introduce 83 distinct meshes
  instead of 242, and the frame drops 37%.
- A drive's exhaust plume draws from 16 SHARED materials per nozzle shape
  instead of one written every frame: a 12-bay point-defence range prepares
  1.3 ms of material a frame instead of 12.7.
- A launched torpedo shares its warhead material by ordnance TYPE instead of
  minting one per shot: the same range draws 17 distinct materials instead of
  105, and 16 crack buckets instead of 149.
- Damage cracks draw from eight SHARED materials per source, and an undamaged section keeps its own so a pristine fleet draws through one pipeline: a 4v4 arena doubles, driven combat gains 8.6%.
- Every particle effect sizes its buffer to the burst it fires: 2048, 512 and 512 instead of 32768 each, so a barrel, a warhead and a launch tube allocate 16 to 64 times less GPU memory.
- A piece of debris takes its collider from at most 64 strided points instead
  of every vertex of an unwelded triangle soup: the same shape for a fifth of
  the price, and slightly more come back usable.
- One blast cuts ONE crater per body however many colliders it overlaps, so
  peak live debris in the WFC arena drops from ~3700 bodies to ~1450.
- A scenario swap never blocks the main thread: queued spawns drain under a
  3 ms per-frame budget and the scenario is held until they land, so the
  LOADING panel animates across the whole transition.
- A load warms every hull the scenario can spawn, so a ship that first appears
  mid-mission arrives dressed instead of flying in placeholder art while its
  glTF loads.
- Held fire on a big rock runs at 47 fps instead of 25: its carve field caps at
  40 cells a side, the seed and remesh run off the main thread, and nothing
  rescans the grid to learn it has not changed.
- A destroyed section DETACHES and tumbles away whole, keeping the art and the
  collider it already had. Nothing computes geometry when something dies: the
  death path drops from 46 ms a run to 2.5 ms.
- An untouched rock costs collision almost nothing: it collides as its hull
  until something puts a hole in it, so a 64-rock field's physics step drops
  from 22 ms to 0.4 and stops outrunning the tick.
- Trigger volumes and salvage crates listen on their own colliders, not on
  every collision in the world: a point-defence range's 22,241 wasted observer
  calls vanish and its p99 drops 8.4 ms to 6.9.
- A scenario samples entity speeds only when a watch or an expression reads
  one, instead of walking every authored entity every frame.

### Fixes

- A missing or renamed asset now says so and stops, instead of leaving the
  loading screen turning forever with nothing logged.
- Treat a torpedo outliving its launch bay as a missing optional effect.
- Fixed: a crater captured hits by its own accumulated size, so every merge
  widened it and one hole ate the rock. Shooting a fresh place now opens a
  fresh hole; a held burst still digs one.
- Fixed: a hit carved what it ASKED for, not what it destroyed, so an overkill
  round bought material twice and a spent plate still threw debris.
- Fixed: closing NOVA OS in the WFC arena resumes combat instead of leaving the match clocks paused.
- Fixed: every ship section burst eight generic gray cubes instead of its own
  art - destruction looked for a mesh on the entity that dies, never on the
  descendants that draw it.
- Fixed: a dying section came apart into slivers - every cut plane ran through
  the world origin, so a piece off to one side was cut by a plane outside
  itself. Sections come off whole now.
- Fixed: a piece the cut missed hung motionless where the section died, because
  it was carried forward with no direction to leave in.
- Fixed: section death drew from the thread RNG and never played the same way
  twice. The kick and the tumble come off the seeded stream.
- Asteroids stop logging "no mass or inertia": a rock builds its collider in
  the same command batch as its body, so no physics tick sees it massless.
- A shot-down torpedo and a broken rock go on rails the moment they die,
  instead of drifting one frame as bodies with no mass left. The physics log
  is quiet.
- **(breaking)** An asteroid without an authored `seed` derives its silhouette
  from its object id: one rock per id still, but the same rock on every load
  instead of a fresh draw.
- `bug_neutralized_quiet` no longer flakes: its control half latches the first
  defending mount instead of re-reading a trigger that drops for a frame while
  the barrel slews.
- A freshly spawned ship no longer swings off its spawn attitude before flying:
  the helm command now initializes from the hull's spawn rotation instead of
  identity.
- A torpedo blast no longer survives Retry and destroys the reloaded scenario's
  rocks: a scenario now owns every transient it spawns, ordnance and cosmetics
  included.
- A sound effect no longer plays into the next scenario: the SFX one-shot's
  audio entity is scenario-scoped, so Retry and scenario switches silence it
  with the teardown.
- A neutralized hull stops shooting down torpedoes. Point defense was the one
  AI behaviour that never read the stand-down, so a wreck with nobody aboard
  kept working its mounts.
- The inside angle of an L is CLAD: a cell whose every outward direction was
  already plated used to be refused, leaving the corner section bare. Plates
  cannot be in each other's way.
- **(breaking)** A ship's HP bar no longer FILLS UP as the ship is shot apart:
  a root's maximum health is pinned to the hull it was built with.
- Weapons no longer offer a mating surface where their business end is: the
  shared PDC sockets its base plate only, and both torpedo bays drop the face
  they fire through.
- **(breaking)** Turret rounds deal their authored damage once, not twice: a
  collision event per event-enabled collider had the hit observer paying out on
  both sides.
- The editor sandbox no longer spawns you on top of the planetoid: it is a
  smaller, seed-pinned body far outside its own gravity reach of the spawn.
- ESC in the editor backs out (closing the parts gallery, then putting the
  armed part down) instead of stacking the pause overlay on top of it.
- TAB no longer arms the NOVA OS where there is no ship to fly: the editor's
  build mode is inside `Playing`, so a TAB there set the freeze state
  invisibly.
- The section keybind chips no longer hang over the parts gallery: they were
  positioned by a system keyed on the free-fly camera controller the gallery
  removes.
- A gallery rebuild no longer flashes its parts across the middle of the
  screen: the preview bundle's `Visibility` overwrote the tile's own hidden
  one.
- Two editor sections can share one keybind - two turrets on one trigger, two
  thrusters together. Only a key the flight rig already drives is refused.

- A mid-menu backdrop reload no longer crashes the UI layout: the menu
  interface renders through its OWN camera, so a camera swap cannot yank its
  render target.
- The menu's interface camera no longer re-renders world-space particle effects
  over the finished frame: the overlay draws on a UI-only render layer.
- Turret aim smoothing decays per second, not per frame: tracking lag no longer
  grows at low framerate, so a PDC on a struggling machine still reaches the
  fire gate.

### Internals & Tooling

- The dev book indexes every environment variable on one page: what each gates, which crate owns it, and whether it is harness-only, tooling or player-facing.
- Every environment variable the game reads is a declared constant with one home, and `tests/env_contract.rs` names the whole set - a new one off the roster fails a test instead of arming nothing.
- `--mute` zeroes the audio output, the other half of the outputs-off pair with `--norender`. `NOVA_MUTE` is its environment twin, and a muted run now says so once at startup.
- New probe capability: `NOVA_PROBE_STEPDIAG` writes a per-fixed-step CSV of avian's own phase timers and body counts; `NOVA_PROBE_STEPDIAG_BODIES` picks the body-count regime its summary covers.
- **(breaking)** Every `NOVA_PERF_*` environment variable is now `NOVA_PROBE_*`, and `NOVA_PERF` is `NOVA_PROBE` - finishing the crate's rename to `nova_probe`. No aliases.
- **(breaking)** `NOVA_SHOT` and its screenshot driver are gone. A range's one settled picture is `nova_screenshot(script)`, a beat on its own autopilot, armed by `NOVA_CAPTURE` like every shot.
- **(breaking)** `NOVA_SHOT_DIR` is now `NOVA_CAPTURE_DIR`: one name for the directory every still, loop and scenario `Screenshot` action stages under.
- A `--features debug` boot prints 62 lines instead of 360: per-item spawn and setup lines are `trace!`, batch lines carry counts, and one `nova=` prefix directive covers all 22 crates, not nine.
- A headless run no longer opens with a bevy ERROR and three WARNs it provokes by construction; the clamp is headless-only, so a rendering run keeps those targets at their normal level.
- A capture breaks its frame down by name: main-world schedules, render phases with `PrepareAssets`, `Prepare`'s sub-sets and submit/present split out, and GPU passes under `NOVA_PROBE_RENDER_DIAG`.
- A capture is measured, not paced: it sizes its window before winit creates it, runs continuously, and REFUSES a run the WM re-sized or the update mode throttled.
- A display cap is the repeat SET's call, never one window's: a period is a constant, so captures that agree on one are refused and a lone steady window is recorded as a suspicion.
- A measured run wears `WM_CLASS` `nova-measure`, so `for_window [class="nova-measure"] move container to workspace 3` keeps captures off the desk. A hidden workspace measures the same as a visible one.
- A capture counts the world it measured - entities per component, archetypes, and mesh instances against DISTINCT mesh handles - beside the frame-time stats.
- `NOVA_PROBE_PRESENT` names the presentation mode instead of requesting it; `wfc_ships --ships 0` stands the empty row, and a measured row holds its capture until the ships have finished spawning.
- The profile table counts deferred command flushes, where observers actually run, and the run report leads its frame section with worst frame, mean and the implied FPS, flagged under 60.
- `probe run --repeat <n>` gates each capture on its mean and median before reading the worst frame, and a capture records fixed steps per frame plus a `NOVA_PROBE_MAX_DELTA` ceiling knob.
- A capture REFUSES a window it cannot measure - a stopped simulation, or a scene the example says is over - with no stats and a named failing check; a scene that can end declares its own window.
- The run report STREAMS trace.json instead of reading it whole: a 2.6 GB trace costs 70 MB of host memory instead of 11 GB, and a trace cut short still ranks what it did record.
- Each `screenshots/` producer captures ONE thing in at most three frames, and its name says what it makes: `screenshot_*` stills, `loop_*` video.
- Make the asteroid gate hold real PDC fire on a shipped-size rock.
- The `system_destruction_finale` range kills a gltf section, a procedural one, a multi-part turret and an asteroid, asserting each breaks into its own art on one per-body budget.
- Controller mods author `steering_lag` in seconds instead of internal PD frequency and damping fields; mixed stacks use the fastest live computer. **(breaking)**
- A driven run turns the wheel (`scroll_lines` / `scroll_pixels`), so a scripted beat reaches a row past the fold instead of skipping it.
- `bug_menu_picker` scrolls the picker and now measures every scenario row, none skipped.
- The `system_nova_os` range opens the ship computer with Tab and clicks a widget THROUGH the CRT glass, asserting the whole forwarded-pointer chain live.
- Fixed: a synthesized click never reached the NOVA OS screen - the forwarded pointer read a button stream `bevy_picking` does not write.
- Fixed: the NOVA OS glass was measured in physical pixels against a logical cursor, so every click missed by the display's scale factor.
- `probe scenario <id|file.ron>` measures a scenario through the game binary - no example, no `[[example]]` block. A loose content file works whether or not it ships.
- `--scenario-file <path>` boots the game into a loose `*.content.ron` without installing it, and `--scenario editor_sandbox` now resolves.
- `--norender` boots with no GPU device, no window and no event loop, so a run measures the simulation alone. `NOVA_NORENDER=1` does the same to any example. Both need a driver to ever end.
- `probe run --norender` takes every pass headless and starts no Xvfb. A speed option: with no device it cannot see a render-side panic, so it runs beside a rendered pass, never instead of one.
- Probe parses with clap: `--help` is generated per verb instead of one hand-kept usage block, and `--render`/`--norender` refuse each other. Same flags, same defaults, same exit codes.
- The bug-to-range doctrine is written down (`AGENTS.md`): reproduce a bug
  as a `systems/` range before the fix, and name every claim on the invariant
  roster.
- Every `systems/` range is named for the KIND of check it is: `system_`
  functionality, `bug_` a regression range, `stress_` load.
  `examples/systems/README.md` is the rule. **(breaking)**
- No `systems/` range asserts a millisecond. `bug_sandbox_soak` holds the
  unshot rock field to colliding as HULLS and records its physics cost instead
  of gating on it.
- Four single-file stress ranges replace the `many_*` sweeps: a thousand rounds,
  a thousand torpedoes, a thousand sections on one hull, a hundred hulls - each
  with exact counts and a drain to zero.
- Examples are filed by WHO they are for: `playable/` a person works,
  `systems/` asserts, `screenshots/` captures. Every autopilot-only example
  now says so in its `--help`. **(breaking)**
- Every `systems/` range names its invariants as `outcome:` markers on the
  `catalog_drift` roster, so a deleted assertion fails a test instead of
  passing green.
- `nova-protocol --scenario <id>` boots straight into that scenario, past the
  main menu; an unknown id refuses the launch and lists every registered id.
- `wfc_arena` opens cold - lines spawn ~305 u out, weapons free at gun range -
  follows look over their ship at the enemy mean, team chevrons mark the
  sides, and small wreckage fragments form junk blobs.
- `wfc_arena` takes a `:player` roster slot: fly a drafted hull with the
  game's own controller and HUD while the example's cameras stand down.
  Refused under the driven walk.
- `wfc_arena` takes a repeatable `--ship TEAM[:STYLE[:SEED]]` roster, loads half
  of every hull's bays with Lances, draws both PDC mounts, and remaps the
  cameras: Q orbit, E overview, 1-4 follows.
- New `greeble_catalog` example: every fixture model the merged styles own in
  named rows on style-tinted pedestals, with idle orbit, focus turntable and
  per-style captures.
- `wfc_arena` grows number-row camera poses (frame, broadside, shoulders,
  top-down) and the shared idle orbit; grave/tilde hides the readout in
  hand-runs of all three bench examples.
- New `screenshot_thruster_gallery` example: the shipped drive, the proposed
  thruster shell size family and the CC0 candidate models in one named row, in
  the fleet capture idiom.
- Seven recipe-generated thruster shells - five 1x1 plus 3x3x1 and 5x5x3 - in
  one mechanical voice (`scripts/gen-thruster-shells.py`, deterministic
  `--check`), judged in `screenshot_thruster_gallery`.
- New `wfc_arena` example: two collapsed hulls drafted from the seed stream
  fight under the campaign AI, clad, over a dressed arena; a scoreboard logs
  shots and damage both ways. `R` re-rolls.
- New `shape_bench` example: a fixed roster of hand-placed structures, clad,
  named and held still, with the skin report printed per subject and style so a
  render is judged next to its numbers.
- The `wfc_ships` row holds still and turns on a slow orbit while nobody flies
  it. The subjects were drifting on their own spawn impulse, not on input.
- Engine tests stopped reading installed mods: seven rigs over `webmods/` are
  rebuilt as four synthetic-scenario rigs with generic ids, so no mod can block
  an engine change.
- New probe capability: `NOVA_PROBE_SNAPSHOT` dumps world state as JSONL - ships,
  sections, fixtures, weapons, ordnance - sorted and rounded so two snapshots of
  one frame diff clean.
- A snapshot carries the DERIVED SKIN: per plate the eight boundary samples, the
  relief and the zone facts, per ship the shape statistics and every cell the
  derivation refused to clad, with the reason.
- The base mod generates its own decoration art: `scripts/gen-greebles.py`
  builds one `.glb` per JSON recipe in `scripts/greeble-recipes/`, gated by
  `--check`.
- The three mesh scripts share one stdlib-only glTF writer,
  `scripts/nova_glb.py`; the 21 shipped ship-part meshes re-cut byte-identical.
- New `wfc_ships` example: wave function collapse over the section
  catalog, where the adjacency rules ARE the link points. Every hull goes
  through `lint_scenario`.
- `wfc_ships` collapses STRUCTURE and nothing else - it builds no skin and
  names no plate. `--bare`, and `C` in a hand-run, turn the skin flag off on
  the same seeds.
- `wfc_ships` states its rule as: a socket may never press into a face that has
  none. Two blank faces may touch. It does NOT model CLEARANCE - the content
  owns that.
- `wfc_ships` no longer masks a face off the drive: the thruster stopped
  calling its own nozzle a mating surface, so the generator only reads link
  points.
- `NOVA_MENU_BACKDROP=<scenario id>` pins the menu's backdrop draw for
  capture and authoring runs; unknown ids fall back to the random pick.
- The autopilot gains a `type_text` gesture: a driven run can type into a text
  field, which `press_key` cannot do.
- `cut-obj-into-parts.py` proposes link-point candidates from recipe seams, one
  socket per shared face; a recipe part can author its own list instead.
- Built-in authoring content gains an explicit `base_content` inventory of
  chapters, backdrops, sandboxes, section prototypes and semantic parts;
  generated RON is unchanged.
- Developer docs are an mdbook published at `/dev/`, sourced from `docs/`;
  the ephemeral-docs scratchpad model is retired.

## [0.10.0] - 2026-08-13

### Ships & Sections

- **(breaking)** Ship integrity uses explicit section link-point mates;
  collider contact and centre spacing no longer make structural edges. `G`
  toggles a MATES overlay.
- **(breaking)** Racer, CargoA and CargoB use semantic parts (`fuselage`,
  `engine_port`, `turret_starboard`); coordinate-named cube prototypes are
  removed.

### Gameplay & Flight

- **(breaking)** Gravity wells are authored by MASS:
  `AsteroidConfig::surface_gravity` becomes `mass`, with reach set by
  `GravitySettings::soi_cutoff_accel`.
- The chase camera takes the SHORT way around the rear (-Z) seam: an orbit that
  crossed it used to swing the long way round as the angle wrapped.

### Combat & Weapons

- Neutralized wrecks leave threat tracking and AI acquisition but stay
  combat-lockable: a hollow wreck chevron and a `NEUTRALIZED` readout.
  Destruction shows a ribbon.
- AI burst cadence ticks on the fixed clock, so AI damage output no longer
  varies with framerate.

### Scenarios & Objectives

- **(breaking)** Scenario world reads use typed queries and declared watched
  variables: `Scenario(Elapsed)` and `Entity(... Speed)` replace the implicit
  values.
- Rust scenario authors gain one public `nova_authoring::scenario_helpers`
  catalog for common expression, filter, watch and action constructors.
- Scenario handlers gain `OnDefeated`, an exact-once ship outcome edge shared
  by neutralization and destruction; it precedes `OnNeutralized` or
  `OnDestroyed`.
- **(breaking)** Scenario lighting is authored content: a new `Light` object
  replaces the engine's hardcoded key light, so a scenario with no light
  renders black.

- Shakedown Run is dressed: the planetoid moves in to ~760u of the spawn and a
  78-rock slalom belt bends around it, every knot clear of the beat pockets.
- `ScatterRegion::Ring` gains an optional `center`, so a belt can circle a body
  that does not sit at the world origin. Omitted, it is the origin as before.
- `ScatterObjects` gains an optional `min_separation`: scattered bodies are
  kept that far from every body scattered so far, across sibling scatters.

### Modding & Mod Portal

- **(breaking)** Recurring `OnOrbit` and `orbit_hold_secs` are replaced by
  one-shot `OnOrbitStart`, `OnOrbitStable`, `OnOrbitUnstable` and `OnOrbitEnd`
  edges.
- **(breaking)** Recurring `OnTravelLock` / `OnCombatLock` and per-player
  `lock_refire_secs` are replaced by one-shot start/end lock lifecycle edges.
- Scenarios gain keyed, pause-frozen timers: `TimerStart`, `TimerCancel`, and a
  one-shot `OnTimerEnd` event with a timer-key filter.
- The scenario lint warns on a zero delay written as a value:
  `auto_advance_secs: Some(0.0)` and a `NextScenario` `delay: Some(0.0)` both
  mean `None`.
- Portal republish so installed copies get the relit scenarios and
  mass-authored wells: Gauntlet Run 1.3.0 -> 1.5.0, The Ledger 1.14.0 ->
  1.16.0.

### Interface & HUD

- Setting buttons commit on RELEASE over the button, not on mouse-down: press
  a wrong option, drag off and release, and nothing changes.
- Every button variant has its own pressed face on BOTH skins: `Ghost` and
  `Primary` had no press feedback, and the hardware Exit/Danger face gains a
  sunk one.
- The phosphor slider track drops its 2px inset, so the lit edge lands on the
  value the click actually commits instead of ~3px off it.
- The block meter no longer rounds a near-full value to full or a near-empty
  one to empty: 98% reads as 98%, 2% reads as 2%.

### Internals & Tooling

- The dev CLIs are subcommands of the game binary: `cargo run content gen|lint`
  and `cargo run --features debug probe run|report` replace the standalone
  bins.
- Probe runs declared frame-time capture and native tracing automatically;
  `--fps` and `--profile` are removed, while slow `--samply` stays opt-in.
- All probe examples use the unified `NovaProbePlugin`; `probe run --correctness-only` keeps behavioral evidence while omitting frame-time and profiling passes, and is the CI windowed gate.
- **(breaking)** `ScenarioConfig` no longer derives `Default`; build one with
  `ScenarioConfig::new(id, name, cubemap)` plus struct-update syntax. A
  defaulted `cubemap` was never a valid scenario.
- `examples/sections/`: five ranges, one per section family, each walking a
  named roster of invariants; `sections_assert_their_invariant_roster` pins all
  27 names.
- `scripts/serve-web.sh`: one-command live web preview - site, game and mod
  portal on free 7XXX ports, proxied onto one origin, all watched.
- `scripts/serve-mods.sh`: builds and serves the mod portal, regenerating on
  every `webmods/` edit.
- Web dev server picks a free port in 7000-7999 instead of `:8090`, and proxies
  `/mods` alongside `/play`.
- `Trunk.toml`: the dev `[[proxy]]` moved to `TRUNK_SERVE_PROXY_BACKEND`; a
  config-file entry conflicts with the env one and panics Trunk.
- Dev shell: added `watchexec`.
- New `nova_autopilot` crate: the automation drivers, the `capture` primitive
  and the run-completion protocol, depending on `bevy` alone.
- Harness environment variables renamed `BCS_* -> NOVA_*`; the deadline's stem
  moved too (`BCS_HARNESS_DEADLINE -> NOVA_AUTOPILOT_DEADLINE`), so it is not a
  pure prefix swap. **(breaking)**
- One capture idiom: the screenshot reel is deleted; a capturing example is an
  ordinary autopilot script whose steps call `nova_debug::harness::shoot`.
  **(breaking)**
- Every shot ACKS: `capture_window` records the path in a `CaptureLog` once the
  PNG is on disk, so a shot step holds on `until(shot_written(name))`.
- Dev wiki: "Automation harness" page for the `nova_autopilot` drivers.
- `nova_autopilot`: curated prelude, crate-level env contract table, and a
  `completion` doc example.
- `nova_autopilot` is predicate-driven: a script is a list of NAMED STEPS, each
  advancing when its predicate over the world holds. A stall error-exits naming
  the step. **(breaking)**
- `nova_autopilot::input`: synthesized keyboard and pointer gestures that leave
  the world in the state a real device would.
- `nova_debug::harness`: Nova-typed predicates `scenario_variable_is`,
  `section_gone` and `player_ship_present`, so a script waits on what the game
  agreed happened.
- **(breaking)** Probe coverage is a HANDSHAKE, not a table: every probe plugin
  declares its capability into `probe-contract.json`, and each check resolves
  against that.
- New `examples/systems/` category: code-built `ScenarioConfig` fixtures for
  cross-cutting systems. `scenario` -> `systems/scenario_grammar`, `playable`
  -> `systems/player_path`. **(breaking)**
- `nova_probe` invariants: a registered monotonic is one-way within a SCENARIO
  LIFE, not for the process - the memory is forgotten on `ScenarioLoaded`.
- `examples/ui/` rebuilt around real pointer input: four of the five runs DRIVE
  the interface and check the live tree after every rebuild. `hud_range` stays
  predicate-driven.
- `nova_autopilot::input`: `click_named` / `hover_named` / `ui_node_centre` /
  `ui_node_rect` resolve a click target by `Name`, so a layout move is
  survivable.
- `nova_debug::harness::REACHED_PLAYING`: the smoke sentinel is a const, named
  by its two emitters.
- **(breaking)** `tests/examples_smoke.rs` is deleted; CI's windowed smoke step
  becomes the probe correctness sweep (`probe run --all` under Xvfb).
- One camera-authority chain (`CameraAuthority { Solve, Additive, Override }`)
  declares who writes the camera `Transform` and in what order.
- Nova owns its health pool, damage typing and destruction pipeline
  (`nova_gameplay::integrity`) rather than importing them; ram damage routes
  through the typed path.
- One persistence store (`nova_assets::persist`) replaces the two hand-rolled
  copies behind the mod set and the settings menu. Storage locations are
  unchanged.
- `AppBuilder::with_main_menu` and `nova_ui`'s `debug` feature are deleted; the
  menu fronts the default app and nothing else. **(breaking)**

## [0.9.1] - 2026-08-02

### Web & Platform

- Web release builds compile after the pause-menu split stopped importing the native-only Exit handler on wasm.

## [0.9.0] - 2026-08-01

### Scenarios & Objectives

- Scenarios tab groups campaigns under collapsible `[-]`/`[+]` headers listing each campaign's ordered chapters - including hidden mid-story chapters - so any chapter is launchable directly for replay.
- Ships are combat-dead when out of the fight: an armed ship that loses ALL
  weapons AND thrusters is NEUTRALIZED, a drifting-wreck state firing a new
  `OnNeutralized` event.

### Modding & Mod Portal

- New `Campaign` content kind: a bundle declares an ordered campaign->scenario
  mapping loaded into `GameCampaigns`; the lint flags a member scenario no
  bundle provides.
- The Ledger (1.12.0 -> 1.13.0): its six chapters group under a "The Ledger"
  header, the hidden reward finale is replayed from it, and titles drop their
  prefixes.

### Interface & HUD

- Objectives post as a **notification stack** at the top of the flight HUD: one
  amber chip per posting, leaving after a dwell or when NOVA OS opens.
- The flight HUD becomes CONTEXTUAL: elements surface only while their
  situation is live, and the `~` cycle collapses from All/Minimal/None to **On
  / Cinematic** (breaking).
- The flight HUD adopts the phosphor chip language: the `[KEY] VERB` cluster
  becomes a bottom-centre **icon dock** of the verbs you can use now, and every
  readout is a chip.
- Scenarios picker holds its layout: the list pane keeps a fixed share of the
  screen, and a campaign's chapters are indented under their `[-]` header.
- The menus and editor chrome adopt the NOVA OS visual language: the navy/cyan
  theme retires for green phosphor, with a persisted Phosphor / Hardware `UI
  skin` setting.
- Player-facing distances and speeds read at 1 world unit = 10 m everywhere;
  the `u`/`u/s` unit retires from the HUD, the map and the wiki. Display-only.
- Comms panel becomes a bottom-left stacked chat surface with optional authored speaker icons, timeout/dismiss controls and skip-to-next backlog control.
- Tab ship-computer drawer: one inset NOVA OS monitor opens on Tab, pauses the
  game and frees the cursor, with a green phosphor terminal screen and CRT
  treatment.
- NOVA OS screen is a real CRT: the terminal renders to an offscreen image
  through one sampling shader, so glyphs bloom and the screen bows with barrel
  curvature.
- NOVA OS terminal readability pass: bright phosphor text on a near-black CRT,
  a dark input strip, fish-style inline completion and the Iosevka terminal
  font.
- NOVA OS terminal prompt: Tab completes commands, Esc closes, and the shell
  has `help`, `log`, `objectives`, `ship`, `clear` and `exit`, with history and
  suggestions.
- NOVA OS help/usage reads like a real shell: a `Usage:` synopsis, an aligned
  `Subcommands:` section, and `command: reason` on a wrong argument.
- Flight objective surface: the always-on compact objectives panel is gone; a posting is a chip in the objective stack and the detailed objective output lives in the NOVA OS `objectives` command.
- NOVA OS command output prints the combined Flight Log, active objectives and live player-ship section status without restoring permanent drawer panes.
- Allegiance markers: a small filled triangle floats above every ship, coloured
  by side (green ally, red threat, grey neutral); your own ship shows none.
- Startup shows a phosphor NOVA OS loading screen while assets preload, instead
  of a blank native window, and hands off to the menu once loading finishes.

### Web & Platform

- The website adopts NOVA OS: the navy/cyan palette retires for green phosphor
  and the site wears the game's PHOSPHOR skin, typeset in JetBrains Mono. `npm
  test` guards drift.
- New `scripts/shoot-web-pages.sh`: builds and serves the site, then
  headless-captures the six page kinds at desktop and mobile widths. `npm run
  ci` runs `npm test` too.

### Fixes

- The combat lock no longer lets go mid-fight: FIRING counts as combat activity
  for the 30 s idle decay, the decay is now visible, and every automatic drop
  names its cause.
- World-anchored nav chips show a full background: fill and border wrap the
  whole label instead of collapsing to a slab in its top-left corner.
- Clicks inside NOVA OS land where the picture is: the pointer was mapped
  through the barrel warp's inverse rather than the warp itself, and skipped
  its overscan.
- Tab drawer scrolling now clamps at the content bottom, so wheel-up responds immediately after reaching the end.
- Web build: NOVA OS text is visible. The UI font shipped with no `.meta`
  sidecar, so under `AssetMetaCheck::Always` every glyph rendered invisible;
  the generator emits it now.

### Internals & Tooling

- `nova_probe` runs are profile-sandboxed: each native child run gets an empty,
  probe-owned profile under its run dir, so a local mod or setting cannot
  decide a result.
- The Iosevka Term terminal font is now credited in `credits/` under its SIL Open Font License 1.1 (copyright and full license text bundled with every build, as the OFL requires).
- Input-prompt key glyphs (JulioCacko's FREE Input Prompts, CC0) move to
  `assets/input-prompts/keyboard/Alt/` and are credited; only the Alt style
  ships.
- Static assets preload through `bevy_asset_loader` collections and are
  load-gated before gameplay. The UI font ships as one Iosevka Term `.ttf` cut
  from the 66 MB `.ttc`.

## [0.8.1] - 2026-07-24

### Web & Platform

- Landing hero gains per-OS download buttons (Windows/macOS/Linux) that deep-link the matching release asset, with a fallback to the releases page.

## [0.8.0] - 2026-07-23

### Gameplay & Flight

- Unarmed AI ships are now non-combatants: no target, no chase, no fire - still targetable, so an escort/convoy must be defended.
- Gravity wells pull only PILOTED ships; an unpiloted bystander floats where it sits instead of being dragged in.
- Lifeline's convoy haulers now fly (unarmed non-combatant AI) instead of drifting off when shoved.

### Scenarios & Objectives

- Base-campaign pacing pass: an objective posts a beat AFTER its comms line, never on the same frame, with a breather between objectives (shared scenario-pacing toolbox).
- Shakedown Run opens slower: Capt. Halloran briefs you over comms (~40s) before objective 1; objective texts shrank to the bare goal.
- New scenario **Lifeline** (ch3 pt1): screen a two-hauler convoy through three raider waves to a live `RELIEF mm:ss` countdown - the first shipped ally content.
- New scenario **Final Tally** (ch3 pt2, finale): survey + orbital picket +
  capital-escort fight in the base chain's first combat gravity well. New Game
  arc is five scenarios.
- Broadside found its voice: story moved to the comms panel, objectives shrank to imperative goals, Victory banners track the Ceres Queen's fate.
- The **Asteroid Field** sandbox is back in the Scenarios picker (it had been wrongly hidden as unreachable).
- New `HudReadout` action: show a scenario variable on the HUD (`Number`/`Integer`/`Time`), an Instrument-tier readout, pause- and teardown-safe.
- New `SetAllegiance` action: flip a ship's allegiance mid-scenario - the neutral-until-provoked primitive.
- New reserved `player_speed` variable: the player's live speed, engine-written and read-only, to gate beats on how fast you fly.

### Modding & Mod Portal

- Gauntlet Run is now a TIME-TRIAL (1.3.0): a live `mm:ss.s` clock and a clean-run bonus, built on `HudReadout`.
- The Ledger grew 1.5.0 -> 1.12.0: a campaign-wide pacing pass, a ch3 stealth
  run, a forking ch4 finale and a fifth reward chapter; re-published to the
  portal.

### Interface & HUD

- The Scenarios picker groups the base storyline as a campaign: scenarios declare an optional `campaign` (name + order); mods can group their own chapters the same way.

### Fixes

- Destroying a sectioned ship no longer crashes: the damage-tint tolerates a section the explosion destroyed the same frame.
- The mouse cursor is hidden while flying, dev builds too; the `--features debug` layer now boots OFF and F11 toggles all of it as one.

### Internals & Tooling

- A pre-commit `cargo fmt --check` hook makes rustfmt drift impossible to LAND (arm once with `scripts/setup-hooks.sh`).
- `content lint` is the single content command: `audit` folded in, plus a
  flight-rig input-overlap check; `--target <mod> --report <path>` writes a
  per-mod report.
- The `nova_perf` crate became `nova_probe`, the run-harness crate (bin names, `NOVA_PERF_*` env vars and output formats unchanged).
- One front door: `probe run <example>` runs a clean pass plus optional trace
  and flamegraph, then a self-contained `report.html` and `checks.json` with a
  verdict.
- New run-timeline recorder (`NOVA_PERF_TIMELINE`): ordered JSONL of state transitions, fired events, variable changes and beat markers, flushed per entry.
- Continuous invariant checks (`NOVA_PERF_INVARIANTS`): per-frame health/velocity/variable/entity-count assertions that land on the timeline.
- Probe timelines emit outcome markers at their assertion sites, so the timeline shows the FEATURE working, not just the process surviving.
- The whole example fleet is probe-evaluable: every cataloged example carries the recorder, invariants and frame-time capture (all inert without probe's env).
- Fleet runs: `probe run <list|category|--all>` run sequentially with an aggregated `index.html`/`index.json` status index and worst-row verdict.
- `--fps` is a dedicated capture-only pass; narrative one-shots are fps-EXEMPT via `Cargo.toml` metadata; the capture window and completion deadline are sized to the request.
- `--baseline` works across a group: regression-check a whole fleet against a prior `--all` run (missing captures are SKIPPED, not errors).
- Harness runs are silent: any bcs harness env zeroes the audio output (`NOVA_MUTE` overrides); the volume setting and menu are untouched.
- Examples moved into purpose directories (`sections|gameplay|ui|screenshots|perf/`) with the root `Cargo.toml` `[[example]]` catalog as the single source of truth; per-category smoke tests.
- Profiled pass renders a top-N costliest-systems table (Perfetto chrome trace via a new `trace` feature); `--samply` adds a flamegraph on a dedicated profile.
- Frame-time rows record run metadata (backend, GPU, resolution, preset, git SHA, host) and build profile; pre-metadata files still load.
- Run-harness hardened for unattended use: a cleaned run dir + `probe-run.json` manifest, so a hung or crashed run still produces a FAILing report.
- bevy_common_systems 0.19.1 -> 0.19.2: `GameEvent` gained public `name()`/`info()` read accessors for external observers.
- `docs/` wipes to only its README at a release (guard-enforced).

## [0.7.0] - 2026-07-18

### Gameplay & Flight

- RCS fine docking thrusters: hold SHIFT to translate along the ship's own axes under a ~2 u/s cap (no rotation) - a per-ship controller verb, withheld in the mainline.
- GOTO/STOP autopilot now settle their arrival on RCS, easing to a stop instead of pulsing on the spot.

### Combat & Weapons

- Turrets fire and aim every muzzle they have: twin-barrel PDCs throw two streams at once, sharing one magazine.
- Hostiles respect line of fire: hold fire (and torpedoes) when hard cover blocks the shot; intangible volumes and PDC-vs-torpedo exempt.
- Weapons auto-reload: a dry magazine refills on its own, so magazine size is a fire-pacing beat and finite ammo is on by default.
- Locking is no longer instant: a lock-on dwell (longer at range) you must hold the target through before it commits.

### Ships & Sections

- Enemy ships show battle damage: a destroyed hostile section reads burnt-black (no intermediate red on the enemy hull).

### Scenarios & Objectives

- Scenarios can declare a win/lose: new `Outcome` action shows a VICTORY/DEFEAT overlay with real buttons and freezes the sim behind it.
- **Broadside**, chapter two of the base storyline: a distress call, a two-corvette ambush, then screen a gunship's torpedoes - chained from Shakedown's victory.
- Broadside plays as two scenarios (checkpoint after the ambush; the gunship retries itself); invulnerable boulders anchor the big fights.
- Campaign + Ledger storytelling pass: fights announce themselves on an arrival
  grace, comms beats spaced on the scenario clock, closing lines moved into the
  banner.
- Scenario transitions got a middle gear: `NextScenario` gained a `delay` and Outcome banners an `auto_advance_secs`; lint warns on pairing an Outcome with a non-lingering switch.
- The Ledger chapter two rebuilt around fair fights (single lane, mook guns, a cover field) and split into two checkpointed acts.
- The Ledger finale's Auditor traded its top-tier turret for the light mook gun on both ending branches.
- Every combat scenario now flies real auto-reloading ammo (unlimited stays an authoring/testing option).
- Ships in scenario RON can author an `allegiance` (e.g. `Some(Neutral)` for bystanders).
- The Asteroid Field sandbox joined the outcome frame (Victory on arrival, Defeat + Retry on death).
- The ship-less Demo Scenario was removed; the example mod's arena is the worked hand-authored RON example now.

### Fixes

- The Rust Tally's side mounts now roll to seat against the spine and face outboard (firing arcs fixed, port/starboard ids un-swapped).
- The Ledger Auditor's torpedo bay comes out of the hull (flush on the bow); a new overlap lint catches the whole class at build time.
- Skybox `.meta` sidecars are honored for every asset (base and mod, shipped or downloaded), fixing the WebGL2 oversized-upload crash.
- Retry no longer blanks the objectives panel (the objectives HUD resets on teardown).
- A ship that loses its last section off the damage path no longer lingers as a 0-HP ghost (structural death backstop).
- Scenario `OnUpdate` handlers now freeze while the game is paused.

### Modding & Mod Portal

- Turret mounts are an arbitrary joint tree **(breaking)**: `root` + recursive
  `children` replace the fixed yaw/pitch/barrel fields; `fire_rate` is
  per-muzzle.
- Scenarios can tell time **(breaking)**: reserved `scenario_elapsed` variable (live unpaused seconds) readable from any filter; writing it is a lint error.
- `orbit_hold_secs` / `lock_refire_secs` are author-tunable per ship (default 5s; a non-positive value is a lint error).
- Mods can ship their own art: a `resources` list + `self://` paths that resolve against the mod's own folder.
- Asset paths always carry a scheme **(breaking)**: `self://`, `dep://<id>/`, `dep://base/`; the base game is a normal mod now and a bare path is an error.
- One `example` mod is the single copy-me tutorial (section overlay, new section, arena, shipped art, comms, Outcomes, menu backdrop).
- New `menu_backdrop` flag: the menu picks a flagged scenario at random; New Game moved to the base bundle's `new_game_scenario`.
- Two new menu backdrops: Waystation Traffic and Scrapyard Drift.
- New `StoryMessage` action: speaker-attributed dialog in a HUD comms panel.
- Broken mod content fails loud: reference errors give a FAILED TO START report; the backdrop rotation skips them.
- `content -- lint` lints every scenario for reference bugs (CI-enforced); `--target <mod>` scopes to one mod.
- **The Ledger**, the first campaign mod on the portal: a four-chapter salvage arc with a two-ending finale. Install from Mods > Explore.
- Every world sound is content-owned: weapon/controller/thruster/crate sound fields and per-target `impact_sound`/`destroy_sound` are authorable refs; base sounds ship under `assets/base/sounds/`.

### Interface & HUD

- Comms chatter is readable: lines queue in arrival order, fade in, hold a dwell and yield; newly posted objectives flash gold.
- The diegetic ammo gauge shows reload as a dimmer sweep filling the pips back up.
- The pause menu gained a Retry button (offered only while a scenario is live).
- The Settings menu is real: a master volume slider, a Low/Medium/High graphics preset, and a keybind reference, remembered across restarts.

### Performance

- Low/Medium presets skip heavy visuals for weak machines: Low is spawn-less (no particle bursts), Medium keeps particles.
- Low also renders the world at ~70% internal resolution and upscales (HUD stays crisp); Medium/High untouched.

### Internals & Tooling

- Content tools are one CLI: `content -- <gen|lint|audit>` replaces the three former binaries.
- The content lint checks mount seating (a base face must seat against an occupied neighbor cell), at build time and in the in-game gate.
- The two 5s scenario-event windows now measure against `scenario_elapsed` (one place freezes and resets them on pause/teardown/retry).
- The balance audit learned acknowledgments (`balance_acks.ron`): intended drama acked with a reason + task; errors can't be acked, a stale ack fails CI.
- `content -- audit` grades each combat scenario's fairness sheet and fails CI on an armed hostile that spawns inside its own range of the player.
- Firing no longer spams an avian "no mass or inertia" warning (bullets carry an explicit angular inertia).
- Debug builds bind F12 to a screenshot saved to Downloads.

## [0.6.0] - 2026-07-16

### Scenarios & Objectives

- Scenarios picker on the main menu: two-pane overlay listing every base and mod-added scenario, with a details pane and Play button; scenarios gained optional `thumbnail` and `hidden` fields.
- New `SetSkybox` action swaps the skybox cubemap mid-scenario, deferred until the image loads so a bad path leaves the sky unchanged.

### Modding & Mod Portal

- Mod dependencies resolve end to end: installs auto-pull missing deps,
  enabling a mod enables its transitive deps, and merge order is
  dependency-respecting topological.
- Static mod portal now publishes on every deploy: validated, sha256-hashed bundles under `/mods/<id>/<version>/` with a generated `catalog.json` (first mod: Gauntlet Run).
- Local mod cache foundation: a `mods://` asset source, a RON installed index, and downloaded bundles that load and merge through the same pipeline as shipped ones.
- Portal client fetches `catalog.json` and installs/uninstalls over the wire on native and web, staged installs verified against size + sha256 and committed only once every file checks out.
- Explore online tab is real: browse the portal in-game and install/update/uninstall on native and web, with per-file progress, offline catalog fallback, and enabled-state-preserving updates.
- Mods menu is a two-pane Factorio-style screen: `Installed` | `Explore online` tabs over the scrollable list with per-row enable checkboxes and a details panel from bundle meta.
- Mods menu hides dev/tooling mods: `hidden: true` entries stay installed and code-enableable by id but drop out of the player list.
- **(breaking)** Mod metadata moved into each `*.bundle.ron` as a `meta` block; `assets/mods.catalog.ron` slimmed to a thin pointer list (catalog and out-of-tree bundle format break).

### Web & Platform

- Particle effects (muzzle flash, projectile trail, torpedo launch/detonation bursts) now render in the web build after moving from WebGL2 to WebGPU.
- Browsers without WebGPU get a clear "WebGPU required" message instead of a black canvas, plus a heads-up under the landing page's "Play in browser" button.

### Performance

- Modding event dispatch is indexed by event name (upstreamed to bevy-common-systems rev 4c81117): 17-24% faster under bursts of 500-5000 handlers, neutral at the realistic one-event-per-frame rate.
- Added a criterion benchmark for the scenario-dispatch hot path (`cargo bench
  -p nova_scenario --bench scenario_dispatch`); the measure-first gate is
  documented in its task.
- Sibling filter-key-interning and condition-eval-compile optimizations were measured and deferred: at realistic event rates their per-handler cost is noise, kept as documented insurance.

### Fixes

- Scenarios picker no longer crashes the renderer on a non-2D thumbnail: such thumbnails are skipped with a warning, and images mount only once loaded.
- Local mod-portal web testing no longer needs a cross-origin `?portal=` override: `scripts/preview-web.sh` serves the portal same-origin as the game, matching production.

### Internals & Tooling

- Screenshot Reel capture set no longer ships in the game assets: its scenario moved into the example that films it, so players and the web build stop downloading a capture tool.

## [0.5.2] - 2026-07-14

### Gameplay & Flight

- Enemies can be authored to ARRIVE instead of appearing: a ship with an
  `engage_delay` grace flies its patrol and holds fire until it is shot at or
  the grace ends.
- Gamepad bindings rounded out: ORBIT -> South, scenario-advance confirm -> DPadDown, and HUD cycle / pause / back-to-editor gained buttons.

### Web & Platform

- The site grew a full wiki (gameplay, ship sections, keybinds, world and meta pages), two new devlogs, and a tutorial trimmed to first-scenario onboarding.

### Fixes

- Thruster hum now attenuates with distance per ship, so another ship's or torpedo's burn no longer plays at full volume from anywhere (your own ship stays exempt).
- Scenario teardown no longer logs an "Entity despawned" command error when a
  ship despawns with autopilot engaged; pinned by regression tests.
- Debug inspector's window-camera placement fix moved upstream into bevy-common-systems (rev 4a743b2), deleting nova's local workaround.

### Internals & Tooling

- CI's example smoke suite is BLOCKING again: the GitHub-runner-only taffy panic is gone, so the 12-example suite gates every push.
- Examples are a testable curriculum: twelve numbered examples from controller
  PD through the boot flow, each self-driving with behavior assertions, all on
  the CI smoke list.
- Example smoke now fails on ANY logged command error, closing the gap where handled remove/despawn warnings (the stale-entity teardown race) sailed past the panic gate.
- Weapon test ranges fire again: the weapons safety had silently disarmed both, so their scripts raise the stance first, with new assertions pinning the fire -> hit chain.

## [0.5.1] - 2026-07-13

### Fixes

- Web build no longer quits with a fatal render validation error on New Game / editor Play: the target inset dropped its `view_formats` override (unsupported on WebGL2) for a plain sRGB target.
- Skybox cubemap's `.meta` loader settings now apply in the shipped app:
  `AssetMetaCheck::Never` had silently ignored them, resurrecting the
  oversized-upload race.

## [0.5.0] - 2026-07-13

### Gameplay & Flight

- Diegetic flight readouts replace the bottom-left status text: speed and engaged-mode chips beside the velocity sphere, an ORBIT radius spoke, and a nav-cyan tint while the autopilot flies.
- Keybind cluster is contextual: rows appear only while their verb can act; scenario-emphasized keys show early, pulsing gold.

### Combat & Weapons

- Deliberate radar locking replaces all passive targeting: hold CTRL to sweep
  and live-lock what you look at; locks stick until the target dies, leaves
  range or goes cold.
- Typed damage (Kinetic / AP / EMP / Explosive) against per-section resistance tables; each turret carries a loaded-ammo slot setting its rounds' type, with a color-coded ammo readout.
- Lock language is slot-colored: RED bracket = combat lock, WHITE = nav lock; relation tint and reticle pips retired, turrets hold the combat lock even during manual aim.
- Turret rounds curve through gravity wells, like ships and torpedoes.
- PDC retuned to point defense: per-hit damage 20 -> 4, so the stream chips targets down over a visible burst instead of one-shotting them.

### Ships & Sections

- Editor: placed sections show their bound key as a chip and can be clicked to rebind (keys or mouse buttons); the build panel scrolls.
- Editor play-test ship is now a passive target instead of an AI combatant.

### Scenarios & Objectives

- Shakedown Run: New Game starts a ~12-beat tutorial (burn, freelook, stop,
  salvage, GOTO, gravity coast, ORBIT, radar lock, live-fire rehearsal,
  scavenger fight).
- Objective conveyance: gold marker chip with live distance to the current target, salvage-crate glow and brackets, keybind emphasis pulses, completion chime and posting blip.
- Scenario primitives: nav beacons and salvage crates with authorable radar
  signatures, despawn-by-id, and new events and actions (OnOrbit, lock edges,
  SetSpeedCap, markers).

### Interface & HUD

- Target viewfinder: a corner inset renders a live magnified 3D view of the
  combat lock, with a red armed frame while hot and a ~2 s freeze-frame kill
  cam on death.
- Main menu (live ambient scene: an AI ship flying a thruster-driven orbit) and ESC pause menu; the game now boots into the menu (new `nova_menu` crate).
- HUD visibility levels: grave/tilde cycles ALL -> MINIMAL -> NONE.

### Web & Platform

- Web landing site (`web/`: TypeScript + Webpack + Tailwind); the Pages deploy serves it at the root with the game under `/play/`.

### Fixes

- Debug builds no longer crash when a scenario advances (the smoke assertion now covers only the boot load), and teardown no longer warns about despawned entities.
- Asteroid kills emit OnDestroyed under the scenario id so scripts can react (this had soft-locked the derelict beat).
- Target inset zooms combat bodies only (ships, torpedoes, asteroids), not beacons, and frames section-less bodies by collider bounds.
- Debug inspector stays on a window camera instead of leaking into render-to-texture cameras.
- HUD apparent-size elements (reticles, brackets) measure real colliders, not invisible trigger volumes.
- Turret bullets are sensor projectiles: same damage, no physical shove, expended on first contact.
- Many Shakedown playtest fixes: beacon trigger park points, orbit-hold
  completion, scavenger spawn timing and leash, invulnerable planetoids,
  objective text and sound pacing.
- F1 back-to-editor is sandbox-only; the debug ammo readout no longer lingers when debug mode is off.

## [0.4.1] - 2026-07-10

### Internals & Tooling

- release-flow: install the `x86_64-apple-darwin` std for the pinned nightly, fixing the macOS universal build.
- CI: one `--features debug` feature set across clippy/tests/examples (one Bevy build), cache saved on failure, windowed examples smoke as a separate non-blocking step.

## [0.4.0] - 2026-07-10

### Gameplay & Flight

- Flight-assist overhaul: assisted velocity-hold mode (WASDQE nudges, X brake latch, soft cap), Z direct Newtonian mode, RCS budget, FA/speed readout.
- Flight computer balances thrust through the live center of mass: differential throttle nulls burn torque, off-axis thrusters recruited for counter-torque.
- Mass-legible handling: turn rate derived from the torque budget and live inertia (stripped ships snap, heavy builds lumber); max_torque 100 -> 40.
- Chase camera: smoothing on all gameplay modes plus a burn push-back lean.

### Combat & Weapons

- Torpedo guidance: proportional navigation, angular lock-on aim-assist, arming gate, blast-radius visual, launch particle burst.
- Player targeting arc: signature auto-acquisition (550 m), focus dwell, per-section fine-lock with aim-snap and cycling, HUD lock markers and focus meter.
- Turret auto-aim with true intercept lead, fed by fine-locked section, else live structure, else camera ray.
- Faction/relation model (hostile/neutral/own) drives acquisition, projectile allegiance, and reticle tint.
- AI combat wave: behavior state machine (Idle/Patrol/Engage/Evade/Retreat),
  fire discipline, point-defense priority, standoff orbit/strafe, patrol
  routes, threat memory.
- Player lock range 2 km -> 20 km (AI sensor scan unchanged).

### Interface & HUD

- HUD substrate: screen-projected indicators (entity/point anchors, apparent-size sizing, clamp-to-edge arrows), turret lead pips, and a locked-target readout (range, closing speed, health).

### Audio & Visuals

- First audio: placeholder SFX (explosions, impacts, turret fire, torpedo launch, throttle-tracking thruster loop) with distance attenuation and throttling.
- Combat juice: trauma-model camera shake and expanding hit/impact flash rings, distance-attenuated and throttled.
- The SFX/juice listener is an explicit marker on the gameplay camera, not "any Camera3d".

### Fixes

- Skybox cubemap reinterpreted into a 6-layer array at load time; the raw 24576 px image exceeded smaller GPUs' texture limit and killed the app.
- Blast damage reaches every body overlapping the blast, not just one.
- Ships, asteroids and torpedoes interpolate between physics ticks (no camera twitch); the chase camera anchors on the live center of mass.
- Projectiles no longer collide with their shooter; shot-down torpedoes die whole and blast-free; destroyed asteroids no longer leave rigid-body husks.
- Section overkill is absorbed instead of propagated; a disabled controller stops torquing the hull.
- Bullet lead solved in the shooter's frame so a moving shooter's rounds land; the AI helm writes slewed absolute rotation commands.
- One hit plays one cue: audio/juice observers ignore propagation re-entry.
- Misc: editor preview controller made inert, turret resting position, one-frame origin snap on camera-mode switch.

### Internals & Tooling

- Example test ranges (`06_torpedo_range`, `08_turret_range`, `10_gameplay`,
  `11_com_range`) with live tuning sliders and a headless autopilot +
  screenshot smoke harness.
- CI workflow: fmt, clippy, and the workspace test suite (windowed examples under Xvfb) on every PR and push to master.
- Integrity, health, blast and mesh-slicer systems consumed from `bevy-common-systems` instead of in-tree copies; torpedo section split into its own module.

## [0.3.1] - 2026-07-07

### Audio & Visuals

- Post-processing camera component.

### Internals & Tooling

- **(breaking)** Upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10); anything built against nova, including from-source mods, must move to the matching versions.
- `bevy_common_systems` externalized as a git dependency; vendored copy removed.
- Added `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, migration notes).

## [0.3.0] - 2025-11-29

### Combat & Weapons

- Torpedo bay section and blast damage.

### Ships & Sections

- Per-section health system.

### Scenarios & Objectives

- OnEnter/OnExit zone events.

### Audio & Visuals

- Improved directional and thruster shaders.

## [0.2.1] - 2025-11-15

### Modding & Mod Portal

- Modding documentation and examples; event system refactor.

## [0.2.0] - 2025-11-08

### Modding & Mod Portal

- Game events and a queue system; scenario and modding capabilities.

### Scenarios & Objectives

- Asteroids with procedural mesh and dynamic destruction.

## [0.1.0] - 2025-10-21

### Ships & Sections

- Basic spaceship sections.

### Scenarios & Objectives

- Editor and simulation scenes.

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.11.0...HEAD
[0.11.0]: https://github.com/alexjercan/nova-protocol/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/alexjercan/nova-protocol/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/alexjercan/nova-protocol/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/alexjercan/nova-protocol/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/alexjercan/nova-protocol/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/alexjercan/nova-protocol/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/alexjercan/nova-protocol/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/alexjercan/nova-protocol/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/alexjercan/nova-protocol/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/alexjercan/nova-protocol/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/alexjercan/nova-protocol/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/alexjercan/nova-protocol/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/alexjercan/nova-protocol/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/alexjercan/nova-protocol/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/alexjercan/nova-protocol/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/alexjercan/nova-protocol/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/alexjercan/nova-protocol/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alexjercan/nova-protocol/releases/tag/v0.1.0
